use a4n_api_contract::{
    generation_tasks::{GenerationTaskHandle, GenerationTaskStartError},
    optimize::{
        AnalyzeChapterInput, ApplyOptimizationPatchesInput, ApplyOptimizationPatchesResponse,
        CreateOptimizationChatSessionInput, ListOptimizeSessionsInput, OptimizationPatchSetSummary,
        OptimizationSessionSummary, OptimizationTurnSummary, OptimizeSessionInput,
        PrepareOptimizationAnalysisSessionInput, RecordOptimizeAgentEventInput,
        SendOptimizationMessageInput, StartOptimizeAnalysisGenerationInput,
        StartOptimizeChatGenerationInput, StartOptimizePatchGenerationInput,
    },
};
use a4n_db::optimize::OptimizationChapterPatchWriteInput;
use a4n_db::transaction::run_in_transaction;
use a4n_domain::project::{
    OptimizationPatchRecord, OptimizationPatchSetRecord, OptimizationSessionRecord,
    OptimizationTurnEventRecord, OptimizationTurnRecord,
};
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatRequest;
use a4n_optimize_host_contract::{
    OptimizeAgentToolApplicationPort, OptimizeAnalysisGenerationApplicationPort,
    OptimizeAnalysisPreviewApplicationPort, OptimizeChatGenerationApplicationPort,
    OptimizeChatPreviewApplicationPort, OptimizePatchApplicationPort, OptimizePatchApplyHostPort,
    OptimizePatchGenerationApplicationPort, OptimizePatchPreviewApplicationPort,
    OptimizeSessionMutationApplicationPort,
};
use a4n_prompt::rendering::PromptRenderer;
use async_trait::async_trait;
use std::collections::HashMap;

use crate::OrchestratorContext;
use crate::{GenerationTaskAccessError, GenerationTaskStart};
use a4n_domain::collaboration::ProjectAccessRole;

use super::{
    OptimizationAgentPermissions, OptimizationPatch, OptimizeChatGenerationApplicationContext,
    OptimizePatchGenerationApplicationContext, apply_optimization_patches_to_content,
    optimization_agent_variables, optimization_analysis_variables, optimization_content_hash,
};

pub struct PrepareOptimizeAnalysisApplicationInput<'a> {
    pub user_id: i64,
    pub session_id: i64,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub variables: &'a HashMap<String, serde_json::Value>,
}

pub struct ApplyOptimizePatchesApplicationInput<'a> {
    pub user_id: i64,
    pub session_id: i64,
    pub base_content_hash: &'a str,
    pub patches: &'a [OptimizationPatch],
    pub host_port: &'a dyn OptimizePatchApplyHostPort,
}

pub struct ApplyOptimizePatchesApplicationResult {
    pub chapter_id: i64,
    pub applied_count: usize,
    pub word_count: i32,
}

impl OrchestratorContext {
    async fn optimize_chapter_for_user(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<a4n_domain::chapter::ChapterRecord, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        self.chapter_by_project_number(project_id, chapter_number)
            .await?
            .ok_or_else(|| {
                AppError::NotFound(format!(
                    "章节不存在: project_id={project_id}, chapter_number={chapter_number}"
                ))
            })
    }

    pub async fn create_optimize_analysis_session_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_number: i32,
        name: &str,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, serde_json::Value>,
    ) -> Result<OptimizationSessionRecord, AppError> {
        let chapter = self
            .optimize_chapter_for_user(user_id, project_id, chapter_number)
            .await?;
        self.create_analysis_session(
            project_id,
            chapter.id,
            name,
            &chapter.content,
            prompt_renderer,
            variables,
        )
        .await
    }

    pub async fn create_optimize_chat_session_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_number: i32,
        name: &str,
        variables: &HashMap<String, serde_json::Value>,
    ) -> Result<OptimizationSessionRecord, AppError> {
        let chapter = self
            .optimize_chapter_for_user(user_id, project_id, chapter_number)
            .await?;
        self.start_chat_session(project_id, chapter.id, name, variables)
            .await
    }

    pub(crate) async fn validated_optimize_chat_generation_session(
        &self,
        user_id: i64,
        session_id: i64,
        message: &str,
    ) -> Result<OptimizationSessionRecord, AppError> {
        crate::optimize::chat::validation::required_chat_user_text(message)?;
        let session = self.optimize_session_for_user(user_id, session_id).await?;
        crate::optimize::chat::validation::ensure_chat_session_accepts_message(
            &session,
            "发送内容优化 Agent 对话",
        )?;
        self.load_optimize_chapter_for_operation(
            &session,
            "发送内容优化 Agent 对话",
            "已删除章节不能继续内容优化对话",
            None,
        )
        .await?;
        Ok(session)
    }

    pub async fn build_create_optimize_analysis_preview_request_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_number: i32,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, serde_json::Value>,
    ) -> Result<ChatRequest, AppError> {
        let chapter = self
            .optimize_chapter_for_user(user_id, project_id, chapter_number)
            .await?;
        self.build_analysis_preview_request(
            project_id,
            chapter.id,
            &chapter.content,
            prompt_renderer,
            variables,
        )
        .await
    }

    pub async fn build_prepare_optimize_analysis_preview_request_for_user_validated(
        &self,
        user_id: i64,
        session_id: i64,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, serde_json::Value>,
    ) -> Result<ChatRequest, AppError> {
        let session = self.optimize_session_for_user(user_id, session_id).await?;
        let chapter = self
            .optimization_chapter_by_id(session.chapter_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("章节不存在: {}", session.chapter_id)))?;
        self.build_analysis_preview_request(
            session.project_id,
            session.chapter_id,
            &chapter.content,
            prompt_renderer,
            variables,
        )
        .await
    }

    pub async fn build_send_user_message_preview_request_for_user_validated(
        &self,
        user_id: i64,
        session_id: i64,
        message: &str,
        permissions: &OptimizationAgentPermissions,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ChatRequest, AppError> {
        self.optimize_session_for_user(user_id, session_id).await?;
        self.build_send_user_message_preview_request(
            session_id,
            message,
            permissions,
            prompt_renderer,
        )
        .await
    }

    pub async fn build_propose_patches_preview_request_for_user_validated(
        &self,
        user_id: i64,
        session_id: i64,
        issue_ids: Option<Vec<String>>,
        prompt_renderer: &dyn PromptRenderer,
        agent_instruction_override: Option<&str>,
    ) -> Result<ChatRequest, AppError> {
        self.optimize_session_for_user(user_id, session_id).await?;
        self.build_propose_patches_preview_request(
            session_id,
            issue_ids,
            prompt_renderer,
            agent_instruction_override,
        )
        .await
    }

    pub async fn prepare_optimize_analysis_session_validated(
        &self,
        input: PrepareOptimizeAnalysisApplicationInput<'_>,
    ) -> Result<OptimizationSessionRecord, AppError> {
        let session = self
            .optimize_session_for_user(input.user_id, input.session_id)
            .await?;
        let result = async {
            let chapter = self
                .optimization_chapter_by_id(session.chapter_id)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("章节不存在: {}", session.chapter_id)))?;
            self.prepare_analysis_session(
                input.session_id,
                &chapter.content,
                input.prompt_renderer,
                input.variables,
            )
            .await
        }
        .await;
        self.record_optimize_operation_failure(input.session_id, "准备内容优化分析", result)
            .await
    }

    pub async fn apply_optimize_patches_validated(
        &self,
        input: ApplyOptimizePatchesApplicationInput<'_>,
    ) -> Result<ApplyOptimizePatchesApplicationResult, AppError> {
        let session = self
            .optimize_session_for_user(input.user_id, input.session_id)
            .await?;
        let result = async {
            if input.patches.is_empty() {
                return Err(AppError::InvalidArgument(
                    "应用补丁列表不能为空".to_string(),
                ));
            }
            let chapter = self
                .optimization_chapter_by_id(session.chapter_id)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("章节不存在: {}", session.chapter_id)))?;
            if chapter.status == "deleted" {
                return Err(AppError::InvalidArgument(
                    "已删除章节不能应用局部补丁".to_string(),
                ));
            }
            if chapter.status == "generating" {
                return Err(AppError::InvalidState {
                    current: chapter.status,
                    operation: "应用局部补丁".to_string(),
                });
            }
            let next_content = apply_optimization_patches_to_content(
                &chapter.content,
                input.base_content_hash,
                input.patches,
            )?;
            let word_count = i32::try_from(next_content.chars().count()).map_err(|_| {
                AppError::InvalidArgument("应用补丁后的章节正文字符数超出 i32 范围".to_string())
            })?;
            let (written, sync_run) =
                run_in_transaction(self.database.connection(), "应用局部补丁", |tx| {
                    Box::pin(async move {
                        let result = async {
                            let written =
                                a4n_db::optimize::write_optimization_chapter_patch_content(
                                    &tx,
                                    OptimizationChapterPatchWriteInput {
                                        chapter,
                                        content: next_content,
                                        word_count,
                                    },
                                )
                                .await
                                .map_err(super::optimize_db_error)?;
                            let sync_run = a4n_chapter_content_core::begin_chapter_content_sync_in_transaction(
                                &tx, &written,
                            )
                            .await?;
                            let content_hash = optimization_content_hash(&written.content);
                            self.record_patches_applied(
                                &tx,
                                input.session_id,
                                written.id,
                                written.word_count,
                                &content_hash,
                                input.patches,
                            )
                            .await?;
                            self.mark_matching_patch_sets_applied(
                                &tx,
                                input.session_id,
                                input.base_content_hash,
                            )
                            .await?;
                            Ok((written, sync_run))
                        }
                        .await;
                        (tx, result)
                    })
                        as a4n_db::transaction::TransactionWork<
                            '_ ,
                            (
                                a4n_domain::chapter::ChapterRecord,
                                a4n_db::workflow::WorkflowRunSummary,
                            ),
                        >
                })
                .await?;
            input
                .host_port
                .settle_chapter_content_sync(sync_run.id)
                .await
                .map_err(|error| AppError::InvalidState {
                    current: format!("正文补丁已保存，资料同步工作流 {} 未完成", sync_run.id),
                    operation: format!("请继续恢复资料同步：{error}"),
                })?;
            Ok(ApplyOptimizePatchesApplicationResult {
                chapter_id: written.id,
                applied_count: input.patches.len(),
                word_count: written.word_count,
            })
        }
        .await;
        self.record_optimize_operation_failure(input.session_id, "应用局部补丁", result)
            .await
    }

    pub async fn discard_pending_optimize_patch_sets_for_user_validated(
        &self,
        user_id: i64,
        session_id: i64,
    ) -> Result<(), AppError> {
        self.optimize_session_for_user(user_id, session_id).await?;
        self.discard_pending_optimization_patch_sets(session_id)
            .await
    }

    pub async fn discard_optimize_patch_set_for_user_validated(
        &self,
        user_id: i64,
        patch_set_id: i64,
    ) -> Result<(), AppError> {
        let patch_set = a4n_db::optimize::optimization_patch_set_by_id(
            self.database.connection(),
            patch_set_id,
        )
        .await
        .map_err(crate::optimize::optimize_db_error)?
        .ok_or_else(|| AppError::NotFound(format!("优化补丁集不存在: {patch_set_id}")))?;
        self.optimize_session_for_user(user_id, patch_set.session_id)
            .await?;
        self.discard_optimization_patch_set(patch_set.session_id, patch_set.id)
            .await
    }

    async fn mark_matching_patch_sets_applied(
        &self,
        tx: &a4n_db::DbTransaction,
        session_id: i64,
        base_content_hash: &str,
    ) -> Result<(), AppError> {
        let patch_sets = a4n_db::optimize::matching_pending_optimization_patch_sets(
            tx,
            session_id,
            base_content_hash,
            a4n_domain::project::OPTIMIZATION_PATCH_SET_STATUS_PENDING,
        )
        .await
        .map_err(super::optimize_db_error)?;
        let patch_set_ids = patch_sets
            .iter()
            .map(|patch_set| patch_set.id)
            .collect::<Vec<_>>();
        a4n_db::optimize::update_optimization_patch_sets_status(
            tx,
            &patch_set_ids,
            a4n_domain::project::OPTIMIZATION_PATCH_SET_STATUS_APPLIED,
            a4n_domain::project::OPTIMIZATION_PATCH_DECISION_ACCEPTED,
            a4n_domain::project::OPTIMIZATION_PATCH_APPLY_STATUS_APPLIED,
        )
        .await
        .map_err(super::optimize_db_error)?;
        Ok(())
    }

    async fn record_optimize_operation_failure<T>(
        &self,
        session_id: i64,
        phase: &str,
        result: Result<T, AppError>,
    ) -> Result<T, AppError> {
        match result {
            Ok(value) => Ok(value),
            Err(error) => match self
                .record_command_error(
                    session_id,
                    phase,
                    &error.to_string(),
                    matches!(error, AppError::Cancelled),
                )
                .await
            {
                Ok(()) => Err(error),
                Err(record_error) => Err(AppError::InvalidState {
                    current: error.to_string(),
                    operation: format!("{phase}失败，且优化失败事件写入失败: {record_error}"),
                }),
            },
        }
    }
}

#[async_trait]
impl a4n_api_contract::optimize::OptimizeSessionReadApplicationPort for OrchestratorContext {
    async fn list_sessions(
        &self,
        user_id: i64,
        input: ListOptimizeSessionsInput,
    ) -> Result<Vec<OptimizationSessionSummary>, AppError> {
        OrchestratorContext::optimize_sessions_for_user(
            self,
            user_id,
            input.project_id,
            input.chapter_id,
        )
        .await?
        .iter()
        .map(optimization_session_summary)
        .collect()
    }

    async fn get_session(
        &self,
        user_id: i64,
        input: OptimizeSessionInput,
    ) -> Result<OptimizationSessionSummary, AppError> {
        optimization_session_summary(
            &OrchestratorContext::optimize_session_for_user(self, user_id, input.session_id)
                .await?,
        )
    }

    async fn list_turns(
        &self,
        user_id: i64,
        input: OptimizeSessionInput,
    ) -> Result<Vec<OptimizationTurnSummary>, AppError> {
        OrchestratorContext::optimize_turns_for_user(self, user_id, input.session_id)
            .await?
            .into_iter()
            .map(|(turn, events)| optimization_turn_summary(&turn, events))
            .collect()
    }

    async fn list_patch_sets(
        &self,
        user_id: i64,
        input: OptimizeSessionInput,
    ) -> Result<Vec<OptimizationPatchSetSummary>, AppError> {
        OrchestratorContext::optimize_patch_sets_for_user(self, user_id, input.session_id)
            .await?
            .into_iter()
            .map(|(patch_set, patches)| optimization_patch_set_summary(&patch_set, patches))
            .collect()
    }
}

#[async_trait]
impl OptimizeAgentToolApplicationPort for OrchestratorContext {
    async fn inspect_context(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
        input: a4n_api_contract::optimize::InspectOptimizationContextInput,
    ) -> Result<serde_json::Value, AppError> {
        crate::optimize::runtime_tools::inspect_project_context(
            self, user_id, project_id, chapter_id, input,
        )
        .await
    }

    async fn read_context(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
        input: a4n_api_contract::optimize::ReadOptimizationContextInput,
    ) -> Result<serde_json::Value, AppError> {
        crate::optimize::runtime_tools::read_project_context(
            self, user_id, project_id, chapter_id, input,
        )
        .await
    }

    async fn search_context(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
        input: a4n_api_contract::optimize::SearchOptimizationContextInput,
    ) -> Result<serde_json::Value, AppError> {
        crate::optimize::runtime_tools::search_project_context(
            self, user_id, project_id, chapter_id, input,
        )
        .await
    }

    async fn propose_article_patch(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
        agent_turn_id: i64,
        input: a4n_api_contract::optimize::OptimizationArticlePatchToolInput,
    ) -> Result<serde_json::Value, AppError> {
        let existing_session = self
            .agent_optimization_session_for_user(user_id, project_id, chapter_id)
            .await?;
        let session = match existing_session {
            Some(session) => session,
            None => {
                let chapter = self
                    .optimization_chapter_by_id(chapter_id)
                    .await?
                    .ok_or_else(|| AppError::NotFound(format!("章节不存在: {chapter_id}")))?;
                self.create_optimize_chat_session_for_user_validated(
                    user_id,
                    project_id,
                    chapter.chapter_number,
                    "章节内容优化 Agent",
                    &crate::optimize::permissions::optimization_agent_variables(
                        &super::permissions::default_agent_permissions(),
                    ),
                )
                .await?
            }
        };
        let permissions =
            crate::optimize::permissions::optimization_permissions_from_session(&session)?;
        let turn = if let Some(existing) = a4n_db::optimize::optimization_turn_by_agent_turn_id(
            self.database.connection(),
            agent_turn_id,
        )
        .await
        .map_err(crate::optimize::optimize_db_error)?
        {
            if existing.session_id != session.id {
                return Err(AppError::InvalidArgument(
                    "Agent Turn 已绑定到其他内容优化会话".into(),
                ));
            }
            if let Some(patch_set) = a4n_db::optimize::optimization_patch_set_by_turn_id(
                self.database.connection(),
                existing.id,
            )
            .await
            .map_err(crate::optimize::optimize_db_error)?
            {
                return persisted_article_patch_result(self, patch_set).await;
            }
            existing
        } else {
            self.create_turn(
                session.id,
                &format!("Agent turn {agent_turn_id}"),
                &permissions,
                Some(agent_turn_id),
            )
            .await?
        };
        let chapter = self
            .optimization_chapter_by_id(chapter_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("章节不存在: {chapter_id}")))?;
        if chapter.status == "deleted" {
            return Err(AppError::InvalidArgument(
                "已删除章节不能生成正文补丁".into(),
            ));
        }
        if chapter.content.trim().is_empty() {
            return Err(AppError::InvalidArgument(
                "章节正文为空，无法生成正文补丁".into(),
            ));
        }
        let base_content_hash = a4n_domain::source_text_hash(&chapter.content);
        let raw = serde_json::to_string(&serde_json::json!({
            "summary": input.summary,
            "patches": input.patches,
        }))?;
        let proposal = crate::optimize::patch::parse_content_optimization_patch_response(
            session.id,
            project_id,
            chapter_id,
            &base_content_hash,
            &chapter.content,
            &raw,
        )?;
        let mut result = serde_json::json!({
            "tool": "edit_article",
            "mode": "pending_confirmation",
            "requires_user_confirmation": true,
            "chapter_id": chapter_id,
            "session_id": session.id,
            "base_content_hash": proposal.base_content_hash,
            "patch_count": proposal.patches.len(),
            "summary": proposal.summary,
            "patches": proposal.patches,
        });
        if let Some(patch_set) = self
            .persist_patch_tool_result(turn.id, session.id, project_id, chapter_id, &result)
            .await?
        {
            result["patch_set_id"] = serde_json::json!(patch_set.id);
            result["resource"] = serde_json::json!({
                "kind": "optimization_patch_set",
                "id": patch_set.id.to_string(),
                "revision": patch_set.updated_at.and_utc().to_rfc3339(),
                "hash": patch_set.base_content_hash,
            });
        }
        Ok(result)
    }
}

async fn persisted_article_patch_result(
    context: &OrchestratorContext,
    patch_set: OptimizationPatchSetRecord,
) -> Result<serde_json::Value, AppError> {
    let rows = a4n_db::optimize::optimization_patch_set_patches(
        context.database.connection(),
        patch_set.id,
    )
    .await
    .map_err(crate::optimize::optimize_db_error)?;
    let patches = rows
        .into_iter()
        .map(|patch| {
            Ok(serde_json::json!({
                "patch_id": patch.patch_id,
                "issue_id": patch.issue_id,
                "operation": patch.operation,
                "old_text": patch.old_text,
                "new_text": patch.new_text,
                "reason": patch.reason,
                "risk": patch.risk,
            }))
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    let base_content_hash = patch_set.base_content_hash.clone();
    Ok(serde_json::json!({
        "tool": "edit_article",
        "mode": "pending_confirmation",
        "requires_user_confirmation": true,
        "chapter_id": patch_set.chapter_id,
        "session_id": patch_set.session_id,
        "base_content_hash": base_content_hash,
        "patch_count": patches.len(),
        "summary": patch_set.summary,
        "patches": patches,
        "patch_set_id": patch_set.id,
        "resource": {
            "kind": "optimization_patch_set",
            "id": patch_set.id.to_string(),
            "revision": patch_set.updated_at.and_utc().to_rfc3339(),
            "hash": patch_set.base_content_hash,
        },
    }))
}

#[async_trait]
impl OptimizeSessionMutationApplicationPort for OrchestratorContext {
    async fn create_analysis_session(
        &self,
        user_id: i64,
        input: AnalyzeChapterInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<OptimizationSessionSummary, AppError> {
        let variables = optimization_analysis_variables(
            &input.mode,
            input.scope.as_deref(),
            &input.permissions,
            input.agent_instruction.as_deref(),
        );
        let session = OrchestratorContext::create_optimize_analysis_session_for_user_validated(
            self,
            user_id,
            input.project_id,
            input.chapter_number,
            &input.name,
            prompt_renderer,
            &variables,
        )
        .await?;
        optimization_session_summary(&session)
    }

    async fn prepare_analysis_session(
        &self,
        user_id: i64,
        input: PrepareOptimizationAnalysisSessionInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<OptimizationSessionSummary, AppError> {
        let variables = optimization_analysis_variables(
            &input.mode,
            input.scope.as_deref(),
            &input.permissions,
            input.agent_instruction.as_deref(),
        );
        let session = OrchestratorContext::prepare_optimize_analysis_session_validated(
            self,
            PrepareOptimizeAnalysisApplicationInput {
                user_id,
                session_id: input.session_id,
                prompt_renderer,
                variables: &variables,
            },
        )
        .await?;
        optimization_session_summary(&session)
    }

    async fn create_chat_session(
        &self,
        user_id: i64,
        input: CreateOptimizationChatSessionInput,
    ) -> Result<OptimizationSessionSummary, AppError> {
        let variables = optimization_agent_variables(&input.permissions);
        let session = OrchestratorContext::create_optimize_chat_session_for_user_validated(
            self,
            user_id,
            input.project_id,
            input.chapter_number,
            &input.name,
            &variables,
        )
        .await?;
        optimization_session_summary(&session)
    }

    async fn delete_session(
        &self,
        user_id: i64,
        input: OptimizeSessionInput,
    ) -> Result<(), AppError> {
        OrchestratorContext::delete_optimize_session_for_user_validated(
            self,
            user_id,
            input.session_id,
        )
        .await
    }

    async fn record_agent_event(
        &self,
        user_id: i64,
        input: RecordOptimizeAgentEventInput,
    ) -> Result<(), AppError> {
        OrchestratorContext::record_optimize_agent_event_for_user_validated(
            self,
            user_id,
            input.session_id,
            &input.content,
        )
        .await
    }

    async fn discard_pending_patch_sets(
        &self,
        user_id: i64,
        input: OptimizeSessionInput,
    ) -> Result<(), AppError> {
        OrchestratorContext::discard_pending_optimize_patch_sets_for_user_validated(
            self,
            user_id,
            input.session_id,
        )
        .await
    }

    async fn discard_patch_set(
        &self,
        user_id: i64,
        input: a4n_api_contract::optimize::DiscardOptimizationPatchSetInput,
    ) -> Result<(), AppError> {
        OrchestratorContext::discard_optimize_patch_set_for_user_validated(
            self,
            user_id,
            input.patch_set_id,
        )
        .await
    }
}

#[async_trait]
impl OptimizePatchApplicationPort for OrchestratorContext {
    async fn apply_patches(
        &self,
        user_id: i64,
        input: ApplyOptimizationPatchesInput,
        host_port: &dyn OptimizePatchApplyHostPort,
    ) -> Result<ApplyOptimizationPatchesResponse, AppError> {
        let result = OrchestratorContext::apply_optimize_patches_validated(
            self,
            ApplyOptimizePatchesApplicationInput {
                user_id,
                session_id: input.session_id,
                base_content_hash: &input.base_content_hash,
                patches: &input.patches,
                host_port,
            },
        )
        .await?;
        Ok(ApplyOptimizationPatchesResponse {
            chapter_id: result.chapter_id,
            applied_count: result.applied_count,
            word_count: result.word_count,
        })
    }
}

#[async_trait]
impl OptimizeAnalysisPreviewApplicationPort for OrchestratorContext {
    async fn preview_create_analysis(
        &self,
        user_id: i64,
        input: AnalyzeChapterInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ChatRequest, AppError> {
        let variables = optimization_analysis_variables(
            &input.mode,
            input.scope.as_deref(),
            &input.permissions,
            input.agent_instruction.as_deref(),
        );
        OrchestratorContext::build_create_optimize_analysis_preview_request_for_user_validated(
            self,
            user_id,
            input.project_id,
            input.chapter_number,
            prompt_renderer,
            &variables,
        )
        .await
    }

    async fn preview_prepare_analysis(
        &self,
        user_id: i64,
        input: PrepareOptimizationAnalysisSessionInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ChatRequest, AppError> {
        let variables = optimization_analysis_variables(
            &input.mode,
            input.scope.as_deref(),
            &input.permissions,
            input.agent_instruction.as_deref(),
        );
        OrchestratorContext::build_prepare_optimize_analysis_preview_request_for_user_validated(
            self,
            user_id,
            input.session_id,
            prompt_renderer,
            &variables,
        )
        .await
    }
}

#[async_trait]
impl OptimizeChatPreviewApplicationPort for OrchestratorContext {
    async fn preview_send_message(
        &self,
        user_id: i64,
        input: SendOptimizationMessageInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ChatRequest, AppError> {
        OrchestratorContext::build_send_user_message_preview_request_for_user_validated(
            self,
            user_id,
            input.session_id,
            &input.message,
            &input.permissions,
            prompt_renderer,
        )
        .await
    }
}

#[async_trait]
impl OptimizePatchPreviewApplicationPort for OrchestratorContext {
    async fn preview_propose_patches(
        &self,
        user_id: i64,
        input: a4n_api_contract::optimize::ProposeOptimizationPatchesInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ChatRequest, AppError> {
        OrchestratorContext::build_propose_patches_preview_request_for_user_validated(
            self,
            user_id,
            input.session_id,
            input.issue_ids,
            prompt_renderer,
            input.agent_instruction.as_deref(),
        )
        .await
    }
}

#[async_trait]
impl OptimizeChatGenerationApplicationPort for OrchestratorContext {
    async fn start_chat_generation(
        &self,
        user_id: i64,
        input: &StartOptimizeChatGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskStartError> {
        OrchestratorContext::start_optimize_chat_generation_task(self, user_id, input)
            .await
            .map_err(GenerationTaskAccessError::into_start_error)
    }

    async fn run_chat_generation(
        &self,
        prompt_renderer: &dyn PromptRenderer,
        agent_max_output_tokens: Option<usize>,
        user_id: i64,
        input: StartOptimizeChatGenerationInput,
        handle: GenerationTaskHandle,
    ) -> Result<(), String> {
        OrchestratorContext::run_optimize_chat_generation_task(
            self,
            OptimizeChatGenerationApplicationContext {
                prompt_renderer,
                agent_max_output_tokens,
            },
            user_id,
            input,
            handle,
        )
        .await
    }
}

#[async_trait]
impl OptimizeAnalysisGenerationApplicationPort for OrchestratorContext {
    async fn start_analysis_generation(
        &self,
        user_id: i64,
        input: &StartOptimizeAnalysisGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskStartError> {
        OrchestratorContext::start_optimize_analysis_generation_task(self, user_id, input)
            .await
            .map_err(GenerationTaskAccessError::into_start_error)
    }

    async fn run_analysis_generation(
        &self,
        user_id: i64,
        input: StartOptimizeAnalysisGenerationInput,
        handle: GenerationTaskHandle,
    ) -> Result<(), String> {
        OrchestratorContext::run_optimize_analysis_generation_task(self, user_id, input, handle)
            .await
    }
}

#[async_trait]
impl OptimizePatchGenerationApplicationPort for OrchestratorContext {
    async fn start_patch_generation(
        &self,
        user_id: i64,
        input: &StartOptimizePatchGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskStartError> {
        OrchestratorContext::start_optimize_patch_generation_task(self, user_id, input)
            .await
            .map_err(GenerationTaskAccessError::into_start_error)
    }

    async fn run_patch_generation(
        &self,
        prompt_renderer: &dyn PromptRenderer,
        user_id: i64,
        input: StartOptimizePatchGenerationInput,
        handle: GenerationTaskHandle,
    ) -> Result<(), String> {
        OrchestratorContext::run_optimize_patch_generation_task(
            self,
            OptimizePatchGenerationApplicationContext { prompt_renderer },
            user_id,
            input,
            handle,
        )
        .await
    }
}

fn optimization_session_summary(
    model: &OptimizationSessionRecord,
) -> Result<OptimizationSessionSummary, AppError> {
    Ok(OptimizationSessionSummary {
        id: model.id,
        project_id: model.project_id,
        chapter_id: model.chapter_id,
        name: model.name.clone(),
        status: model.status.clone(),
        issues: model.issues.clone(),
        agent_permissions: required_permissions(
            model.agent_permissions.as_ref(),
            "optimization_session.agent_permissions",
        )?,
        agent_instruction: model.agent_instruction.clone(),
        fixes_applied: model.fixes_applied,
        summary: model.summary.clone(),
        created_at: format_utc_timestamp(model.created_at),
        updated_at: format_utc_timestamp(model.updated_at),
    })
}

fn optimization_turn_summary(
    turn: &OptimizationTurnRecord,
    events: Vec<OptimizationTurnEventRecord>,
) -> Result<OptimizationTurnSummary, AppError> {
    Ok(OptimizationTurnSummary {
        id: turn.id,
        session_id: turn.session_id,
        agent_turn_id: turn.agent_turn_id,
        user_text: turn.user_text.clone(),
        status: turn.status.clone(),
        permissions_snapshot: required_permissions(
            turn.permissions_snapshot.as_ref(),
            "optimization_turn.permissions_snapshot",
        )?,
        final_reply: turn.final_reply.clone(),
        error: turn.error.clone(),
        events: events
            .into_iter()
            .map(
                |event| a4n_api_contract::optimize::OptimizationTurnEventSummary {
                    id: event.id,
                    turn_id: event.turn_id,
                    seq: event.seq,
                    kind: event.kind,
                    status: event.status,
                    public_text: event.public_text,
                    payload: event.payload,
                    created_at: format_utc_timestamp(event.created_at),
                },
            )
            .collect(),
        created_at: format_utc_timestamp(turn.created_at),
        updated_at: format_utc_timestamp(turn.updated_at),
    })
}

fn optimization_patch_set_summary(
    patch_set: &OptimizationPatchSetRecord,
    patches: Vec<OptimizationPatchRecord>,
) -> Result<OptimizationPatchSetSummary, AppError> {
    Ok(OptimizationPatchSetSummary {
        id: patch_set.id,
        turn_id: patch_set.turn_id,
        session_id: patch_set.session_id,
        chapter_id: patch_set.chapter_id,
        base_content_hash: patch_set.base_content_hash.clone(),
        status: patch_set.status.clone(),
        summary: patch_set.summary.clone(),
        patches: patches
            .into_iter()
            .map(|patch| {
                Ok(OptimizationPatch {
                    patch_id: patch.patch_id,
                    issue_id: patch.issue_id,
                    operation: parse_patch_operation(&patch.operation)?,
                    old_text: patch.old_text,
                    new_text: patch.new_text,
                    reason: patch.reason,
                    risk: patch.risk,
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?,
        created_at: format_utc_timestamp(patch_set.created_at),
        updated_at: format_utc_timestamp(patch_set.updated_at),
    })
}

fn format_utc_timestamp(value: chrono::NaiveDateTime) -> String {
    value.and_utc().to_rfc3339()
}

fn required_permissions(
    value: Option<&serde_json::Value>,
    field: &str,
) -> Result<OptimizationAgentPermissions, AppError> {
    serde_json::from_value(value.cloned().ok_or_else(|| AppError::InvalidState {
        current: format!("{field}_missing"),
        operation: format!("读取 {field}"),
    })?)
    .map_err(|error| AppError::InvalidState {
        current: error.to_string(),
        operation: format!("读取 {field}"),
    })
}

fn parse_patch_operation(
    value: &str,
) -> Result<a4n_api_contract::optimize::OptimizationPatchOperation, AppError> {
    match value {
        "replace" => Ok(a4n_api_contract::optimize::OptimizationPatchOperation::Replace),
        "delete" => Ok(a4n_api_contract::optimize::OptimizationPatchOperation::Delete),
        "insert_before" => Ok(a4n_api_contract::optimize::OptimizationPatchOperation::InsertBefore),
        "insert_after" => Ok(a4n_api_contract::optimize::OptimizationPatchOperation::InsertAfter),
        _ => Err(AppError::InvalidState {
            current: value.to_string(),
            operation: "读取 optimization_patch.operation".to_string(),
        }),
    }
}

#[cfg(test)]
mod record_boundary_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn turn_summary_rejects_invalid_permissions_and_preserves_event_payload() {
        let now = chrono::Utc::now().naive_utc();
        let mut turn = OptimizationTurnRecord {
            id: 3,
            session_id: 7,
            agent_turn_id: None,
            user_text: "继续优化".to_string(),
            status: "completed".to_string(),
            permissions_snapshot: None,
            final_reply: Some("已完成".to_string()),
            error: None,
            created_at: now,
            updated_at: now,
        };
        let missing = optimization_turn_summary(&turn, Vec::new()).unwrap_err();
        assert!(
            matches!(missing, AppError::InvalidState { current, .. } if current == "optimization_turn.permissions_snapshot_missing")
        );

        turn.permissions_snapshot = Some(json!({"read_project_detail": true}));
        assert!(matches!(
            optimization_turn_summary(&turn, Vec::new()),
            Err(AppError::InvalidState { .. })
        ));

        turn.permissions_snapshot = Some(
            OptimizationAgentPermissions {
                read_project_detail: true,
                read_blueprint: false,
                read_outlines: false,
                read_character_profiles: false,
                read_foreshadowing: false,
                automatic_writeback: false,
            }
            .as_value(),
        );
        let event = OptimizationTurnEventRecord {
            id: 9,
            turn_id: 3,
            seq: 1,
            kind: "assistant_update".to_string(),
            status: "completed".to_string(),
            public_text: "已检查".to_string(),
            payload: Some(json!({"source": "chapter"})),
            created_at: now,
        };
        let summary = optimization_turn_summary(&turn, vec![event]).unwrap();
        let value = serde_json::to_value(summary).unwrap();
        assert_eq!(value["permissions_snapshot"]["read_project_detail"], true);
        assert_eq!(value["events"][0]["payload"]["source"], "chapter");
        assert_eq!(value["events"][0]["created_at"], format_utc_timestamp(now));
    }
}
