use a4n_db::canvas::CanvasGraphRecordDocument;
use a4n_db_models::models::{canvas::canvas_node, generation::workflow_checkpoint};

use crate::canvas::{
    canvas_config_i64, ensure_canvas_provider_slot_ready, i64_to_i32,
    source_text::collect_canvas_text_source_context,
};

use super::contracts::CanvasImageNodeInput;

pub fn build_canvas_image_node_input_for_preview(
    node: &canvas_node::Model,
    document: &CanvasGraphRecordDocument,
) -> Result<CanvasImageNodeInput, String> {
    ensure_canvas_provider_slot_ready(&node.config_json, "图片")?;
    build_canvas_image_node_input(node, document, None)
}

pub(super) fn build_canvas_image_node_input(
    node: &canvas_node::Model,
    document: &CanvasGraphRecordDocument,
    checkpoints: Option<&[workflow_checkpoint::Model]>,
) -> Result<CanvasImageNodeInput, String> {
    let sources =
        collect_canvas_text_source_context(node, document, checkpoints, "GenerateImageNode")?;
    if sources.text.trim().is_empty() {
        return Err("GenerateImageNode 缺少 prompt 输入".to_string());
    }
    let width = canvas_config_i64(&node.config_json, "width")
        .and_then(i64_to_i32)
        .ok_or_else(|| "GenerateImageNode 必须设置正整数 width".to_string())?;
    let height = canvas_config_i64(&node.config_json, "height")
        .and_then(i64_to_i32)
        .ok_or_else(|| "GenerateImageNode 必须设置正整数 height".to_string())?;
    Ok(CanvasImageNodeInput {
        prompt: sources.text,
        config_id: canvas_config_i64(&node.config_json, "image_config_id"),
        width,
        height,
        canvas_text_asset_id: sources.canvas_text_asset_id,
        source_node_ids: sources.source_node_ids,
    })
}
