use a4n_db::{
    canvas::{CanvasImageMediaAssetEnsureInput, CanvasNodeArtifactUpdateInput},
    generation::{
        GENERATION_SPEC_TARGET_ENTITY_KIND_CANVAS_NODE, GENERATION_SPEC_TARGET_KIND_CANVAS_IMAGE,
    },
};
use a4n_domain::image::IMAGE_TYPE_CANVAS;
use a4n_domain::{CanvasNodeRefKind, canvas::FreeformCanvasGenerationSpecUpsertInput};
use a4n_error::AppError;
use a4n_image_contract::ImageGenerationRequest;
use serde_json::json;

use crate::{
    OrchestratorContext,
    media::{
        ImageGenerationExecutionInput, ImageGenerationHostPort, ImageGenerationPostCommitAction,
        ImageGenerationProgress, ImageGenerationWorkflowBinding,
    },
    project::ProjectSourceMode,
    workflow::{WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_RUNNING},
};

use super::{contracts::CanvasImageNodeExecutionInput, input::build_canvas_image_node_input};

impl OrchestratorContext {
    pub async fn run_canvas_image_node_validated(
        &self,
        input: CanvasImageNodeExecutionInput<'_>,
        host_port: &dyn ImageGenerationHostPort,
        progress: &ImageGenerationProgress<'_>,
    ) -> Result<a4n_api_contract::canvas::CanvasRunSummary, AppError> {
        let checkpoints = match input.shared_run {
            Some(run) => Some(self.workflow_checkpoint_models(run.workflow_run_id).await?),
            None => None,
        };
        let image_input =
            build_canvas_image_node_input(input.node, input.document, checkpoints.as_deref())
                .map_err(AppError::InvalidArgument)?;
        let step_key = crate::canvas::canvas_node_step_key(input.node.id)
            .map_err(AppError::InvalidArgument)?;
        let input_json = json!({
            "project_id": input.project_id,
            "graph_id": input.graph_id,
            "node_id": input.node.id,
            "prompt": image_input.prompt,
            "config_id": image_input.config_id,
            "width": image_input.width,
            "height": image_input.height,
            "canvas_text_asset_id": image_input.canvas_text_asset_id,
            "source_node_ids": image_input.source_node_ids,
        });
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
                .upsert_canvas_image_generation_spec(&input, &image_input)
                .await?;
            lifecycle.generation_spec_id = spec.as_ref().map(|spec| spec.id);
            let config = host_port
                .resolve_image_config(image_input.config_id)
                .await?;
            let metadata_json = Some(json!({
                "config_id": config.id,
                "backend": config.backend,
                "image_type": IMAGE_TYPE_CANVAS,
                "canvas": {
                    "graph_id": input.graph_id,
                    "node_id": input.node.id,
                    "workflow_run_id": lifecycle.run.workflow_run_id,
                    "source_node_ids": image_input.source_node_ids,
                },
                "source": "video_canvas",
            }));
            let outcome = self
                .run_image_generation_validated(
                    host_port,
                    ImageGenerationExecutionInput {
                        requested_by_user_id: input.requested_by_user_id,
                        project_id: input.project_id,
                        chapter_id: None,
                        image_type: IMAGE_TYPE_CANVAS.to_string(),
                        command: "generate_canvas_image".to_string(),
                        file_extra: None,
                        file_context: "图片文件".to_string(),
                        width: image_input.width,
                        height: image_input.height,
                        request: ImageGenerationRequest {
                            prompt: image_input.prompt.clone(),
                            width: image_input.width,
                            height: image_input.height,
                            model: config.model.clone(),
                            reference_images: Vec::new(),
                        },
                        config,
                        panel_id: None,
                        metadata_json,
                        runtime_config_snapshot: host_port.runtime_config_snapshot(),
                        workflow: ImageGenerationWorkflowBinding::Attached {
                            workflow_run_id: lifecycle.run.workflow_run_id,
                        },
                        post_commit: ImageGenerationPostCommitAction::None,
                    },
                    progress,
                )
                .await?;
            if let Some(spec) = &spec {
                self.mark_canvas_generation_spec_status_record(spec.id, WORKFLOW_STATUS_COMPLETED)
                    .await?;
            }
            let media_asset_id = self
                .sync_canvas_image_generation_asset(
                    input.project_id,
                    outcome.summary.id,
                    spec.as_ref().map(|spec| spec.id),
                )
                .await?;
            let artifact_json = json!({
                "canvas_runtime_result": true,
                "data_type": "image",
                "generated_image_id": outcome.summary.id,
                "file_path": outcome.summary.file_path,
                "generation_spec_id": spec.as_ref().map(|spec| spec.id),
                "media_asset_id": media_asset_id,
            });
            self.update_canvas_node_artifact_record(CanvasNodeArtifactUpdateInput {
                node_id: input.node.id,
                ref_type: CanvasNodeRefKind::GeneratedImage.to_string(),
                ref_id: outcome.summary.id,
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

    async fn upsert_canvas_image_generation_spec(
        &self,
        input: &CanvasImageNodeExecutionInput<'_>,
        image_input: &super::contracts::CanvasImageNodeInput,
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
        let source_material_json = json!({
            "prompt": image_input.prompt,
            "source_node_ids": image_input.source_node_ids,
        });
        self.upsert_freeform_canvas_generation_spec_record(
            FreeformCanvasGenerationSpecUpsertInput {
                project_id: input.project_id,
                target_entity_id: input.node.id,
                canvas_text_asset_id: image_input.canvas_text_asset_id,
                target_kind: GENERATION_SPEC_TARGET_KIND_CANVAS_IMAGE.to_string(),
                target_entity_kind: GENERATION_SPEC_TARGET_ENTITY_KIND_CANVAS_NODE.to_string(),
                status: WORKFLOW_STATUS_RUNNING.to_string(),
                params_json: json!({
                    "canvas_graph_id": input.graph_id,
                    "canvas_node_id": input.node.id,
                    "source_material": source_material_json,
                    "params": {
                        "workflow": "generate_image",
                        "config_id": image_input.config_id,
                        "node_config": input.node.config_json,
                    },
                }),
            },
        )
        .await
        .map(Some)
    }

    async fn sync_canvas_image_generation_asset(
        &self,
        project_id: i64,
        image_id: i64,
        generation_spec_id: Option<i64>,
    ) -> Result<Option<i64>, AppError> {
        let Some(generation_spec_id) = generation_spec_id else {
            return Ok(None);
        };
        let image =
            self.generated_image_by_id(image_id)
                .await?
                .ok_or_else(|| AppError::InvalidState {
                    current: "image_generated".to_string(),
                    operation: format!("画布图片产物不存在: {image_id}"),
                })?;
        let asset = self
            .ensure_canvas_image_media_asset_record(CanvasImageMediaAssetEnsureInput {
                project_id,
                image,
                generation_spec_id: Some(generation_spec_id),
                origin: "generated_canvas_image".to_string(),
            })
            .await?;
        Ok(Some(asset.id))
    }
}
