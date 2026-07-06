//! 内容优化服务：分析→用户选择→局部补丁工作流
//!
//! 对应文档 15-内容优化系统

mod agent_loop;
mod agent_loop_executor;
mod analysis;
mod chat;
mod chat_loop_executor;
mod context;
mod messages;
mod patch;
mod patch_apply;
mod patch_sets;
mod permissions;
mod runtime_tools;
mod session_events;
mod session_names;
mod status;
mod streaming;
mod turns;

use chrono::Utc;
use sea_orm::*;
use std::sync::Arc;

use a4n_core::config::app_config::llm_token_profiles;
use a4n_core::error::AppError;
use a4n_core::models::ai::{optimization_session, optimization_session_message};
use a4n_core::models::chapter::chapter;
use a4n_engine::llm::provider::{ChatRequest, LlmProvider, LlmTraceContext};
use a4n_engine::prompt::engine::PromptEngine;
use serde_json::json;

use crate::generation::context_aware_chat_request_trace_json;
use crate::llm_checkpoint::{
    CompletedLlmCallCheckpointInput, llm_call_checkpoint_fingerprint_sha256,
    load_completed_llm_call_checkpoint, required_llm_call_checkpoint_json,
    save_completed_llm_call_checkpoint,
};
use crate::llm_context::{
    COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedger,
    LlmContextLedgerPackDraft, LlmContextManifest, context_aware_system_user_request,
};
use crate::source::source_content_sha256;
use crate::workflow::WorkflowRunService;
use context::{build_content_optimization_context, optimization_context_manifest};
use messages::{build_content_optimization_patch_user_message, select_optimization_issues};
use patch::parse_content_optimization_patch_response;
pub use patch::{
    OptimizationPatch, OptimizationPatchApplyResult, OptimizationPatchProposal,
    apply_optimization_patches_to_content, optimization_content_hash,
};
pub use permissions::OptimizeAgentPermissions;
use permissions::insert_optimization_agent_capability_variables;
use runtime_tools::{EDIT_ARTICLE_TOOL, optimization_runtime_tool_specs};
use session_names::required_available_session_name;
use status::is_cancelled_optimize_error;
pub use streaming::{OptimizeLlmStream, OptimizeProgress};
use streaming::{collect_streaming_chat_response, report_optimize_progress};
use turns::{
    EVENT_KIND_ASSISTANT_UPDATE, EVENT_KIND_ERROR, EVENT_KIND_TOOL_RESULT, EVENT_STATUS_COMPLETED,
    EVENT_STATUS_FAILED, TURN_STATUS_FAILED,
};

const OPTIMIZATION_CHAT_HISTORY_LIMIT: usize = 12;
const OPTIMIZATION_CHAT_CONTENT_LIMIT: usize = 6000;
const OPTIMIZATION_PATCH_OLD_TEXT_MAX_CHARS: usize = 2000;
const OPTIMIZATION_PATCH_NEW_TEXT_MAX_CHARS: usize = 4000;
const OPTIMIZATION_PATCH_MAX_COUNT: usize = 20;

pub struct OptimizeSendUserMessageInput<'a, 'progress, 'stream> {
    pub session_id: i64,
    pub message: &'a str,
    pub permissions_override: Option<&'a OptimizeAgentPermissions>,
    pub llm: &'a dyn LlmProvider,
    pub prompt_engine: &'a dyn PromptEngine,
    pub workflow_run_id: i64,
    pub progress: Option<OptimizeProgress<'progress>>,
    pub stream: Option<OptimizeLlmStream<'stream>>,
}

pub struct OptimizeProposePatchesInput<'a, 'progress, 'stream> {
    pub session_id: i64,
    pub issue_ids: Option<Vec<String>>,
    pub llm: &'a dyn LlmProvider,
    pub prompt_engine: &'a dyn PromptEngine,
    pub agent_instruction_override: Option<&'a str>,
    pub workflow_run_id: i64,
    pub progress: Option<OptimizeProgress<'progress>>,
    pub stream: Option<OptimizeLlmStream<'stream>>,
}

#[derive(Clone)]
pub struct OptimizeService {
    db: Arc<DatabaseConnection>,
}

struct PatchRequestBase {
    session: optimization_session::Model,
    chapter: chapter::Model,
    override_instruction: Option<String>,
}

struct PatchPromptBundle {
    request: ChatRequest,
    user_message: String,
    base_content_hash: String,
    issue_count: usize,
    permissions: OptimizeAgentPermissions,
}

fn build_content_optimization_patch_request(
    project_id: i64,
    session_id: i64,
    system_prompt: String,
    user_message: String,
    chapter_content: &str,
    project_context: &serde_json::Value,
) -> Result<ChatRequest, AppError> {
    let manifest = build_content_optimization_patch_context_manifest(
        chapter_content,
        project_context,
        user_message.clone(),
    )?;
    Ok(context_aware_system_user_request(
        system_prompt,
        manifest,
        Some(0.2),
        None,
        Some(llm_token_profiles::STANDARD.into()),
    )?
    .with_json_response_format()
    .with_trace_context(LlmTraceContext {
        project_id: Some(project_id),
        root_project_id: None,
        workflow: "content_optimization_patch".to_string(),
        step_key: format!("session_{session_id}_patch"),
        generation_run_id: None,
        workflow_run_id: None,
        call_key: Some(format!("session_{session_id}_patch")),
        input_fingerprint_sha256: None,
        context_plan_json: None,
        context_pack_refs_json: None,
    }))
}

fn optimize_llm_call_input_fingerprint_json(
    request: &ChatRequest,
) -> Result<serde_json::Value, AppError> {
    context_aware_chat_request_trace_json(request)
}

fn response_content_from_checkpoint(
    checkpoint_json: &serde_json::Value,
    call_key: &str,
) -> Result<(String, Option<serde_json::Value>, Option<String>), AppError> {
    let response_content = checkpoint_json
        .get("response_content")
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_response_content".to_string(),
            operation: format!("读取内容优化 LLM 调用断点 {call_key}"),
        })?;
    let usage = checkpoint_json.get("usage").cloned();
    let finish_reason = checkpoint_json
        .get("finish_reason")
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned);
    Ok((response_content, usage, finish_reason))
}

fn build_content_optimization_patch_context_manifest(
    chapter_content: &str,
    project_context: &serde_json::Value,
    dynamic_user_content: String,
) -> Result<LlmContextManifest, AppError> {
    let chapter_content = chapter_content.trim();
    if chapter_content.is_empty() {
        return Err(AppError::InvalidArgument(
            "内容优化补丁缺少章节正文".to_string(),
        ));
    }
    let mut ledger = LlmContextLedger::new();
    let content_hash = source_content_sha256(chapter_content);
    ledger.add_pack(crate::llm_context::LlmContextLedgerPackInput {
        scope_kind: ("chapter").into(),
        pack_kind: ("optimization_patch_chapter_content").into(),
        source_refs_json: Some(json!({
            "source": "chapter_content",
            "content_sha256": content_hash,
        })),
        config_identity_json: json!({
            "ledger_version": "optimization_patch_context_v1",
            "kind": "chapter_content",
        }),
        required: true,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FULL_RAW,
                json!({
                    "kind": "chapter_content",
                    "content": chapter_content,
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FOCUSED_RAW,
                json!({
                    "kind": "chapter_content",
                    "content": chapter_content,
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_MINIMAL,
                json!({
                    "kind": "chapter_content",
                    "content": chapter_content,
                }),
            ),
        ],
    })?;

    let context_hash = source_content_sha256(&serde_json::to_string(project_context)?);
    ledger.add_pack(crate::llm_context::LlmContextLedgerPackInput {
        scope_kind: ("chapter").into(),
        pack_kind: ("optimization_patch_project_context").into(),
        source_refs_json: Some(json!({
            "source": "content_optimization_context",
            "content_sha256": context_hash,
        })),
        config_identity_json: json!({
            "ledger_version": "optimization_patch_context_v1",
            "kind": "project_context",
        }),
        required: true,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FULL_RAW,
                json!({
                    "kind": "project_context",
                    "context": project_context,
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FOCUSED_RAW,
                json!({
                    "kind": "project_context",
                    "context": project_context,
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_MINIMAL,
                json!({
                    "kind": "project_context_manifest",
                    "manifest": optimization_context_manifest(project_context),
                }),
            ),
        ],
    })?;

    Ok(ledger.into_manifest(
        dynamic_user_content,
        Some(
            "内容优化补丁必须返回严格 JSON 对象；只允许 summary 和 patches，patches 只能包含局部 replace/delete/insert_before/insert_after 补丁。"
                .to_string(),
        ),
    ))
}

impl OptimizeService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 提供给提示词的运行时工具清单（OpenAI function calling 格式）。
    fn tool_specs(&self) -> Vec<serde_json::Value> {
        optimization_runtime_tool_specs()
    }

    /// 列出项目的优化会话
    pub async fn list_sessions(
        &self,
        project_id: i64,
        chapter_id: Option<i64>,
    ) -> Result<Vec<optimization_session::Model>, AppError> {
        let mut query = optimization_session::Entity::find()
            .filter(optimization_session::Column::ProjectId.eq(project_id));
        if let Some(chapter_id) = chapter_id {
            query = query.filter(optimization_session::Column::ChapterId.eq(chapter_id));
        }
        query
            .order_by_desc(optimization_session::Column::UpdatedAt)
            .order_by_desc(optimization_session::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    /// 获取会话及消息
    pub async fn get_session(
        &self,
        session_id: i64,
    ) -> Result<optimization_session::Model, AppError> {
        optimization_session::Entity::find_by_id(session_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound("优化会话不存在".into()))
    }

    /// 获取会话消息
    pub async fn session_messages(
        &self,
        session_id: i64,
    ) -> Result<Vec<optimization_session_message::Model>, AppError> {
        optimization_session_message::Entity::find()
            .filter(optimization_session_message::Column::SessionId.eq(session_id))
            .order_by_asc(optimization_session_message::Column::CreatedAt)
            .order_by_asc(optimization_session_message::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    async fn load_optimize_chapter_for_operation(
        &self,
        session: &optimization_session::Model,
        operation: &str,
        deleted_message: &str,
        empty_content_message: Option<&str>,
    ) -> Result<chapter::Model, AppError> {
        let chapter = chapter::Entity::find_by_id(session.chapter_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("章节不存在: {}", session.chapter_id)))?;
        if chapter.status == "deleted" {
            return Err(AppError::InvalidArgument(deleted_message.to_string()));
        }
        if chapter.status == "generating" {
            return Err(AppError::InvalidState {
                current: chapter.status,
                operation: operation.to_string(),
            });
        }
        if let Some(message) = empty_content_message
            && chapter.content.trim().is_empty()
        {
            return Err(AppError::InvalidArgument(message.to_string()));
        }
        Ok(chapter)
    }

    pub async fn build_propose_patches_preview_request(
        &self,
        session_id: i64,
        issue_ids: Option<Vec<String>>,
        prompt_engine: &dyn PromptEngine,
        agent_instruction_override: Option<&str>,
    ) -> Result<ChatRequest, AppError> {
        let base = self
            .load_patch_request_base(session_id, agent_instruction_override, "预览局部修复补丁")
            .await?;
        Ok(self
            .build_patch_prompt_bundle(&base, issue_ids, prompt_engine)
            .await?
            .request)
    }

    async fn load_patch_request_base(
        &self,
        session_id: i64,
        agent_instruction_override: Option<&str>,
        operation: &str,
    ) -> Result<PatchRequestBase, AppError> {
        let session = self.get_session(session_id).await?;
        if matches!(
            session.status.as_str(),
            "analyzing" | "fixing" | "cancelled"
        ) {
            return Err(AppError::InvalidState {
                current: session.status,
                operation: operation.to_string(),
            });
        }
        let chapter = self
            .load_optimize_chapter_for_operation(
                &session,
                operation,
                "已删除章节不能生成局部补丁",
                Some("章节正文为空，无法生成局部补丁"),
            )
            .await?;
        let override_instruction = agent_instruction_override
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(ToOwned::to_owned);
        Ok(PatchRequestBase {
            session,
            chapter,
            override_instruction,
        })
    }

    async fn build_patch_prompt_bundle(
        &self,
        base: &PatchRequestBase,
        issue_ids: Option<Vec<String>>,
        prompt_engine: &dyn PromptEngine,
    ) -> Result<PatchPromptBundle, AppError> {
        let selected_issues = select_optimization_issues(&base.session, issue_ids)?;
        let permissions = OptimizeAgentPermissions::from_session(&base.session);
        let analysis_context = build_content_optimization_context(
            self.db.as_ref(),
            base.session.project_id,
            base.session.chapter_id,
            &permissions,
        )
        .await?;
        let base_content_hash = optimization_content_hash(&base.chapter.content);
        let mut prompt_vars = std::collections::HashMap::new();
        prompt_vars.insert("agent_permissions".to_string(), permissions.as_value());
        prompt_vars.insert(
            "agent_permissions_prompt".to_string(),
            serde_json::Value::String(serde_json::to_string_pretty(&permissions.as_value())?),
        );
        insert_optimization_agent_capability_variables(&mut prompt_vars, &permissions)?;
        prompt_vars.insert(
            "agent_instruction".to_string(),
            base.override_instruction
                .as_deref()
                .unwrap_or("无额外用户指令")
                .to_string()
                .into(),
        );
        let system_prompt = prompt_engine.render(
            "content_optimization_patch",
            &prompt_vars,
            Some(base.session.project_id),
        )?;
        let user_message = build_content_optimization_patch_user_message(
            &base_content_hash,
            &selected_issues,
            &permissions,
            base.override_instruction.as_deref(),
        )?;
        let request = build_content_optimization_patch_request(
            base.session.project_id,
            base.session.id,
            system_prompt,
            user_message.clone(),
            &base.chapter.content,
            &analysis_context,
        )?;
        Ok(PatchPromptBundle {
            request,
            user_message,
            base_content_hash,
            issue_count: selected_issues.len(),
            permissions,
        })
    }

    pub async fn propose_patches(
        &self,
        input: OptimizeProposePatchesInput<'_, '_, '_>,
    ) -> Result<OptimizationPatchProposal, AppError> {
        let OptimizeProposePatchesInput {
            session_id,
            issue_ids,
            llm,
            prompt_engine,
            agent_instruction_override,
            workflow_run_id,
            mut progress,
            mut stream,
        } = input;
        report_optimize_progress(&mut progress, 0, 4, "正在读取章节与问题上下文")?;
        let base = self
            .load_patch_request_base(session_id, agent_instruction_override, "生成局部修复补丁")
            .await?;
        if let Some(instruction) = &base.override_instruction {
            self.save_message(base.session.id, "user", instruction)
                .await?;
        }
        report_optimize_progress(&mut progress, 1, 4, "正在渲染局部补丁提示词")?;
        let PatchPromptBundle {
            mut request,
            user_message,
            base_content_hash,
            issue_count,
            permissions,
        } = self
            .build_patch_prompt_bundle(&base, issue_ids, prompt_engine)
            .await?;
        let turn_text = base
            .override_instruction
            .as_deref()
            .unwrap_or("生成局部修复补丁");
        let turn = self
            .create_turn(base.session.id, turn_text, &permissions)
            .await?;
        let session = base.session;
        let chapter = base.chapter;
        self.save_agent_event(
            session.id,
            &format!(
                "准备生成局部修复补丁，覆盖 {} 个问题。补丁只会生成提案，不会自动写回正文。",
                issue_count
            ),
        )
        .await?;

        self.save_message(session.id, "user", &user_message).await?;
        self.save_agent_event(session.id, "调用 LLM 生成局部修复补丁。")
            .await?;
        self.append_turn_event(
            turn.id,
            EVENT_KIND_ASSISTANT_UPDATE,
            EVENT_STATUS_COMPLETED,
            "根据当前问题列表生成局部正文补丁，补丁只会进入审阅状态。",
            Some(json!({
                "issue_count": issue_count,
            })),
        )
        .await?;

        report_optimize_progress(&mut progress, 2, 4, "正在调用模型生成局部补丁")?;
        let call_key = format!("optimization_session:{}:patch", session.id);
        if let Some(trace_context) = request.trace_context.as_mut() {
            trace_context.workflow_run_id = Some(workflow_run_id);
            trace_context.call_key = Some(call_key.clone());
            trace_context.input_fingerprint_sha256 = None;
        }
        let input_fingerprint_json = optimize_llm_call_input_fingerprint_json(&request)?;
        let input_fingerprint_sha256 =
            llm_call_checkpoint_fingerprint_sha256(&input_fingerprint_json)?;
        if let Some(trace_context) = request.trace_context.as_mut() {
            trace_context.input_fingerprint_sha256 = Some(input_fingerprint_sha256);
        }
        let llm_request_payload =
            serde_json::to_string_pretty(&context_aware_chat_request_trace_json(&request)?)?;
        self.record_llm_request(session.id, "局部补丁生成", &llm_request_payload)
            .await?;
        WorkflowRunService::new(self.db.clone())
            .mark_current_step(workflow_run_id, &call_key)
            .await?;

        let (response, proposal, restored) = if let Some(checkpoint) =
            load_completed_llm_call_checkpoint(
                self.db.clone(),
                workflow_run_id,
                &call_key,
                &input_fingerprint_json,
            )
            .await?
        {
            let checkpoint_json = required_llm_call_checkpoint_json(checkpoint, &call_key)?;
            let (response_content, usage, finish_reason) =
                response_content_from_checkpoint(&checkpoint_json, &call_key)?;
            let proposal = match parse_content_optimization_patch_response(
                session.id,
                chapter.project_id,
                chapter.id,
                &base_content_hash,
                &chapter.content,
                &response_content,
            ) {
                Ok(proposal) => proposal,
                Err(err) => {
                    self.append_turn_event(
                        turn.id,
                        EVENT_KIND_ERROR,
                        EVENT_STATUS_FAILED,
                        &format!("局部补丁校验失败：{err}"),
                        None,
                    )
                    .await?;
                    self.fail_turn(turn.id, TURN_STATUS_FAILED, &err.to_string())
                        .await?;
                    return Err(err);
                }
            };
            let parsed_output = checkpoint_json
                .get("parsed_output")
                .cloned()
                .ok_or_else(|| AppError::InvalidState {
                    current: "missing_parsed_output".to_string(),
                    operation: format!("读取内容优化 LLM 调用断点 {call_key}"),
                })?;
            let reparsed_output = serde_json::to_value(&proposal)?;
            if parsed_output != reparsed_output {
                return Err(AppError::InvalidState {
                    current: "checkpoint_parsed_output_mismatch".to_string(),
                    operation: format!("恢复内容优化 LLM 调用断点 {call_key}"),
                });
            }
            self.save_agent_event(session.id, "已恢复局部补丁 LLM 调用断点。")
                .await?;
            (
                a4n_engine::llm::provider::ChatResponse {
                    message: a4n_engine::llm::provider::ChatMessage::assistant(response_content),
                    usage: usage
                        .map(serde_json::from_value)
                        .transpose()
                        .map_err(|err| AppError::InvalidState {
                            current: "invalid_checkpoint_usage".to_string(),
                            operation: format!("恢复内容优化 LLM 调用断点 {call_key}: {err}"),
                        })?,
                    finish_reason,
                },
                proposal,
                true,
            )
        } else {
            let response = match collect_streaming_chat_response(llm, request, &mut stream).await {
                Ok(response) => response,
                Err(err) => {
                    self.append_turn_event(
                        turn.id,
                        EVENT_KIND_ERROR,
                        EVENT_STATUS_FAILED,
                        &format!("局部补丁模型调用失败：{err}"),
                        None,
                    )
                    .await?;
                    self.fail_turn(turn.id, TURN_STATUS_FAILED, &err.to_string())
                        .await?;
                    self.save_agent_event(session.id, &format!("LLM 局部补丁调用失败：{err}"))
                        .await?;
                    return Err(err);
                }
            };
            let proposal = match parse_content_optimization_patch_response(
                session.id,
                chapter.project_id,
                chapter.id,
                &base_content_hash,
                &chapter.content,
                &response.message.content,
            ) {
                Ok(proposal) => proposal,
                Err(err) => {
                    self.append_turn_event(
                        turn.id,
                        EVENT_KIND_ERROR,
                        EVENT_STATUS_FAILED,
                        &format!("局部补丁校验失败：{err}"),
                        None,
                    )
                    .await?;
                    self.fail_turn(turn.id, TURN_STATUS_FAILED, &err.to_string())
                        .await?;
                    return Err(err);
                }
            };
            (response, proposal, false)
        };

        self.record_llm_response(session.id, "局部补丁生成", &response.message.content)
            .await?;
        self.save_agent_event(session.id, "LLM 已返回局部补丁，正在保存并校验。")
            .await?;
        self.save_message(session.id, "assistant", &response.message.content)
            .await?;
        report_optimize_progress(&mut progress, 3, 4, "模型已返回，正在校验补丁")?;
        if !restored {
            let usage = response
                .usage
                .as_ref()
                .map(serde_json::to_value)
                .transpose()?;
            save_completed_llm_call_checkpoint(
                self.db.clone(),
                CompletedLlmCallCheckpointInput {
                    workflow_run_id,
                    call_key: call_key.clone(),
                    call_order: 1,
                    input_fingerprint_json,
                    checkpoint_json: json!({
                        "output": proposal.clone(),
                        "parsed_output": proposal.clone(),
                        "response_content": response.message.content,
                        "usage": usage,
                        "finish_reason": response.finish_reason,
                    }),
                    artifacts_json: None,
                },
            )
            .await?;
        }
        let patch_set = self.persist_patch_proposal(turn.id, &proposal).await?;
        self.save_message_with_tool_calls(
            session.id,
            "tool",
            &serde_json::to_string_pretty(&json!({
                "tool": EDIT_ARTICLE_TOOL,
                "mode": "pending_confirmation",
                "requires_user_confirmation": true,
                "chapter_id": chapter.id,
                "base_content_hash": base_content_hash,
                "patch_count": proposal.patches.len(),
                "summary": &proposal.summary,
                "patches": &proposal.patches,
            }))?,
            Some(json!({
                "tool": EDIT_ARTICLE_TOOL,
                "chapter_id": chapter.id,
            })),
        )
        .await?;
        if let Some(patch_set) = patch_set {
            self.append_turn_event(
                turn.id,
                EVENT_KIND_TOOL_RESULT,
                EVENT_STATUS_COMPLETED,
                &format!("已生成 {} 个局部补丁，等待审阅。", proposal.patches.len()),
                Some(json!({
                    "tool": EDIT_ARTICLE_TOOL,
                    "patch_set_id": patch_set.id,
                    "patch_count": proposal.patches.len(),
                    "summary": &proposal.summary,
                })),
            )
            .await?;
        } else {
            self.append_turn_event(
                turn.id,
                EVENT_KIND_ASSISTANT_UPDATE,
                EVENT_STATUS_COMPLETED,
                "没有生成可安全唯一定位的局部补丁。",
                Some(json!({
                    "tool": EDIT_ARTICLE_TOOL,
                    "summary": &proposal.summary,
                })),
            )
            .await?;
        }

        let mut active: optimization_session::ActiveModel = session.into();
        active.summary = Set(Some(format!(
            "已生成 {} 个局部补丁提案，等待用户审阅后应用",
            proposal.patches.len()
        )));
        active.updated_at = Set(Utc::now().naive_utc());
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        self.save_agent_event(
            session_id,
            &format!("局部补丁校验通过，共 {} 个。", proposal.patches.len()),
        )
        .await?;
        report_optimize_progress(&mut progress, 4, 4, "局部补丁已生成")?;
        self.finish_turn(
            turn.id,
            &format!(
                "已生成 {} 个局部补丁提案，等待你在正文 diff 中审阅。",
                proposal.patches.len()
            ),
        )
        .await?;

        Ok(proposal)
    }
}

#[cfg(test)]
mod tests;
