use a4n_db_models::models::ai::optimization_session_message;
use a4n_error::AppError;
use serde_json::Value;

use super::super::{OptimizationSessionMessageInsertInput, OrchestratorContext};

impl OrchestratorContext {
    pub async fn record_llm_request(
        &self,
        session_id: i64,
        phase: &str,
        content: &str,
    ) -> Result<(), AppError> {
        let payload = serde_json::to_string_pretty(&serde_json::json!({
            "phase": phase,
            "content": content,
        }))?;
        self.save_message(session_id, "llm_request", &payload)
            .await?;
        Ok(())
    }

    pub async fn record_llm_response(
        &self,
        session_id: i64,
        phase: &str,
        content: &str,
    ) -> Result<(), AppError> {
        let payload = serde_json::to_string_pretty(&serde_json::json!({
            "phase": phase,
            "content": content,
        }))?;
        self.save_message(session_id, "llm_response", &payload)
            .await?;
        Ok(())
    }

    pub(in crate::optimize) async fn save_message(
        &self,
        session_id: i64,
        role: &str,
        content: &str,
    ) -> Result<optimization_session_message::Model, AppError> {
        self.save_message_with_tool_calls(session_id, role, content, None)
            .await
    }

    pub(in crate::optimize) async fn save_agent_event(
        &self,
        session_id: i64,
        content: &str,
    ) -> Result<optimization_session_message::Model, AppError> {
        self.save_message(session_id, "agent_event", content).await
    }

    pub(in crate::optimize) async fn save_message_with_tool_calls(
        &self,
        session_id: i64,
        role: &str,
        content: &str,
        tool_calls: Option<Value>,
    ) -> Result<optimization_session_message::Model, AppError> {
        self.insert_optimization_session_message(OptimizationSessionMessageInsertInput {
            session_id,
            role: role.to_string(),
            content: content.to_string(),
            tool_calls,
        })
        .await
    }
}
