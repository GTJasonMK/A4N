use std::collections::HashSet;

use a4n_core::models::canvas::canvas_node;

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
    let Some(ref_type) = node
        .ref_type
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        if node.ref_id.is_some() {
            return Err(format!(
                "画布节点 ref_type 为空时不能设置 ref_id: canvas_node_id={}",
                node.id
            ));
        }
        return Ok(());
    };
    match ref_type {
        "inline" => {
            if node.ref_id.is_some() {
                return Err(format!(
                    "inline 画布节点不能设置 ref_id: canvas_node_id={}",
                    node.id
                ));
            }
            Ok(())
        }
        "config" => {
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
        "legacy_generated_image" => {
            require_canvas_node_ref_id(node, ref_type, refs.image_ids, "生成图片")
        }
        "legacy_generated_video" => {
            require_canvas_node_ref_id(node, ref_type, refs.video_ids, "生成视频")
        }
        "canvas_text_asset" => {
            require_canvas_node_ref_id(node, ref_type, refs.text_asset_ids, "画布文本资产")
        }
        "generation_spec" => {
            require_canvas_node_ref_id(node, ref_type, refs.generation_spec_ids, "生成意图")
        }
        "prompt_artifact" => {
            require_canvas_node_ref_id(node, ref_type, refs.prompt_artifact_ids, "提示词产物")
        }
        "media_asset" => {
            require_canvas_node_ref_id(node, ref_type, refs.media_asset_ids, "媒体资产")
        }
        "video_segment" => {
            require_canvas_node_ref_id(node, ref_type, refs.video_segment_ids, "视频片段")
        }
        "assembly_timeline" => {
            require_canvas_node_ref_id(node, ref_type, refs.assembly_timeline_ids, "成片时间线")
        }
        "legacy_chapter_video_prompt" => Err(format!(
            "视频画布项目包不能包含章节视频提示词引用: canvas_node_id={}",
            node.id
        )),
        other => Err(format!(
            "视频画布项目包包含未知画布节点引用类型: canvas_node_id={}, ref_type={other}",
            node.id
        )),
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
