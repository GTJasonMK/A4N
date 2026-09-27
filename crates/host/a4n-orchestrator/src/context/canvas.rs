//! Canvas 领域私有 owner。
//!
//! 持有 `DbHandle`，封装画布图读写、生成规格/提示词产物、媒体资产与 Video Studio 导入导出的数据库行为。

use a4n_canvas_contract::{
    CanvasEditingNodeApplicationPort, CanvasEditingNodeHostPort,
    CanvasGraphExecutionApplicationPort, CanvasGraphExecutionHostPort,
    CanvasGraphFailureApplicationPort, CanvasGraphPreparationApplicationPort,
    CanvasGraphPreparationHostPort, CanvasGraphResumePreparationInput,
    CanvasGraphRunPreparationInput, CanvasImageNodeApplicationInput,
    CanvasLlmTextNodeApplicationInput, CanvasOutputNodeApplicationInput,
    CanvasOutputNodeApplicationPort, CanvasPreviewNodeApplicationInput,
    CanvasProbeNodeApplicationInput, CanvasProbeNodeApplicationPort,
    CanvasRenderTextNodeApplicationInput, CanvasRenderTextNodeApplicationPort,
    CanvasSingleNodeRunPreparationInput, CanvasVideoNodeApplicationInput, GenerateCanvasImageInput,
    GenerateCanvasVideoInput, PreparedCanvasGraphRun,
};
use a4n_db::DbHandle;
use a4n_db::canvas::{
    CanvasAssemblyContractSyncSummary, CanvasGraphRecordDocument, CanvasImageMediaAssetEnsureInput,
    CanvasMediaAssetProbeRecord, CanvasNodeArtifactUpdateInput, CanvasNodeObservationUpdateInput,
    CanvasVideoMediaAssetEnsureInput, VideoStudioProjectExportRows,
    VideoStudioProjectImportMediaFileWriter, validate_canvas_record_document_edge_contract,
};
use a4n_db_models::models::{
    canvas::canvas_text_asset,
    generation::{generation_spec, prompt_artifact},
    media::media_asset,
    project::novel_project,
};
use a4n_domain::canvas::{
    CanvasAssemblyContractSyncInput, CanvasPromptArtifactUpsertInput, CanvasTextAssetRecordInput,
    FreeformCanvasGenerationSpecUpsertInput,
};
use a4n_error::AppError;
use chrono::{NaiveDateTime, Utc};

use super::OrchestratorContext;
use crate::canvas::{
    CanvasImageNodeApplicationPort, CanvasLlmTextNodeApplicationPort, CanvasNodeRequestPreview,
    CanvasNodeRunHandle, CanvasPreviewApplicationPort, CanvasVideoNodeApplicationPort,
    canvas_db_error, video_studio_project_import_error,
};
use crate::project::VideoStudioProjectPackage;
use a4n_domain::collaboration::ProjectAccessRole;

#[cfg(test)]
use a4n_db::canvas::CanvasGraphRecordInput;

/// Canvas 领域 application owner。
#[derive(Clone)]
pub(super) struct CanvasApplication {
    database: DbHandle,
}

impl CanvasApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    fn connection(&self) -> &sea_orm::DatabaseConnection {
        self.database.connection()
    }

    async fn get_or_create_canvas_graph_record(
        &self,
        project_id: i64,
        graph_id: Option<i64>,
        now: NaiveDateTime,
    ) -> Result<CanvasGraphRecordDocument, AppError> {
        a4n_db::canvas::get_or_create_canvas_graph_record(
            self.connection(),
            project_id,
            graph_id,
            now,
        )
        .await
        .map_err(canvas_db_error)
    }

    #[cfg(test)]
    async fn save_canvas_graph_record(
        &self,
        project_id: i64,
        input: CanvasGraphRecordInput,
        now: NaiveDateTime,
    ) -> Result<CanvasGraphRecordDocument, AppError> {
        a4n_db::canvas::save_canvas_graph_record(self.connection(), project_id, input, now)
            .await
            .map(|saved| saved.document)
            .map_err(canvas_db_error)
    }

    async fn upsert_canvas_text_asset_record(
        &self,
        project_id: i64,
        input: CanvasTextAssetRecordInput,
        now: NaiveDateTime,
    ) -> Result<canvas_text_asset::Model, AppError> {
        a4n_db::canvas::upsert_canvas_text_asset_record(self.connection(), project_id, input, now)
            .await
            .map_err(canvas_db_error)
    }

    async fn upsert_freeform_canvas_generation_spec_record(
        &self,
        input: FreeformCanvasGenerationSpecUpsertInput,
    ) -> Result<generation_spec::Model, AppError> {
        a4n_db::canvas::upsert_freeform_canvas_generation_spec_record(self.connection(), input)
            .await
            .map_err(canvas_db_error)
    }

    async fn mark_canvas_generation_spec_status_record(
        &self,
        generation_spec_id: i64,
        status: &str,
    ) -> Result<(), AppError> {
        a4n_db::canvas::mark_canvas_generation_spec_status_record(
            self.connection(),
            generation_spec_id,
            status,
        )
        .await
        .map_err(canvas_db_error)
    }

    async fn upsert_canvas_prompt_artifact_record(
        &self,
        input: CanvasPromptArtifactUpsertInput,
    ) -> Result<prompt_artifact::Model, AppError> {
        a4n_db::canvas::upsert_canvas_prompt_artifact_record(self.connection(), input)
            .await
            .map_err(canvas_db_error)
    }

    async fn update_canvas_node_artifact_record(
        &self,
        input: CanvasNodeArtifactUpdateInput,
    ) -> Result<(), AppError> {
        a4n_db::canvas::update_canvas_node_artifact_record(self.connection(), input)
            .await
            .map_err(canvas_db_error)
    }

    async fn update_canvas_node_observation_record(
        &self,
        input: CanvasNodeObservationUpdateInput,
    ) -> Result<(), AppError> {
        a4n_db::canvas::update_canvas_node_observation_record(self.connection(), input)
            .await
            .map_err(canvas_db_error)
    }

    async fn ensure_canvas_image_media_asset_record(
        &self,
        input: CanvasImageMediaAssetEnsureInput,
    ) -> Result<media_asset::Model, AppError> {
        a4n_db::canvas::ensure_canvas_image_media_asset_record(self.connection(), input)
            .await
            .map_err(canvas_db_error)
    }

    async fn ensure_canvas_video_media_asset_record(
        &self,
        input: CanvasVideoMediaAssetEnsureInput,
    ) -> Result<media_asset::Model, AppError> {
        a4n_db::canvas::ensure_canvas_video_media_asset_record(self.connection(), input)
            .await
            .map_err(canvas_db_error)
    }

    async fn sync_canvas_assembly_contract_records(
        &self,
        input: CanvasAssemblyContractSyncInput,
    ) -> Result<CanvasAssemblyContractSyncSummary, AppError> {
        a4n_db::canvas::sync_canvas_assembly_contract_records(self.connection(), input)
            .await
            .map_err(canvas_db_error)
    }

    async fn canvas_media_asset_by_id(
        &self,
        media_asset_id: i64,
    ) -> Result<Option<CanvasMediaAssetProbeRecord>, AppError> {
        a4n_db::canvas::canvas_media_asset_by_id(self.connection(), media_asset_id)
            .await
            .map_err(canvas_db_error)
    }

    pub(super) async fn video_studio_project_export_rows(
        &self,
        project_id: i64,
    ) -> Result<VideoStudioProjectExportRows, AppError> {
        a4n_db::canvas::video_studio_project_export_rows(self.connection(), project_id)
            .await
            .map_err(canvas_db_error)
    }

    /// 落库已校验的视频画布包（媒体文件写入由 Host writer 完成）。
    pub(super) async fn import_video_studio_package<Writer>(
        &self,
        user_id: i64,
        package: VideoStudioProjectPackage,
        now: NaiveDateTime,
        media_file_writer: &mut Writer,
    ) -> Result<novel_project::Model, AppError>
    where
        Writer: VideoStudioProjectImportMediaFileWriter,
    {
        let rows = package.into_import_rows();
        a4n_db::canvas::import_video_studio_project_record(
            self.connection(),
            user_id,
            rows,
            now,
            media_file_writer,
        )
        .await
        .map_err(video_studio_project_import_error)
    }
}

impl OrchestratorContext {
    pub(crate) async fn get_or_create_canvas_graph_record(
        &self,
        project_id: i64,
        graph_id: Option<i64>,
        now: NaiveDateTime,
    ) -> Result<CanvasGraphRecordDocument, AppError> {
        self.canvas
            .get_or_create_canvas_graph_record(project_id, graph_id, now)
            .await
    }

    #[cfg(test)]
    pub(crate) async fn save_canvas_graph_record(
        &self,
        project_id: i64,
        input: CanvasGraphRecordInput,
        now: NaiveDateTime,
    ) -> Result<CanvasGraphRecordDocument, AppError> {
        self.canvas
            .save_canvas_graph_record(project_id, input, now)
            .await
    }

    pub(crate) async fn upsert_canvas_text_asset_record(
        &self,
        project_id: i64,
        input: CanvasTextAssetRecordInput,
        now: NaiveDateTime,
    ) -> Result<canvas_text_asset::Model, AppError> {
        self.canvas
            .upsert_canvas_text_asset_record(project_id, input, now)
            .await
    }

    pub(crate) async fn upsert_freeform_canvas_generation_spec_record(
        &self,
        input: FreeformCanvasGenerationSpecUpsertInput,
    ) -> Result<generation_spec::Model, AppError> {
        self.canvas
            .upsert_freeform_canvas_generation_spec_record(input)
            .await
    }

    pub(crate) async fn mark_canvas_generation_spec_status_record(
        &self,
        generation_spec_id: i64,
        status: &str,
    ) -> Result<(), AppError> {
        self.canvas
            .mark_canvas_generation_spec_status_record(generation_spec_id, status)
            .await
    }

    pub(crate) async fn upsert_canvas_prompt_artifact_record(
        &self,
        input: CanvasPromptArtifactUpsertInput,
    ) -> Result<prompt_artifact::Model, AppError> {
        self.canvas
            .upsert_canvas_prompt_artifact_record(input)
            .await
    }

    pub(crate) async fn update_canvas_node_artifact_record(
        &self,
        input: CanvasNodeArtifactUpdateInput,
    ) -> Result<(), AppError> {
        self.canvas.update_canvas_node_artifact_record(input).await
    }

    pub(crate) async fn update_canvas_node_observation_record(
        &self,
        input: CanvasNodeObservationUpdateInput,
    ) -> Result<(), AppError> {
        self.canvas
            .update_canvas_node_observation_record(input)
            .await
    }

    pub(crate) async fn ensure_canvas_image_media_asset_record(
        &self,
        input: CanvasImageMediaAssetEnsureInput,
    ) -> Result<media_asset::Model, AppError> {
        self.canvas
            .ensure_canvas_image_media_asset_record(input)
            .await
    }

    pub(crate) async fn ensure_canvas_video_media_asset_record(
        &self,
        input: CanvasVideoMediaAssetEnsureInput,
    ) -> Result<media_asset::Model, AppError> {
        self.canvas
            .ensure_canvas_video_media_asset_record(input)
            .await
    }

    pub(crate) async fn sync_canvas_assembly_contract_records(
        &self,
        input: CanvasAssemblyContractSyncInput,
    ) -> Result<CanvasAssemblyContractSyncSummary, AppError> {
        self.canvas
            .sync_canvas_assembly_contract_records(input)
            .await
    }

    pub(crate) async fn canvas_media_asset_by_id(
        &self,
        media_asset_id: i64,
    ) -> Result<Option<CanvasMediaAssetProbeRecord>, AppError> {
        self.canvas.canvas_media_asset_by_id(media_asset_id).await
    }

    /// 画布整图运行：校验后将 workflow 标为 completed（节点执行仍由 Host 推进）。
    pub(crate) async fn complete_canvas_graph_workflow_validated(
        &self,
        workflow_run_id: i64,
    ) -> Result<a4n_db_models::models::generation::workflow_run::Model, AppError> {
        self.complete_workflow_run_validated(workflow_run_id).await
    }

    /// 画布整图：校验后创建 workflow_run（RUNNING）。
    pub(crate) async fn begin_canvas_graph_workflow_validated(
        &self,
        project_id: i64,
        graph_id: i64,
        mode: &str,
        unique_suffix: i64,
        input_fingerprint_json: serde_json::Value,
        first_step_key: Option<String>,
    ) -> Result<a4n_db_models::models::generation::workflow_run::Model, AppError> {
        let input = crate::canvas::canvas_graph_workflow_run_input(
            project_id,
            graph_id,
            mode,
            unique_suffix,
            input_fingerprint_json,
            first_step_key,
        )
        .map_err(AppError::InvalidArgument)?;
        self.begin_workflow_run_validated(input).await
    }

    /// 画布单节点：校验后创建 workflow_run（RUNNING）。
    pub(crate) async fn begin_canvas_single_node_workflow_validated(
        &self,
        project_id: i64,
        graph_id: i64,
        node_id: i64,
        unique_suffix: i64,
        input_fingerprint_json: serde_json::Value,
        step_key: String,
    ) -> Result<a4n_db_models::models::generation::workflow_run::Model, AppError> {
        let input = crate::canvas::canvas_single_node_workflow_run_input(
            project_id,
            graph_id,
            node_id,
            unique_suffix,
            input_fingerprint_json,
            step_key,
        )
        .map_err(AppError::InvalidArgument)?;
        self.begin_workflow_run_validated(input).await
    }

    /// 画布节点独立 step run：校验后创建 workflow_run（RUNNING）。
    pub(crate) async fn begin_canvas_node_step_workflow_validated(
        &self,
        project_id: i64,
        graph_id: i64,
        mode: &str,
        step_key: &str,
        input_fingerprint_json: serde_json::Value,
    ) -> Result<a4n_db_models::models::generation::workflow_run::Model, AppError> {
        let input = crate::canvas::canvas_node_step_workflow_run_input(
            project_id,
            graph_id,
            mode,
            step_key,
            input_fingerprint_json,
        )
        .map_err(AppError::InvalidArgument)?;
        self.begin_workflow_run_validated(input).await
    }
}

#[async_trait::async_trait]
impl CanvasEditingNodeApplicationPort for OrchestratorContext {
    async fn run_editing(
        &self,
        input: a4n_canvas_contract::CanvasEditingNodeApplicationInput,
        host_port: &dyn CanvasEditingNodeHostPort,
    ) -> Result<a4n_api_contract::canvas::CanvasRunSummary, AppError> {
        self.verify_project_permission(input.project_id, input.user_id, ProjectAccessRole::Editor)
            .await?;
        let document = self
            .get_or_create_canvas_graph_record(
                input.project_id,
                Some(input.graph_id),
                Utc::now().naive_utc(),
            )
            .await?;
        validate_canvas_record_document_edge_contract(&document)
            .map_err(crate::canvas::canvas_db_error)?;
        let node = document
            .nodes
            .iter()
            .find(|node| node.id == input.node_id)
            .ok_or_else(|| AppError::NotFound(format!("画布节点不存在: {}", input.node_id)))?;
        self.run_canvas_editing_node_validated(
            crate::canvas::CanvasEditingNodeExecutionInput {
                user_id: input.user_id,
                project_id: input.project_id,
                graph_id: input.graph_id,
                node,
                document: &document,
                shared_run: input.shared_run.map(|run| CanvasNodeRunHandle {
                    workflow_run_id: run.workflow_run_id,
                    finalize_on_success: run.finalize_on_success,
                    graph_version: run.graph_version,
                }),
                mode: &input.mode,
            },
            host_port,
        )
        .await
    }
}

#[async_trait::async_trait]
impl CanvasProbeNodeApplicationPort for OrchestratorContext {
    async fn run_probe(
        &self,
        input: CanvasProbeNodeApplicationInput,
    ) -> Result<a4n_api_contract::canvas::CanvasRunSummary, AppError> {
        self.verify_project_permission(input.project_id, input.user_id, ProjectAccessRole::Editor)
            .await?;
        let document = self
            .get_or_create_canvas_graph_record(
                input.project_id,
                Some(input.graph_id),
                Utc::now().naive_utc(),
            )
            .await?;
        validate_canvas_record_document_edge_contract(&document)
            .map_err(crate::canvas::canvas_db_error)?;
        let node = document
            .nodes
            .iter()
            .find(|node| node.id == input.node_id)
            .ok_or_else(|| AppError::NotFound(format!("画布节点不存在: {}", input.node_id)))?;
        self.run_canvas_probe_node_validated(crate::canvas::CanvasProbeNodeExecutionInput {
            project_id: input.project_id,
            graph_id: input.graph_id,
            node,
            document: &document,
            shared_run: input.shared_run.map(|run| CanvasNodeRunHandle {
                workflow_run_id: run.workflow_run_id,
                finalize_on_success: run.finalize_on_success,
                graph_version: run.graph_version,
            }),
            mode: &input.mode,
        })
        .await
    }
}

#[async_trait::async_trait]
impl CanvasOutputNodeApplicationPort for OrchestratorContext {
    async fn run_output(
        &self,
        input: CanvasOutputNodeApplicationInput,
    ) -> Result<a4n_api_contract::canvas::CanvasRunSummary, AppError> {
        self.verify_project_permission(input.project_id, input.user_id, ProjectAccessRole::Editor)
            .await?;
        let document = self
            .get_or_create_canvas_graph_record(
                input.project_id,
                Some(input.graph_id),
                Utc::now().naive_utc(),
            )
            .await?;
        validate_canvas_record_document_edge_contract(&document)
            .map_err(crate::canvas::canvas_db_error)?;
        let node = document
            .nodes
            .iter()
            .find(|node| node.id == input.node_id)
            .ok_or_else(|| AppError::NotFound(format!("画布节点不存在: {}", input.node_id)))?;
        self.run_canvas_output_node_validated(crate::canvas::CanvasOutputNodeExecutionInput {
            project_id: input.project_id,
            graph_id: input.graph_id,
            node,
            document: &document,
            shared_run: input.shared_run.map(|run| CanvasNodeRunHandle {
                workflow_run_id: run.workflow_run_id,
                finalize_on_success: run.finalize_on_success,
                graph_version: run.graph_version,
            }),
            mode: &input.mode,
        })
        .await
    }
}

#[async_trait::async_trait]
impl CanvasRenderTextNodeApplicationPort for OrchestratorContext {
    async fn run_render_text(
        &self,
        input: CanvasRenderTextNodeApplicationInput,
    ) -> Result<a4n_api_contract::canvas::CanvasRunSummary, AppError> {
        self.verify_project_permission(input.project_id, input.user_id, ProjectAccessRole::Editor)
            .await?;
        let document = self
            .get_or_create_canvas_graph_record(
                input.project_id,
                Some(input.graph_id),
                Utc::now().naive_utc(),
            )
            .await?;
        validate_canvas_record_document_edge_contract(&document)
            .map_err(crate::canvas::canvas_db_error)?;
        let node = document
            .nodes
            .iter()
            .find(|node| node.id == input.node_id)
            .ok_or_else(|| AppError::NotFound(format!("画布节点不存在: {}", input.node_id)))?;
        self.run_canvas_render_text_node_validated(crate::canvas::CanvasTextNodeExecutionInput {
            project_id: input.project_id,
            graph_id: input.graph_id,
            node,
            document: &document,
            shared_run: input.shared_run.map(|run| CanvasNodeRunHandle {
                workflow_run_id: run.workflow_run_id,
                finalize_on_success: run.finalize_on_success,
                graph_version: run.graph_version,
            }),
            mode: &input.mode,
        })
        .await
    }
}

#[async_trait::async_trait]
impl CanvasPreviewApplicationPort for OrchestratorContext {
    async fn prepare_node(
        &self,
        input: CanvasPreviewNodeApplicationInput,
    ) -> Result<CanvasNodeRequestPreview, AppError> {
        self.verify_project_permission(input.project_id, input.user_id, ProjectAccessRole::Viewer)
            .await?;
        let document = self
            .get_or_create_canvas_graph_record(
                input.project_id,
                Some(input.graph_id),
                Utc::now().naive_utc(),
            )
            .await?;
        validate_canvas_record_document_edge_contract(&document)
            .map_err(crate::canvas::canvas_db_error)?;
        let node = document
            .nodes
            .iter()
            .find(|node| node.id == input.node_id)
            .ok_or_else(|| AppError::NotFound(format!("画布节点不存在: {}", input.node_id)))?;
        match node.node_kind.as_str() {
            "GenerateImageNode" => {
                let image =
                    crate::canvas::build_canvas_image_node_input_for_preview(node, &document)
                        .map_err(AppError::InvalidArgument)?;
                Ok(CanvasNodeRequestPreview::Image(GenerateCanvasImageInput {
                    project_id: input.project_id,
                    graph_id: input.graph_id,
                    node_id: input.node_id,
                    workflow_run_id: None,
                    prompt: image.prompt,
                    config_id: image.config_id,
                    width: image.width,
                    height: image.height,
                    source_node_ids: image.source_node_ids,
                }))
            }
            "GenerateVideoNode" => {
                let video =
                    crate::canvas::build_canvas_video_node_input_for_preview(node, &document)
                        .map_err(AppError::InvalidArgument)?;
                Ok(CanvasNodeRequestPreview::Video(GenerateCanvasVideoInput {
                    user_id: input.user_id,
                    project_id: input.project_id,
                    graph_id: input.graph_id,
                    node_id: input.node_id,
                    workflow_run_id: None,
                    prompt: video.prompt,
                    negative_prompt: video.negative_prompt,
                    config_id: video.config_id,
                    width: video.width,
                    height: video.height,
                    duration_secs: video.duration_secs,
                    fps: video.fps,
                    first_frame_image_id: video.first_frame_image_id,
                    last_frame_image_id: video.last_frame_image_id,
                    reference_image_ids: video.reference_image_ids,
                    source_node_ids: video.source_node_ids,
                }))
            }
            "LlmTextNode" => Ok(CanvasNodeRequestPreview::Llm(
                crate::canvas::build_canvas_llm_text_request_for_preview(
                    input.project_id,
                    input.graph_id,
                    node,
                    &document,
                )
                .map_err(AppError::InvalidArgument)?,
            )),
            other => Err(AppError::InvalidArgument(format!(
                "画布节点不支持模型请求预览: {other}"
            ))),
        }
    }
}

#[async_trait::async_trait]
impl CanvasImageNodeApplicationPort for OrchestratorContext {
    async fn run_image(
        &self,
        input: CanvasImageNodeApplicationInput,
        host_port: &dyn crate::media::ImageGenerationHostPort,
        progress: &crate::media::ImageGenerationProgress<'_>,
    ) -> Result<a4n_api_contract::canvas::CanvasRunSummary, AppError> {
        self.verify_project_permission(input.project_id, input.user_id, ProjectAccessRole::Editor)
            .await?;
        let document = self
            .get_or_create_canvas_graph_record(
                input.project_id,
                Some(input.graph_id),
                Utc::now().naive_utc(),
            )
            .await?;
        validate_canvas_record_document_edge_contract(&document)
            .map_err(crate::canvas::canvas_db_error)?;
        let node = document
            .nodes
            .iter()
            .find(|node| node.id == input.node_id)
            .ok_or_else(|| AppError::NotFound(format!("画布节点不存在: {}", input.node_id)))?;
        self.run_canvas_image_node_validated(
            crate::canvas::CanvasImageNodeExecutionInput {
                requested_by_user_id: input.user_id,
                project_id: input.project_id,
                graph_id: input.graph_id,
                node,
                document: &document,
                shared_run: input.shared_run.map(|run| CanvasNodeRunHandle {
                    workflow_run_id: run.workflow_run_id,
                    finalize_on_success: run.finalize_on_success,
                    graph_version: run.graph_version,
                }),
                mode: &input.mode,
            },
            host_port,
            progress,
        )
        .await
    }
}

#[async_trait::async_trait]
impl CanvasLlmTextNodeApplicationPort for OrchestratorContext {
    async fn run_llm_text(
        &self,
        input: CanvasLlmTextNodeApplicationInput,
        llm_port: &dyn crate::canvas::CanvasTextGenerationLlmPort,
    ) -> Result<a4n_api_contract::canvas::CanvasRunSummary, AppError> {
        self.verify_project_permission(input.project_id, input.user_id, ProjectAccessRole::Editor)
            .await?;
        let document = self
            .get_or_create_canvas_graph_record(
                input.project_id,
                Some(input.graph_id),
                Utc::now().naive_utc(),
            )
            .await?;
        validate_canvas_record_document_edge_contract(&document)
            .map_err(crate::canvas::canvas_db_error)?;
        let node = document
            .nodes
            .iter()
            .find(|node| node.id == input.node_id)
            .ok_or_else(|| AppError::NotFound(format!("画布节点不存在: {}", input.node_id)))?;
        self.run_canvas_llm_text_node_validated(
            crate::canvas::CanvasTextNodeExecutionInput {
                project_id: input.project_id,
                graph_id: input.graph_id,
                node,
                document: &document,
                shared_run: input.shared_run.map(|run| CanvasNodeRunHandle {
                    workflow_run_id: run.workflow_run_id,
                    finalize_on_success: run.finalize_on_success,
                    graph_version: run.graph_version,
                }),
                mode: &input.mode,
            },
            llm_port,
        )
        .await
    }
}

#[async_trait::async_trait]
impl CanvasVideoNodeApplicationPort for OrchestratorContext {
    async fn run_video(
        &self,
        input: CanvasVideoNodeApplicationInput,
        host_port: &dyn crate::canvas::CanvasVideoGenerationHostPort,
    ) -> Result<a4n_api_contract::canvas::CanvasRunSummary, AppError> {
        self.verify_project_permission(input.project_id, input.user_id, ProjectAccessRole::Editor)
            .await?;
        let document = self
            .get_or_create_canvas_graph_record(
                input.project_id,
                Some(input.graph_id),
                Utc::now().naive_utc(),
            )
            .await?;
        validate_canvas_record_document_edge_contract(&document)
            .map_err(crate::canvas::canvas_db_error)?;
        let node = document
            .nodes
            .iter()
            .find(|node| node.id == input.node_id)
            .ok_or_else(|| AppError::NotFound(format!("画布节点不存在: {}", input.node_id)))?;
        self.run_canvas_video_node_validated(
            crate::canvas::CanvasVideoNodeExecutionInput {
                user_id: input.user_id,
                project_id: input.project_id,
                graph_id: input.graph_id,
                node,
                document: &document,
                shared_run: input.shared_run.map(|run| CanvasNodeRunHandle {
                    workflow_run_id: run.workflow_run_id,
                    finalize_on_success: run.finalize_on_success,
                    graph_version: run.graph_version,
                }),
                mode: &input.mode,
            },
            host_port,
        )
        .await
    }
}

#[async_trait::async_trait]
impl CanvasGraphPreparationApplicationPort for OrchestratorContext {
    async fn prepare_graph(
        &self,
        input: CanvasGraphRunPreparationInput,
        host_port: &dyn CanvasGraphPreparationHostPort,
    ) -> Result<PreparedCanvasGraphRun, String> {
        self.prepare_canvas_graph_run_validated(input, host_port)
            .await
    }

    async fn prepare_single_node(
        &self,
        input: CanvasSingleNodeRunPreparationInput,
        host_port: &dyn CanvasGraphPreparationHostPort,
    ) -> Result<PreparedCanvasGraphRun, String> {
        self.prepare_canvas_single_node_run_validated(input, host_port)
            .await
    }

    async fn prepare_resume(
        &self,
        input: CanvasGraphResumePreparationInput,
        host_port: &dyn CanvasGraphPreparationHostPort,
    ) -> Result<PreparedCanvasGraphRun, String> {
        self.prepare_canvas_graph_resume_validated(input, host_port)
            .await
    }
}

#[async_trait::async_trait]
impl CanvasGraphExecutionApplicationPort for OrchestratorContext {
    async fn run_prepared(
        &self,
        prepared: PreparedCanvasGraphRun,
        host_port: &dyn CanvasGraphExecutionHostPort,
    ) -> Result<a4n_api_contract::canvas::CanvasRunSummary, String> {
        self.run_prepared_canvas_graph_validated(prepared, host_port)
            .await
    }
}

#[async_trait::async_trait]
impl CanvasGraphFailureApplicationPort for OrchestratorContext {
    async fn fail_unsubmitted(&self, workflow_run_id: i64, error: &str) -> Result<(), String> {
        self.fail_workflow_run_validated(workflow_run_id, error)
            .await
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
}
