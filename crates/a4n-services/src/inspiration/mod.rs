//! 灵感对话服务：流式/非流式AI对话 + 历史记录
//!
//! 对应文档 02-灵感对话系统

use chrono::Utc;
use sea_orm::*;
use std::sync::Arc;

use a4n_core::config::app_config::llm_token_profiles;
use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter;
use a4n_core::models::project::{
    blueprint_character, blueprint_relationship, novel_blueprint, novel_project,
};
use a4n_core::models::user::{conversation, message};
use a4n_engine::llm::provider::{
    ChatRequest, LlmProvider, LlmTraceContext, UsageInfo, is_truncated_finish_reason,
};
use a4n_engine::prompt::engine::PromptEngine;
use futures::StreamExt;
use serde_json::{Value, json};

use crate::blueprint::blueprint_llm_context;
use crate::chapter::analysis::summary_text;
use crate::generation::context_aware_chat_request_trace_json;
use crate::llm_checkpoint::{
    CompletedLlmCallCheckpointInput, llm_call_checkpoint_fingerprint_sha256,
    load_completed_llm_call_checkpoint, required_llm_call_checkpoint_json,
    required_llm_call_workflow_run_id, save_completed_llm_call_checkpoint,
};
use crate::llm_context::{
    COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedger,
    LlmContextLedgerPackDraft, context_aware_system_user_request,
};
use crate::source::source_content_sha256;
use crate::workflow::WorkflowRunService;

#[derive(Clone)]
pub struct InspirationService {
    db: Arc<DatabaseConnection>,
}

pub struct InspirationConverseStreamInput<'a, F>
where
    F: FnMut(&str, u32),
{
    pub project_id: i64,
    pub user_input: &'a str,
    pub llm: &'a dyn LlmProvider,
    pub prompt_engine: &'a dyn PromptEngine,
    pub variables: &'a std::collections::HashMap<String, serde_json::Value>,
    pub workflow_run_id: Option<i64>,
    pub on_chunk: F,
}

struct InspirationCheckpointRequest {
    request: ChatRequest,
    workflow_run_id: i64,
    call_key: String,
    fingerprint_json: Value,
}

impl InspirationService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 获取对话历史消息（按时间正序）
    pub async fn history(
        &self,
        project_id: i64,
        limit: u64,
    ) -> Result<Vec<message::Model>, AppError> {
        // 查找活跃对话
        let conv = self.get_or_create_conversation(project_id).await?;

        message::Entity::find()
            .filter(message::Column::ConversationId.eq(conv.id))
            .order_by_desc(message::Column::CreatedAt)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map(newest_history_in_prompt_order)
            .map_err(AppError::Database)
    }

    /// 保存消息到当前活跃对话
    pub async fn save_message(
        &self,
        project_id: i64,
        role: &str,
        content: &str,
    ) -> Result<message::Model, AppError> {
        let role = normalize_message_role(role)?;
        let content = required_message_content(content)?;
        let conv = self.get_or_create_conversation(project_id).await?;
        let count = conv
            .message_count
            .checked_add(1)
            .ok_or_else(|| AppError::InvalidArgument("对话消息数超出 i32 范围".to_string()))?;
        let now = Utc::now().naive_utc();

        let msg = message::ActiveModel {
            conversation_id: Set(conv.id),
            role: Set(role.to_string()),
            content: Set(content.to_string()),
            tool_calls: Set(None),
            prompt_tokens: Set(None),
            completion_tokens: Set(None),
            chapter_number: Set(None),
            is_important: Set(false),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(self.db.as_ref())
        .await
        .map_err(AppError::Database)?;

        // 更新消息计数
        let mut conv_active: conversation::ActiveModel = conv.into();
        conv_active.message_count = Set(count);
        conv_active.updated_at = Set(now);
        conv_active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;

        Ok(msg)
    }

    async fn save_exchange(
        &self,
        project_id: i64,
        user_input: &str,
        assistant_output: &str,
    ) -> Result<(), AppError> {
        let user_input = required_message_content(user_input)?;
        let assistant_output = required_message_content(assistant_output)?;
        let conv = self.get_or_create_conversation(project_id).await?;
        let count = conv
            .message_count
            .checked_add(2)
            .ok_or_else(|| AppError::InvalidArgument("对话消息数超出 i32 范围".to_string()))?;
        let txn = self.db.begin().await.map_err(AppError::Database)?;
        let now = Utc::now().naive_utc();
        message::ActiveModel {
            conversation_id: Set(conv.id),
            role: Set("user".to_string()),
            content: Set(user_input.to_string()),
            tool_calls: Set(None),
            prompt_tokens: Set(None),
            completion_tokens: Set(None),
            chapter_number: Set(None),
            is_important: Set(false),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(&txn)
        .await
        .map_err(AppError::Database)?;
        message::ActiveModel {
            conversation_id: Set(conv.id),
            role: Set("assistant".to_string()),
            content: Set(assistant_output.to_string()),
            tool_calls: Set(None),
            prompt_tokens: Set(None),
            completion_tokens: Set(None),
            chapter_number: Set(None),
            is_important: Set(false),
            created_at: Set(now + chrono::Duration::milliseconds(1)),
            ..Default::default()
        }
        .insert(&txn)
        .await
        .map_err(AppError::Database)?;

        let mut conv_active: conversation::ActiveModel = conv.into();
        conv_active.message_count = Set(count);
        conv_active.updated_at = Set(now);
        conv_active.update(&txn).await.map_err(AppError::Database)?;
        txn.commit().await.map_err(AppError::Database)?;
        Ok(())
    }

    /// 非流式对话
    pub async fn converse(
        &self,
        project_id: i64,
        user_input: &str,
        llm: &dyn LlmProvider,
        prompt_engine: &dyn PromptEngine,
        variables: &std::collections::HashMap<String, serde_json::Value>,
        workflow_run_id: Option<i64>,
    ) -> Result<String, AppError> {
        let request = self
            .build_inspiration_request(project_id, user_input, prompt_engine, variables)
            .await?;

        let output = self
            .complete_inspiration_call(project_id, request, llm, workflow_run_id)
            .await?;
        self.save_exchange(project_id, user_input, &output).await?;
        Ok(output)
    }

    /// 流式灵感对话
    ///
    /// 与 `converse` 行为一致：先完成并校验 LLM 输出，再一次性保存 user/assistant
    /// 消息。区别是走 `chat_stream`，每收到一个 chunk 调用 `on_chunk` 推到前端。
    pub async fn converse_stream<F>(
        &self,
        input: InspirationConverseStreamInput<'_, F>,
    ) -> Result<String, AppError>
    where
        F: FnMut(&str, u32),
    {
        let InspirationConverseStreamInput {
            project_id,
            user_input,
            llm,
            prompt_engine,
            variables,
            workflow_run_id,
            mut on_chunk,
        } = input;
        let request = self
            .build_inspiration_request(project_id, user_input, prompt_engine, variables)
            .await?;

        let request = self
            .prepare_inspiration_checkpoint_request(project_id, request, workflow_run_id)
            .await?;
        if let Some(restored) = self
            .load_inspiration_checkpoint(
                project_id,
                request.workflow_run_id,
                &request.fingerprint_json,
            )
            .await?
        {
            on_chunk(&restored, 0);
            self.save_exchange(project_id, user_input, &restored)
                .await?;
            return Ok(restored);
        }

        let mut stream = llm.chat_stream(request.request.clone()).await?;

        let mut accumulated = String::new();
        let mut idx: u32 = 0;
        let mut finish_reason: Option<String> = None;
        let mut usage: Option<UsageInfo> = None;
        while let Some(item) = stream.next().await {
            let chunk = item?;
            if let Some(delta) = chunk.content.as_deref()
                && !delta.is_empty()
            {
                accumulated.push_str(delta);
                on_chunk(delta, idx);
                idx += 1;
            }
            if chunk.usage.is_some() {
                usage = chunk.usage;
            }
            if chunk.finish_reason.is_some() {
                finish_reason = chunk.finish_reason;
                break;
            }
        }
        if is_truncated_finish_reason(finish_reason.as_deref()) {
            return Err(AppError::InvalidArgument(
                "灵感对话输出达到当前 max_tokens 上限，JSON 可能被截断；请调大标准输出档位或缩短本轮上下文后重试".to_string(),
            ));
        }

        validate_inspiration_response_json(&accumulated)?;
        self.save_inspiration_checkpoint(
            request.workflow_run_id,
            request.call_key,
            request.fingerprint_json,
            &accumulated,
            finish_reason,
            usage,
        )
        .await?;
        self.save_exchange(project_id, user_input, &accumulated)
            .await?;
        Ok(accumulated)
    }

    async fn complete_inspiration_call(
        &self,
        project_id: i64,
        request: ChatRequest,
        llm: &dyn LlmProvider,
        workflow_run_id: Option<i64>,
    ) -> Result<String, AppError> {
        let request = self
            .prepare_inspiration_checkpoint_request(project_id, request, workflow_run_id)
            .await?;
        if let Some(restored) = self
            .load_inspiration_checkpoint(
                project_id,
                request.workflow_run_id,
                &request.fingerprint_json,
            )
            .await?
        {
            return Ok(restored);
        }

        let response = llm.chat(request.request).await?;
        if is_truncated_finish_reason(response.finish_reason.as_deref()) {
            return Err(AppError::InvalidArgument(
                "灵感对话输出达到当前 max_tokens 上限，JSON 可能被截断；请调大标准输出档位或缩短本轮上下文后重试".to_string(),
            ));
        }
        validate_inspiration_response_json(&response.message.content)?;
        self.save_inspiration_checkpoint(
            request.workflow_run_id,
            request.call_key,
            request.fingerprint_json,
            &response.message.content,
            response.finish_reason,
            response.usage,
        )
        .await?;
        Ok(response.message.content)
    }

    async fn prepare_inspiration_checkpoint_request(
        &self,
        project_id: i64,
        mut request: ChatRequest,
        workflow_run_id: Option<i64>,
    ) -> Result<InspirationCheckpointRequest, AppError> {
        let call_key = format!("inspiration_conversation.project_{project_id}.turn");
        let workflow_run_id = required_llm_call_workflow_run_id(workflow_run_id, &call_key)?;
        let fingerprint_json = inspiration_call_fingerprint_json(project_id, &call_key, &request)?;
        let input_fingerprint_sha256 = llm_call_checkpoint_fingerprint_sha256(&fingerprint_json)?;
        if let Some(trace_context) = request.trace_context.as_mut() {
            trace_context.workflow_run_id = Some(workflow_run_id);
            trace_context.call_key = Some(call_key.clone());
            trace_context.input_fingerprint_sha256 = Some(input_fingerprint_sha256);
        }
        WorkflowRunService::new(self.db.clone())
            .mark_current_step(workflow_run_id, &call_key)
            .await?;
        Ok(InspirationCheckpointRequest {
            request,
            workflow_run_id,
            call_key,
            fingerprint_json,
        })
    }

    async fn load_inspiration_checkpoint(
        &self,
        project_id: i64,
        workflow_run_id: i64,
        fingerprint_json: &Value,
    ) -> Result<Option<String>, AppError> {
        let call_key = format!("inspiration_conversation.project_{project_id}.turn");
        let Some(checkpoint) = load_completed_llm_call_checkpoint(
            self.db.clone(),
            workflow_run_id,
            &call_key,
            fingerprint_json,
        )
        .await?
        else {
            return Ok(None);
        };
        let checkpoint_json = required_llm_call_checkpoint_json(checkpoint, &call_key)?;
        let parsed_output =
            required_checkpoint_string(&checkpoint_json, &call_key, "parsed_output")?;
        let response_content =
            required_checkpoint_string(&checkpoint_json, &call_key, "response_content")?;
        let reparsed = validate_inspiration_response_json(&response_content)?;
        if reparsed != parsed_output {
            return Err(AppError::InvalidState {
                current: "checkpoint_parsed_output_mismatch".to_string(),
                operation: format!("恢复灵感对话 LLM 调用断点 {call_key}"),
            });
        }
        Ok(Some(reparsed))
    }

    async fn save_inspiration_checkpoint(
        &self,
        workflow_run_id: i64,
        call_key: String,
        fingerprint_json: Value,
        output: &str,
        finish_reason: Option<String>,
        usage: Option<UsageInfo>,
    ) -> Result<(), AppError> {
        save_completed_llm_call_checkpoint(
            self.db.clone(),
            CompletedLlmCallCheckpointInput {
                workflow_run_id,
                call_key,
                call_order: 1,
                input_fingerprint_json: fingerprint_json,
                checkpoint_json: json!({
                    "output": output,
                    "parsed_output": output,
                    "response_content": output,
                    "finish_reason": finish_reason,
                    "usage": usage,
                }),
                artifacts_json: None,
            },
        )
        .await
    }

    pub async fn build_inspiration_request(
        &self,
        project_id: i64,
        user_input: &str,
        prompt_engine: &dyn PromptEngine,
        variables: &std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<ChatRequest, AppError> {
        let user_input = required_message_content(user_input)?;
        let history = self.history(project_id, 20).await?;
        let system_content = prompt_engine.render("inspiration", variables, Some(project_id))?;
        let project_context = self.project_context(project_id).await?;
        let mut context_variables = std::collections::HashMap::new();
        context_variables.insert(
            "project_context_json".to_string(),
            json!(serde_json::to_string_pretty(&project_context)?),
        );
        let project_context_guard = prompt_engine.render(
            "inspiration_project_context_guard",
            &context_variables,
            Some(project_id),
        )?;
        let project_context_hash = source_content_sha256(&serde_json::to_string(&project_context)?);
        let guard_hash = source_content_sha256(&project_context_guard);
        let mut ledger = LlmContextLedger::new();
        ledger.add_pack(crate::llm_context::LlmContextLedgerPackInput {
            scope_kind: ("inspiration_conversation").into(),
            pack_kind: ("inspiration_project_context").into(),
            source_refs_json: Some(json!({
                "project_id": project_id,
                "project_context_sha256": project_context_hash,
                "guard_sha256": guard_hash,
            })),
            config_identity_json: json!({
                "ledger_version": "inspiration_conversation_context_v1",
            }),
            required: true,
            stable_prefix: true,
            variants: vec![
                LlmContextLedgerPackDraft::new(
                    COMPRESSION_FULL_RAW,
                    json!({
                        "guard": project_context_guard,
                        "project_context": project_context.clone(),
                    }),
                ),
                LlmContextLedgerPackDraft::new(
                    COMPRESSION_FOCUSED_RAW,
                    json!({
                        "guard": project_context_guard,
                        "project_context": project_context.clone(),
                    }),
                ),
                LlmContextLedgerPackDraft::new(
                    COMPRESSION_MINIMAL,
                    json!({
                        "guard": "回答必须优先尊重已有项目事实；不要把数据库技术字段当作故事设定。",
                        "project_context": project_context.clone(),
                    }),
                ),
            ],
        })?;
        let dynamic_user_content =
            Self::build_inspiration_dynamic_user_content(&history, user_input)?;
        let history_hash = source_content_sha256(&serde_json::to_string(
            &Self::inspiration_history_payload(&history),
        )?);
        let fingerprint = source_content_sha256(&serde_json::to_string(&json!({
            "project_id": project_id,
            "project_context_sha256": project_context_hash,
            "history_sha256": history_hash,
            "user_input": user_input,
        }))?);
        let manifest = ledger.into_manifest(
            dynamic_user_content,
            Some(
                "灵感对话必须返回符合 inspiration 系统提示要求的 JSON 对象；回答必须优先尊重稳定上下文包 inspiration_project_context 中的项目事实。"
                    .to_string(),
            ),
        );

        Ok(context_aware_system_user_request(
            system_content,
            manifest,
            Some(0.8),
            None,
            Some(llm_token_profiles::STANDARD.into()),
        )?
        .with_stream_usage()
        .with_trace_context(LlmTraceContext {
            project_id: Some(project_id),
            root_project_id: None,
            workflow: "inspiration_conversation".to_string(),
            step_key: "conversation_turn".to_string(),
            generation_run_id: None,
            workflow_run_id: None,
            call_key: Some(format!("project_{project_id}_conversation_turn")),
            input_fingerprint_sha256: Some(fingerprint),
            context_plan_json: None,
            context_pack_refs_json: None,
        }))
    }

    /// 获取或创建项目的活跃灵感对话
    async fn get_or_create_conversation(
        &self,
        project_id: i64,
    ) -> Result<conversation::Model, AppError> {
        ensure_project_exists(self.db.as_ref(), project_id).await?;
        let existing = conversation::Entity::find()
            .filter(conversation::Column::ProjectId.eq(project_id))
            .filter(conversation::Column::ConversationType.eq("inspiration"))
            .filter(conversation::Column::IsActive.eq(true))
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;

        if let Some(conv) = existing {
            Ok(conv)
        } else {
            let now = Utc::now().naive_utc();
            conversation::ActiveModel {
                project_id: Set(project_id),
                title: Set(Some("灵感对话".to_string())),
                conversation_type: Set("inspiration".to_string()),
                summary: Set(None),
                extracted_facts: Set(None),
                message_count: Set(0),
                is_active: Set(true),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(self.db.as_ref())
            .await
            .map_err(AppError::Database)
        }
    }

    fn inspiration_history_payload(history: &[message::Model]) -> Vec<Value> {
        history
            .iter()
            .filter_map(|item| match item.role.as_str() {
                "user" | "assistant" => Some(json!({
                    "role": item.role,
                    "content": item.content,
                })),
                _ => None,
            })
            .collect()
    }

    fn build_inspiration_dynamic_user_content(
        history: &[message::Model],
        user_input: &str,
    ) -> Result<String, AppError> {
        let user_input = required_message_content(user_input)?;
        let payload = json!({
            "context_refs": {
                "project_context": "稳定上下文包 inspiration_project_context",
            },
            "conversation": {
                "recent_history": Self::inspiration_history_payload(history),
                "latest_user_input": user_input,
            },
            "response_requirement": {
                "format": "strict_json_object",
                "follow_system_prompt": "必须遵守 inspiration 系统提示词中的 JSON 结构、inspired_options 控件规则、conversation_state 和 is_complete 约束。",
                "context_rule": "回答必须结合 latest_user_input 与 recent_history，并优先尊重 inspiration_project_context 中已有项目事实。"
            }
        });
        Ok(format!(
            "继续灵感对话。只返回系统提示词要求的严格 JSON 对象，不要输出 Markdown、代码块、解释文字或额外字段。\n\n输入：\n{}",
            serde_json::to_string_pretty(&payload)?
        ))
    }

    async fn project_context(&self, project_id: i64) -> Result<Value, AppError> {
        let project = ensure_project_exists(self.db.as_ref(), project_id).await?;
        let mut context = serde_json::Map::new();
        context.insert(
            "project".to_string(),
            json!({
                "title": project.title,
                "description": optional_trimmed_string(&project.description),
                "project_type": project.project_type,
                "status": project.status,
                "target_chapters": project.target_chapters,
                "current_chapter": project.current_chapter,
                "total_chapters": project.total_chapters,
                "chapters_per_part": project.chapters_per_part,
            }),
        );

        if let Some(blueprint) = novel_blueprint::Entity::find()
            .filter(novel_blueprint::Column::ProjectId.eq(project_id))
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
        {
            let characters = blueprint_character::Entity::find()
                .filter(blueprint_character::Column::ProjectId.eq(project_id))
                .all(self.db.as_ref())
                .await
                .map_err(AppError::Database)?;
            let relationships = blueprint_relationship::Entity::find()
                .filter(blueprint_relationship::Column::ProjectId.eq(project_id))
                .all(self.db.as_ref())
                .await
                .map_err(AppError::Database)?;
            context.insert(
                "blueprint".to_string(),
                blueprint_llm_context(&blueprint, &characters, &relationships),
            );
        }

        let mut recent_chapters = chapter::Entity::find()
            .filter(chapter::Column::ProjectId.eq(project_id))
            .filter(chapter::Column::Status.ne("deleted"))
            .order_by_desc(chapter::Column::ChapterNumber)
            .limit(6)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        recent_chapters.reverse();
        let recent_chapters = recent_chapters
            .iter()
            .filter_map(inspiration_chapter_context)
            .collect::<Vec<_>>();
        if !recent_chapters.is_empty() {
            context.insert("recent_chapters".to_string(), Value::Array(recent_chapters));
        }

        Ok(Value::Object(context))
    }
}

async fn ensure_project_exists(
    db: &impl ConnectionTrait,
    project_id: i64,
) -> Result<novel_project::Model, AppError> {
    novel_project::Entity::find_by_id(project_id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))
}

fn newest_history_in_prompt_order(mut rows: Vec<message::Model>) -> Vec<message::Model> {
    rows.reverse();
    rows
}

fn optional_trimmed_string(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn inspiration_chapter_context(ch: &chapter::Model) -> Option<Value> {
    if ch.content.trim().is_empty() && ch.summary.as_deref().unwrap_or("").trim().is_empty() {
        return None;
    }
    let summary = ch
        .analysis_data
        .as_ref()
        .and_then(summary_text)
        .or_else(|| {
            ch.summary
                .as_deref()
                .map(str::trim)
                .filter(|summary| !summary.is_empty())
                .map(ToOwned::to_owned)
        });
    Some(json!({
        "chapter_number": ch.chapter_number,
        "title": ch.title.as_deref().map(str::trim).filter(|title| !title.is_empty()),
        "summary": summary,
        "word_count": ch.word_count,
    }))
}

fn normalize_message_role(role: &str) -> Result<&str, AppError> {
    let role = role.trim();
    match role {
        "user" | "assistant" | "system" | "tool" => Ok(role),
        "" => Err(AppError::InvalidArgument("消息角色不能为空".to_string())),
        other => Err(AppError::InvalidArgument(format!("消息角色无效: {other}"))),
    }
}

fn required_message_content(content: &str) -> Result<&str, AppError> {
    let content = content.trim();
    if content.is_empty() {
        Err(AppError::InvalidArgument("消息内容不能为空".to_string()))
    } else {
        Ok(content)
    }
}

fn inspiration_call_fingerprint_json(
    project_id: i64,
    call_key: &str,
    request: &ChatRequest,
) -> Result<Value, AppError> {
    Ok(json!({
        "schema_version": "inspiration_conversation_call_v1",
        "workflow": "inspiration_conversation",
        "call_key": call_key,
        "project_id": project_id,
        "request": context_aware_chat_request_trace_json(request)?,
    }))
}

fn validate_inspiration_response_json(raw: &str) -> Result<String, AppError> {
    let trimmed = raw.trim();
    let value: Value = serde_json::from_str(trimmed).map_err(|err| {
        AppError::InvalidArgument(format!(
            "灵感对话必须返回严格 JSON 对象: {err}; 输出预览：{}",
            inspiration_response_preview(trimmed)
        ))
    })?;
    if !value.is_object() {
        return Err(AppError::InvalidArgument(
            "灵感对话必须返回严格 JSON 对象，不能返回数组、字符串或其他 JSON 类型".to_string(),
        ));
    }
    Ok(trimmed.to_string())
}

fn required_checkpoint_string(
    checkpoint_json: &Value,
    call_key: &str,
    field: &str,
) -> Result<String, AppError> {
    checkpoint_json
        .get(field)
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| AppError::InvalidState {
            current: format!("missing_or_invalid_checkpoint_field:{field}"),
            operation: format!("读取灵感对话 LLM 调用断点 {call_key}"),
        })
}

fn inspiration_response_preview(raw: &str) -> String {
    if raw.is_empty() {
        return "<empty>".to_string();
    }
    raw.chars().take(180).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use sea_orm::{DatabaseConnection, EntityTrait, PaginatorTrait, Set};
    use std::sync::Arc;

    fn sample_message(id: i64) -> message::Model {
        message::Model {
            id,
            conversation_id: 1,
            role: "user".to_string(),
            content: format!("消息 {id}"),
            tool_calls: None,
            prompt_tokens: None,
            completion_tokens: None,
            chapter_number: None,
            is_important: false,
            created_at: NaiveDate::from_ymd_opt(2026, 5, 31)
                .unwrap()
                .and_hms_opt(0, 0, id as u32)
                .unwrap(),
        }
    }

    async fn setup_db() -> DatabaseConnection {
        a4n_migration::test_support::connect_test_database().await
    }

    async fn insert_project(db: &DatabaseConnection) -> novel_project::Model {
        let now = Utc::now().naive_utc();
        novel_project::ActiveModel {
            user_id: Set(1),
            title: Set("灵感项目".to_string()),
            description: Set("用于灵感服务测试".to_string()),
            project_type: Set("novel_long".to_string()),
            status: Set("draft".to_string()),
            target_chapters: Set(30),
            current_chapter: Set(0),
            total_chapters: Set(None),
            chapters_per_part: Set(Some(10)),
            avatar_svg: Set(None),
            avatar_animal: Set(None),
            raw_blueprint: Set(None),
            import_analysis_progress: Set(None),
            export_config: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap()
    }

    fn service(db: DatabaseConnection) -> InspirationService {
        InspirationService::new(Arc::new(db))
    }

    async fn message_count(db: &DatabaseConnection) -> u64 {
        message::Entity::find().count(db).await.unwrap()
    }

    #[test]
    fn newest_history_in_prompt_order_restores_chronological_order() {
        let rows = vec![sample_message(5), sample_message(4), sample_message(3)];
        let ordered = newest_history_in_prompt_order(rows);

        let ids = ordered.iter().map(|message| message.id).collect::<Vec<_>>();
        assert_eq!(ids, vec![3, 4, 5]);
    }

    #[test]
    fn optional_trimmed_string_omits_blank_values() {
        assert_eq!(
            optional_trimmed_string("  星潮  "),
            Some("星潮".to_string())
        );
        assert_eq!(optional_trimmed_string(" \n\t "), None);
    }

    #[test]
    fn inspiration_dynamic_user_content_keeps_history_dynamic_and_context_referenced() {
        let mut assistant = sample_message(4);
        assistant.role = "assistant".to_string();
        assistant.content = "可以从时间循环切入。".to_string();
        let mut user = sample_message(5);
        user.role = "user".to_string();
        user.content = "我想写校园悬疑。".to_string();

        let content = InspirationService::build_inspiration_dynamic_user_content(
            &[assistant, user],
            "继续深化主角",
        )
        .expect("dynamic inspiration input should serialize");

        assert!(content.contains("inspiration_project_context"));
        assert!(content.contains("可以从时间循环切入。"));
        assert!(content.contains("我想写校园悬疑。"));
        assert!(content.contains("继续深化主角"));
        assert!(content.contains("strict_json_object"));
        assert!(!content.contains("project_context_json"));
    }

    #[test]
    fn validate_inspiration_response_json_accepts_object() {
        let output = validate_inspiration_response_json(
            r#" { "reply": "可以从主角误解切入", "is_complete": false } "#,
        )
        .expect("json object should be accepted");

        assert_eq!(
            output,
            r#"{ "reply": "可以从主角误解切入", "is_complete": false }"#
        );
    }

    #[test]
    fn validate_inspiration_response_json_rejects_array() {
        let err = validate_inspiration_response_json(r#"[{"reply":"ok"}]"#)
            .expect_err("array must not be accepted as inspiration response");

        assert!(err.to_string().contains("严格 JSON 对象"));
    }

    #[test]
    fn validate_inspiration_response_json_rejects_markdown_fence() {
        let err = validate_inspiration_response_json("```json\n{\"reply\":\"ok\"}\n```")
            .expect_err("markdown fenced json must not be accepted");

        assert!(err.to_string().contains("严格 JSON 对象"));
    }

    #[tokio::test]
    async fn save_message_rejects_unknown_role_without_insert() {
        let db = setup_db().await;
        let project = insert_project(&db).await;
        let service = service(db.clone());

        let err = service
            .save_message(project.id, "alien", "有效内容")
            .await
            .expect_err("unknown role should be rejected");

        assert!(
            matches!(err, AppError::InvalidArgument(message) if message.contains("消息角色无效"))
        );
        assert_eq!(message_count(&db).await, 0);
    }

    #[tokio::test]
    async fn save_message_rejects_blank_content_without_insert() {
        let db = setup_db().await;
        let project = insert_project(&db).await;
        let service = service(db.clone());

        let err = service
            .save_message(project.id, "user", " \n\t ")
            .await
            .expect_err("blank message content should be rejected");

        assert!(
            matches!(err, AppError::InvalidArgument(message) if message.contains("消息内容不能为空"))
        );
        assert_eq!(message_count(&db).await, 0);
    }

    #[tokio::test]
    async fn save_message_normalizes_text_and_persists_plain_message() {
        let db = setup_db().await;
        let project = insert_project(&db).await;
        let service = service(db);

        let msg = service
            .save_message(project.id, " user ", "  关键灵感  ")
            .await
            .unwrap();

        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "关键灵感");
        assert_eq!(msg.chapter_number, None);
        assert!(!msg.is_important);
    }
}
