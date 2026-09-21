use std::collections::HashSet;

use a4n_db_models::models::canvas::canvas_node;
use a4n_domain::CanvasNodeRefKind;

pub(in crate::project::video_studio_package::validation) struct CanvasNodeRefIds<'a> {
    pub(in crate::project::video_studio_package::validation) text_asset_ids: &'a HashSet<i64>,
    pub(in crate::project::video_studio_package::validation) generation_spec_ids: &'a HashSet<i64>,
    pub(in crate::project::video_studio_package::validation) prompt_artifact_ids: &'a HashSet<i64>,
    pub(in crate::project::video_studio_package::validation) image_ids: &'a HashSet<i64>,
    pub(in crate::project::video_studio_package::validation) video_ids: &'a HashSet<i64>,
    pub(in crate::project::video_studio_package::validation) media_asset_ids: &'a HashSet<i64>,
    pub(in crate::project::video_studio_package::validation) video_segment_ids: &'a HashSet<i64>,
    pub(in crate::project::video_studio_package::validation) assembly_timeline_ids:
        &'a HashSet<i64>,
}

pub(in crate::project::video_studio_package::validation) fn validate_canvas_node_ref(
    node: &canvas_node::Model,
    refs: &CanvasNodeRefIds<'_>,
) -> Result<(), String> {
    let Some(ref_type) = node.ref_type.as_deref() else {
        if node.ref_id.is_some() {
            return Err(format!(
                "画布节点 ref_type 为空时不能设置 ref_id: canvas_node_id={}",
                node.id
            ));
        }
        return Ok(());
    };
    let kind = ref_type
        .parse::<CanvasNodeRefKind>()
        .map_err(|error| format!("canvas_node_id={}, {error}", node.id))?;
    match kind {
        CanvasNodeRefKind::Inline => {
            if node.ref_id.is_some() {
                return Err(format!(
                    "inline 画布节点不能设置 ref_id: canvas_node_id={}",
                    node.id
                ));
            }
            Ok(())
        }
        CanvasNodeRefKind::Config => {
            if let Some(ref_id) = node.ref_id
                && ref_id <= 0
            {
                return Err(format!(
                    "config 画布节点 ref_id 必须是正整数: canvas_node_id={}",
                    node.id
                ));
            }
            Ok(())
        }
        CanvasNodeRefKind::GeneratedImage => {
            require_canvas_node_ref_id(node, ref_type, refs.image_ids, "生成图片")
        }
        CanvasNodeRefKind::GeneratedVideo => {
            require_canvas_node_ref_id(node, ref_type, refs.video_ids, "生成视频")
        }
        CanvasNodeRefKind::CanvasTextAsset => {
            require_canvas_node_ref_id(node, ref_type, refs.text_asset_ids, "画布文本资产")
        }
        CanvasNodeRefKind::GenerationSpec => {
            require_canvas_node_ref_id(node, ref_type, refs.generation_spec_ids, "生成意图")
        }
        CanvasNodeRefKind::PromptArtifact => {
            require_canvas_node_ref_id(node, ref_type, refs.prompt_artifact_ids, "提示词产物")
        }
        CanvasNodeRefKind::MediaAsset => {
            require_canvas_node_ref_id(node, ref_type, refs.media_asset_ids, "媒体资产")
        }
        CanvasNodeRefKind::VideoSegment => {
            require_canvas_node_ref_id(node, ref_type, refs.video_segment_ids, "视频片段")
        }
        CanvasNodeRefKind::AssemblyTimeline => {
            require_canvas_node_ref_id(node, ref_type, refs.assembly_timeline_ids, "成片时间线")
        }
    }
}

fn require_canvas_node_ref_id(
    node: &canvas_node::Model,
    ref_type: &str,
    allowed_ids: &HashSet<i64>,
    label: &str,
) -> Result<(), String> {
    let ref_id = node.ref_id.ok_or_else(|| {
        format!(
            "{ref_type} 画布节点必须设置 ref_id: canvas_node_id={}",
            node.id
        )
    })?;
    if ref_id <= 0 {
        return Err(format!(
            "{ref_type} 画布节点 ref_id 必须是正整数: canvas_node_id={}",
            node.id
        ));
    }
    if !allowed_ids.contains(&ref_id) {
        return Err(format!(
            "画布节点引用了不存在的{label}: canvas_node_id={}, ref_id={ref_id}",
            node.id
        ));
    }
    Ok(())
}
