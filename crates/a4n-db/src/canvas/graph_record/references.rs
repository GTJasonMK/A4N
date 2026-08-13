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
use sea_orm::{ConnectionTrait, EntityTrait};

pub(super) async fn validate_canvas_graph_record_references<C>(
    db: &C,
    project_id: i64,
    input: &CanvasGraphRecordInput,
) -> Result<(), CanvasDbError>
where
    C: ConnectionTrait,
{
    for node in &input.nodes {
        let Some(kind) = validate_canvas_record_node_ref_shape(node)? else {
            continue;
        };
        let Some(ref_id) = node.ref_id else {
            continue;
        };
        match kind {
            CanvasNodeRefKind::Inline | CanvasNodeRefKind::Config => {}
            CanvasNodeRefKind::CanvasTextAsset => {
                let row = canvas_text_asset::Entity::find_by_id(ref_id)
                    .one(db)
                    .await?
                    .ok_or_else(|| {
                        CanvasDbError::NotFound(format!("画布文本资产不存在: {ref_id}"))
                    })?;
                ensure_canvas_record_ref_project(
                    row.project_id,
                    project_id,
                    "画布文本资产",
                    ref_id,
                )?;
            }
            CanvasNodeRefKind::PromptArtifact => {
                let row = prompt_artifact::Entity::find_by_id(ref_id)
                    .one(db)
                    .await?
                    .ok_or_else(|| {
                        CanvasDbError::NotFound(format!("提示词产物不存在: {ref_id}"))
                    })?;
                let spec = generation_spec::Entity::find_by_id(row.generation_spec_id)
                    .one(db)
                    .await?
                    .ok_or_else(|| {
                        CanvasDbError::NotFound(format!(
                            "提示词产物缺少生成意图: prompt_artifact_id={ref_id}"
                        ))
                    })?;
                ensure_canvas_record_ref_project(
                    spec.project_id,
                    project_id,
                    "提示词产物",
                    ref_id,
                )?;
            }
            CanvasNodeRefKind::MediaAsset => {
                let row = media_asset::Entity::find_by_id(ref_id)
                    .one(db)
                    .await?
                    .ok_or_else(|| CanvasDbError::NotFound(format!("媒体资产不存在: {ref_id}")))?;
                ensure_canvas_record_ref_project(row.project_id, project_id, "媒体资产", ref_id)?;
            }
            CanvasNodeRefKind::VideoSegment => {
                let row = video_segment::Entity::find_by_id(ref_id)
                    .one(db)
                    .await?
                    .ok_or_else(|| CanvasDbError::NotFound(format!("视频片段不存在: {ref_id}")))?;
                ensure_canvas_record_ref_project(row.project_id, project_id, "视频片段", ref_id)?;
            }
            CanvasNodeRefKind::AssemblyTimeline => {
                let row = assembly_timeline::Entity::find_by_id(ref_id)
                    .one(db)
                    .await?
                    .ok_or_else(|| {
                        CanvasDbError::NotFound(format!("成片时间线不存在: {ref_id}"))
                    })?;
                ensure_canvas_record_ref_project(row.project_id, project_id, "成片时间线", ref_id)?;
            }
            CanvasNodeRefKind::GenerationSpec => {
                let row = generation_spec::Entity::find_by_id(ref_id)
                    .one(db)
                    .await?
                    .ok_or_else(|| CanvasDbError::NotFound(format!("生成意图不存在: {ref_id}")))?;
                ensure_canvas_record_ref_project(row.project_id, project_id, "生成意图", ref_id)?;
            }
            CanvasNodeRefKind::GeneratedImage => {
                let row = generated_image::Entity::find_by_id(ref_id)
                    .one(db)
                    .await?
                    .ok_or_else(|| CanvasDbError::NotFound(format!("生成图片不存在: {ref_id}")))?;
                ensure_canvas_record_ref_project(row.project_id, project_id, "生成图片", ref_id)?;
            }
            CanvasNodeRefKind::GeneratedVideo => {
                let row = generated_video::Entity::find_by_id(ref_id)
                    .one(db)
                    .await?
                    .ok_or_else(|| CanvasDbError::NotFound(format!("生成视频不存在: {ref_id}")))?;
                ensure_canvas_record_ref_project(row.project_id, project_id, "生成视频", ref_id)?;
            }
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
