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
    let now = Utc::now().naive_utc();
    let active = cleanup_outbox::ActiveModel {
        id: NotSet,
        dedupe_key: Set(dedupe_key.clone()),
        cleanup_kind: Set(cleanup_kind.to_string()),
        project_id: Set(project_id),
        requested_by_user_id: Set(requested_by_user_id),
        task_id: Set(task_id.clone()),
        attempt: Set(attempt),
        payload_version: Set(payload_version),
        payload_json: Set(payload_json.clone()),
        payload_sha256: Set(payload_sha256.clone()),
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
    };
    cleanup_outbox::Entity::insert(active)
        .on_conflict(
            OnConflict::column(cleanup_outbox::Column::DedupeKey)
                .do_nothing()
                .to_owned(),
        )
        .do_nothing()
        .exec_without_returning(db)
        .await?;
    let existing = cleanup_outbox::Entity::find()
        .filter(cleanup_outbox::Column::DedupeKey.eq(&dedupe_key))
        .one(db)
        .await?
        .ok_or_else(|| {
            DbErr::Custom(format!(
                "cleanup outbox 插入或 dedupe 冲突后找不到记录: {dedupe_key}"
            ))
        })?;
    if existing.cleanup_kind != cleanup_kind
        || existing.project_id != project_id
        || existing.requested_by_user_id != requested_by_user_id
        || existing.task_id != task_id
        || existing.attempt != attempt
        || existing.payload_version != payload_version
        || existing.payload_json != payload_json
        || existing.payload_sha256 != payload_sha256
    {
        return Err(DbErr::Custom(format!(
            "cleanup outbox dedupe_key 已绑定其他 payload: {dedupe_key}"
        )));
    }
    Ok(existing)
}
