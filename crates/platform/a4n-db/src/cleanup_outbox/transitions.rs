use super::contracts::{
    CLEANUP_LEASE_SECONDS, CLEANUP_STATUS_COMPLETED, CLEANUP_STATUS_FAILED, CLEANUP_STATUS_PENDING,
    CLEANUP_STATUS_RUNNING, GENERATION_ARTIFACT_UNBIND_KIND, GENERATION_PROVIDER_JOB_CLEANUP_KIND,
    GENERATION_STAGING_ARTIFACT_DELETE_KIND, GenerationCleanupObservationOutcome,
    GenerationCleanupOutcome, GenerationCleanupPayload, GenerationCleanupRetryOutcome,
    GenerationCleanupScope, GenerationProviderJobCleanupOperation,
    GenerationProviderJobCleanupPayload, parse_generation_cleanup_payload,
};
use a4n_db_models::models::task::cleanup_outbox;
use a4n_service_protocol::redact_sensitive_message;
use chrono::{Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ActiveValue::NotSet, ColumnTrait, ConnectionTrait, DbErr, EntityTrait,
    QueryFilter, QuerySelect, Set, TransactionTrait,
};

const GENERATION_CLEANUP_RETRY_DELAYS: [Duration; 5] = [
    Duration::minutes(1),
    Duration::minutes(5),
    Duration::minutes(30),
    Duration::hours(2),
    Duration::hours(12),
];
const GENERATION_CLEANUP_DEADLINE: Duration = Duration::days(30);

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
    update_claimed_cleanup(db, id, lease_token, Some("completed"), None, None).await
}

pub async fn complete_claimed_generation_cleanup<C>(
    db: &C,
    id: i64,
    lease_token: &str,
    outcome: GenerationCleanupOutcome,
) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    update_claimed_cleanup(db, id, lease_token, Some(outcome.as_str()), None, None).await
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
    update_claimed_cleanup(db, id, lease_token, None, Some(error), Some(retry_after)).await
}

pub async fn retry_or_fail_claimed_generation_cleanup(
    db: &(impl ConnectionTrait + TransactionTrait),
    id: i64,
    lease_token: &str,
    error: String,
) -> Result<GenerationCleanupRetryOutcome, DbErr> {
    let tx = db.begin().await?;
    let Some(record) = cleanup_outbox::Entity::find_by_id(id)
        .filter(cleanup_outbox::Column::Status.eq(CLEANUP_STATUS_RUNNING))
        .filter(cleanup_outbox::Column::LeaseToken.eq(lease_token))
        .lock_exclusive()
        .one(&tx)
        .await?
    else {
        return Err(DbErr::Custom(format!(
            "cleanup outbox lease 已失效: id={id}"
        )));
    };
    if !matches!(
        record.cleanup_kind.as_str(),
        GENERATION_PROVIDER_JOB_CLEANUP_KIND
            | GENERATION_STAGING_ARTIFACT_DELETE_KIND
            | GENERATION_ARTIFACT_UNBIND_KIND
    ) {
        return Err(DbErr::Custom(format!(
            "cleanup outbox {id} 不是 generation cleanup kind"
        )));
    }
    let attempt_count = usize::try_from(record.attempt_count).map_err(|_| {
        DbErr::Custom(format!(
            "generation cleanup attempt_count 无效: id={id}, attempt_count={}",
            record.attempt_count
        ))
    })?;
    if attempt_count > GENERATION_CLEANUP_RETRY_DELAYS.len() {
        return Err(DbErr::Custom(format!(
            "generation cleanup attempt_count 超出重试上限: id={id}, attempt_count={attempt_count}"
        )));
    }

    let now = Utc::now().naive_utc();
    let deadline_exceeded = record.created_at + GENERATION_CLEANUP_DEADLINE <= now;
    let error = redact_sensitive_message(error);
    let error = if deadline_exceeded {
        format!("{error}；provider job cleanup 超过 30 天生命周期期限")
    } else {
        error
    };
    let (outcome, status, next_attempt_at, next_attempt_count) = if deadline_exceeded {
        (
            GenerationCleanupRetryOutcome::Exhausted,
            CLEANUP_STATUS_FAILED,
            now,
            record.attempt_count,
        )
    } else {
        match GENERATION_CLEANUP_RETRY_DELAYS.get(attempt_count) {
            Some(delay) => (
                GenerationCleanupRetryOutcome::Scheduled,
                CLEANUP_STATUS_PENDING,
                now + *delay,
                record.attempt_count + 1,
            ),
            None => (
                GenerationCleanupRetryOutcome::Exhausted,
                CLEANUP_STATUS_FAILED,
                now,
                record.attempt_count,
            ),
        }
    };
    let mut active: cleanup_outbox::ActiveModel = record.into();
    active.status = Set(status.to_string());
    active.attempt_count = Set(next_attempt_count);
    active.next_attempt_at = Set(next_attempt_at);
    active.lease_token = Set(None);
    active.lease_expires_at = Set(None);
    active.last_error = Set(Some(error));
    active.outcome = Set(None);
    active.updated_at = Set(now);
    active.completed_at = Set(None);
    active.update(&tx).await?;
    tx.commit().await?;
    Ok(outcome)
}

/// Requeue a claimed provider reconcile observation without consuming a
/// provider-attempt slot. The row is locked for the whole transition so a
/// worker restart cannot lose the observation.
pub async fn observe_claimed_generation_reconcile(
    db: &(impl ConnectionTrait + TransactionTrait),
    id: i64,
    lease_token: &str,
    retry_after: Duration,
) -> Result<GenerationCleanupObservationOutcome, DbErr> {
    let tx = db.begin().await?;
    let Some(record) = claimed_generation_cleanup_record(&tx, id, lease_token).await? else {
        return Err(DbErr::Custom(format!(
            "cleanup outbox lease 已失效: id={id}"
        )));
    };
    let GenerationCleanupPayload::ProviderJob(payload) = parse_generation_cleanup_payload(&record)?
    else {
        return Err(DbErr::Custom(format!(
            "cleanup outbox {id} 不是 provider job reconcile"
        )));
    };
    if payload.operation != GenerationProviderJobCleanupOperation::Reconcile {
        return Err(DbErr::Custom(format!(
            "cleanup outbox {id} 不是 provider job reconcile"
        )));
    }

    let now = Utc::now().naive_utc();
    let deadline_exceeded = record.created_at + GENERATION_CLEANUP_DEADLINE <= now;
    let next_observation_count = if deadline_exceeded {
        record.observation_count
    } else {
        record.observation_count.checked_add(1).ok_or_else(|| {
            DbErr::Custom(format!(
                "generation cleanup observation_count 溢出: id={id}"
            ))
        })?
    };
    let mut active: cleanup_outbox::ActiveModel = record.into();
    active.lease_token = Set(None);
    active.lease_expires_at = Set(None);
    active.updated_at = Set(now);
    active.completed_at = Set(None);
    active.outcome = Set(None);
    if deadline_exceeded {
        active.status = Set(CLEANUP_STATUS_FAILED.to_string());
        active.next_attempt_at = Set(now);
        active.last_error = Set(Some(
            "provider job reconcile 超过 30 天生命周期期限，已停止重试".to_string(),
        ));
    } else {
        active.status = Set(CLEANUP_STATUS_PENDING.to_string());
        active.observation_count = Set(next_observation_count);
        active.next_attempt_at = Set(now + retry_after);
        active.last_error = Set(None);
    }
    active.update(&tx).await?;
    tx.commit().await?;
    Ok(if deadline_exceeded {
        GenerationCleanupObservationOutcome::DeadlineExceeded
    } else {
        GenerationCleanupObservationOutcome::Rescheduled
    })
}

/// Atomically turn a completed remote reconcile into the single canonical
/// provider-delete intent, then finish the reconcile row.
pub async fn schedule_provider_delete_from_reconcile(
    db: &(impl ConnectionTrait + TransactionTrait),
    id: i64,
    lease_token: &str,
) -> Result<cleanup_outbox::Model, DbErr> {
    let tx = db.begin().await?;
    let Some(record) = claimed_generation_cleanup_record(&tx, id, lease_token).await? else {
        return Err(DbErr::Custom(format!(
            "cleanup outbox lease 已失效: id={id}"
        )));
    };
    let GenerationCleanupPayload::ProviderJob(payload) = parse_generation_cleanup_payload(&record)?
    else {
        return Err(DbErr::Custom(format!(
            "cleanup outbox {id} 不是 provider job reconcile"
        )));
    };
    if payload.operation != GenerationProviderJobCleanupOperation::Reconcile {
        return Err(DbErr::Custom(format!(
            "cleanup outbox {id} 不是 provider job reconcile"
        )));
    }
    let requested_by_user_id = record.requested_by_user_id.ok_or_else(|| {
        DbErr::Custom(format!(
            "generation cleanup 缺少 requested_by_user_id: id={id}"
        ))
    })?;
    let task_id = record
        .task_id
        .clone()
        .ok_or_else(|| DbErr::Custom(format!("generation cleanup 缺少 task_id: id={id}")))?;
    let attempt = record
        .attempt
        .and_then(|value| u32::try_from(value).ok())
        .ok_or_else(|| DbErr::Custom(format!("generation cleanup attempt 无效: id={id}")))?;
    let scope = GenerationCleanupScope {
        requested_by_user_id,
        project_id: record.project_id,
        task_id,
        attempt,
    };
    let delete_payload = GenerationProviderJobCleanupPayload::new(
        GenerationProviderJobCleanupOperation::Delete,
        payload.provider_kind,
        payload.provider_job_id,
        payload.provider_idempotency_key_sha256,
    )
    .map_err(|error| DbErr::Custom(error.to_string()))?;
    let delete_intent = super::intent::insert_generation_provider_job_cleanup_intent(
        &tx,
        format!("{}:delete", record.dedupe_key),
        &scope,
        delete_payload,
    )
    .await?;

    let now = Utc::now().naive_utc();
    let mut active: cleanup_outbox::ActiveModel = record.into();
    active.status = Set(CLEANUP_STATUS_COMPLETED.to_string());
    active.next_attempt_at = Set(now);
    active.lease_token = Set(None);
    active.lease_expires_at = Set(None);
    active.last_error = Set(None);
    active.outcome = Set(Some(
        GenerationCleanupOutcome::DeleteScheduled
            .as_str()
            .to_string(),
    ));
    active.updated_at = Set(now);
    active.completed_at = Set(Some(now));
    active.update(&tx).await?;
    tx.commit().await?;
    Ok(delete_intent)
}

async fn claimed_generation_cleanup_record<C>(
    db: &C,
    id: i64,
    lease_token: &str,
) -> Result<Option<cleanup_outbox::Model>, DbErr>
where
    C: ConnectionTrait,
{
    let record = cleanup_outbox::Entity::find_by_id(id)
        .filter(cleanup_outbox::Column::Status.eq(CLEANUP_STATUS_RUNNING))
        .filter(cleanup_outbox::Column::LeaseToken.eq(lease_token))
        .lock_exclusive()
        .one(db)
        .await?;
    if let Some(record) = record.as_ref()
        && !matches!(
            record.cleanup_kind.as_str(),
            GENERATION_PROVIDER_JOB_CLEANUP_KIND
                | GENERATION_STAGING_ARTIFACT_DELETE_KIND
                | GENERATION_ARTIFACT_UNBIND_KIND
        )
    {
        return Err(DbErr::Custom(format!(
            "cleanup outbox {id} 不是 generation cleanup kind"
        )));
    }
    Ok(record)
}

async fn update_claimed_cleanup<C>(
    db: &C,
    id: i64,
    lease_token: &str,
    outcome: Option<&str>,
    error: Option<String>,
    retry_after: Option<Duration>,
) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    let now = Utc::now().naive_utc();
    let result = cleanup_outbox::Entity::update_many()
        .set(cleanup_outbox::ActiveModel {
            status: Set(if outcome.is_some() {
                CLEANUP_STATUS_COMPLETED.to_string()
            } else {
                CLEANUP_STATUS_PENDING.to_string()
            }),
            next_attempt_at: Set(now + retry_after.unwrap_or_else(Duration::zero)),
            lease_token: Set(None),
            lease_expires_at: Set(None),
            last_error: Set(error),
            outcome: Set(outcome.map(str::to_string)),
            updated_at: Set(now),
            completed_at: Set(outcome.map(|_| now)),
            ..not_set_cleanup_update()
        })
        .filter(cleanup_outbox::Column::Id.eq(id))
        .filter(cleanup_outbox::Column::Status.eq(CLEANUP_STATUS_RUNNING))
        .filter(cleanup_outbox::Column::LeaseToken.eq(lease_token))
        .exec(db)
        .await?;
    require_current_lease(id, result.rows_affected)
}

pub async fn fail_claimed_cleanup<C>(
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
            outcome: Set(None),
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
        requested_by_user_id: NotSet,
        task_id: NotSet,
        attempt: NotSet,
        payload_version: NotSet,
        payload_json: NotSet,
        payload_sha256: NotSet,
        status: NotSet,
        attempt_count: NotSet,
        observation_count: NotSet,
        next_attempt_at: NotSet,
        lease_token: NotSet,
        lease_expires_at: NotSet,
        last_error: NotSet,
        outcome: NotSet,
        created_at: NotSet,
        updated_at: NotSet,
        completed_at: NotSet,
    }
}
