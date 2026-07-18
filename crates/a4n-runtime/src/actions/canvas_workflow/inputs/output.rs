use super::contracts::CanvasOutputValue;
use super::rules::incoming_edges;
use super::sources::{
    source_generated_image_id, source_generated_video_id, source_media_asset_id, source_text_value,
};
use crate::state::AppState;
use a4n_core::models::canvas::canvas_node;
use a4n_orchestrator::canvas::CanvasGraphRecordDocument as CanvasGraphDocument;

pub(in crate::actions::canvas_workflow) async fn build_canvas_output_value(
    state: Option<&AppState>,
    workflow_run_id: Option<i64>,
    node: &canvas_node::Model,
    document: &CanvasGraphDocument,
    expected_type: &str,
    consumer_label: &str,
) -> Result<CanvasOutputValue, String> {
    let incoming = incoming_edges(document, node.id);
    let mut output = CanvasOutputValue {
        text: None,
        generated_image_id: None,
        generated_video_id: None,
        media_asset_id: None,
        source_node_ids: Vec::new(),
    };
    for edge in incoming {
        let source = document
            .nodes
            .iter()
            .find(|candidate| candidate.id == edge.source_node_id)
            .ok_or_else(|| format!("画布连线引用不存在的源节点: {}", edge.source_node_id))?;
        output.source_node_ids.push(source.id);
        match expected_type {
            "text" => {
                if let Some(text) =
                    source_text_value(state, workflow_run_id, source, document, consumer_label)
                        .await?
                {
                    output.text = Some(text);
                }
            }
            "image" => {
                output.generated_image_id =
                    source_generated_image_id(state, workflow_run_id, source, consumer_label)
                        .await?;
                output.media_asset_id =
                    source_media_asset_id(state, workflow_run_id, source, consumer_label).await?;
            }
            "video" => {
                output.generated_video_id =
                    source_generated_video_id(state, workflow_run_id, source, consumer_label)
                        .await?;
                output.media_asset_id =
                    source_media_asset_id(state, workflow_run_id, source, consumer_label).await?;
            }
            "audio" => {
                output.media_asset_id =
                    source_media_asset_id(state, workflow_run_id, source, consumer_label).await?;
            }
            other => return Err(format!("输出节点数据类型无效: {other}")),
        }
    }
    match expected_type {
        "text" if output.text.as_deref().unwrap_or("").trim().is_empty() => {
            Err(format!("{consumer_label} 缺少文本输入"))
        }
        "image" if output.generated_image_id.is_none() && output.media_asset_id.is_none() => {
            Err(format!("{consumer_label} 缺少图片输入"))
        }
        "video" if output.generated_video_id.is_none() && output.media_asset_id.is_none() => {
            Err(format!("{consumer_label} 缺少视频输入"))
        }
        "audio" if output.media_asset_id.is_none() => Err(format!("{consumer_label} 缺少音频输入")),
        _ => Ok(output),
    }
}
