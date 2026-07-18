use crate::canvas::CanvasDbError;

use super::super::types::CanvasNodeRecordInput;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::canvas::graph_record) enum CanvasRecordNodeRefKind {
    Inline,
    Config,
    CanvasTextAsset,
    PromptArtifact,
    MediaAsset,
    VideoSegment,
    AssemblyTimeline,
    GenerationSpec,
    GeneratedImage,
    GeneratedVideo,
    LegacyGeneratedImage,
    LegacyGeneratedVideo,
    LegacyChapterVideoPrompt,
}

pub(in crate::canvas::graph_record) fn validate_canvas_record_node_ref_shape(
    node: &CanvasNodeRecordInput,
) -> Result<Option<CanvasRecordNodeRefKind>, CanvasDbError> {
    let Some(ref_type) = node
        .ref_type
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        if node.ref_id.is_some() {
            return Err(CanvasDbError::InvalidArgument(
                "node.ref_id 不能在 node.ref_type 为空时设置".to_string(),
            ));
        }
        return Ok(None);
    };
    let kind = parse_canvas_record_node_ref_kind(ref_type)?;
    match kind {
        CanvasRecordNodeRefKind::Inline => {
            if node.ref_id.is_some() {
                return Err(CanvasDbError::InvalidArgument(
                    "inline 节点引用不能设置 ref_id".to_string(),
                ));
            }
        }
        CanvasRecordNodeRefKind::Config => {
            if let Some(ref_id) = node.ref_id {
                ensure_canvas_record_positive_ref_id(ref_id, ref_type)?;
            }
        }
        _ => {
            let ref_id = node.ref_id.ok_or_else(|| {
                CanvasDbError::InvalidArgument(format!("{ref_type} 节点引用必须设置 ref_id"))
            })?;
            ensure_canvas_record_positive_ref_id(ref_id, ref_type)?;
        }
    }
    Ok(Some(kind))
}

fn parse_canvas_record_node_ref_kind(
    value: &str,
) -> Result<CanvasRecordNodeRefKind, CanvasDbError> {
    match value.trim() {
        "inline" => Ok(CanvasRecordNodeRefKind::Inline),
        "config" => Ok(CanvasRecordNodeRefKind::Config),
        "canvas_text_asset" => Ok(CanvasRecordNodeRefKind::CanvasTextAsset),
        "prompt_artifact" => Ok(CanvasRecordNodeRefKind::PromptArtifact),
        "media_asset" => Ok(CanvasRecordNodeRefKind::MediaAsset),
        "video_segment" => Ok(CanvasRecordNodeRefKind::VideoSegment),
        "assembly_timeline" => Ok(CanvasRecordNodeRefKind::AssemblyTimeline),
        "generation_spec" => Ok(CanvasRecordNodeRefKind::GenerationSpec),
        "generated_image" => Ok(CanvasRecordNodeRefKind::GeneratedImage),
        "generated_video" => Ok(CanvasRecordNodeRefKind::GeneratedVideo),
        "legacy_generated_image" => Ok(CanvasRecordNodeRefKind::LegacyGeneratedImage),
        "legacy_generated_video" => Ok(CanvasRecordNodeRefKind::LegacyGeneratedVideo),
        "legacy_chapter_video_prompt" => Ok(CanvasRecordNodeRefKind::LegacyChapterVideoPrompt),
        other => Err(CanvasDbError::InvalidArgument(format!(
            "未知画布节点引用类型: {other}"
        ))),
    }
}

fn ensure_canvas_record_positive_ref_id(ref_id: i64, ref_type: &str) -> Result<(), CanvasDbError> {
    if ref_id > 0 {
        Ok(())
    } else {
        Err(CanvasDbError::InvalidArgument(format!(
            "{ref_type} 节点引用 ref_id 必须是正整数"
        )))
    }
}
