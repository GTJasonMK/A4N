use super::contracts::{
    CLEANUP_LEASE_SECONDS, CLEANUP_STATUS_COMPLETED, CLEANUP_STATUS_FAILED, CLEANUP_STATUS_PENDING,
    CLEANUP_STATUS_RUNNING,
};
use a4n_core::models::task::cleanup_outbox;
use chrono::{Duration, Utc};
use sea_orm::{
    ActiveValue::NotSet, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, Set,
};

pub async fn renew_claimed_cleanup<C>(db: &C, id: i64, lease_token: &str) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    let now = Utc::now().naive_utc();
    let result = cleanup_outbox::Entity::update_many()
        .set(cleanup_outbox::ActiveModel {
            lease_expires_at: Set(Some(now + Duration::seconds(CLEANUP_LEASE_SECONDS))),
            updated_at: Set(now),
            ..not_set_cleanup_update()
        })
        .filter(cleanup_outbox::Column::Id.eq(id))
        .filter(cleanup_outbox::Column::Status.eq(CLEANUP_STATUS_RUNNING))
        .filter(cleanup_outbox::Column::LeaseToken.eq(lease_token))
        .exec(db)
        .await?;
    require_current_lease(id, result.rows_affected)
}

pub async fn complete_claimed_cleanup<C>(db: &C, id: i64, lease_token: &str) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    update_claimed_cleanup(db, id, lease_token, true, None, None).await
}

pub async fn retry_claimed_cleanup<C>(
    db: &C,
    id: i64,
    lease_token: &str,
    error: String,
    retry_after: Duration,
) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    update_claimed_cleanup(db, id, lease_token, false, Some(error), Some(retry_after)).await
}

async fn update_claimed_cleanup<C>(
    db: &C,
    id: i64,
    lease_token: &str,
    completed: bool,
    error: Option<String>,
    retry_after: Option<Duration>,
) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    let now = Utc::now().naive_utc();
    let result = cleanup_outbox::Entity::update_many()
        .set(cleanup_outbox::ActiveModel {
            status: Set(if completed {
                CLEANUP_STATUS_COMPLETED.to_string()
            } else {
                CLEANUP_STATUS_PENDING.to_string()
            }),
            next_attempt_at: Set(now + retry_after.unwrap_or_else(Duration::zero)),
            lease_token: Set(None),
            lease_expires_at: Set(None),
            last_error: Set(error),
            updated_at: Set(now),
            completed_at: Set(completed.then_some(now)),
            ..not_set_cleanup_update()
        })
        .filter(cleanup_outbox::Column::Id.eq(id))
        .filter(cleanup_outbox::Column::Status.eq(CLEANUP_STATUS_RUNNING))
        .filter(cleanup_outbox::Column::LeaseToken.eq(lease_token))
        .exec(db)
        .await?;
    require_current_lease(id, result.rows_affected)
}

pub(super) async fn fail_claimed_cleanup<C>(
    db: &C,
    id: i64,
    lease_token: &str,
    error: String,
) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    let now = Utc::now().naive_utc();
    let result = cleanup_outbox::Entity::update_many()
        .set(cleanup_outbox::ActiveModel {
            status: Set(CLEANUP_STATUS_FAILED.to_string()),
            next_attempt_at: Set(now),
            lease_token: Set(None),
            lease_expires_at: Set(None),
            last_error: Set(Some(error)),
            updated_at: Set(now),
            completed_at: Set(None),
            ..not_set_cleanup_update()
        })
        .filter(cleanup_outbox::Column::Id.eq(id))
        .filter(cleanup_outbox::Column::Status.eq(CLEANUP_STATUS_RUNNING))
        .filter(cleanup_outbox::Column::LeaseToken.eq(lease_token))
        .exec(db)
        .await?;
    require_current_lease(id, result.rows_affected)
}

fn require_current_lease(id: i64, rows_affected: u64) -> Result<(), DbErr> {
    if rows_affected == 1 {
        Ok(())
    } else {
        Err(DbErr::Custom(format!(
            "cleanup outbox lease 已失效: id={id}"
        )))
    }
}

fn not_set_cleanup_update() -> cleanup_outbox::ActiveModel {
    cleanup_outbox::ActiveModel {
        id: NotSet,
        dedupe_key: NotSet,
        cleanup_kind: NotSet,
        project_id: NotSet,
        payload_json: NotSet,
        status: NotSet,
        attempt_count: NotSet,
        next_attempt_at: NotSet,
        lease_token: NotSet,
        lease_expires_at: NotSet,
        last_error: NotSet,
        created_at: NotSet,
        updated_at: NotSet,
        completed_at: NotSet,
    }
}
