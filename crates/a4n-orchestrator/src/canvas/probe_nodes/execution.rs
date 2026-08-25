use a4n_core::error::AppError;
use a4n_db::canvas::CanvasNodeObservationUpdateInput;
use serde_json::{Value, json};

use crate::{
    OrchestratorContext,
    canvas::{CanvasOutputDataType, CanvasOutputValue},
};

use super::contracts::CanvasProbeNodeExecutionInput;

impl OrchestratorContext {
    pub async fn run_canvas_probe_node_validated(
        &self,
        input: CanvasProbeNodeExecutionInput<'_>,
    ) -> Result<a4n_api_contract::canvas::CanvasRunSummary, AppError> {
        let data_type =
            probe_node_data_type(&input.node.node_kind).map_err(AppError::InvalidArgument)?;
        let workflow_run_id = input.shared_run.map(|run| run.workflow_run_id);
        let output = self
            .resolve_canvas_output_value_validated(
                workflow_run_id,
                input.node,
                input.document,
                data_type,
                &input.node.node_kind,
            )
            .await?;
        let step_key = crate::canvas::canvas_node_step_key(input.node.id)
            .map_err(AppError::InvalidArgument)?;
        let input_json = json!({
            "project_id": input.project_id,
            "graph_id": input.graph_id,
            "node_id": input.node.id,
            "data_type": data_type.as_str(),
            "source_node_ids": output.source_node_ids,
        });
        let lifecycle = self
            .start_canvas_node_execution(
                input.project_id,
                input.graph_id,
                input.mode,
                step_key,
                input_json,
                input.shared_run,
            )
            .await?;

        let result = async {
            let artifact_json = self.canvas_probe_artifact_json(data_type, &output).await?;
            self.update_canvas_node_observation_record(CanvasNodeObservationUpdateInput {
                node_id: input.node.id,
                config_patch: canvas_probe_observation_patch(
                    &artifact_json,
                    &output.source_node_ids,
                ),
            })
            .await?;
            self.finish_canvas_node_with_artifacts(&lifecycle, artifact_json)
                .await
        }
        .await;

        match result {
            Ok(summary) => Ok(summary),
            Err(primary) => self.finish_canvas_node_error(lifecycle, primary).await,
        }
    }

    async fn canvas_probe_artifact_json(
        &self,
        data_type: CanvasOutputDataType,
        output: &CanvasOutputValue,
    ) -> Result<Value, AppError> {
        match data_type {
            CanvasOutputDataType::Text => {
                let text = output
                    .text
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| {
                        AppError::InvalidArgument("TextProbeNode 缺少文本输入".to_string())
                    })?;
                Ok(json!({
                    "data_type": "text",
                    "text": text,
                    "source_node_ids": output.source_node_ids,
                }))
            }
            CanvasOutputDataType::Image => {
                if let Some(image_id) = output.generated_image_id {
                    let image = self.generated_image_by_id(image_id).await?.ok_or_else(|| {
                        AppError::InvalidArgument(format!("图片探针引用的图片不存在: {image_id}"))
                    })?;
                    return Ok(json!({
                        "data_type": "image",
                        "generated_image_id": image_id,
                        "media_asset_id": output.media_asset_id,
                        "file_path": image.file_path,
                        "source_node_ids": output.source_node_ids,
                    }));
                }
                let media_asset_id = output.media_asset_id.ok_or_else(|| {
                    AppError::InvalidArgument("ImageProbeNode 缺少图片输入".to_string())
                })?;
                let asset = self.canvas_media_asset_by_id(media_asset_id).await?;
                Ok(json!({
                    "data_type": "image",
                    "media_asset_id": media_asset_id,
                    "file_path": asset.as_ref().and_then(|asset| asset.file_path.clone()),
                    "thumbnail_path": asset.as_ref().and_then(|asset| asset.thumbnail_path.clone()),
                    "source_node_ids": output.source_node_ids,
                }))
            }
            CanvasOutputDataType::Video => {
                if let Some(video_id) = output.generated_video_id {
                    let video = self.generated_video_by_id(video_id).await?.ok_or_else(|| {
                        AppError::InvalidArgument(format!("视频探针引用的视频不存在: {video_id}"))
                    })?;
                    return Ok(json!({
                        "data_type": "video",
                        "generated_video_id": video_id,
                        "media_asset_id": output.media_asset_id,
                        "file_path": video.file_path,
                        "thumbnail_path": video.thumbnail_path,
                        "source_node_ids": output.source_node_ids,
                    }));
                }
                let media_asset_id = output.media_asset_id.ok_or_else(|| {
                    AppError::InvalidArgument("VideoProbeNode 缺少视频输入".to_string())
                })?;
                let asset = self.canvas_media_asset_by_id(media_asset_id).await?;
                Ok(json!({
                    "data_type": "video",
                    "media_asset_id": media_asset_id,
                    "file_path": asset.as_ref().and_then(|asset| asset.file_path.clone()),
                    "thumbnail_path": asset.as_ref().and_then(|asset| asset.thumbnail_path.clone()),
                    "duration_secs": asset.as_ref().and_then(|asset| asset.duration_secs),
                    "source_node_ids": output.source_node_ids,
                }))
            }
            CanvasOutputDataType::Audio => {
                let media_asset_id = output.media_asset_id.ok_or_else(|| {
                    AppError::InvalidArgument("AudioProbeNode 缺少音频输入".to_string())
                })?;
                let asset = self.canvas_media_asset_by_id(media_asset_id).await?;
                Ok(json!({
                    "data_type": "audio",
                    "media_asset_id": media_asset_id,
                    "file_path": asset.as_ref().and_then(|asset| asset.file_path.clone()),
                    "duration_secs": asset.as_ref().and_then(|asset| asset.duration_secs),
                    "source_node_ids": output.source_node_ids,
                }))
            }
        }
    }
}

pub(super) fn probe_node_data_type(node_kind: &str) -> Result<CanvasOutputDataType, String> {
    match node_kind {
        "TextProbeNode" => Ok(CanvasOutputDataType::Text),
        "ImageProbeNode" => Ok(CanvasOutputDataType::Image),
        "VideoProbeNode" => Ok(CanvasOutputDataType::Video),
        "AudioProbeNode" => Ok(CanvasOutputDataType::Audio),
        other => Err(format!("探针节点类型无效: {other}")),
    }
}

pub(super) fn canvas_probe_observation_patch(
    artifact_json: &Value,
    source_node_ids: &[i64],
) -> Value {
    json!({
        "observed_data_type": artifact_json.get("data_type").and_then(Value::as_str),
        "observed_text": artifact_json.get("text").and_then(Value::as_str),
        "generated_image_id": artifact_json.get("generated_image_id").and_then(Value::as_i64),
        "generated_video_id": artifact_json.get("generated_video_id").and_then(Value::as_i64),
        "media_asset_id": artifact_json.get("media_asset_id").and_then(Value::as_i64),
        "file_path": artifact_json.get("file_path").and_then(Value::as_str),
        "thumbnail_path": artifact_json.get("thumbnail_path").and_then(Value::as_str),
        "duration_secs": artifact_json.get("duration_secs").and_then(Value::as_f64),
        "source_node_ids": source_node_ids,
    })
}
