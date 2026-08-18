use crate::canvas::CanvasDbError;
use a4n_domain::{EXTERNAL_IMPORT_MODEL, GeneratedMediaReference};
use a4n_core::models::{
    ai::{generated_image, generated_video},
    generation::generation_spec,
    media::{media_asset, media_provenance},
};
use sea_orm::{ActiveModelTrait, ConnectionTrait, DbBackend, EntityTrait, QuerySelect, Set};

pub(super) async fn upsert_canvas_image_media_provenance<C>(
    db: &C,
    asset: &media_asset::Model,
    image: &generated_image::Model,
    generation_spec_id: Option<i64>,
    now: chrono::NaiveDateTime,
) -> Result<(), CanvasDbError>
where
    C: ConnectionTrait,
{
    validate_canvas_media_generation_spec(db, asset, generation_spec_id).await?;
    let request_snapshot_json = Some(serde_json::json!({
        "source": "video_canvas",
        "generated_image_id": image.id,
        "image_type": image.image_type,
        "metadata": image.metadata_json,
    }));
    let existing = canvas_media_provenance_for_update(db, asset.id).await?;
    let model = media_provenance::ActiveModel {
        media_asset_id: Set(asset.id),
        generation_spec_id: Set(generation_spec_id),
        prompt_artifact_id: Set(None),
        provider_job_id: Set(None),
        source_media_asset_id: Set(None),
        source_external_path: Set(Some(GeneratedMediaReference::Image(image.id).to_string())),
        imported_from_user: Set(image.model == EXTERNAL_IMPORT_MODEL),
        request_snapshot_json: Set(request_snapshot_json),
        created_at: Set(now),
    };
    if existing.is_some() {
        model.update(db).await?;
    } else {
        model.insert(db).await?;
    }
    Ok(())
}

pub(super) async fn upsert_canvas_video_media_provenance<C>(
    db: &C,
    asset: &media_asset::Model,
    video: &generated_video::Model,
    generation_spec_id: Option<i64>,
    now: chrono::NaiveDateTime,
) -> Result<(), CanvasDbError>
where
    C: ConnectionTrait,
{
    validate_canvas_media_generation_spec(db, asset, generation_spec_id).await?;
    let request_snapshot_json = Some(serde_json::json!({
        "source": "video_canvas",
        "generated_video_id": video.id,
        "video_type": video.video_type,
        "workflow_mode": video.workflow_mode,
        "metadata": video.metadata_json,
    }));
    let existing = canvas_media_provenance_for_update(db, asset.id).await?;
    if existing.is_some() {
        media_provenance::ActiveModel {
            media_asset_id: Set(asset.id),
            generation_spec_id: Set(generation_spec_id),
            prompt_artifact_id: Set(None),
            provider_job_id: Set(video.provider_job_id.clone()),
            source_media_asset_id: Set(None),
            source_external_path: Set(Some(GeneratedMediaReference::Video(video.id).to_string())),
            imported_from_user: Set(video.model == EXTERNAL_IMPORT_MODEL),
            request_snapshot_json: Set(request_snapshot_json),
            created_at: Set(now),
        }
        .update(db)
        .await?;
    } else {
        media_provenance::ActiveModel {
            media_asset_id: Set(asset.id),
            generation_spec_id: Set(generation_spec_id),
            prompt_artifact_id: Set(None),
            provider_job_id: Set(video.provider_job_id.clone()),
            source_media_asset_id: Set(None),
            source_external_path: Set(Some(GeneratedMediaReference::Video(video.id).to_string())),
            imported_from_user: Set(video.model == EXTERNAL_IMPORT_MODEL),
            request_snapshot_json: Set(request_snapshot_json),
            created_at: Set(now),
        }
        .insert(db)
        .await?;
    }
    Ok(())
}

async fn validate_canvas_media_generation_spec<C>(
    db: &C,
    asset: &media_asset::Model,
    generation_spec_id: Option<i64>,
) -> Result<(), CanvasDbError>
where
    C: ConnectionTrait,
{
    let Some(generation_spec_id) = generation_spec_id else {
        return Ok(());
    };
    let mut query = generation_spec::Entity::find_by_id(generation_spec_id);
    if db.get_database_backend() != DbBackend::Sqlite {
        query = query.lock_exclusive();
    }
    let spec = query
        .one(db)
        .await?
        .ok_or_else(|| CanvasDbError::NotFound(format!("生成意图不存在: {generation_spec_id}")))?;
    if spec.project_id != asset.project_id || spec.root_project_id != asset.root_project_id {
        return Err(CanvasDbError::InvalidArgument(format!(
            "画布媒体来源生成意图 {} 不属于媒体资产项目族",
            spec.id
        )));
    }
    match asset.source_snapshot_id {
        Some(source_snapshot_id) if spec.source_snapshot_id != Some(source_snapshot_id) => {
            return Err(CanvasDbError::InvalidArgument(format!(
                "画布媒体来源生成意图 {} 的 source_snapshot_id 与媒体资产不一致",
                spec.id
            )));
        }
        None if spec.source_snapshot_id.is_some() || spec.source_snapshot_chapter_id.is_some() => {
            return Err(CanvasDbError::InvalidArgument(format!(
                "freeform 画布媒体资产不能引用带源快照上下文的生成意图 {}",
                spec.id
            )));
        }
        _ => {}
    }
    if let Some(source_snapshot_chapter_id) = asset.source_snapshot_chapter_id
        && spec.source_snapshot_chapter_id != Some(source_snapshot_chapter_id)
    {
        return Err(CanvasDbError::InvalidArgument(format!(
            "画布媒体来源生成意图 {} 的 source_snapshot_chapter_id 与媒体资产不一致",
            spec.id
        )));
    }
    if let Some(source_chapter_id) = asset.source_chapter_id
        && spec.source_chapter_id != Some(source_chapter_id)
    {
        return Err(CanvasDbError::InvalidArgument(format!(
            "画布媒体来源生成意图 {} 的 source_chapter_id 与媒体资产不一致",
            spec.id
        )));
    }
    Ok(())
}

async fn canvas_media_provenance_for_update<C>(
    db: &C,
    media_asset_id: i64,
) -> Result<Option<media_provenance::Model>, CanvasDbError>
where
    C: ConnectionTrait,
{
    let mut query = media_provenance::Entity::find_by_id(media_asset_id);
    if db.get_database_backend() != DbBackend::Sqlite {
        query = query.lock_exclusive();
    }
    Ok(query.one(db).await?)
}
