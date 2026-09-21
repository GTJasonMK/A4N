use a4n_db::canvas::CanvasGraphRecordDocument;
use a4n_db_models::models::{canvas::canvas_node, generation::workflow_checkpoint};
use a4n_domain::CanvasNodeRefKind;

use crate::canvas::{
    incoming_edges, source_node, source_node_output_id, source_text::source_text_value,
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
        let source = source_node(document, edge.source_node_id)?;
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
                output.generated_image_id = source_node_output_id(
                    checkpoints,
                    source,
                    consumer_label,
                    "generated_image_id",
                    CanvasNodeRefKind::GeneratedImage,
                )?;
                output.media_asset_id = source_node_output_id(
                    checkpoints,
                    source,
                    consumer_label,
                    "media_asset_id",
                    CanvasNodeRefKind::MediaAsset,
                )?;
            }
            CanvasOutputDataType::Video => {
                if require_video_target_port && edge.target_port != "video" {
                    continue;
                }
                output.generated_video_id = source_node_output_id(
                    checkpoints,
                    source,
                    consumer_label,
                    "generated_video_id",
                    CanvasNodeRefKind::GeneratedVideo,
                )?;
                output.media_asset_id = source_node_output_id(
                    checkpoints,
                    source,
                    consumer_label,
                    "media_asset_id",
                    CanvasNodeRefKind::MediaAsset,
                )?;
            }
            CanvasOutputDataType::Audio => {
                output.media_asset_id = source_node_output_id(
                    checkpoints,
                    source,
                    consumer_label,
                    "media_asset_id",
                    CanvasNodeRefKind::MediaAsset,
                )?;
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
