use super::super::inputs::CanvasOutputValue;
use crate::AppState;
use serde_json::{Value, json};

pub(super) async fn canvas_probe_artifact_json(
    state: &AppState,
    data_type: &str,
    output: &CanvasOutputValue,
) -> Result<Value, String> {
    match data_type {
        "text" => {
            let text = output
                .text
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| "TextProbeNode 缺少文本输入".to_string())?;
            Ok(json!({
                "data_type": "text",
                "text": text,
                "source_node_ids": output.source_node_ids}))
        }
        "image" => {
            if let Some(image_id) = output.generated_image_id {
                let image = state
                    .orchestrator
                    .generated_image_by_id(image_id)
                    .await
                    .map_err(|err| err.to_string())?
                    .ok_or_else(|| format!("图片探针引用的图片不存在: {image_id}"))?;
                return Ok(json!({
                    "data_type": "image",
                    "generated_image_id": image_id,
                    "media_asset_id": output.media_asset_id,
                    "file_path": image.file_path,
                    "source_node_ids": output.source_node_ids}));
            }
            let media_asset_id = output
                .media_asset_id
                .ok_or_else(|| "ImageProbeNode 缺少图片输入".to_string())?;
            let asset = state
                .orchestrator
                .canvas_media_asset_by_id(media_asset_id)
                .await
                .map_err(|err| err.to_string())?;
            Ok(json!({
                "data_type": "image",
                "media_asset_id": media_asset_id,
                "file_path": asset.as_ref().and_then(|asset| asset.file_path.clone()),
                "thumbnail_path": asset.as_ref().and_then(|asset| asset.thumbnail_path.clone()),
                "source_node_ids": output.source_node_ids}))
        }
        "video" => {
            if let Some(video_id) = output.generated_video_id {
                let video = state
                    .orchestrator
                    .generated_video_by_id(video_id)
                    .await
                    .map_err(|err| err.to_string())?
                    .ok_or_else(|| format!("视频探针引用的视频不存在: {video_id}"))?;
                return Ok(json!({
                    "data_type": "video",
                    "generated_video_id": video_id,
                    "media_asset_id": output.media_asset_id,
                    "file_path": video.file_path,
                    "thumbnail_path": video.thumbnail_path,
                    "source_node_ids": output.source_node_ids}));
            }
            let media_asset_id = output
                .media_asset_id
                .ok_or_else(|| "VideoProbeNode 缺少视频输入".to_string())?;
            let asset = state
                .orchestrator
                .canvas_media_asset_by_id(media_asset_id)
                .await
                .map_err(|err| err.to_string())?;
            Ok(json!({
                "data_type": "video",
                "media_asset_id": media_asset_id,
                "file_path": asset.as_ref().and_then(|asset| asset.file_path.clone()),
                "thumbnail_path": asset.as_ref().and_then(|asset| asset.thumbnail_path.clone()),
                "duration_secs": asset.as_ref().and_then(|asset| asset.duration_secs),
                "source_node_ids": output.source_node_ids}))
        }
        "audio" => {
            let media_asset_id = output
                .media_asset_id
                .ok_or_else(|| "AudioProbeNode 缺少音频输入".to_string())?;
            let asset = state
                .orchestrator
                .canvas_media_asset_by_id(media_asset_id)
                .await
                .map_err(|err| err.to_string())?;
            Ok(json!({
                "data_type": "audio",
                "media_asset_id": media_asset_id,
                "file_path": asset.as_ref().and_then(|asset| asset.file_path.clone()),
                "duration_secs": asset.as_ref().and_then(|asset| asset.duration_secs),
                "source_node_ids": output.source_node_ids}))
        }
        other => Err(format!("探针数据类型无效: {other}")),
    }
}
