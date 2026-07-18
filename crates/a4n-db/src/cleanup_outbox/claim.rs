use super::contracts::{
    CLEANUP_LEASE_SECONDS, CLEANUP_STATUS_PENDING, CLEANUP_STATUS_RUNNING,
    ClaimedMediaWorkflowV2Reset, ClaimedProjectCleanup, MEDIA_WORKFLOW_CONTRACT_V2_RESET_KIND,
    PROJECT_DELETE_CLEANUP_KIND, parse_media_workflow_v2_reset_payload,
    parse_project_deletion_payload,
};
use super::transitions::fail_claimed_cleanup;
use a4n_core::models::task::cleanup_outbox;
use chrono::{Duration, Utc};
use sea_orm::sea_query::{LockBehavior, LockType};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, DbErr, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, TransactionTrait,
};

pub async fn claim_project_deletion_cleanup(
    db: &(impl ConnectionTrait + TransactionTrait),
    project_id: Option<i64>,
) -> Result<Option<ClaimedProjectCleanup>, DbErr> {
    loop {
        let Some(record) =
            claim_cleanup_by_kind(db, PROJECT_DELETE_CLEANUP_KIND, project_id).await?
        else {
            return Ok(None);
        };
        match parse_project_deletion_payload(&record) {
            Ok(payload) => return Ok(Some(ClaimedProjectCleanup { record, payload })),
            Err(error) => {
                fail_claimed_cleanup(
                    db,
                    record.id,
                    required_lease_token(&record)?,
                    error.to_string(),
                )
                .await?;
            }
        }
    }
}

pub async fn claim_media_workflow_v2_reset(
    db: &(impl ConnectionTrait + TransactionTrait),
    project_id: Option<i64>,
) -> Result<Option<ClaimedMediaWorkflowV2Reset>, DbErr> {
    loop {
        let Some(record) =
            claim_cleanup_by_kind(db, MEDIA_WORKFLOW_CONTRACT_V2_RESET_KIND, project_id).await?
        else {
            return Ok(None);
        };
        match parse_media_workflow_v2_reset_payload(&record) {
            Ok(payload) => return Ok(Some(ClaimedMediaWorkflowV2Reset { record, payload })),
            Err(error) => {
                fail_claimed_cleanup(
                    db,
                    record.id,
                    required_lease_token(&record)?,
                    error.to_string(),
                )
                .await?;
            }
        }
    }
}

async fn claim_cleanup_by_kind(
    db: &(impl ConnectionTrait + TransactionTrait),
    cleanup_kind: &str,
    project_id: Option<i64>,
) -> Result<Option<cleanup_outbox::Model>, DbErr> {
    let tx = db.begin().await?;
    let now = Utc::now().naive_utc();
    let due = Condition::any()
        .add(
            Condition::all()
                .add(cleanup_outbox::Column::Status.eq(CLEANUP_STATUS_PENDING))
                .add(cleanup_outbox::Column::NextAttemptAt.lte(now)),
        )
        .add(
            Condition::all()
                .add(cleanup_outbox::Column::Status.eq(CLEANUP_STATUS_RUNNING))
                .add(cleanup_outbox::Column::LeaseExpiresAt.lte(now)),
        );
    let mut query = cleanup_outbox::Entity::find()
        .filter(cleanup_outbox::Column::CleanupKind.eq(cleanup_kind))
        .filter(due)
        .order_by_asc(cleanup_outbox::Column::NextAttemptAt)
        .order_by_asc(cleanup_outbox::Column::Id)
        .lock_with_behavior(LockType::Update, LockBehavior::SkipLocked);
    if let Some(project_id) = project_id {
        query = query.filter(cleanup_outbox::Column::ProjectId.eq(project_id));
    }
    let Some(record) = query.one(&tx).await? else {
        tx.commit().await?;
        return Ok(None);
    };
    let lease_token = uuid::Uuid::new_v4().to_string();
    let attempt_count = record
        .attempt_count
        .checked_add(1)
        .ok_or_else(|| DbErr::Custom("清理任务 attempt_count 超出 i32 范围".to_string()))?;
    let mut active: cleanup_outbox::ActiveModel = record.into();
    active.status = Set(CLEANUP_STATUS_RUNNING.to_string());
    active.attempt_count = Set(attempt_count);
    active.lease_token = Set(Some(lease_token));
    active.lease_expires_at = Set(Some(now + Duration::seconds(CLEANUP_LEASE_SECONDS)));
    active.updated_at = Set(now);
    let record = active.update(&tx).await?;
    tx.commit().await?;
    Ok(Some(record))
}

fn required_lease_token(record: &cleanup_outbox::Model) -> Result<&str, DbErr> {
    record.lease_token.as_deref().ok_or_else(|| {
        DbErr::Custom(format!(
            "已 claim 的 cleanup outbox 缺少 lease_token: id={}",
            record.id
        ))
    })
}

pub async fn media_workflow_v2_reset_pending<C>(db: &C, project_id: i64) -> Result<bool, DbErr>
where
    C: ConnectionTrait,
{
    Ok(cleanup_outbox::Entity::find()
        .filter(cleanup_outbox::Column::ProjectId.eq(project_id))
        .filter(cleanup_outbox::Column::CleanupKind.eq(MEDIA_WORKFLOW_CONTRACT_V2_RESET_KIND))
        .filter(cleanup_outbox::Column::Status.is_in([
            CLEANUP_STATUS_PENDING,
            CLEANUP_STATUS_RUNNING,
            super::contracts::CLEANUP_STATUS_FAILED,
        ]))
        .one(db)
        .await?
        .is_some())
}
