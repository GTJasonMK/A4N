use a4n_db_models::models::ai::optimization_session;
use a4n_error::AppError;
use serde_json::json;

use crate::optimize::OrchestratorContext;
use crate::optimize::agent_loop::{AgentLoopDecision, final_reply};
use crate::optimize::turns::{EVENT_KIND_ERROR, EVENT_STATUS_FAILED};

impl OrchestratorContext {
    pub(super) async fn finish_chat_agent_loop(
        &self,
        session: &optimization_session::Model,
        turn_id: i64,
        step: usize,
        decision: &AgentLoopDecision,
    ) -> Result<String, AppError> {
        let reply = match final_reply(&decision.arguments) {
            Ok(reply) => reply,
            Err(err) => {
                self.append_turn_event(
                    turn_id,
                    EVENT_KIND_ERROR,
                    EVENT_STATUS_FAILED,
                    &format!("final 参数无效：{err}"),
                    Some(json!({
                        "step": step,
                        "action": decision.action.as_str(),
                        "arguments": decision.arguments.clone(),
                    })),
                )
                .await?;
                self.save_agent_event(session.id, &format!("普通对话 final 参数无效：{err}"))
                    .await?;
                return Err(err);
            }
        };
        self.save_agent_event(
            session.id,
            "普通对话 Agent 已返回 final，等待 generation terminal 事务保存正式消息。",
        )
        .await?;
        Ok(reply)
    }

    pub(super) async fn fail_chat_agent_loop_runaway(
        &self,
        session: &optimization_session::Model,
        turn_id: i64,
    ) -> Result<String, AppError> {
        let err = AppError::InvalidArgument(
            "普通对话 Agent Loop 未收敛：模型持续请求新动作但没有返回 final".to_string(),
        );
        self.append_turn_event(
            turn_id,
            EVENT_KIND_ERROR,
            EVENT_STATUS_FAILED,
            &err.to_string(),
            None,
        )
        .await?;
        self.save_agent_event(session.id, &format!("普通对话 Agent Loop 未完成：{err}"))
            .await?;
        Err(err)
    }
}
