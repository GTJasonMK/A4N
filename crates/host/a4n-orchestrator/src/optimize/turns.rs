use a4n_db::optimize::{
    OptimizationTurnCreateInput, OptimizationTurnEventAppendInput, OptimizationTurnUpdateInput,
};
use a4n_db_models::models::ai::{optimization_turn, optimization_turn_event};
use a4n_domain::project::OptimizationTurnRecord;
use a4n_error::AppError;
use serde_json::Value;

use super::{OptimizationAgentPermissions, OrchestratorContext, optimize_db_error};

const TURN_STATUS_RUNNING: &str = "running";
pub(super) const TURN_STATUS_FAILED: &str = "failed";
pub(super) const TURN_STATUS_CANCELLED: &str = "cancelled";

pub(super) const EVENT_STATUS_RUNNING: &str = "running";
pub(super) const EVENT_STATUS_COMPLETED: &str = "completed";
pub(super) const EVENT_STATUS_FAILED: &str = "failed";

pub(super) const EVENT_KIND_ASSISTANT_UPDATE: &str = "assistant_update";
pub(super) const EVENT_KIND_TOOL_CALL: &str = "tool_call";
pub(super) const EVENT_KIND_TOOL_RESULT: &str = "tool_result";
pub(super) const EVENT_KIND_ERROR: &str = "error";

impl OrchestratorContext {
    pub(super) async fn create_turn(
        &self,
        session_id: i64,
        user_text: &str,
        permissions: &OptimizationAgentPermissions,
        agent_turn_id: Option<i64>,
    ) -> Result<optimization_turn::Model, AppError> {
        let text = user_text.trim();
        if text.is_empty() {
            return Err(AppError::InvalidArgument(
                "Turn 用户输入不能为空".to_string(),
            ));
        }
        a4n_db::optimize::create_optimization_turn(
            self.database.connection(),
            OptimizationTurnCreateInput {
                session_id,
                agent_turn_id,
                user_text: text.to_string(),
                status: TURN_STATUS_RUNNING.to_string(),
                permissions_snapshot: Some(permissions.as_value()),
            },
        )
        .await
        .map_err(optimize_db_error)
    }

    pub(super) async fn list_turns(
        &self,
        session_id: i64,
    ) -> Result<Vec<OptimizationTurnRecord>, AppError> {
        self.optimization_turns(session_id).await
    }

    pub(super) async fn append_turn_event(
        &self,
        turn_id: i64,
        kind: &str,
        status: &str,
        public_text: &str,
        payload: Option<Value>,
    ) -> Result<optimization_turn_event::Model, AppError> {
        let kind = kind.trim();
        let status = status.trim();
        let public_text = public_text.trim();
        if kind.is_empty() {
            return Err(AppError::InvalidArgument(
                "Turn event kind 不能为空".to_string(),
            ));
        }
        if status.is_empty() {
            return Err(AppError::InvalidArgument(
                "Turn event status 不能为空".to_string(),
            ));
        }
        if public_text.is_empty() {
            return Err(AppError::InvalidArgument(
                "Turn event public_text 不能为空".to_string(),
            ));
        }

        a4n_db::optimize::append_optimization_turn_event(
            self.database.connection(),
            OptimizationTurnEventAppendInput {
                turn_id,
                kind: kind.to_string(),
                status: status.to_string(),
                public_text: public_text.to_string(),
                payload,
            },
        )
        .await
        .map_err(optimize_db_error)
    }

    pub(super) async fn fail_turn(
        &self,
        turn_id: i64,
        status: &str,
        error: &str,
    ) -> Result<optimization_turn::Model, AppError> {
        let status = status.trim();
        let error = error.trim();
        if !matches!(status, TURN_STATUS_FAILED | TURN_STATUS_CANCELLED) {
            return Err(AppError::InvalidArgument(format!(
                "Turn 失败状态非法：{status}"
            )));
        }
        if error.is_empty() {
            return Err(AppError::InvalidArgument("Turn error 不能为空".to_string()));
        }
        let turn = self.get_turn(turn_id).await?;
        a4n_db::optimize::update_optimization_turn(
            self.database.connection(),
            OptimizationTurnUpdateInput {
                turn,
                status: Some(status.to_string()),
                final_reply: None,
                error: Some(Some(error.to_string())),
            },
        )
        .await
        .map_err(optimize_db_error)
    }

    pub(super) async fn get_turn(
        &self,
        turn_id: i64,
    ) -> Result<optimization_turn::Model, AppError> {
        a4n_db::optimize::optimization_turn_by_id(self.database.connection(), turn_id)
            .await
            .map_err(optimize_db_error)?
            .ok_or_else(|| AppError::NotFound(format!("优化 Turn 不存在: {turn_id}")))
    }
}
