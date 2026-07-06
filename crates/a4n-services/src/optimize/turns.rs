use a4n_core::error::AppError;
use a4n_core::models::ai::{optimization_turn, optimization_turn_event};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde_json::Value;

use super::{OptimizeAgentPermissions, OptimizeService};

const TURN_STATUS_RUNNING: &str = "running";
const TURN_STATUS_COMPLETED: &str = "completed";
pub(super) const TURN_STATUS_FAILED: &str = "failed";
pub(super) const TURN_STATUS_CANCELLED: &str = "cancelled";

pub(super) const EVENT_STATUS_RUNNING: &str = "running";
pub(super) const EVENT_STATUS_COMPLETED: &str = "completed";
pub(super) const EVENT_STATUS_FAILED: &str = "failed";

pub(super) const EVENT_KIND_ASSISTANT_UPDATE: &str = "assistant_update";
pub(super) const EVENT_KIND_TOOL_CALL: &str = "tool_call";
pub(super) const EVENT_KIND_TOOL_RESULT: &str = "tool_result";
pub(super) const EVENT_KIND_ERROR: &str = "error";

impl OptimizeService {
    pub async fn create_turn(
        &self,
        session_id: i64,
        user_text: &str,
        permissions: &OptimizeAgentPermissions,
    ) -> Result<optimization_turn::Model, AppError> {
        let text = user_text.trim();
        if text.is_empty() {
            return Err(AppError::InvalidArgument(
                "Turn 用户输入不能为空".to_string(),
            ));
        }
        let now = Utc::now().naive_utc();
        optimization_turn::ActiveModel {
            session_id: Set(session_id),
            user_text: Set(text.to_string()),
            status: Set(TURN_STATUS_RUNNING.to_string()),
            permissions_snapshot: Set(Some(permissions.as_value())),
            final_reply: Set(None),
            error: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(self.db.as_ref())
        .await
        .map_err(AppError::Database)
    }

    pub async fn list_turns(
        &self,
        session_id: i64,
    ) -> Result<Vec<optimization_turn::Model>, AppError> {
        optimization_turn::Entity::find()
            .filter(optimization_turn::Column::SessionId.eq(session_id))
            .order_by_asc(optimization_turn::Column::CreatedAt)
            .order_by_asc(optimization_turn::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn list_turn_events(
        &self,
        turn_id: i64,
    ) -> Result<Vec<optimization_turn_event::Model>, AppError> {
        optimization_turn_event::Entity::find()
            .filter(optimization_turn_event::Column::TurnId.eq(turn_id))
            .order_by_asc(optimization_turn_event::Column::Seq)
            .order_by_asc(optimization_turn_event::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn append_turn_event(
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

        let next_seq = optimization_turn_event::Entity::find()
            .filter(optimization_turn_event::Column::TurnId.eq(turn_id))
            .order_by_desc(optimization_turn_event::Column::Seq)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .map(|event| event.seq.saturating_add(1))
            .unwrap_or(1);

        optimization_turn_event::ActiveModel {
            turn_id: Set(turn_id),
            seq: Set(next_seq),
            kind: Set(kind.to_string()),
            status: Set(status.to_string()),
            public_text: Set(public_text.to_string()),
            payload: Set(payload),
            created_at: Set(Utc::now().naive_utc()),
            ..Default::default()
        }
        .insert(self.db.as_ref())
        .await
        .map_err(AppError::Database)
    }

    pub async fn finish_turn(
        &self,
        turn_id: i64,
        final_reply: &str,
    ) -> Result<optimization_turn::Model, AppError> {
        let reply = final_reply.trim();
        if reply.is_empty() {
            return Err(AppError::InvalidArgument(
                "Turn final_reply 不能为空".to_string(),
            ));
        }
        let turn = self.get_turn(turn_id).await?;
        let mut active: optimization_turn::ActiveModel = turn.into();
        active.status = Set(TURN_STATUS_COMPLETED.to_string());
        active.final_reply = Set(Some(reply.to_string()));
        active.error = Set(None);
        active.updated_at = Set(Utc::now().naive_utc());
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn fail_turn(
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
        let mut active: optimization_turn::ActiveModel = turn.into();
        active.status = Set(status.to_string());
        active.error = Set(Some(error.to_string()));
        active.updated_at = Set(Utc::now().naive_utc());
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub(super) async fn get_turn(
        &self,
        turn_id: i64,
    ) -> Result<optimization_turn::Model, AppError> {
        optimization_turn::Entity::find_by_id(turn_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("优化 Turn 不存在: {turn_id}")))
    }
}
