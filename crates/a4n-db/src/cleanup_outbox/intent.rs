use super::contracts::{
    CLEANUP_STATUS_PENDING, FILE_ARTIFACT_CLEANUP_KIND, FileArtifactCleanupPayload,
    GENERATION_ARTIFACT_UNBIND_KIND, GENERATION_PROVIDER_JOB_CLEANUP_KIND,
    GENERATION_STAGING_ARTIFACT_DELETE_KIND, GenerationArtifactUnbindPayload,
    GenerationCleanupScope, GenerationProviderJobCleanupPayload,
    GenerationStagingArtifactDeletePayload, MEDIA_WORKFLOW_CONTRACT_V2_RESET_KIND,
    MediaWorkflowV2ResetPayload, PROJECT_DELETE_CLEANUP_KIND, ProjectDeletionCleanupPayload,
};
use a4n_core::models::task::cleanup_outbox;
use chrono::Utc;
use sea_orm::sea_query::OnConflict;
use sea_orm::{
    ActiveValue::NotSet, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, Set,
};
use std::collections::HashMap;

struct PreparedCleanupIntent {
    dedupe_key: String,
    cleanup_kind: String,
    project_id: Option<i64>,
    requested_by_user_id: Option<i64>,
    task_id: Option<String>,
    attempt: Option<i32>,
    payload_version: i32,
    payload_json: serde_json::Value,
    payload_sha256: String,
}

impl PreparedCleanupIntent {
    fn active_model(&self, now: chrono::NaiveDateTime) -> cleanup_outbox::ActiveModel {
        cleanup_outbox::ActiveModel {
            id: NotSet,
            dedupe_key: Set(self.dedupe_key.clone()),
            cleanup_kind: Set(self.cleanup_kind.clone()),
            project_id: Set(self.project_id),
            requested_by_user_id: Set(self.requested_by_user_id),
            task_id: Set(self.task_id.clone()),
            attempt: Set(self.attempt),
            payload_version: Set(self.payload_version),
            payload_json: Set(self.payload_json.clone()),
            payload_sha256: Set(self.payload_sha256.clone()),
            status: Set(CLEANUP_STATUS_PENDING.to_string()),
            attempt_count: Set(0),
            observation_count: Set(0),
            next_attempt_at: Set(now),
            lease_token: Set(None),
            lease_expires_at: Set(None),
            last_error: Set(None),
            outcome: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            completed_at: Set(None),
        }
    }
}

pub async fn insert_project_deletion_cleanup_intent<C>(
    db: &C,
    dedupe_key: String,
    payload: ProjectDeletionCleanupPayload,
) -> Result<cleanup_outbox::Model, DbErr>
where
    C: ConnectionTrait,
{
    let payload = super::contracts::validated_project_deletion_cleanup_payload(
        payload.project_id,
        payload.media_paths,
    )?;
    insert_cleanup_intent(
        db,
        dedupe_key,
        PROJECT_DELETE_CLEANUP_KIND,
        Some(payload.project_id),
        None,
        None,
        None,
        serde_json::to_value(payload).map_err(|error| {
            DbErr::Custom(format!("序列化项目删除 cleanup payload 失败: {error}"))
        })?,
    )
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
    insert_cleanup_intent(
        db,
        dedupe_key,
        MEDIA_WORKFLOW_CONTRACT_V2_RESET_KIND,
        Some(payload.project_id),
        None,
        None,
        None,
        serde_json::to_value(payload).map_err(|error| {
            DbErr::Custom(format!("序列化媒体工作流 v2 reset payload 失败: {error}"))
        })?,
    )
    .await
}

pub async fn insert_media_workflow_v2_reset_intents<C, I>(
    db: &C,
    intents: I,
) -> Result<Vec<cleanup_outbox::Model>, DbErr>
where
    C: ConnectionTrait,
    I: IntoIterator<Item = (String, MediaWorkflowV2ResetPayload)>,
{
    let specs = intents
        .into_iter()
        .map(|(dedupe_key, payload)| {
            let project_id = payload.project_id;
            let payload_json = serde_json::to_value(payload).map_err(|error| {
                DbErr::Custom(format!("序列化媒体工作流 v2 reset payload 失败: {error}"))
            })?;
            prepare_cleanup_intent(
                dedupe_key,
                MEDIA_WORKFLOW_CONTRACT_V2_RESET_KIND,
                Some(project_id),
                None,
                None,
                None,
                payload_json,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    if specs.is_empty() {
        return Ok(Vec::new());
    }

    let dedupe_keys = specs
        .iter()
        .map(|spec| spec.dedupe_key.clone())
        .collect::<Vec<_>>();
    let now = Utc::now().naive_utc();
    cleanup_outbox::Entity::insert_many(specs.iter().map(|spec| spec.active_model(now)))
        .on_conflict(
            OnConflict::column(cleanup_outbox::Column::DedupeKey)
                .do_nothing()
                .to_owned(),
        )
        .do_nothing()
        .exec_without_returning(db)
        .await?;
    let records = cleanup_outbox::Entity::find()
        .filter(cleanup_outbox::Column::DedupeKey.is_in(dedupe_keys))
        .all(db)
        .await?
        .into_iter()
        .map(|record| (record.dedupe_key.clone(), record))
        .collect::<HashMap<_, _>>();

    specs
        .iter()
        .map(|spec| {
            let existing = records.get(&spec.dedupe_key).ok_or_else(|| {
                DbErr::Custom(format!(
                    "cleanup outbox 插入或 dedupe 冲突后找不到记录: {}",
                    spec.dedupe_key
                ))
            })?;
            validate_existing_cleanup_intent(existing, spec)?;
            Ok(existing.clone())
        })
        .collect()
}

pub async fn insert_file_artifact_cleanup_intent<C>(
    db: &C,
    dedupe_key: String,
    payload: FileArtifactCleanupPayload,
) -> Result<cleanup_outbox::Model, DbErr>
where
    C: ConnectionTrait,
{
    let payload = super::contracts::validated_file_artifact_cleanup_payload(
        payload.project_id,
        payload.paths,
    )?;
    insert_cleanup_intent(
        db,
        dedupe_key,
        FILE_ARTIFACT_CLEANUP_KIND,
        Some(payload.project_id),
        None,
        None,
        None,
        serde_json::to_value(payload).map_err(|error| {
            DbErr::Custom(format!("序列化文件 artifact cleanup payload 失败: {error}"))
        })?,
    )
    .await
}

pub async fn insert_generation_provider_job_cleanup_intent<C>(
    db: &C,
    dedupe_key: String,
    scope: &GenerationCleanupScope,
    payload: GenerationProviderJobCleanupPayload,
) -> Result<cleanup_outbox::Model, DbErr>
where
    C: ConnectionTrait,
{
    insert_generation_cleanup_intent(
        db,
        dedupe_key,
        GENERATION_PROVIDER_JOB_CLEANUP_KIND,
        scope,
        serde_json::to_value(payload).map_err(|error| {
            DbErr::Custom(format!(
                "序列化 generation provider cleanup payload 失败: {error}"
            ))
        })?,
    )
    .await
}

pub async fn insert_generation_staging_artifact_delete_intent<C>(
    db: &C,
    dedupe_key: String,
    scope: &GenerationCleanupScope,
    payload: GenerationStagingArtifactDeletePayload,
) -> Result<cleanup_outbox::Model, DbErr>
where
    C: ConnectionTrait,
{
    insert_generation_cleanup_intent(
        db,
        dedupe_key,
        GENERATION_STAGING_ARTIFACT_DELETE_KIND,
        scope,
        serde_json::to_value(payload).map_err(|error| {
            DbErr::Custom(format!(
                "序列化 generation staging cleanup payload 失败: {error}"
            ))
        })?,
    )
    .await
}

pub async fn insert_generation_artifact_unbind_intent<C>(
    db: &C,
    dedupe_key: String,
    scope: &GenerationCleanupScope,
    payload: GenerationArtifactUnbindPayload,
) -> Result<cleanup_outbox::Model, DbErr>
where
    C: ConnectionTrait,
{
    insert_generation_cleanup_intent(
        db,
        dedupe_key,
        GENERATION_ARTIFACT_UNBIND_KIND,
        scope,
        serde_json::to_value(payload).map_err(|error| {
            DbErr::Custom(format!(
                "序列化 generation artifact unbind payload 失败: {error}"
            ))
        })?,
    )
    .await
}

async fn insert_generation_cleanup_intent<C>(
    db: &C,
    dedupe_key: String,
    cleanup_kind: &str,
    scope: &GenerationCleanupScope,
    payload_json: serde_json::Value,
) -> Result<cleanup_outbox::Model, DbErr>
where
    C: ConnectionTrait,
{
    scope.validate()?;
    let attempt = i32::try_from(scope.attempt)
        .map_err(|_| DbErr::Custom("generation cleanup attempt 超出 i32 范围".to_string()))?;
    insert_cleanup_intent(
        db,
        dedupe_key,
        cleanup_kind,
        scope.project_id,
        Some(scope.requested_by_user_id),
        Some(scope.task_id.clone()),
        Some(attempt),
        payload_json,
    )
    .await
}

async fn insert_cleanup_intent<C>(
    db: &C,
    dedupe_key: String,
    cleanup_kind: &str,
    project_id: Option<i64>,
    requested_by_user_id: Option<i64>,
    task_id: Option<String>,
    attempt: Option<i32>,
    payload_json: serde_json::Value,
) -> Result<cleanup_outbox::Model, DbErr>
where
    C: ConnectionTrait,
{
    let spec = prepare_cleanup_intent(
        dedupe_key,
        cleanup_kind,
        project_id,
        requested_by_user_id,
        task_id,
        attempt,
        payload_json,
    )?;
    cleanup_outbox::Entity::insert(spec.active_model(Utc::now().naive_utc()))
        .on_conflict(
            OnConflict::column(cleanup_outbox::Column::DedupeKey)
                .do_nothing()
                .to_owned(),
        )
        .do_nothing()
        .exec_without_returning(db)
        .await?;
    let existing = cleanup_outbox::Entity::find()
        .filter(cleanup_outbox::Column::DedupeKey.eq(&spec.dedupe_key))
        .one(db)
        .await?
        .ok_or_else(|| {
            DbErr::Custom(format!(
                "cleanup outbox 插入或 dedupe 冲突后找不到记录: {}",
                spec.dedupe_key
            ))
        })?;
    validate_existing_cleanup_intent(&existing, &spec)?;
    Ok(existing)
}

fn prepare_cleanup_intent(
    dedupe_key: String,
    cleanup_kind: &str,
    project_id: Option<i64>,
    requested_by_user_id: Option<i64>,
    task_id: Option<String>,
    attempt: Option<i32>,
    payload_json: serde_json::Value,
) -> Result<PreparedCleanupIntent, DbErr> {
    let dedupe_key = dedupe_key.trim().to_string();
    if dedupe_key.is_empty() {
        return Err(DbErr::Custom(
            "cleanup outbox dedupe_key 不能为空".to_string(),
        ));
    }
    if project_id.is_some_and(|project_id| project_id <= 0) {
        return Err(DbErr::Custom(
            "cleanup outbox project_id 必须为正整数".to_string(),
        ));
    }
    let payload_version = payload_json
        .get("version")
        .and_then(serde_json::Value::as_i64)
        .and_then(|version| i32::try_from(version).ok())
        .filter(|version| *version > 0)
        .ok_or_else(|| DbErr::Custom("cleanup payload version 必须为正整数".to_string()))?;
    let payload_sha256 = crate::generation::canonical_json_sha256(&payload_json)
        .map_err(|error| DbErr::Custom(format!("cleanup payload canonical hash 失败: {error}")))?;
    Ok(PreparedCleanupIntent {
        dedupe_key,
        cleanup_kind: cleanup_kind.to_string(),
        project_id,
        requested_by_user_id,
        task_id,
        attempt,
        payload_version,
        payload_json,
        payload_sha256,
    })
}

fn validate_existing_cleanup_intent(
    existing: &cleanup_outbox::Model,
    expected: &PreparedCleanupIntent,
) -> Result<(), DbErr> {
    if existing.cleanup_kind != expected.cleanup_kind
        || existing.project_id != expected.project_id
        || existing.requested_by_user_id != expected.requested_by_user_id
        || existing.task_id != expected.task_id
        || existing.attempt != expected.attempt
        || existing.payload_version != expected.payload_version
        || existing.payload_json != expected.payload_json
        || existing.payload_sha256 != expected.payload_sha256
    {
        return Err(DbErr::Custom(format!(
            "cleanup outbox dedupe_key 已绑定其他 payload: {}",
            expected.dedupe_key
        )));
    }
    Ok(())
}
