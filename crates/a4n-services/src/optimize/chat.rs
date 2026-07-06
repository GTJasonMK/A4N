use std::collections::HashMap;

use a4n_core::error::AppError;
use a4n_core::models::ai::{optimization_session, optimization_turn};
use a4n_engine::llm::provider::ChatRequest;
use a4n_engine::prompt::engine::PromptEngine;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, Set};
use serde_json::Value;

use super::agent_loop::{build_first_agent_loop_chat_request, insert_current_chapter_content};
use super::chat_loop_executor::ChatAgentLoopInput;
use super::context::{build_content_optimization_context, optimization_context_manifest};
use super::messages::{build_content_optimization_chat_user_message, optimization_chat_history};
use super::permissions::OptimizeAgentPermissions;
use super::turns::{TURN_STATUS_CANCELLED, TURN_STATUS_FAILED};
use super::{
    OptimizeSendUserMessageInput, OptimizeService, is_cancelled_optimize_error,
    report_optimize_progress, required_available_session_name,
};

struct ChatAgentContext {
    permissions: OptimizeAgentPermissions,
    chat_context: Value,
    context_manifest: Value,
    conversation_history: Value,
}

fn required_chat_user_text(message: &str) -> Result<&str, AppError> {
    let user_text = message.trim();
    if user_text.is_empty() {
        return Err(AppError::InvalidArgument("对话消息不能为空".to_string()));
    }
    Ok(user_text)
}

fn ensure_chat_session_accepts_message(
    session: &optimization_session::Model,
    operation: &str,
) -> Result<(), AppError> {
    if matches!(
        session.status.as_str(),
        "analyzing" | "fixing" | "cancelled"
    ) {
        return Err(AppError::InvalidState {
            current: session.status.clone(),
            operation: operation.to_string(),
        });
    }
    Ok(())
}

impl OptimizeService {
    /// 创建一个不先跑分析的普通交互会话。
    pub async fn start_chat_session(
        &self,
        project_id: i64,
        chapter_id: i64,
        name: &str,
        variables: &HashMap<String, Value>,
    ) -> Result<optimization_session::Model, AppError> {
        let name =
            required_available_session_name(self.db.as_ref(), project_id, chapter_id, name).await?;
        let now = Utc::now().naive_utc();
        let permissions = OptimizeAgentPermissions::from_variables(variables);
        let analysis_context = build_content_optimization_context(
            self.db.as_ref(),
            project_id,
            chapter_id,
            &permissions,
        )
        .await?;
        let context_manifest =
            serde_json::to_string_pretty(&optimization_context_manifest(&analysis_context))?;
        let session = optimization_session::ActiveModel {
            project_id: Set(project_id),
            chapter_id: Set(chapter_id),
            name: Set(Some(name)),
            status: Set("waiting_user".to_string()),
            issues: Set(None),
            agent_permissions: Set(Some(permissions.as_value())),
            agent_instruction: Set(None),
            fixes_applied: Set(0),
            summary: Set(Some("普通交互会话".to_string())),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(self.db.as_ref())
        .await
        .map_err(AppError::Database)?;

        self.save_message(session.id, "context", &context_manifest)
            .await?;
        self.save_message(
            session.id,
            "agent_event",
            "已创建普通交互会话，等待用户指令。",
        )
        .await?;

        Ok(session)
    }

    pub async fn send_user_message_with_permissions(
        &self,
        input: OptimizeSendUserMessageInput<'_, '_, '_>,
    ) -> Result<optimization_turn::Model, AppError> {
        let OptimizeSendUserMessageInput {
            session_id,
            message,
            permissions_override,
            llm,
            prompt_engine,
            workflow_run_id,
            mut progress,
            mut stream,
        } = input;
        let user_text = required_chat_user_text(message)?;

        report_optimize_progress(&mut progress, 0, 4, "正在读取优化会话上下文")?;
        let session = self.get_session(session_id).await?;
        ensure_chat_session_accepts_message(&session, "发送内容优化 Agent 对话")?;

        let chapter = self
            .load_optimize_chapter_for_operation(
                &session,
                "发送内容优化 Agent 对话",
                "已删除章节不能继续内容优化对话",
                None,
            )
            .await?;

        let mut chat_context = self
            .build_chat_agent_context(&session, permissions_override)
            .await?;
        insert_current_chapter_content(&mut chat_context.chat_context, &chapter.content);

        let turn = self
            .create_turn(session_id, user_text, &chat_context.permissions)
            .await?;
        self.save_message(session_id, "user", user_text).await?;
        self.save_agent_event(
            session_id,
            "收到用户消息，正在构建普通对话 Agent Loop 输入。",
        )
        .await?;
        report_optimize_progress(&mut progress, 1, 4, "正在渲染 Agent 对话提示词")?;
        let (system_prompt, base_user_message) = match self.build_chat_agent_prompt_bundle(
            &session,
            user_text,
            &chat_context,
            prompt_engine,
        ) {
            Ok(bundle) => bundle,
            Err(err) => {
                self.save_agent_event(session_id, &format!("渲染普通交互提示词失败：{err}"))
                    .await?;
                self.fail_turn(turn.id, TURN_STATUS_FAILED, &err.to_string())
                    .await?;
                return Err(err);
            }
        };
        match self
            .run_chat_agent_loop(ChatAgentLoopInput {
                turn_id: turn.id,
                session: &session,
                system_prompt,
                base_user_message,
                chat_context: chat_context.chat_context,
                llm,
                workflow_run_id,
                progress: &mut progress,
                stream: &mut stream,
            })
            .await
        {
            Ok(reply) => {
                self.finish_turn(turn.id, &reply).await?;
            }
            Err(err) => {
                let status = if is_cancelled_optimize_error(&err.to_string()) {
                    TURN_STATUS_CANCELLED
                } else {
                    TURN_STATUS_FAILED
                };
                self.fail_turn(turn.id, status, &err.to_string()).await?;
                return Err(err);
            }
        };
        let mut active: optimization_session::ActiveModel = session.into();
        active.updated_at = Set(Utc::now().naive_utc());
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;

        report_optimize_progress(&mut progress, 4, 4, "Agent 对话已更新")?;
        self.get_turn(turn.id).await
    }

    pub async fn build_send_user_message_preview_request(
        &self,
        session_id: i64,
        message: &str,
        permissions_override: Option<&OptimizeAgentPermissions>,
        prompt_engine: &dyn PromptEngine,
    ) -> Result<ChatRequest, AppError> {
        let user_text = required_chat_user_text(message)?;

        let session = self.get_session(session_id).await?;
        ensure_chat_session_accepts_message(&session, "预览内容优化 Agent 对话")?;
        let chat_context = self
            .build_chat_agent_context(&session, permissions_override)
            .await?;
        let (system_prompt, base_user_message) =
            self.build_chat_agent_prompt_bundle(&session, user_text, &chat_context, prompt_engine)?;
        build_first_agent_loop_chat_request(
            session.project_id,
            Some(session.id),
            "content_optimization_chat_agent_loop",
            system_prompt,
            &base_user_message,
        )
    }

    async fn build_chat_agent_context(
        &self,
        session: &optimization_session::Model,
        permissions_override: Option<&OptimizeAgentPermissions>,
    ) -> Result<ChatAgentContext, AppError> {
        let previous_turns = self.list_turns(session.id).await?;
        let conversation_history = optimization_chat_history(&previous_turns);
        let permissions = permissions_override
            .cloned()
            .unwrap_or_else(|| OptimizeAgentPermissions::from_session(session));
        let analysis_context = build_content_optimization_context(
            self.db.as_ref(),
            session.project_id,
            session.chapter_id,
            &permissions,
        )
        .await?;
        let context_manifest = optimization_context_manifest(&analysis_context);
        Ok(ChatAgentContext {
            permissions,
            chat_context: analysis_context,
            context_manifest,
            conversation_history,
        })
    }

    fn build_chat_agent_prompt_bundle(
        &self,
        session: &optimization_session::Model,
        user_text: &str,
        chat_context: &ChatAgentContext,
        prompt_engine: &dyn PromptEngine,
    ) -> Result<(String, String), AppError> {
        let mut prompt_vars = HashMap::new();
        prompt_vars.insert(
            "agent_instruction".to_string(),
            user_text.to_string().into(),
        );
        prompt_vars.insert(
            "tools_prompt".to_string(),
            serde_json::to_string_pretty(&self.tool_specs())?.into(),
        );
        let system_prompt = prompt_engine.render(
            "content_optimization_chat",
            &prompt_vars,
            Some(session.project_id),
        )?;
        let base_user_message = build_content_optimization_chat_user_message(
            user_text,
            session,
            &chat_context.conversation_history,
            &chat_context.context_manifest,
        )?;
        Ok((system_prompt, base_user_message))
    }
}
