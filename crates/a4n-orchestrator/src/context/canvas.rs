//! Canvas 领域私有 owner。
//!
//! 持有 `DbHandle`，封装画布图读写、生成规格/提示词产物、媒体资产与 Video Studio 导入导出的数据库行为。

use a4n_core::error::AppError;
use a4n_core::models::{
    canvas::canvas_text_asset,
    generation::{generation_spec, prompt_artifact},
    media::media_asset,
    project::novel_project,
};
use a4n_db::DbHandle;
use a4n_db::canvas::{
    CanvasAssemblyContractSyncInput, CanvasAssemblyContractSyncSummary, CanvasGraphRecordDocument,
    CanvasGraphRecordInput, CanvasImageMediaAssetEnsureInput, CanvasNodeArtifactUpdateInput,
    CanvasNodeObservationUpdateInput, CanvasPromptArtifactUpsertInput, CanvasTextAssetRecordInput,
    CanvasVideoMediaAssetEnsureInput, FreeformCanvasGenerationSpecUpsertInput,
    VideoStudioProjectExportRows, VideoStudioProjectImportMediaFileWriter,
};
use chrono::NaiveDateTime;

use super::OrchestratorContext;
use crate::canvas::errors::{canvas_db_error, video_studio_project_import_error};
use crate::project::VideoStudioProjectPackage;

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

    async fn save_canvas_graph_record(
        &self,
        project_id: i64,
        input: CanvasGraphRecordInput,
        now: NaiveDateTime,
    ) -> Result<CanvasGraphRecordDocument, AppError> {
        a4n_db::canvas::save_canvas_graph_record(self.connection(), project_id, input, now)
            .await
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
    ) -> Result<Option<media_asset::Model>, AppError> {
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
    pub async fn get_or_create_canvas_graph_record(
        &self,
        project_id: i64,
        graph_id: Option<i64>,
        now: NaiveDateTime,
    ) -> Result<CanvasGraphRecordDocument, AppError> {
        self.canvas
            .get_or_create_canvas_graph_record(project_id, graph_id, now)
            .await
    }

    pub async fn save_canvas_graph_record(
        &self,
        project_id: i64,
        input: CanvasGraphRecordInput,
        now: NaiveDateTime,
    ) -> Result<CanvasGraphRecordDocument, AppError> {
        self.canvas
            .save_canvas_graph_record(project_id, input, now)
            .await
    }

    pub async fn upsert_canvas_text_asset_record(
        &self,
        project_id: i64,
        input: CanvasTextAssetRecordInput,
        now: NaiveDateTime,
    ) -> Result<canvas_text_asset::Model, AppError> {
        self.canvas
            .upsert_canvas_text_asset_record(project_id, input, now)
            .await
    }

    pub async fn upsert_freeform_canvas_generation_spec_record(
        &self,
        input: FreeformCanvasGenerationSpecUpsertInput,
    ) -> Result<generation_spec::Model, AppError> {
        self.canvas
            .upsert_freeform_canvas_generation_spec_record(input)
            .await
    }

    pub async fn mark_canvas_generation_spec_status_record(
        &self,
        generation_spec_id: i64,
        status: &str,
    ) -> Result<(), AppError> {
        self.canvas
            .mark_canvas_generation_spec_status_record(generation_spec_id, status)
            .await
    }

    pub async fn upsert_canvas_prompt_artifact_record(
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

    pub async fn ensure_canvas_image_media_asset_record(
        &self,
        input: CanvasImageMediaAssetEnsureInput,
    ) -> Result<media_asset::Model, AppError> {
        self.canvas
            .ensure_canvas_image_media_asset_record(input)
            .await
    }

    pub async fn ensure_canvas_video_media_asset_record(
        &self,
        input: CanvasVideoMediaAssetEnsureInput,
    ) -> Result<media_asset::Model, AppError> {
        self.canvas
            .ensure_canvas_video_media_asset_record(input)
            .await
    }

    pub async fn sync_canvas_assembly_contract_records(
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
    ) -> Result<Option<media_asset::Model>, AppError> {
        self.canvas.canvas_media_asset_by_id(media_asset_id).await
    }

    /// 画布整图运行：校验后将 workflow 标为 completed（节点执行仍由 Host 推进）。
    pub(crate) async fn complete_canvas_graph_workflow_validated(
        &self,
        workflow_run_id: i64,
    ) -> Result<a4n_core::models::generation::workflow_run::Model, AppError> {
        if workflow_run_id <= 0 {
            return Err(AppError::InvalidArgument(
                "workflow_run_id 必须大于 0".to_string(),
            ));
        }
        self.complete_workflow_run_validated(workflow_run_id).await
    }

    /// 画布独立节点 run 成功：按 finalize_on_success 决定是否将 workflow 标 completed。
    /// 返回 (可选已 finalize 的 run, 用于 Host 摘要的 status 字符串)。
    pub async fn complete_canvas_node_workflow_validated(
        &self,
        workflow_run_id: i64,
        finalize_on_success: bool,
    ) -> Result<
        (
            Option<a4n_core::models::generation::workflow_run::Model>,
            String,
        ),
        AppError,
    > {
        crate::workflow::require_positive_workflow_run_id(workflow_run_id)
            .map_err(AppError::InvalidArgument)?;
        if !crate::canvas::canvas_node_should_finalize_on_success(finalize_on_success) {
            return Ok((None, crate::workflow::WORKFLOW_STATUS_COMPLETED.to_string()));
        }
        let completed = self
            .finalize_workflow_run_outcome_validated(
                workflow_run_id,
                crate::canvas::canvas_node_success_finalize_outcome(),
                None,
            )
            .await?;
        let status = completed.status.clone();
        Ok((Some(completed), status))
    }

    /// 视频画布节点关联 workflow：按 status 完成/失败/非终态回落（消除 Host 裸 mark）。
    pub async fn finalize_canvas_video_workflow_validated(
        &self,
        workflow_run_id: i64,
        status: &str,
        error_message: Option<&str>,
    ) -> Result<a4n_core::models::generation::workflow_run::Model, AppError> {
        crate::workflow::require_positive_workflow_run_id(workflow_run_id)
            .map_err(AppError::InvalidArgument)?;
        let action = crate::canvas::canvas_video_workflow_status_action(status)
            .map_err(AppError::InvalidArgument)?;
        match action {
            crate::canvas::CanvasVideoWorkflowStatusAction::Complete => {
                self.complete_workflow_run_validated(workflow_run_id).await
            }
            crate::canvas::CanvasVideoWorkflowStatusAction::Fail => {
                self.fail_workflow_run_validated(workflow_run_id, error_message)
                    .await
            }
            crate::canvas::CanvasVideoWorkflowStatusAction::MarkNonTerminal => {
                // 非 completed/failed：按调用方 status 回落；can_resume=false（与历史 Host 约定一致）。
                self.mark_workflow_status(workflow_run_id, status, false, error_message)
                    .await
            }
        }
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
    ) -> Result<a4n_core::models::generation::workflow_run::Model, AppError> {
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
    ) -> Result<a4n_core::models::generation::workflow_run::Model, AppError> {
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
    pub async fn begin_canvas_node_step_workflow_validated(
        &self,
        project_id: i64,
        graph_id: i64,
        mode: &str,
        step_key: &str,
        input_fingerprint_json: serde_json::Value,
    ) -> Result<a4n_core::models::generation::workflow_run::Model, AppError> {
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
