use super::contracts::{
    CLEANUP_STATUS_PENDING, MEDIA_WORKFLOW_CONTRACT_V2_RESET_KIND, MediaWorkflowV2ResetPayload,
    PROJECT_DELETE_CLEANUP_KIND, ProjectDeletionCleanupPayload,
};
use a4n_core::models::task::cleanup_outbox;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ActiveValue::NotSet, ConnectionTrait, DbErr, Set};

pub async fn insert_project_deletion_cleanup_intent<C>(
    db: &C,
    dedupe_key: String,
    payload: ProjectDeletionCleanupPayload,
) -> Result<cleanup_outbox::Model, DbErr>
where
    C: ConnectionTrait,
{
    let now = Utc::now().naive_utc();
    cleanup_outbox::ActiveModel {
        id: NotSet,
        dedupe_key: Set(dedupe_key),
        cleanup_kind: Set(PROJECT_DELETE_CLEANUP_KIND.to_string()),
        project_id: Set(payload.project_id),
        payload_json: Set(serde_json::to_value(payload).map_err(|error| {
            DbErr::Custom(format!("序列化项目删除 cleanup payload 失败: {error}"))
        })?),
        status: Set(CLEANUP_STATUS_PENDING.to_string()),
        attempt_count: Set(0),
        next_attempt_at: Set(now),
        lease_token: Set(None),
        lease_expires_at: Set(None),
        last_error: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        completed_at: Set(None),
    }
    .insert(db)
    .await
}

pub async fn insert_media_workflow_v2_reset_intent<C>(
    db: &C,
    dedupe_key: String,
    payload: MediaWorkflowV2ResetPayload,
) -> Result<cleanup_outbox::Model, DbErr>
where
    C: ConnectionTrait,
{
    let now = Utc::now().naive_utc();
    cleanup_outbox::ActiveModel {
        id: NotSet,
        dedupe_key: Set(dedupe_key),
        cleanup_kind: Set(MEDIA_WORKFLOW_CONTRACT_V2_RESET_KIND.to_string()),
        project_id: Set(payload.project_id),
        payload_json: Set(serde_json::to_value(payload).map_err(|error| {
            DbErr::Custom(format!("序列化媒体工作流 v2 reset payload 失败: {error}"))
        })?),
        status: Set(CLEANUP_STATUS_PENDING.to_string()),
        attempt_count: Set(0),
        next_attempt_at: Set(now),
        lease_token: Set(None),
        lease_expires_at: Set(None),
        last_error: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        completed_at: Set(None),
    }
    .insert(db)
    .await
}
