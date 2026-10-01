use super::super::{
    WORKFLOW_STATUS_COMPLETED,
    types::{
        CanvasMediaSourceContext, CanvasVideoMediaAssetEnsureInput,
        CanvasVideoMediaSourceEnsureInput,
    },
};
use super::{
    CanvasMediaAssetUpsertInput, CanvasMediaAssetUpsertRecord,
    binding::{ensure_canvas_generated_video_bindings, ensure_canvas_media_binding},
    context::canvas_media_source_context_for_update,
    lookup::canvas_generated_video_for_media_ensure,
    provenance::{
        upsert_canvas_video_media_provenance, upsert_canvas_video_media_provenance_records,
    },
    upsert_canvas_media_asset_record, upsert_canvas_media_asset_records,
};
use crate::canvas::CanvasDbError;
use crate::media::media_asset_record_from_model;
use a4n_db_models::models::media::media_asset;
use sea_orm::{ConnectionTrait, TransactionTrait};

pub async fn ensure_canvas_video_media_asset_record<C>(
    db: &C,
    input: CanvasVideoMediaAssetEnsureInput,
) -> Result<a4n_domain::media::MediaAssetRecord, CanvasDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let tx = db.begin().await?;
    let asset = ensure_canvas_video_media_asset_record_in(&tx, input).await?;
    tx.commit().await?;
    Ok(media_asset_record_from_model(asset))
}

pub(in crate::canvas::artifacts) async fn ensure_canvas_video_media_asset_record_in<C>(
    db: &C,
    input: CanvasVideoMediaAssetEnsureInput,
) -> Result<media_asset::Model, CanvasDbError>
where
    C: ConnectionTrait,
{
    let expected_chapter_id = input.video.chapter_id;
    let source_context =
        canvas_media_source_context_for_update(db, input.project_id, expected_chapter_id).await?;
    let video =
        canvas_generated_video_for_media_ensure(db, input.project_id, input.video.id).await?;
    if video.chapter_id != expected_chapter_id {
        return Err(CanvasDbError::InvalidArgument(format!(
            "视频 {} 的章节归属在媒体登记期间发生变化，请重试",
            video.id
        )));
    }
    ensure_canvas_video_media_asset_record_with_context(
        db,
        CanvasVideoMediaSourceEnsureInput {
            project_id: input.project_id,
            video: video.into(),
            generation_spec_id: input.generation_spec_id,
            origin: input.origin,
        },
        &source_context,
    )
    .await
}

pub(in crate::canvas::artifacts) async fn ensure_canvas_video_media_asset_record_from_source_in<C>(
    db: &C,
    input: CanvasVideoMediaSourceEnsureInput,
    source_context: &CanvasMediaSourceContext,
) -> Result<media_asset::Model, CanvasDbError>
where
    C: ConnectionTrait,
{
    ensure_canvas_video_media_asset_record_with_context(db, input, source_context).await
}

pub(in crate::canvas::artifacts) async fn ensure_canvas_video_media_asset_records_from_source_in<
    C,
>(
    db: &C,
    inputs: Vec<CanvasVideoMediaSourceEnsureInput>,
    source_contexts: &[CanvasMediaSourceContext],
) -> Result<Vec<media_asset::Model>, CanvasDbError>
where
    C: ConnectionTrait,
{
    if inputs.len() != source_contexts.len() {
        return Err(CanvasDbError::InvalidArgument(format!(
            "画布视频媒体来源与上下文数量不一致: videos={}, contexts={}",
            inputs.len(),
            source_contexts.len()
        )));
    }
    if inputs.is_empty() {
        return Ok(Vec::new());
    }
    let records = inputs
        .iter()
        .zip(source_contexts)
        .map(|(input, source_context)| {
            canvas_video_media_asset_upsert_record(input, source_context)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let assets = upsert_canvas_media_asset_records(db, records).await?;
    let videos = inputs
        .iter()
        .map(|input| input.video.clone())
        .collect::<Vec<_>>();
    let generation_spec_ids = inputs
        .iter()
        .map(|input| input.generation_spec_id)
        .collect::<Vec<_>>();
    upsert_canvas_video_media_provenance_records(db, &assets, &videos, &generation_spec_ids)
        .await?;
    let bindings = assets
        .iter()
        .zip(&inputs)
        .map(|(asset, input)| (asset.id, input.video.id))
        .collect::<Vec<_>>();
    ensure_canvas_generated_video_bindings(db, &bindings).await?;
    Ok(assets)
}

async fn ensure_canvas_video_media_asset_record_with_context<C>(
    db: &C,
    input: CanvasVideoMediaSourceEnsureInput,
    source_context: &CanvasMediaSourceContext,
) -> Result<media_asset::Model, CanvasDbError>
where
    C: ConnectionTrait,
{
    let record = canvas_video_media_asset_upsert_record(&input, source_context)?;
    let asset = upsert_canvas_media_asset_record(db, record.input, &record.source_context).await?;

    upsert_canvas_video_media_provenance(db, &asset, &input.video, input.generation_spec_id)
        .await?;
    ensure_canvas_media_binding(
        db,
        asset.id,
        "generated_video",
        input.video.id,
        "generated_result",
    )
    .await?;
    Ok(asset)
}

fn canvas_video_media_asset_upsert_record(
    input: &CanvasVideoMediaSourceEnsureInput,
    source_context: &CanvasMediaSourceContext,
) -> Result<CanvasMediaAssetUpsertRecord, CanvasDbError> {
    if input.video.status != WORKFLOW_STATUS_COMPLETED {
        return Err(CanvasDbError::InvalidArgument(format!(
            "视频尚未完成，不能登记为媒体资产: {}",
            input.video.id
        )));
    }
    let file_path = input
        .video
        .file_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            CanvasDbError::InvalidArgument(format!(
                "视频缺少文件路径，不能登记为媒体资产: {}",
                input.video.id
            ))
        })?
        .to_string();
    Ok(CanvasMediaAssetUpsertRecord {
        input: CanvasMediaAssetUpsertInput {
            project_id: input.project_id,
            media_kind: "video",
            origin: input.origin.clone(),
            file_path,
            thumbnail_path: input.video.thumbnail_path.clone(),
            width: Some(input.video.width),
            height: Some(input.video.height),
            duration_secs: Some(input.video.duration_secs),
            fps: Some(input.video.fps as f32),
            file_size: input.video.file_size,
        },
        source_context: source_context.clone(),
    })
}
