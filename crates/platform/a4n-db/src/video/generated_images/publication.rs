//! 图片文件与 `generated_image` 记录之间的可恢复发布事务。

use super::super::{
    VideoDbError, lock_generated_media_project, lock_generated_media_project_for_update,
    lock_image_publication_reclaim_project_for_update,
};
use super::create::{
    image_generation_parsed_output, image_generation_response_summary,
    insert_generated_image_record, validate_create_generated_image_record_input,
};
use a4n_db_models::models::{ai::generated_image, media::image_publication};
use a4n_domain::generation::GenerationRunTraceTarget;
use a4n_domain::media::{
    CreateGeneratedImageRecordDraftInput, CreateImagePublicationIntentInput, GeneratedImageRecord,
    IMAGE_PUBLICATION_STATUS_PENDING, IMAGE_PUBLICATION_STATUS_PUBLISHED,
    IMAGE_PUBLICATION_STATUS_RECLAIMED, IMAGE_PUBLICATION_STATUS_RECLAIMING,
    ImagePublicationFileFacts,
};
use chrono::Utc;
use sea_orm::sea_query::{Expr, OnConflict};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, TransactionTrait,
};
use serde_json::Value;

use crate::video::media_mapping::generated_image_record_from_model;

pub struct ImagePublicationRequest<'a> {
    pub project_id: i64,
    pub requested_by_user_id: i64,
    pub image_type: &'a str,
    pub prompt: &'a str,
    pub chapter_id: Option<i64>,
    pub panel_id: Option<&'a str>,
}

pub fn validate_image_publication_request(
    publication: &image_publication::Model,
    request: ImagePublicationRequest<'_>,
) -> Result<(), VideoDbError> {
    if publication.project_id != request.project_id
        || publication.requested_by_user_id != request.requested_by_user_id
        || publication.image_type != request.image_type.trim()
        || publication.prompt != request.prompt.trim()
        || publication.chapter_id != request.chapter_id
        || publication.panel_id.as_deref()
            != request
                .panel_id
                .map(str::trim)
                .filter(|value| !value.is_empty())
    {
        return Err(VideoDbError::InvalidArgument(
            "publication_id 已绑定到不同的图片发布意图".to_string(),
        ));
    }
    Ok(())
}

pub async fn update_image_publication_metadata<C>(
    db: &C,
    publication_id: &str,
    expected_version: i64,
    metadata_json: Value,
) -> Result<image_publication::Model, VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let publication_id = required_publication_id(publication_id)?;
    if expected_version <= 0 {
        return Err(VideoDbError::InvalidArgument(
            "图片发布版本必须大于 0".to_string(),
        ));
    }
    if !metadata_json.is_object() {
        return Err(VideoDbError::InvalidArgument(
            "图片发布 metadata_json 必须是 JSON 对象".to_string(),
        ));
    }
    let txn = db.begin().await.map_err(VideoDbError::Database)?;
    let snapshot = image_publication::Entity::find_by_id(&publication_id)
        .one(&txn)
        .await
        .map_err(VideoDbError::Database)?
        .ok_or_else(|| VideoDbError::NotFound(format!("图片发布意图不存在: {publication_id}")))?;
    lock_generated_media_project(&txn, snapshot.project_id).await?;
    let publication = image_publication::Entity::find_by_id(&publication_id)
        .lock_exclusive()
        .one(&txn)
        .await
        .map_err(VideoDbError::Database)?
        .ok_or_else(|| VideoDbError::NotFound(format!("图片发布意图不存在: {publication_id}")))?;
    if publication.metadata_json.as_ref() == Some(&metadata_json) {
        txn.commit().await.map_err(VideoDbError::Database)?;
        return Ok(publication);
    }
    if publication.status != IMAGE_PUBLICATION_STATUS_PENDING {
        return Err(VideoDbError::InvalidState {
            current: publication.status,
            operation: "更新图片发布元数据".to_string(),
        });
    }
    if publication.version != expected_version {
        return Err(VideoDbError::InvalidState {
            current: format!(
                "publication_version={},expected_version={expected_version}",
                publication.version
            ),
            operation: "更新图片发布元数据".to_string(),
        });
    }
    let next_version =
        expected_version
            .checked_add(1)
            .ok_or_else(|| VideoDbError::InvalidState {
                current: expected_version.to_string(),
                operation: "更新图片发布版本".to_string(),
            })?;
    let now = Utc::now().naive_utc();
    let mut active: image_publication::ActiveModel = publication.into();
    active.metadata_json = Set(Some(metadata_json));
    active.version = Set(next_version);
    active.updated_at = Set(now);
    let publication = active.update(&txn).await.map_err(VideoDbError::Database)?;
    txn.commit().await.map_err(VideoDbError::Database)?;
    Ok(publication)
}

pub async fn create_image_publication_intent<C>(
    db: &C,
    input: CreateImagePublicationIntentInput,
) -> Result<image_publication::Model, VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let input = normalize_intent_input(input)?;
    let txn = db.begin().await.map_err(VideoDbError::Database)?;
    lock_generated_media_project(&txn, input.project_id).await?;

    let now = Utc::now().naive_utc();
    let image_input =
        validate_create_generated_image_record_input(&txn, input.image.clone()).await?;
    let active = image_publication::ActiveModel {
        publication_id: Set(input.publication_id.clone()),
        project_id: Set(input.project_id),
        requested_by_user_id: Set(input.requested_by_user_id),
        staging_path: Set(input.staging_path.clone()),
        final_path: Set(input.final_path.clone()),
        image_type: Set(image_input.image_type.clone()),
        prompt: Set(image_input.prompt.clone()),
        model: Set(image_input.model.clone()),
        width: Set(image_input.width),
        height: Set(image_input.height),
        expected_file_size: Set(input.expected_file_size),
        expected_sha256: Set(input.expected_sha256.clone()),
        file_size: Set(None),
        file_sha256: Set(None),
        chapter_id: Set(image_input.chapter_id),
        panel_id: Set(image_input.panel_id.clone()),
        metadata_json: Set(image_input.metadata_json.clone()),
        status: Set(IMAGE_PUBLICATION_STATUS_PENDING.to_string()),
        version: Set(1),
        image_id: Set(None),
        last_error: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };
    image_publication::Entity::insert(active)
        .on_conflict(
            OnConflict::column(image_publication::Column::PublicationId)
                .do_nothing()
                .to_owned(),
        )
        .exec_without_returning(&txn)
        .await
        .map_err(VideoDbError::Database)?;

    let publication = image_publication::Entity::find_by_id(&input.publication_id)
        .one(&txn)
        .await
        .map_err(VideoDbError::Database)?
        .ok_or_else(|| VideoDbError::InvalidState {
            current: input.publication_id.clone(),
            operation: "读取图片发布意图".to_string(),
        })?;
    if input.allow_pending_replacement {
        if publication.status == IMAGE_PUBLICATION_STATUS_PUBLISHED {
            ensure_same_logical_intent(&publication, &input, &image_input)?;
            txn.commit().await.map_err(VideoDbError::Database)?;
            return Ok(publication);
        }
        if publication.status == IMAGE_PUBLICATION_STATUS_PENDING
            && same_logical_intent(&publication, &input, &image_input)
            && same_output_intent(&publication, &input, &image_input)
        {
            txn.commit().await.map_err(VideoDbError::Database)?;
            return Ok(publication);
        }
        if publication.status == IMAGE_PUBLICATION_STATUS_PENDING {
            let publication =
                replace_pending_image_publication_intent(&txn, publication, &input, &image_input)
                    .await?;
            txn.commit().await.map_err(VideoDbError::Database)?;
            return Ok(publication);
        }
    }
    ensure_same_intent(&publication, &input, &image_input)?;
    txn.commit().await.map_err(VideoDbError::Database)?;
    Ok(publication)
}

pub async fn image_publication_by_id<C>(
    db: &C,
    publication_id: &str,
) -> Result<Option<image_publication::Model>, VideoDbError>
where
    C: ConnectionTrait,
{
    let publication_id = required_publication_id(publication_id)?;
    image_publication::Entity::find_by_id(publication_id)
        .one(db)
        .await
        .map_err(VideoDbError::Database)
}

pub async fn image_publication_by_image_id<C>(
    db: &C,
    image_id: i64,
) -> Result<Option<image_publication::Model>, VideoDbError>
where
    C: ConnectionTrait,
{
    if image_id <= 0 {
        return Err(VideoDbError::InvalidArgument(
            "image_id 必须是正整数".to_string(),
        ));
    }
    image_publication::Entity::find()
        .filter(image_publication::Column::ImageId.eq(image_id))
        .one(db)
        .await
        .map_err(VideoDbError::Database)
}

pub async fn active_image_publications_for_paths<C>(
    db: &C,
    project_id: i64,
    paths: &[String],
) -> Result<Vec<image_publication::Model>, VideoDbError>
where
    C: ConnectionTrait,
{
    if paths.is_empty() {
        return Ok(Vec::new());
    }
    image_publication::Entity::find()
        .filter(image_publication::Column::ProjectId.eq(project_id))
        .filter(image_publication::Column::Status.is_in([
            IMAGE_PUBLICATION_STATUS_PENDING,
            IMAGE_PUBLICATION_STATUS_PUBLISHED,
        ]))
        .filter(
            Condition::any()
                .add(image_publication::Column::StagingPath.is_in(paths.to_vec()))
                .add(image_publication::Column::FinalPath.is_in(paths.to_vec())),
        )
        .all(db)
        .await
        .map_err(VideoDbError::Database)
}

pub async fn project_has_active_image_publication<C>(
    db: &C,
    project_id: i64,
) -> Result<bool, VideoDbError>
where
    C: ConnectionTrait,
{
    image_publication::Entity::find()
        .select_only()
        .column(image_publication::Column::PublicationId)
        .filter(image_publication::Column::ProjectId.eq(project_id))
        .filter(image_publication::Column::Status.is_in([
            IMAGE_PUBLICATION_STATUS_PENDING,
            IMAGE_PUBLICATION_STATUS_PUBLISHED,
        ]))
        .into_tuple::<String>()
        .one(db)
        .await
        .map(|publication| publication.is_some())
        .map_err(VideoDbError::Database)
}

pub(crate) async fn reclaim_image_publications_for_deleted_images<C>(
    db: &C,
    image_ids: &[i64],
) -> Result<u64, sea_orm::DbErr>
where
    C: ConnectionTrait,
{
    let mut image_ids = image_ids
        .iter()
        .copied()
        .filter(|image_id| *image_id > 0)
        .collect::<Vec<_>>();
    image_ids.sort_unstable();
    image_ids.dedup();
    if image_ids.is_empty() {
        return Ok(0);
    }

    image_publication::Entity::update_many()
        .col_expr(
            image_publication::Column::Status,
            Expr::value(IMAGE_PUBLICATION_STATUS_RECLAIMING.to_string()),
        )
        .col_expr(
            image_publication::Column::Version,
            Expr::col(image_publication::Column::Version).add(1),
        )
        .col_expr(
            image_publication::Column::LastError,
            Expr::value(Some("generated_image 记录已删除".to_string())),
        )
        .col_expr(
            image_publication::Column::UpdatedAt,
            Expr::value(Utc::now().naive_utc()),
        )
        .filter(image_publication::Column::ImageId.is_in(image_ids))
        .filter(image_publication::Column::Status.eq(IMAGE_PUBLICATION_STATUS_PUBLISHED))
        .exec(db)
        .await
        .map(|result| result.rows_affected)
}

pub async fn publish_image_publication<C>(
    db: &C,
    publication_id: &str,
    facts: ImagePublicationFileFacts,
) -> Result<GeneratedImageRecord, VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    publish_image_publication_inner(db, publication_id, facts, None)
        .await
        .map(generated_image_record_from_model)
}

pub async fn publish_image_publication_with_generation_run_checkpoint<C>(
    db: &C,
    publication_id: &str,
    facts: ImagePublicationFileFacts,
    generation_run: crate::generation::RecordCompletedGenerationRunWithCheckpointInput,
) -> Result<GeneratedImageRecord, VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    publish_image_publication_inner(db, publication_id, facts, Some(generation_run))
        .await
        .map(generated_image_record_from_model)
}

async fn publish_image_publication_inner<C>(
    db: &C,
    publication_id: &str,
    facts: ImagePublicationFileFacts,
    mut generation_run: Option<crate::generation::RecordCompletedGenerationRunWithCheckpointInput>,
) -> Result<generated_image::Model, VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let publication_id = required_publication_id(publication_id)?;
    let facts = normalize_file_facts(facts)?;
    let txn = db.begin().await.map_err(VideoDbError::Database)?;
    let snapshot = image_publication::Entity::find_by_id(&publication_id)
        .one(&txn)
        .await
        .map_err(VideoDbError::Database)?
        .ok_or_else(|| VideoDbError::NotFound(format!("图片发布意图不存在: {publication_id}")))?;
    lock_generated_media_project_for_update(&txn, snapshot.project_id).await?;
    let publication = image_publication::Entity::find_by_id(&publication_id)
        .lock_exclusive()
        .one(&txn)
        .await
        .map_err(VideoDbError::Database)?
        .ok_or_else(|| VideoDbError::NotFound(format!("图片发布意图不存在: {publication_id}")))?;
    if publication.status == IMAGE_PUBLICATION_STATUS_PUBLISHED {
        let image = published_image_for_replay(&txn, &publication, &facts).await?;
        txn.commit().await.map_err(VideoDbError::Database)?;
        return Ok(image);
    }
    if publication.status != IMAGE_PUBLICATION_STATUS_PENDING {
        return Err(VideoDbError::InvalidState {
            current: publication.status,
            operation: "发布图片".to_string(),
        });
    }
    lock_generated_media_project(&txn, publication.project_id).await?;
    if facts.file_size != publication.expected_file_size
        || facts.sha256 != publication.expected_sha256
    {
        return Err(VideoDbError::InvalidArgument(format!(
            "图片文件事实与发布意图不一致: publication_id={publication_id}"
        )));
    }

    let image_input = publication_image_input(&publication);
    let image_input = validate_create_generated_image_record_input(&txn, image_input).await?;
    if let Some(generation_run) = generation_run.as_ref()
        && generation_run.run.project_id != image_input.project_id
    {
        return Err(VideoDbError::InvalidArgument(
            "图片生成运行记录 project_id 与图片发布意图不一致".to_string(),
        ));
    }
    let image = insert_generated_image_record(&txn, image_input)
        .await
        .map_err(VideoDbError::Database)?;
    if let Some(mut generation_run) = generation_run.take() {
        let target = GenerationRunTraceTarget {
            target_kind: image.image_type.clone(),
            target_table: "generated_image".to_string(),
            target_id: image.id,
        };
        generation_run.run.targets.push(target.clone());
        generation_run.run.response_summary_json =
            image_generation_response_summary(generation_run.run.response_summary_json, &image)?;
        generation_run.checkpoint.parsed_output_json =
            image_generation_parsed_output(generation_run.checkpoint.parsed_output_json, &image)?;
        generation_run.checkpoint.artifact_refs_json = Some(
            crate::generation::append_generation_artifact_target(
                generation_run.checkpoint.artifact_refs_json,
                target,
            )
            .map_err(VideoDbError::from)?,
        );
        crate::generation::record_completed_generation_run_with_checkpoint_in(
            &txn,
            generation_run.run,
            generation_run.checkpoint,
        )
        .await
        .map_err(VideoDbError::from)?;
    }
    let now = Utc::now().naive_utc();
    let next_version =
        publication
            .version
            .checked_add(1)
            .ok_or_else(|| VideoDbError::InvalidState {
                current: publication.version.to_string(),
                operation: "更新图片发布版本".to_string(),
            })?;
    let mut active: image_publication::ActiveModel = publication.into();
    active.status = Set(IMAGE_PUBLICATION_STATUS_PUBLISHED.to_string());
    active.file_size = Set(Some(facts.file_size));
    active.file_sha256 = Set(Some(facts.sha256));
    active.image_id = Set(Some(image.id));
    active.version = Set(next_version);
    active.last_error = Set(None);
    active.updated_at = Set(now);
    active.update(&txn).await.map_err(VideoDbError::Database)?;
    txn.commit().await.map_err(VideoDbError::Database)?;
    Ok(image)
}

async fn published_image_for_replay<C>(
    db: &C,
    publication: &image_publication::Model,
    facts: &ImagePublicationFileFacts,
) -> Result<generated_image::Model, VideoDbError>
where
    C: ConnectionTrait,
{
    if facts.file_size != publication.expected_file_size
        || facts.sha256 != publication.expected_sha256
    {
        return Err(VideoDbError::InvalidArgument(format!(
            "图片文件事实与已发布意图不一致: publication_id={}",
            publication.publication_id
        )));
    }
    if publication.file_size != Some(facts.file_size)
        || publication.file_sha256.as_deref() != Some(facts.sha256.as_str())
    {
        return Err(VideoDbError::InvalidState {
            current: format!(
                "published:file_size={:?},file_sha256_present={}",
                publication.file_size,
                publication.file_sha256.is_some()
            ),
            operation: "读取已发布图片文件事实".to_string(),
        });
    }
    let image = match publication.image_id {
        Some(image_id) => generated_image::Entity::find_by_id(image_id)
            .one(db)
            .await
            .map_err(VideoDbError::Database)?,
        None => None,
    };
    let Some(image) = image else {
        let image_input =
            validate_create_generated_image_record_input(db, publication_image_input(publication))
                .await?;
        let image = insert_generated_image_record(db, image_input)
            .await
            .map_err(VideoDbError::Database)?;
        let next_version =
            publication
                .version
                .checked_add(1)
                .ok_or_else(|| VideoDbError::InvalidState {
                    current: publication.version.to_string(),
                    operation: "更新图片发布版本".to_string(),
                })?;
        let mut active: image_publication::ActiveModel = publication.clone().into();
        active.image_id = Set(Some(image.id));
        active.version = Set(next_version);
        active.last_error = Set(None);
        active.updated_at = Set(Utc::now().naive_utc());
        active.update(db).await.map_err(VideoDbError::Database)?;
        return Ok(image);
    };
    if image.project_id != publication.project_id
        || image.file_path != publication.final_path
        || image.file_size != Some(publication.expected_file_size)
        || image.image_type != publication.image_type
    {
        return Err(VideoDbError::InvalidState {
            current: format!("published:image_id={:?}", publication.image_id),
            operation: "校验已发布图片身份".to_string(),
        });
    }
    Ok(image)
}

pub async fn image_publications_for_recovery<C>(
    db: &C,
    limit: usize,
    published_after: Option<&str>,
) -> Result<Vec<image_publication::Model>, VideoDbError>
where
    C: ConnectionTrait,
{
    image_publications_for_recovery_for_model(db, limit, published_after, None).await
}

pub async fn image_publications_for_recovery_for_model<C>(
    db: &C,
    limit: usize,
    published_after: Option<&str>,
    model: Option<&str>,
) -> Result<Vec<image_publication::Model>, VideoDbError>
where
    C: ConnectionTrait,
{
    let limit = u64::try_from(limit)
        .map_err(|_| VideoDbError::InvalidArgument("图片发布恢复 limit 无效".to_string()))?;
    let mut pending_query = image_publication::Entity::find()
        .filter(image_publication::Column::Status.is_in([
            IMAGE_PUBLICATION_STATUS_PENDING,
            IMAGE_PUBLICATION_STATUS_RECLAIMING,
        ]))
        .order_by_asc(image_publication::Column::UpdatedAt)
        .limit(limit);
    if let Some(model) = model {
        pending_query = pending_query.filter(image_publication::Column::Model.eq(model));
    }
    let mut publications = pending_query
        .all(db)
        .await
        .map_err(VideoDbError::Database)?;
    if publications.len() < limit as usize {
        let published_limit = limit as usize - publications.len();
        let mut published_query = image_publication::Entity::find()
            .filter(image_publication::Column::Status.eq(IMAGE_PUBLICATION_STATUS_PUBLISHED))
            .order_by_asc(image_publication::Column::PublicationId)
            .limit(published_limit as u64);
        if let Some(model) = model {
            published_query = published_query.filter(image_publication::Column::Model.eq(model));
        }
        let page_query = match published_after {
            Some(after) => published_query
                .clone()
                .filter(image_publication::Column::PublicationId.gt(after)),
            None => published_query.clone(),
        };
        let mut published = page_query.all(db).await.map_err(VideoDbError::Database)?;
        if published.is_empty() && published_after.is_some() {
            // Complete the cycle without changing domain timestamps just for an audit.
            published = published_query
                .all(db)
                .await
                .map_err(VideoDbError::Database)?;
        }
        publications.extend(published);
    }
    Ok(publications)
}

pub async fn mark_image_publication_reclaiming<C>(
    db: &C,
    publication_id: &str,
    version: i64,
    reason: String,
) -> Result<image_publication::Model, VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let publication_id = required_publication_id(publication_id)?;
    if version <= 0 {
        return Err(VideoDbError::InvalidArgument(
            "图片发布版本必须大于 0".to_string(),
        ));
    }
    let reason = required_reason(reason)?;
    let now = Utc::now().naive_utc();
    let next_version = version
        .checked_add(1)
        .ok_or_else(|| VideoDbError::InvalidState {
            current: version.to_string(),
            operation: "更新图片发布版本".to_string(),
        })?;
    let txn = db.begin().await.map_err(VideoDbError::Database)?;
    let snapshot = image_publication::Entity::find_by_id(&publication_id)
        .one(&txn)
        .await
        .map_err(VideoDbError::Database)?
        .ok_or_else(|| VideoDbError::NotFound(format!("图片发布意图不存在: {publication_id}")))?;
    lock_image_publication_reclaim_project_for_update(&txn, snapshot.project_id).await?;
    let result = image_publication::Entity::update_many()
        .col_expr(
            image_publication::Column::Status,
            sea_orm::sea_query::Expr::value(IMAGE_PUBLICATION_STATUS_RECLAIMING),
        )
        .col_expr(
            image_publication::Column::Version,
            sea_orm::sea_query::Expr::value(next_version),
        )
        .col_expr(
            image_publication::Column::LastError,
            sea_orm::sea_query::Expr::value(Some(reason)),
        )
        .col_expr(
            image_publication::Column::UpdatedAt,
            sea_orm::sea_query::Expr::value(now),
        )
        .filter(image_publication::Column::PublicationId.eq(&publication_id))
        .filter(image_publication::Column::Version.eq(version))
        .filter(image_publication::Column::Status.is_in([
            IMAGE_PUBLICATION_STATUS_PENDING,
            IMAGE_PUBLICATION_STATUS_PUBLISHED,
        ]))
        .exec(&txn)
        .await
        .map_err(VideoDbError::Database)?;
    let publication = image_publication::Entity::find_by_id(&publication_id)
        .one(&txn)
        .await
        .map_err(VideoDbError::Database)?
        .ok_or_else(|| VideoDbError::NotFound(format!("图片发布意图不存在: {publication_id}")))?;
    if result.rows_affected == 0
        && !(publication.status == IMAGE_PUBLICATION_STATUS_RECLAIMING
            && publication.version >= version)
    {
        return Err(VideoDbError::InvalidState {
            current: publication.status,
            operation: "转入图片回收".to_string(),
        });
    }
    txn.commit().await.map_err(VideoDbError::Database)?;
    Ok(publication)
}

pub async fn mark_image_publication_reclaimed<C>(
    db: &C,
    publication_id: &str,
    version: i64,
    reason: Option<String>,
) -> Result<image_publication::Model, VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let publication_id = required_publication_id(publication_id)?;
    if version <= 0 {
        return Err(VideoDbError::InvalidArgument(
            "图片发布版本必须大于 0".to_string(),
        ));
    }
    let reason = reason.map(required_reason).transpose()?;
    let now = Utc::now().naive_utc();
    let next_version = version
        .checked_add(1)
        .ok_or_else(|| VideoDbError::InvalidState {
            current: version.to_string(),
            operation: "更新图片发布版本".to_string(),
        })?;
    let txn = db.begin().await.map_err(VideoDbError::Database)?;
    let snapshot = image_publication::Entity::find_by_id(&publication_id)
        .one(&txn)
        .await
        .map_err(VideoDbError::Database)?
        .ok_or_else(|| VideoDbError::NotFound(format!("图片发布意图不存在: {publication_id}")))?;
    lock_image_publication_reclaim_project_for_update(&txn, snapshot.project_id).await?;
    let result = image_publication::Entity::update_many()
        .col_expr(
            image_publication::Column::Status,
            sea_orm::sea_query::Expr::value(IMAGE_PUBLICATION_STATUS_RECLAIMED),
        )
        .col_expr(
            image_publication::Column::Version,
            sea_orm::sea_query::Expr::value(next_version),
        )
        .col_expr(
            image_publication::Column::LastError,
            sea_orm::sea_query::Expr::value(reason),
        )
        .col_expr(
            image_publication::Column::UpdatedAt,
            sea_orm::sea_query::Expr::value(now),
        )
        .filter(image_publication::Column::PublicationId.eq(&publication_id))
        .filter(image_publication::Column::Version.eq(version))
        .filter(image_publication::Column::Status.eq(IMAGE_PUBLICATION_STATUS_RECLAIMING))
        .exec(&txn)
        .await
        .map_err(VideoDbError::Database)?;
    let publication = image_publication::Entity::find_by_id(&publication_id)
        .one(&txn)
        .await
        .map_err(VideoDbError::Database)?
        .ok_or_else(|| VideoDbError::NotFound(format!("图片发布意图不存在: {publication_id}")))?;
    if result.rows_affected == 0 && publication.status != IMAGE_PUBLICATION_STATUS_RECLAIMED {
        return Err(VideoDbError::InvalidState {
            current: publication.status,
            operation: "完成图片回收".to_string(),
        });
    }
    txn.commit().await.map_err(VideoDbError::Database)?;
    Ok(publication)
}

fn normalize_intent_input(
    mut input: CreateImagePublicationIntentInput,
) -> Result<CreateImagePublicationIntentInput, VideoDbError> {
    input.publication_id = required_publication_id(&input.publication_id)?;
    if input.project_id <= 0 || input.requested_by_user_id <= 0 {
        return Err(VideoDbError::InvalidArgument(
            "图片发布项目和用户必须为正整数".to_string(),
        ));
    }
    input.staging_path = project_scoped_path(input.project_id, &input.staging_path, "暂存路径")?;
    input.final_path = project_scoped_path(input.project_id, &input.final_path, "最终路径")?;
    if input.staging_path == input.final_path {
        return Err(VideoDbError::InvalidArgument(
            "图片发布暂存路径和最终路径不能相同".to_string(),
        ));
    }
    if input.expected_file_size < 0 {
        return Err(VideoDbError::InvalidArgument(
            "图片预期文件大小不能小于 0".to_string(),
        ));
    }
    input.expected_sha256 = normalize_sha256(&input.expected_sha256)?;
    if input.image.project_id != input.project_id {
        return Err(VideoDbError::InvalidArgument(
            "图片发布项目与图片记录项目不一致".to_string(),
        ));
    }
    if let Some(file_size) = input.image.file_size
        && file_size != input.expected_file_size
    {
        return Err(VideoDbError::InvalidArgument(
            "图片记录文件大小与发布意图不一致".to_string(),
        ));
    }
    input.image.file_path = input.final_path.clone();
    input.image.file_size = Some(input.expected_file_size);
    Ok(input)
}

fn ensure_same_intent(
    publication: &image_publication::Model,
    input: &CreateImagePublicationIntentInput,
    image: &super::CreateGeneratedImageRecordInput,
) -> Result<(), VideoDbError> {
    ensure_same_logical_intent(publication, input, image)?;
    if !same_output_intent(publication, input, image) {
        return Err(VideoDbError::InvalidArgument(
            "publication_id 已绑定到不同的图片发布意图".to_string(),
        ));
    }
    Ok(())
}

fn same_output_intent(
    publication: &image_publication::Model,
    input: &CreateImagePublicationIntentInput,
    image: &super::CreateGeneratedImageRecordInput,
) -> bool {
    publication.staging_path == input.staging_path
        && publication.final_path == input.final_path
        && publication.model == image.model
        && publication.width == image.width
        && publication.height == image.height
        && publication.expected_file_size == input.expected_file_size
        && publication.expected_sha256 == input.expected_sha256
        && metadata_matches_replayed_portrait(
            &publication.image_type,
            publication.metadata_json.as_ref(),
            image.metadata_json.as_ref(),
        )
}

fn ensure_same_logical_intent(
    publication: &image_publication::Model,
    input: &CreateImagePublicationIntentInput,
    image: &super::CreateGeneratedImageRecordInput,
) -> Result<(), VideoDbError> {
    validate_image_publication_request(
        publication,
        ImagePublicationRequest {
            project_id: input.project_id,
            requested_by_user_id: input.requested_by_user_id,
            image_type: &image.image_type,
            prompt: &image.prompt,
            chapter_id: image.chapter_id,
            panel_id: image.panel_id.as_deref(),
        },
    )
}

fn same_logical_intent(
    publication: &image_publication::Model,
    input: &CreateImagePublicationIntentInput,
    image: &super::CreateGeneratedImageRecordInput,
) -> bool {
    ensure_same_logical_intent(publication, input, image).is_ok()
}

async fn replace_pending_image_publication_intent<C>(
    db: &C,
    publication: image_publication::Model,
    input: &CreateImagePublicationIntentInput,
    image: &super::CreateGeneratedImageRecordInput,
) -> Result<image_publication::Model, VideoDbError>
where
    C: ConnectionTrait,
{
    let mut stale_paths = Vec::new();
    if publication.staging_path != input.staging_path {
        stale_paths.push(publication.staging_path.clone());
    }
    if publication.final_path != input.final_path {
        stale_paths.push(publication.final_path.clone());
    }
    if !stale_paths.is_empty() {
        crate::cleanup_outbox::insert_file_artifact_cleanup_intent(
            db,
            format!(
                "image-publication-retry:{}:{}",
                publication.publication_id, publication.version
            ),
            crate::cleanup_outbox::FileArtifactCleanupPayload::new(
                publication.project_id,
                stale_paths,
            ),
        )
        .await
        .map_err(VideoDbError::Database)?;
    }
    let next_version =
        publication
            .version
            .checked_add(1)
            .ok_or_else(|| VideoDbError::InvalidState {
                current: publication.version.to_string(),
                operation: "更新图片发布版本".to_string(),
            })?;
    let now = Utc::now().naive_utc();
    let mut active: image_publication::ActiveModel = publication.into();
    active.staging_path = Set(input.staging_path.clone());
    active.final_path = Set(input.final_path.clone());
    active.image_type = Set(image.image_type.clone());
    active.prompt = Set(image.prompt.clone());
    active.model = Set(image.model.clone());
    active.width = Set(image.width);
    active.height = Set(image.height);
    active.chapter_id = Set(image.chapter_id);
    active.panel_id = Set(image.panel_id.clone());
    active.expected_file_size = Set(input.expected_file_size);
    active.expected_sha256 = Set(input.expected_sha256.clone());
    active.file_size = Set(None);
    active.file_sha256 = Set(None);
    active.image_id = Set(None);
    active.metadata_json = Set(image.metadata_json.clone());
    active.status = Set(IMAGE_PUBLICATION_STATUS_PENDING.to_string());
    active.version = Set(next_version);
    active.last_error = Set(None);
    active.updated_at = Set(now);
    active.update(db).await.map_err(VideoDbError::Database)
}

fn publication_image_input(
    publication: &image_publication::Model,
) -> CreateGeneratedImageRecordDraftInput {
    CreateGeneratedImageRecordDraftInput {
        project_id: publication.project_id,
        image_type: publication.image_type.clone(),
        file_path: publication.final_path.clone(),
        prompt: publication.prompt.clone(),
        model: publication.model.clone(),
        width: publication.width,
        height: publication.height,
        file_size: Some(publication.expected_file_size),
        chapter_id: publication.chapter_id,
        panel_id: publication.panel_id.clone(),
        metadata_json: publication.metadata_json.clone(),
        created_at: Some(publication.created_at),
    }
}

fn metadata_matches_replayed_portrait(
    image_type: &str,
    stored: Option<&Value>,
    incoming: Option<&Value>,
) -> bool {
    if stored == incoming {
        return true;
    }
    if image_type != "portrait" {
        return false;
    }
    let (Some(stored), Some(incoming)) = (stored.and_then(Value::as_object), incoming) else {
        return false;
    };
    let Some(incoming) = incoming.as_object() else {
        return false;
    };
    stored.get("portrait_id").and_then(Value::as_i64).is_some()
        && incoming
            .iter()
            .all(|(key, value)| stored.get(key) == Some(value))
}

fn project_scoped_path(project_id: i64, path: &str, label: &str) -> Result<String, VideoDbError> {
    let path = path.trim();
    let prefix = format!("projects/{project_id}/");
    let raw = std::path::Path::new(path);
    if path.is_empty()
        || !path.starts_with(&prefix)
        || path.len() == prefix.len()
        || path.ends_with('/')
        || path.contains('\\')
        || raw.is_absolute()
        || raw
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(VideoDbError::InvalidArgument(format!(
            "图片发布{label}必须是项目目录内的安全相对路径"
        )));
    }
    Ok(path.to_string())
}

fn required_publication_id(value: &str) -> Result<String, VideoDbError> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':'))
    {
        return Err(VideoDbError::InvalidArgument(
            "图片 publication_id 无效".to_string(),
        ));
    }
    Ok(value.to_string())
}

fn normalize_sha256(value: &str) -> Result<String, VideoDbError> {
    let value = value.trim().to_ascii_lowercase();
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(VideoDbError::InvalidArgument(
            "图片 SHA-256 必须是 64 位十六进制字符串".to_string(),
        ));
    }
    Ok(value)
}

fn normalize_file_facts(
    mut facts: ImagePublicationFileFacts,
) -> Result<ImagePublicationFileFacts, VideoDbError> {
    if facts.file_size < 0 {
        return Err(VideoDbError::InvalidArgument(
            "图片实际文件大小不能小于 0".to_string(),
        ));
    }
    facts.sha256 = normalize_sha256(&facts.sha256)?;
    Ok(facts)
}

fn required_reason(value: String) -> Result<String, VideoDbError> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(VideoDbError::InvalidArgument(
            "图片发布状态变更原因不能为空".to_string(),
        ));
    }
    Ok(value)
}

#[cfg(test)]
mod tests;
