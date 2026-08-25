use sea_orm::{ConnectionTrait, TransactionTrait};
use std::collections::{BTreeMap, BTreeSet};

use crate::canvas::CanvasDbError;

use super::super::media::{
    canvas_media_source_context_for_update, ensure_canvas_media_binding,
    ensure_canvas_video_media_asset_record_from_source_in,
    ensure_canvas_video_media_asset_records_from_source_in,
};
use super::super::types::{
    CanvasAssemblyContractSyncInput, CanvasAssemblyContractSyncSummary, CanvasMediaSourceContext,
    CanvasVideoMediaSourceEnsureInput,
};
use super::segments::replace_canvas_assembly_segments;
use super::timeline::upsert_canvas_assembly_timeline;
use super::videos::{
    canvas_assembly_result_video, canvas_assembly_result_video_for_update,
    canvas_assembly_source_videos, canvas_video_media_origin,
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
    let CanvasAssemblyContractSyncInput {
        project_id,
        graph_id,
        node_id,
        assembly_video_id,
        source_videos,
        generation_spec_id,
    } = input;
    let source_video_ids = source_videos
        .iter()
        .map(|source_video| source_video.video_id)
        .collect::<Vec<_>>();
    let source_video_probes =
        canvas_assembly_source_videos(db, project_id, &source_video_ids, false).await?;
    let mut source_contexts = BTreeMap::<Option<i64>, CanvasMediaSourceContext>::new();
    for chapter_id in source_video_probes
        .iter()
        .map(|video| video.chapter_id)
        .collect::<BTreeSet<_>>()
    {
        source_contexts.insert(
            chapter_id,
            canvas_media_source_context_for_update(db, project_id, chapter_id).await?,
        );
    }
    let source_video_rows =
        canvas_assembly_source_videos(db, project_id, &source_video_ids, true).await?;
    for (probe, video) in source_video_probes.iter().zip(&source_video_rows) {
        if probe.id != video.id || probe.chapter_id != video.chapter_id {
            return Err(CanvasDbError::InvalidArgument(format!(
                "视频 {} 的章节归属在媒体登记期间发生变化，请重试",
                probe.id
            )));
        }
    }
    let mut source_inputs = Vec::with_capacity(source_video_rows.len());
    let mut source_input_contexts = Vec::with_capacity(source_video_rows.len());
    for video in source_video_rows {
        let origin = canvas_video_media_origin(&video).to_string();
        let source_context = source_contexts.get(&video.chapter_id).ok_or_else(|| {
            CanvasDbError::InvalidArgument(format!("成片源视频 {} 缺少章节媒体上下文", video.id))
        })?;
        source_inputs.push(CanvasVideoMediaSourceEnsureInput {
            project_id,
            video,
            generation_spec_id: None,
            origin,
        });
        source_input_contexts.push(source_context.clone());
    }
    let source_assets = ensure_canvas_video_media_asset_records_from_source_in(
        db,
        source_inputs,
        &source_input_contexts,
    )
    .await?;

    let assembly_probe = canvas_assembly_result_video(db, assembly_video_id).await?;
    let assembly_source_context = match source_contexts.get(&assembly_probe.chapter_id) {
        Some(context) => context.clone(),
        None => {
            canvas_media_source_context_for_update(db, project_id, assembly_probe.chapter_id)
                .await?
        }
    };
    let assembly_row =
        canvas_assembly_result_video_for_update(db, project_id, assembly_video_id).await?;
    if assembly_row.chapter_id != assembly_probe.chapter_id {
        return Err(CanvasDbError::InvalidArgument(format!(
            "视频 {} 的章节归属在媒体登记期间发生变化，请重试",
            assembly_row.id
        )));
    }
    let assembly_row_id = assembly_row.id;
    let preview_asset = ensure_canvas_video_media_asset_record_from_source_in(
        db,
        CanvasVideoMediaSourceEnsureInput {
            project_id,
            video: assembly_row,
            generation_spec_id,
            origin: "assembled_preview".to_string(),
        },
        &assembly_source_context,
    )
    .await?;
    ensure_canvas_media_binding(
        db,
        preview_asset.id,
        "generated_video",
        assembly_row_id,
        "generated_result",
    )
    .await?;

    let timeline =
        upsert_canvas_assembly_timeline(db, project_id, graph_id, node_id, preview_asset.id)
            .await?;
    replace_canvas_assembly_segments(db, project_id, timeline.id, &source_assets).await?;
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
