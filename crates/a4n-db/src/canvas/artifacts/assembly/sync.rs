use sea_orm::{ConnectionTrait, TransactionTrait};

use crate::canvas::CanvasDbError;

use super::super::media::{ensure_canvas_media_binding, ensure_canvas_video_media_asset_record_in};
use super::super::types::{
    CanvasAssemblyContractSyncInput, CanvasAssemblyContractSyncSummary,
    CanvasVideoMediaAssetEnsureInput,
};
use super::segments::replace_canvas_assembly_segments;
use super::timeline::upsert_canvas_assembly_timeline;
use super::videos::{
    canvas_assembly_result_video, canvas_assembly_source_video, canvas_video_media_origin,
};

pub async fn sync_canvas_assembly_contract_records<C>(
    db: &C,
    input: CanvasAssemblyContractSyncInput,
) -> Result<CanvasAssemblyContractSyncSummary, CanvasDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let tx = db.begin().await?;
    let summary = sync_canvas_assembly_contract_records_in(&tx, input).await?;
    tx.commit().await?;
    Ok(summary)
}

async fn sync_canvas_assembly_contract_records_in<C>(
    db: &C,
    input: CanvasAssemblyContractSyncInput,
) -> Result<CanvasAssemblyContractSyncSummary, CanvasDbError>
where
    C: ConnectionTrait,
{
    let mut source_assets = Vec::with_capacity(input.source_videos.len());
    for source_video in input.source_videos {
        let video =
            canvas_assembly_source_video(db, input.project_id, source_video.video_id).await?;
        let origin = canvas_video_media_origin(&video).to_string();
        source_assets.push(
            ensure_canvas_video_media_asset_record_in(
                db,
                CanvasVideoMediaAssetEnsureInput {
                    project_id: input.project_id,
                    video,
                    generation_spec_id: None,
                    origin,
                },
            )
            .await?,
        );
    }

    let assembly_row = canvas_assembly_result_video(db, input.assembly_video_id).await?;
    let preview_asset = ensure_canvas_video_media_asset_record_in(
        db,
        CanvasVideoMediaAssetEnsureInput {
            project_id: input.project_id,
            video: assembly_row.clone(),
            generation_spec_id: input.generation_spec_id,
            origin: "assembled_preview".to_string(),
        },
    )
    .await?;
    ensure_canvas_media_binding(
        db,
        preview_asset.id,
        "generated_video",
        assembly_row.id,
        "generated_result",
    )
    .await?;

    let timeline = upsert_canvas_assembly_timeline(
        db,
        input.project_id,
        input.graph_id,
        input.node_id,
        preview_asset.id,
    )
    .await?;
    replace_canvas_assembly_segments(db, input.project_id, timeline.id, &source_assets).await?;
    ensure_canvas_media_binding(
        db,
        preview_asset.id,
        "assembly_timeline",
        timeline.id,
        "generated_result",
    )
    .await?;

    Ok(CanvasAssemblyContractSyncSummary {
        preview_media_asset_id: preview_asset.id,
    })
}
