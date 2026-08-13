use a4n_core::models::{canvas::canvas_node, generation::workflow_checkpoint};
use a4n_db::canvas::CanvasGraphRecordDocument;
use a4n_domain::CanvasNodeRefKind;
use serde_json::Value;

use crate::canvas::{
    incoming_edges, source_checkpoint::source_operation_checkpoint_output,
    source_text::source_text_value,
};

use super::contracts::{CanvasOutputDataType, CanvasOutputValue};

pub(crate) fn collect_canvas_output_value(
    node: &canvas_node::Model,
    document: &CanvasGraphRecordDocument,
    checkpoints: Option<&[workflow_checkpoint::Model]>,
    data_type: CanvasOutputDataType,
    consumer_label: &str,
    require_video_target_port: bool,
) -> Result<CanvasOutputValue, String> {
    let mut output = CanvasOutputValue {
        text: None,
        generated_image_id: None,
        generated_video_id: None,
        media_asset_id: None,
        source_node_ids: Vec::new(),
    };
    for edge in incoming_edges(document, node.id) {
        let source = document
            .nodes
            .iter()
            .find(|candidate| candidate.id == edge.source_node_id)
            .ok_or_else(|| format!("画布连线引用不存在的源节点: {}", edge.source_node_id))?;
        output.source_node_ids.push(source.id);
        match data_type {
            CanvasOutputDataType::Text => {
                if let Some(text) =
                    source_text_value(source, document, checkpoints, consumer_label)?
                {
                    output.text = Some(text);
                }
            }
            CanvasOutputDataType::Image => {
                output.generated_image_id = source_checkpoint_i64(
                    checkpoints,
                    source,
                    consumer_label,
                    "generated_image_id",
                )?
                .or_else(|| source_ref_id(source, CanvasNodeRefKind::GeneratedImage));
                output.media_asset_id =
                    source_checkpoint_i64(checkpoints, source, consumer_label, "media_asset_id")?
                        .or_else(|| source_ref_id(source, CanvasNodeRefKind::MediaAsset));
            }
            CanvasOutputDataType::Video => {
                if require_video_target_port && edge.target_port != "video" {
                    continue;
                }
                output.generated_video_id = source_checkpoint_i64(
                    checkpoints,
                    source,
                    consumer_label,
                    "generated_video_id",
                )?
                .or_else(|| source_ref_id(source, CanvasNodeRefKind::GeneratedVideo));
                output.media_asset_id =
                    source_checkpoint_i64(checkpoints, source, consumer_label, "media_asset_id")?
                        .or_else(|| source_ref_id(source, CanvasNodeRefKind::MediaAsset));
            }
            CanvasOutputDataType::Audio => {
                output.media_asset_id =
                    source_checkpoint_i64(checkpoints, source, consumer_label, "media_asset_id")?
                        .or_else(|| source_ref_id(source, CanvasNodeRefKind::MediaAsset));
            }
        }
    }
    Ok(output)
}

pub(crate) fn validate_canvas_output_value(
    output: &CanvasOutputValue,
    data_type: CanvasOutputDataType,
    consumer_label: &str,
) -> Result<(), String> {
    match data_type {
        CanvasOutputDataType::Text if output.text.as_deref().unwrap_or("").trim().is_empty() => {
            Err(format!("{consumer_label} 缺少文本输入"))
        }
        CanvasOutputDataType::Image
            if output.generated_image_id.is_none() && output.media_asset_id.is_none() =>
        {
            Err(format!("{consumer_label} 缺少图片输入"))
        }
        CanvasOutputDataType::Video
            if output.generated_video_id.is_none() && output.media_asset_id.is_none() =>
        {
            Err(format!("{consumer_label} 缺少视频输入"))
        }
        CanvasOutputDataType::Audio if output.media_asset_id.is_none() => {
            Err(format!("{consumer_label} 缺少音频输入"))
        }
        _ => Ok(()),
    }
}

fn source_checkpoint_i64(
    checkpoints: Option<&[workflow_checkpoint::Model]>,
    node: &canvas_node::Model,
    consumer_label: &str,
    key: &str,
) -> Result<Option<i64>, String> {
    Ok(
        source_operation_checkpoint_output(checkpoints, node, consumer_label)?
            .and_then(|value| value.get(key))
            .and_then(Value::as_i64),
    )
}

fn source_ref_id(node: &canvas_node::Model, kind: CanvasNodeRefKind) -> Option<i64> {
    node.ref_type
        .as_deref()
        .filter(|ref_type| *ref_type == kind.as_str())
        .and(node.ref_id)
}
