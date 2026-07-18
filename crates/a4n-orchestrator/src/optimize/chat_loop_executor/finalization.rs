use a4n_core::error::AppError;
use a4n_core::models::ai::optimization_session;
use serde_json::json;

use crate::optimize::agent_loop::{AgentLoopDecision, final_reply};
use crate::optimize::turns::{EVENT_KIND_ERROR, EVENT_STATUS_FAILED};
use crate::optimize::{OptimizeProgress, OrchestratorContext, report_optimize_progress};

impl OrchestratorContext {
    pub(super) async fn finish_chat_agent_loop(
        &self,
        session: &optimization_session::Model,
        turn_id: i64,
        step: usize,
        decision: &AgentLoopDecision,
        progress: &mut Option<OptimizeProgress<'_>>,
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
            "普通对话 Agent 已返回 final，正在保存 assistant 消息。",
        )
        .await?;
        self.save_message(session.id, "assistant", &reply).await?;
        report_optimize_progress(progress, 3, 4, "Agent 已返回，已保存回复")?;
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
