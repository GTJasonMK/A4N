use a4n_core::models::{canvas::canvas_node, generation::workflow_checkpoint};
use a4n_db::canvas::CanvasGraphRecordDocument;

use crate::canvas::{canvas_config_i64, source_text::collect_canvas_text_source_context};

use super::contracts::CanvasImageNodeInput;

pub fn build_canvas_image_node_input_for_preview(
    node: &canvas_node::Model,
    document: &CanvasGraphRecordDocument,
) -> Result<CanvasImageNodeInput, String> {
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
    Ok(CanvasImageNodeInput {
        prompt: sources.text,
        config_id: canvas_config_i64(&node.config_json, "image_config_id"),
        canvas_text_asset_id: sources.canvas_text_asset_id,
        source_node_ids: sources.source_node_ids,
    })
}
