use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatRequest;

use super::validation::{ensure_chat_session_accepts_message, required_chat_user_text};
use crate::optimize::agent_loop::{
    build_first_agent_loop_chat_request, insert_current_chapter_content,
};
use crate::optimize::chat::loop_executor::ChatAgentLoopInput;
use crate::optimize::turns::{TURN_STATUS_CANCELLED, TURN_STATUS_FAILED};
use crate::optimize::{
    AgentLoopExecutionError, OptimizationAgentPermissions, OptimizeChatTurnExecution,
    OptimizeSendUserMessageInput, OrchestratorContext,
};
use a4n_prompt::rendering::PromptRenderer;

impl OrchestratorContext {
    pub(in crate::optimize) async fn send_user_message_with_permissions(
        &self,
        input: OptimizeSendUserMessageInput<'_, '_>,
    ) -> Result<OptimizeChatTurnExecution, AgentLoopExecutionError> {
        let OptimizeSendUserMessageInput {
            session_id,
            message,
            permissions,
            prompt_renderer,
            workflow_run_id,
            generation,
        } = input;
        let user_text = required_chat_user_text(message)?;

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

        let mut chat_context = self.build_chat_agent_context(&session, permissions).await?;
        insert_current_chapter_content(&mut chat_context.chat_context, &chapter.content);

        self.save_agent_event(
            session_id,
            "收到用户消息，正在构建普通对话 Agent Loop 输入。",
        )
        .await?;
        let (system_prompt, base_user_message) = match self
            .build_chat_agent_prompt_bundle(&session, user_text, &chat_context, prompt_renderer)
            .await
        {
            Ok(bundle) => bundle,
            Err(err) => {
                self.save_agent_event(session_id, &format!("渲染普通交互提示词失败：{err}"))
                    .await?;
                return Err(err.into());
            }
        };
        let turn = self
            .create_turn(session_id, user_text, &chat_context.permissions, None)
            .await?;
        let reply = match self
            .run_chat_agent_loop(ChatAgentLoopInput {
                turn_id: turn.id,
                session: &session,
                system_prompt,
                base_user_message,
                chat_context: chat_context.chat_context,
                workflow_run_id,
                generation,
            })
            .await
        {
            Ok(reply) => reply,
            Err(err) => {
                let status = if err.is_cancelled() {
                    TURN_STATUS_CANCELLED
                } else {
                    TURN_STATUS_FAILED
                };
                self.fail_turn(turn.id, status, &err.to_string()).await?;
                return Err(err);
            }
        };
        Ok(OptimizeChatTurnExecution {
            turn_id: turn.id,
            user_text: user_text.to_string(),
            final_reply: reply,
        })
    }

    pub(crate) async fn build_send_user_message_preview_request(
        &self,
        session_id: i64,
        message: &str,
        permissions: &OptimizationAgentPermissions,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ChatRequest, AppError> {
        let user_text = required_chat_user_text(message)?;

        let session = self.get_session(session_id).await?;
        ensure_chat_session_accepts_message(&session, "预览内容优化 Agent 对话")?;
        let chat_context = self.build_chat_agent_context(&session, permissions).await?;
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
