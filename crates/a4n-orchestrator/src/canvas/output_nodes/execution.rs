use a4n_core::error::AppError;
use a4n_db::canvas::{CanvasNodeArtifactUpdateInput, CanvasTextAssetRecordInput};
use a4n_domain::CanvasNodeRefKind;
use serde_json::{Value, json};

use crate::{
    OrchestratorContext,
    canvas::{canvas_config_string, source_canvas_text_asset_id},
};

use super::{
    contracts::{CanvasOutputDataType, CanvasOutputNodeExecutionInput, CanvasOutputValue},
    input::{collect_canvas_output_value, validate_canvas_output_value},
};

impl OrchestratorContext {
    pub(crate) async fn resolve_canvas_output_value_validated(
        &self,
        workflow_run_id: Option<i64>,
        node: &a4n_core::models::canvas::canvas_node::Model,
        document: &a4n_db::canvas::CanvasGraphRecordDocument,
        data_type: CanvasOutputDataType,
        consumer_label: &str,
    ) -> Result<CanvasOutputValue, AppError> {
        let checkpoints = match workflow_run_id {
            Some(run_id) => Some(self.workflow_checkpoint_models(run_id).await?),
            None => None,
        };
        let output = collect_canvas_output_value(
            node,
            document,
            checkpoints.as_deref(),
            data_type,
            consumer_label,
            false,
        )
        .map_err(AppError::InvalidArgument)?;
        validate_canvas_output_value(&output, data_type, consumer_label)
            .map_err(AppError::InvalidArgument)?;
        Ok(output)
    }

    pub async fn run_canvas_output_node_validated(
        &self,
        input: CanvasOutputNodeExecutionInput<'_>,
    ) -> Result<crate::canvas::CanvasNodeRunSummary, AppError> {
        let data_type = CanvasOutputDataType::from_output_node_kind(&input.node.node_kind)
            .map_err(AppError::InvalidArgument)?;
        let checkpoints = match input.shared_run {
            Some(run) => Some(self.workflow_checkpoint_models(run.workflow_run_id).await?),
            None => None,
        };
        let output = collect_canvas_output_value(
            input.node,
            input.document,
            checkpoints.as_deref(),
            data_type,
            &input.node.node_kind,
            data_type == CanvasOutputDataType::Video,
        )
        .map_err(AppError::InvalidArgument)?;
        validate_output_node_value(&output, data_type, &input.node.node_kind)
            .map_err(AppError::InvalidArgument)?;
        let step_key = crate::canvas::canvas_node_step_key(input.node.id)
            .map_err(AppError::InvalidArgument)?;
        let input_json = json!({
            "project_id": input.project_id,
            "graph_id": input.graph_id,
            "node_id": input.node.id,
            "data_type": data_type.as_str(),
            "generated_image_id": output.generated_image_id,
            "generated_video_id": output.generated_video_id,
            "media_asset_id": output.media_asset_id,
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
            let (ref_type, ref_id, artifact_json) = self
                .persist_canvas_output_artifact(input.project_id, input.node, data_type, &output)
                .await?;
            self.update_canvas_node_artifact_record(CanvasNodeArtifactUpdateInput {
                node_id: input.node.id,
                ref_type,
                ref_id,
                config_patch: artifact_json.clone(),
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

    async fn persist_canvas_output_artifact(
        &self,
        project_id: i64,
        node: &a4n_core::models::canvas::canvas_node::Model,
        data_type: CanvasOutputDataType,
        output: &CanvasOutputValue,
    ) -> Result<(String, i64, Value), AppError> {
        match data_type {
            CanvasOutputDataType::Text => {
                let text = output
                    .text
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| {
                        AppError::InvalidArgument("TextOutputNode 缺少文本输入".to_string())
                    })?;
                let title = canvas_config_string(&node.config_json, "title")
                    .unwrap_or_else(|| "文本输出".to_string());
                let asset = self
                    .upsert_canvas_text_asset_record(
                        project_id,
                        CanvasTextAssetRecordInput {
                            id: source_canvas_text_asset_id(node),
                            title,
                            text_kind: "canvas_output".to_string(),
                            content: text.to_string(),
                        },
                        chrono::Utc::now().naive_utc(),
                    )
                    .await?;
                let artifact = json!({
                    "data_type": "text",
                    "canvas_text_asset_id": asset.id,
                    "title": asset.title,
                    "text": asset.content,
                });
                Ok((
                    CanvasNodeRefKind::CanvasTextAsset.to_string(),
                    asset.id,
                    artifact,
                ))
            }
            CanvasOutputDataType::Image => {
                if let Some(image_id) = output.generated_image_id {
                    let image = self.generated_image_by_id(image_id).await?.ok_or_else(|| {
                        AppError::InvalidArgument(format!("图片输出不存在: {image_id}"))
                    })?;
                    let artifact = json!({
                        "data_type": "image",
                        "generated_image_id": image_id,
                        "file_path": image.file_path,
                        "media_asset_id": output.media_asset_id,
                    });
                    return Ok((
                        CanvasNodeRefKind::GeneratedImage.to_string(),
                        image_id,
                        artifact,
                    ));
                }
                let media_asset_id = output.media_asset_id.ok_or_else(|| {
                    AppError::InvalidArgument("ImageOutputNode 缺少图片输入".to_string())
                })?;
                let asset = self.canvas_media_asset_by_id(media_asset_id).await?;
                let artifact = json!({
                    "data_type": "image",
                    "media_asset_id": media_asset_id,
                    "file_path": asset.as_ref().and_then(|asset| asset.file_path.clone()),
                    "thumbnail_path": asset.as_ref().and_then(|asset| asset.thumbnail_path.clone()),
                });
                Ok((
                    CanvasNodeRefKind::MediaAsset.to_string(),
                    media_asset_id,
                    artifact,
                ))
            }
            CanvasOutputDataType::Audio => {
                let media_asset_id = output.media_asset_id.ok_or_else(|| {
                    AppError::InvalidArgument("AudioOutputNode 缺少音频输入".to_string())
                })?;
                let asset = self.canvas_media_asset_by_id(media_asset_id).await?;
                let artifact = json!({
                    "data_type": "audio",
                    "media_asset_id": media_asset_id,
                    "file_path": asset.as_ref().and_then(|asset| asset.file_path.clone()),
                    "duration_secs": asset.as_ref().and_then(|asset| asset.duration_secs),
                });
                Ok((
                    CanvasNodeRefKind::MediaAsset.to_string(),
                    media_asset_id,
                    artifact,
                ))
            }
            CanvasOutputDataType::Video => {
                let video_id = output.generated_video_id.ok_or_else(|| {
                    AppError::InvalidArgument("VideoOutputNode 缺少可输出的视频输入".to_string())
                })?;
                let video = self.generated_video_by_id(video_id).await?.ok_or_else(|| {
                    AppError::InvalidArgument(format!("画布输出视频不存在: {video_id}"))
                })?;
                let artifact = json!({
                    "data_type": "video",
                    "generated_video_id": video_id,
                    "file_path": video.file_path,
                    "thumbnail_path": video.thumbnail_path,
                    "output": {
                        "kind": "video",
                        "generated_video_id": video_id,
                        "file_path": video.file_path,
                        "thumbnail_path": video.thumbnail_path,
                    },
                });
                Ok((
                    CanvasNodeRefKind::GeneratedVideo.to_string(),
                    video_id,
                    artifact,
                ))
            }
        }
    }
}

fn validate_output_node_value(
    output: &CanvasOutputValue,
    data_type: CanvasOutputDataType,
    consumer_label: &str,
) -> Result<(), String> {
    if data_type == CanvasOutputDataType::Video && output.generated_video_id.is_none() {
        return Err("VideoOutputNode 缺少可输出的视频输入".to_string());
    }
    validate_canvas_output_value(output, data_type, consumer_label)
}
