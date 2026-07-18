use a4n_core::error::AppError;
use a4n_core::models::ai::optimization_turn;
use a4n_db::optimize::OptimizationSessionUpdateInput;
use a4n_llm::provider::ChatRequest;

use super::validation::{ensure_chat_session_accepts_message, required_chat_user_text};
use crate::optimize::agent_loop::{
    build_first_agent_loop_chat_request, insert_current_chapter_content,
};
use crate::optimize::chat_loop_executor::ChatAgentLoopInput;
use crate::optimize::turns::{TURN_STATUS_CANCELLED, TURN_STATUS_FAILED};
use crate::optimize::{
    OptimizeAgentPermissions, OptimizeSendUserMessageInput, OrchestratorContext,
    is_cancelled_optimize_error, optimize_db_error, report_optimize_progress,
};
use a4n_prompt::rendering::PromptRenderer;

impl OrchestratorContext {
    pub async fn send_user_message_with_permissions(
        &self,
        input: OptimizeSendUserMessageInput<'_, '_, '_>,
    ) -> Result<optimization_turn::Model, AppError> {
        let OptimizeSendUserMessageInput {
            session_id,
            message,
            permissions_override,
            prompt_renderer,
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
        let (system_prompt, base_user_message) = match self
            .build_chat_agent_prompt_bundle(&session, user_text, &chat_context, prompt_renderer)
            .await
        {
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
                let status = if is_cancelled_optimize_error(&err) {
                    TURN_STATUS_CANCELLED
                } else {
                    TURN_STATUS_FAILED
                };
                self.fail_turn(turn.id, status, &err.to_string()).await?;
                return Err(err);
            }
        };
        a4n_db::optimize::update_optimization_session(
            self.db.connection(),
            OptimizationSessionUpdateInput {
                session,
                status: None,
                issues: None,
                agent_permissions: None,
                agent_instruction: None,
                fixes_applied: None,
                summary: None,
            },
        )
        .await
        .map_err(optimize_db_error)?;

        report_optimize_progress(&mut progress, 4, 4, "Agent 对话已更新")?;
        self.get_turn(turn.id).await
    }

    pub async fn build_send_user_message_preview_request(
        &self,
        session_id: i64,
        message: &str,
        permissions_override: Option<&OptimizeAgentPermissions>,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ChatRequest, AppError> {
        let user_text = required_chat_user_text(message)?;

        let session = self.get_session(session_id).await?;
        ensure_chat_session_accepts_message(&session, "预览内容优化 Agent 对话")?;
        let chat_context = self
            .build_chat_agent_context(&session, permissions_override)
            .await?;
        let (system_prompt, base_user_message) = self
            .build_chat_agent_prompt_bundle(&session, user_text, &chat_context, prompt_renderer)
            .await?;
        build_first_agent_loop_chat_request(
            session.project_id,
            Some(session.id),
            "content_optimization_chat_agent_loop",
            system_prompt,
            &base_user_message,
        )
    }
}
