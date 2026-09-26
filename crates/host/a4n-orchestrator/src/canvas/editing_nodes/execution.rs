use a4n_canvas_contract::{
    CanvasAssembleVideosHostInput, CanvasEditingNodeHostPort, CanvasExtractFrameHostInput,
    CanvasTrimVideoHostInput,
};
use a4n_db::{
    canvas::{
        CanvasImageMediaAssetEnsureInput, CanvasNodeArtifactUpdateInput,
        CanvasVideoMediaAssetEnsureInput,
    },
    generation::{
        GENERATION_SPEC_TARGET_ENTITY_KIND_CANVAS_NODE,
        GENERATION_SPEC_TARGET_KIND_CANVAS_ASSEMBLY, GENERATION_SPEC_TARGET_KIND_CANVAS_IMAGE,
        GENERATION_SPEC_TARGET_KIND_CANVAS_VIDEO,
    },
};
use a4n_domain::{
    CanvasNodeRefKind,
    canvas::{
        CanvasAssemblyContractSyncInput, CanvasAssemblySourceVideoInput,
        FreeformCanvasGenerationSpecUpsertInput,
    },
};
use a4n_error::AppError;
use serde_json::{Value, json};

use crate::{
    OrchestratorContext,
    project::ProjectSourceMode,
    workflow::{WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_RUNNING},
};

use super::{
    contracts::{
        CanvasEditingNodeExecutionInput, CanvasEditingNodeInput, CanvasExtractFrameNodeInput,
        CanvasTrimVideoNodeInput,
    },
    input::build_canvas_editing_node_input,
};

impl OrchestratorContext {
    pub async fn run_canvas_editing_node_validated(
        &self,
        input: CanvasEditingNodeExecutionInput<'_>,
        host_port: &dyn CanvasEditingNodeHostPort,
    ) -> Result<a4n_api_contract::canvas::CanvasRunSummary, AppError> {
        let checkpoints = match input.shared_run {
            Some(run) => Some(self.workflow_checkpoint_models(run.workflow_run_id).await?),
            None => None,
        };
        let node_input =
            build_canvas_editing_node_input(input.node, input.document, checkpoints.as_deref())
                .map_err(AppError::InvalidArgument)?;
        let step_key = crate::canvas::canvas_node_step_key(input.node.id)
            .map_err(AppError::InvalidArgument)?;
        let input_json = editing_input_json(&input, &node_input);
        let mut lifecycle = self
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
            let spec = self
                .upsert_canvas_editing_generation_spec(&input, &node_input)
                .await?;
            lifecycle.generation_spec_id = spec.as_ref().map(|row| row.id);
            let artifact_json = match &node_input {
                CanvasEditingNodeInput::ExtractFrame(node_input) => {
                    let output = host_port
                        .extract_frame(CanvasExtractFrameHostInput {
                            user_id: input.user_id,
                            project_id: input.project_id,
                            workflow_run_id: lifecycle.run.workflow_run_id,
                            video_id: node_input.video_id,
                            frame_kind: node_input.frame_kind,
                            at_secs: node_input.at_secs,
                        })
                        .await?;
                    let image = self
                        .generated_image_by_id(output.generated_image_id)
                        .await?
                        .ok_or_else(|| AppError::InvalidState {
                            current: "extract_frame_completed".to_string(),
                            operation: format!("画布抽帧产物不存在: {}", output.generated_image_id),
                        })?;
                    let media_asset = self
                        .ensure_canvas_image_media_asset_record(CanvasImageMediaAssetEnsureInput {
                            project_id: input.project_id,
                            image: image.clone(),
                            generation_spec_id: spec.as_ref().map(|row| row.id),
                            origin: "canvas_extracted_frame".to_string(),
                        })
                        .await?;
                    json!({
                        "canvas_runtime_result": true,
                        "data_type": "image",
                        "generated_image_id": image.id,
                        "file_path": image.file_path,
                        "generation_spec_id": spec.as_ref().map(|row| row.id),
                        "media_asset_id": media_asset.id,
                    })
                }
                CanvasEditingNodeInput::TrimVideo(node_input) => {
                    let output = host_port
                        .trim_video(CanvasTrimVideoHostInput {
                            user_id: input.user_id,
                            project_id: input.project_id,
                            workflow_run_id: lifecycle.run.workflow_run_id,
                            video_id: node_input.video_id,
                            start_secs: node_input.start_secs,
                            end_secs: node_input.end_secs,
                        })
                        .await?;
                    let video = self
                        .generated_video_by_id(output.generated_video_id)
                        .await?
                        .ok_or_else(|| AppError::InvalidState {
                            current: "trim_video_completed".to_string(),
                            operation: format!("画布裁剪产物不存在: {}", output.generated_video_id),
                        })?;
                    let media_asset = self
                        .ensure_canvas_video_media_asset_record(CanvasVideoMediaAssetEnsureInput {
                            project_id: input.project_id,
                            video: video.clone(),
                            generation_spec_id: spec.as_ref().map(|row| row.id),
                            origin: "canvas_trim".to_string(),
                        })
                        .await?;
                    json!({
                        "canvas_runtime_result": true,
                        "data_type": "video",
                        "generated_video_id": video.id,
                        "file_path": video.file_path,
                        "thumbnail_path": video.thumbnail_path,
                        "generation_spec_id": spec.as_ref().map(|row| row.id),
                        "media_asset_id": media_asset.id,
                    })
                }
                CanvasEditingNodeInput::AssembleVideo(node_input) => {
                    let output = host_port
                        .assemble_videos(CanvasAssembleVideosHostInput {
                            user_id: input.user_id,
                            project_id: input.project_id,
                            workflow_run_id: lifecycle.run.workflow_run_id,
                            video_ids: node_input.video_ids.clone(),
                            title: node_input.title.clone(),
                        })
                        .await?;
                    let video = self
                        .generated_video_by_id(output.generated_video_id)
                        .await?
                        .ok_or_else(|| AppError::InvalidState {
                            current: "assemble_video_completed".to_string(),
                            operation: format!("画布成片产物不存在: {}", output.generated_video_id),
                        })?;
                    let contract = self
                        .sync_canvas_assembly_contract_records(CanvasAssemblyContractSyncInput {
                            project_id: input.project_id,
                            graph_id: input.graph_id,
                            node_id: input.node.id,
                            assembly_video_id: video.id,
                            source_videos: node_input
                                .video_ids
                                .iter()
                                .copied()
                                .map(|video_id| CanvasAssemblySourceVideoInput { video_id })
                                .collect(),
                            generation_spec_id: spec.as_ref().map(|row| row.id),
                        })
                        .await?;
                    json!({
                        "canvas_runtime_result": true,
                        "data_type": "video",
                        "generated_video_id": video.id,
                        "file_path": video.file_path,
                        "thumbnail_path": video.thumbnail_path,
                        "generation_spec_id": spec.as_ref().map(|row| row.id),
                        "media_asset_id": contract.preview_media_asset_id,
                    })
                }
            };
            if let Some(spec) = &spec {
                self.mark_canvas_generation_spec_status_record(spec.id, WORKFLOW_STATUS_COMPLETED)
                    .await?;
            }
            let (ref_type, ref_id) = match artifact_json.get("data_type").and_then(Value::as_str) {
                Some("image") => (
                    CanvasNodeRefKind::GeneratedImage,
                    artifact_json
                        .get("generated_image_id")
                        .and_then(Value::as_i64)
                        .ok_or_else(|| AppError::InvalidState {
                            current: "extract_frame_completed".to_string(),
                            operation: "画布图片结果缺少 generated_image_id".to_string(),
                        })?,
                ),
                Some("video") => (
                    CanvasNodeRefKind::GeneratedVideo,
                    artifact_json
                        .get("generated_video_id")
                        .and_then(Value::as_i64)
                        .ok_or_else(|| AppError::InvalidState {
                            current: "canvas_video_edit_completed".to_string(),
                            operation: "画布视频结果缺少 generated_video_id".to_string(),
                        })?,
                ),
                other => {
                    return Err(AppError::InvalidState {
                        current: format!("canvas_edit_data_type={other:?}"),
                        operation: "保存画布编辑节点结果".to_string(),
                    });
                }
            };
            self.update_canvas_node_artifact_record(CanvasNodeArtifactUpdateInput {
                node_id: input.node.id,
                ref_type: ref_type.to_string(),
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

    async fn upsert_canvas_editing_generation_spec(
        &self,
        input: &CanvasEditingNodeExecutionInput<'_>,
        node_input: &CanvasEditingNodeInput,
    ) -> Result<Option<a4n_db_models::models::generation::generation_spec::Model>, AppError> {
        let project = self.project_by_id(input.project_id).await?.ok_or_else(|| {
            AppError::InvalidArgument(format!("项目不存在: {}", input.project_id))
        })?;
        if project
            .source_mode()
            .map_err(|error| AppError::InvalidArgument(format!("项目 {}: {error}", project.id)))?
            != ProjectSourceMode::Freeform
        {
            return Ok(None);
        }
        let (target_kind, workflow, canvas_text_asset_id, source_material_json) =
            editing_spec_fields(node_input);
        self.upsert_freeform_canvas_generation_spec_record(
            FreeformCanvasGenerationSpecUpsertInput {
                project_id: input.project_id,
                target_entity_id: input.node.id,
                canvas_text_asset_id,
                target_kind: target_kind.to_string(),
                target_entity_kind: GENERATION_SPEC_TARGET_ENTITY_KIND_CANVAS_NODE.to_string(),
                status: WORKFLOW_STATUS_RUNNING.to_string(),
                params_json: json!({
                    "canvas_graph_id": input.graph_id,
                    "canvas_node_id": input.node.id,
                    "source_material": source_material_json,
                    "params": {
                        "workflow": workflow,
                        "node_config": input.node.config_json,
                    },
                }),
            },
        )
        .await
        .map(Some)
    }
}

fn editing_input_json(
    input: &CanvasEditingNodeExecutionInput<'_>,
    node_input: &CanvasEditingNodeInput,
) -> Value {
    match node_input {
        CanvasEditingNodeInput::ExtractFrame(node_input) => json!({
            "project_id": input.project_id,
            "graph_id": input.graph_id,
            "node_id": input.node.id,
            "video_id": node_input.video_id,
            "frame_kind": node_input.frame_kind,
            "at_secs": node_input.at_secs,
            "source_node_ids": node_input.source_node_ids,
        }),
        CanvasEditingNodeInput::TrimVideo(node_input) => json!({
            "project_id": input.project_id,
            "graph_id": input.graph_id,
            "node_id": input.node.id,
            "video_id": node_input.video_id,
            "start_secs": node_input.start_secs,
            "end_secs": node_input.end_secs,
            "source_node_ids": node_input.source_node_ids,
        }),
        CanvasEditingNodeInput::AssembleVideo(node_input) => json!({
            "project_id": input.project_id,
            "graph_id": input.graph_id,
            "node_id": input.node.id,
            "video_ids": node_input.video_ids,
            "title": node_input.title,
            "source_node_ids": node_input.source_node_ids,
        }),
    }
}

fn editing_spec_fields(
    node_input: &CanvasEditingNodeInput,
) -> (&'static str, &'static str, Option<i64>, Value) {
    match node_input {
        CanvasEditingNodeInput::ExtractFrame(CanvasExtractFrameNodeInput {
            video_id,
            frame_kind,
            at_secs,
            canvas_text_asset_id,
            source_node_ids,
        }) => (
            GENERATION_SPEC_TARGET_KIND_CANVAS_IMAGE,
            "extract_frame",
            *canvas_text_asset_id,
            json!({
                "video_id": video_id,
                "frame_kind": frame_kind,
                "at_secs": at_secs,
                "source_node_ids": source_node_ids,
            }),
        ),
        CanvasEditingNodeInput::TrimVideo(CanvasTrimVideoNodeInput {
            video_id,
            start_secs,
            end_secs,
            canvas_text_asset_id,
            source_node_ids,
        }) => (
            GENERATION_SPEC_TARGET_KIND_CANVAS_VIDEO,
            "trim_video",
            *canvas_text_asset_id,
            json!({
                "video_id": video_id,
                "start_secs": start_secs,
                "end_secs": end_secs,
                "source_node_ids": source_node_ids,
            }),
        ),
        CanvasEditingNodeInput::AssembleVideo(node_input) => (
            GENERATION_SPEC_TARGET_KIND_CANVAS_ASSEMBLY,
            "assemble",
            node_input.canvas_text_asset_id,
            json!({
                "video_ids": node_input.video_ids,
                "title": node_input.title,
                "source_node_ids": node_input.source_node_ids,
            }),
        ),
    }
}
