use super::types::CanvasGraphRecordInput;
use super::validation::validate_canvas_record_node_ref_shape;
use crate::canvas::CanvasDbError;
use a4n_core::models::{
    ai::{generated_image, generated_video},
    canvas::canvas_text_asset,
    generation::{generation_spec, prompt_artifact},
    media::media_asset,
    video_project::{assembly_timeline, video_segment},
};
use a4n_domain::CanvasNodeRefKind;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect};
use std::collections::{HashMap, HashSet};

pub(super) async fn validate_canvas_graph_record_references<C>(
    db: &C,
    project_id: i64,
    input: &CanvasGraphRecordInput,
) -> Result<(), CanvasDbError>
where
    C: ConnectionTrait,
{
    let mut references = Vec::new();
    let mut canvas_text_asset_ids = HashSet::new();
    let mut prompt_artifact_ids = HashSet::new();
    let mut media_asset_ids = HashSet::new();
    let mut video_segment_ids = HashSet::new();
    let mut assembly_timeline_ids = HashSet::new();
    let mut generation_spec_ids = HashSet::new();
    let mut generated_image_ids = HashSet::new();
    let mut generated_video_ids = HashSet::new();

    for node in &input.nodes {
        let Some(kind) = validate_canvas_record_node_ref_shape(node)? else {
            continue;
        };
        let Some(ref_id) = node.ref_id else {
            continue;
        };
        match kind {
            CanvasNodeRefKind::Inline | CanvasNodeRefKind::Config => continue,
            CanvasNodeRefKind::CanvasTextAsset => {
                canvas_text_asset_ids.insert(ref_id);
            }
            CanvasNodeRefKind::PromptArtifact => {
                prompt_artifact_ids.insert(ref_id);
            }
            CanvasNodeRefKind::MediaAsset => {
                media_asset_ids.insert(ref_id);
            }
            CanvasNodeRefKind::VideoSegment => {
                video_segment_ids.insert(ref_id);
            }
            CanvasNodeRefKind::AssemblyTimeline => {
                assembly_timeline_ids.insert(ref_id);
            }
            CanvasNodeRefKind::GenerationSpec => {
                generation_spec_ids.insert(ref_id);
            }
            CanvasNodeRefKind::GeneratedImage => {
                generated_image_ids.insert(ref_id);
            }
            CanvasNodeRefKind::GeneratedVideo => {
                generated_video_ids.insert(ref_id);
            }
        }
        references.push((kind, ref_id));
    }

    if references.is_empty() {
        return Ok(());
    }

    let canvas_text_assets = if canvas_text_asset_ids.is_empty() {
        HashMap::<i64, i64>::new()
    } else {
        canvas_text_asset::Entity::find()
            .select_only()
            .column(canvas_text_asset::Column::Id)
            .column(canvas_text_asset::Column::ProjectId)
            .filter(canvas_text_asset::Column::Id.is_in(canvas_text_asset_ids.iter().copied()))
            .into_tuple::<(i64, i64)>()
            .all(db)
            .await?
            .into_iter()
            .collect()
    };
    let prompt_artifacts = if prompt_artifact_ids.is_empty() {
        HashMap::<i64, i64>::new()
    } else {
        prompt_artifact::Entity::find()
            .select_only()
            .column(prompt_artifact::Column::Id)
            .column(prompt_artifact::Column::GenerationSpecId)
            .filter(prompt_artifact::Column::Id.is_in(prompt_artifact_ids.iter().copied()))
            .into_tuple::<(i64, i64)>()
            .all(db)
            .await?
            .into_iter()
            .collect()
    };
    for generation_spec_id in prompt_artifacts.values() {
        generation_spec_ids.insert(*generation_spec_id);
    }
    let generation_specs = if generation_spec_ids.is_empty() {
        HashMap::<i64, i64>::new()
    } else {
        generation_spec::Entity::find()
            .select_only()
            .column(generation_spec::Column::Id)
            .column(generation_spec::Column::ProjectId)
            .filter(generation_spec::Column::Id.is_in(generation_spec_ids.iter().copied()))
            .into_tuple::<(i64, i64)>()
            .all(db)
            .await?
            .into_iter()
            .collect()
    };
    let media_assets = if media_asset_ids.is_empty() {
        HashMap::<i64, i64>::new()
    } else {
        media_asset::Entity::find()
            .select_only()
            .column(media_asset::Column::Id)
            .column(media_asset::Column::ProjectId)
            .filter(media_asset::Column::Id.is_in(media_asset_ids.iter().copied()))
            .into_tuple::<(i64, i64)>()
            .all(db)
            .await?
            .into_iter()
            .collect()
    };
    let video_segments = if video_segment_ids.is_empty() {
        HashMap::<i64, i64>::new()
    } else {
        video_segment::Entity::find()
            .select_only()
            .column(video_segment::Column::Id)
            .column(video_segment::Column::ProjectId)
            .filter(video_segment::Column::Id.is_in(video_segment_ids.iter().copied()))
            .into_tuple::<(i64, i64)>()
            .all(db)
            .await?
            .into_iter()
            .collect()
    };
    let assembly_timelines = if assembly_timeline_ids.is_empty() {
        HashMap::<i64, i64>::new()
    } else {
        assembly_timeline::Entity::find()
            .select_only()
            .column(assembly_timeline::Column::Id)
            .column(assembly_timeline::Column::ProjectId)
            .filter(assembly_timeline::Column::Id.is_in(assembly_timeline_ids.iter().copied()))
            .into_tuple::<(i64, i64)>()
            .all(db)
            .await?
            .into_iter()
            .collect()
    };
    let generated_images = if generated_image_ids.is_empty() {
        HashMap::<i64, i64>::new()
    } else {
        generated_image::Entity::find()
            .select_only()
            .column(generated_image::Column::Id)
            .column(generated_image::Column::ProjectId)
            .filter(generated_image::Column::Id.is_in(generated_image_ids.iter().copied()))
            .into_tuple::<(i64, i64)>()
            .all(db)
            .await?
            .into_iter()
            .collect()
    };
    let generated_videos = if generated_video_ids.is_empty() {
        HashMap::<i64, i64>::new()
    } else {
        generated_video::Entity::find()
            .select_only()
            .column(generated_video::Column::Id)
            .column(generated_video::Column::ProjectId)
            .filter(generated_video::Column::Id.is_in(generated_video_ids.iter().copied()))
            .into_tuple::<(i64, i64)>()
            .all(db)
            .await?
            .into_iter()
            .collect()
    };

    for (kind, ref_id) in references {
        match kind {
            CanvasNodeRefKind::CanvasTextAsset => {
                let row = canvas_text_assets.get(&ref_id).ok_or_else(|| {
                    CanvasDbError::NotFound(format!("画布文本资产不存在: {ref_id}"))
                })?;
                ensure_canvas_record_ref_project(*row, project_id, "画布文本资产", ref_id)?;
            }
            CanvasNodeRefKind::PromptArtifact => {
                let row = prompt_artifacts.get(&ref_id).ok_or_else(|| {
                    CanvasDbError::NotFound(format!("提示词产物不存在: {ref_id}"))
                })?;
                let spec = generation_specs.get(row).ok_or_else(|| {
                    CanvasDbError::NotFound(format!(
                        "提示词产物缺少生成意图: prompt_artifact_id={ref_id}"
                    ))
                })?;
                ensure_canvas_record_ref_project(*spec, project_id, "提示词产物", ref_id)?;
            }
            CanvasNodeRefKind::MediaAsset => {
                let row = media_assets
                    .get(&ref_id)
                    .ok_or_else(|| CanvasDbError::NotFound(format!("媒体资产不存在: {ref_id}")))?;
                ensure_canvas_record_ref_project(*row, project_id, "媒体资产", ref_id)?;
            }
            CanvasNodeRefKind::VideoSegment => {
                let row = video_segments
                    .get(&ref_id)
                    .ok_or_else(|| CanvasDbError::NotFound(format!("视频片段不存在: {ref_id}")))?;
                ensure_canvas_record_ref_project(*row, project_id, "视频片段", ref_id)?;
            }
            CanvasNodeRefKind::AssemblyTimeline => {
                let row = assembly_timelines.get(&ref_id).ok_or_else(|| {
                    CanvasDbError::NotFound(format!("成片时间线不存在: {ref_id}"))
                })?;
                ensure_canvas_record_ref_project(*row, project_id, "成片时间线", ref_id)?;
            }
            CanvasNodeRefKind::GenerationSpec => {
                let row = generation_specs
                    .get(&ref_id)
                    .ok_or_else(|| CanvasDbError::NotFound(format!("生成意图不存在: {ref_id}")))?;
                ensure_canvas_record_ref_project(*row, project_id, "生成意图", ref_id)?;
            }
            CanvasNodeRefKind::GeneratedImage => {
                let row = generated_images
                    .get(&ref_id)
                    .ok_or_else(|| CanvasDbError::NotFound(format!("生成图片不存在: {ref_id}")))?;
                ensure_canvas_record_ref_project(*row, project_id, "生成图片", ref_id)?;
            }
            CanvasNodeRefKind::GeneratedVideo => {
                let row = generated_videos
                    .get(&ref_id)
                    .ok_or_else(|| CanvasDbError::NotFound(format!("生成视频不存在: {ref_id}")))?;
                ensure_canvas_record_ref_project(*row, project_id, "生成视频", ref_id)?;
            }
            CanvasNodeRefKind::Inline | CanvasNodeRefKind::Config => unreachable!(),
        }
    }
    Ok(())
}

fn ensure_canvas_record_ref_project(
    record_project_id: i64,
    project_id: i64,
    label: &str,
    ref_id: i64,
) -> Result<(), CanvasDbError> {
    if record_project_id == project_id {
        Ok(())
    } else {
        Err(CanvasDbError::Forbidden(format!(
            "画布节点引用的{label}不属于当前项目: ref_id={ref_id}"
        )))
    }
}
