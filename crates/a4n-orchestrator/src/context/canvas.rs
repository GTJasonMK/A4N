//! Canvas 领域私有 owner。
//!
//! 持有 `DbHandle`，封装画布图读写、生成规格/提示词产物、媒体资产与 Video Studio 导入导出的数据库行为。

use a4n_api_contract::canvas::{
    CanvasEdgeSummary, CanvasGraphDocument, CanvasGraphInput, CanvasGraphSummary, CanvasJsonObject,
    CanvasNodeSummary, CanvasTextAssetInput, CanvasTextAssetSummary,
};
use a4n_core::error::AppError;
use a4n_core::models::{
    canvas::canvas_text_asset,
    generation::{generation_spec, prompt_artifact},
    media::media_asset,
    project::novel_project,
};
use a4n_db::DbHandle;
use a4n_db::canvas::{
    CanvasAssemblyContractSyncInput, CanvasAssemblyContractSyncSummary, CanvasEdgeRecordInput,
    CanvasGraphRecordDocument, CanvasGraphRecordInput, CanvasImageMediaAssetEnsureInput,
    CanvasMediaAssetProbeRecord, CanvasNodeArtifactUpdateInput, CanvasNodeObservationUpdateInput,
    CanvasNodeRecordInput, CanvasPromptArtifactUpsertInput, CanvasTextAssetRecordInput,
    CanvasVideoMediaAssetEnsureInput, FreeformCanvasGenerationSpecUpsertInput,
    VideoStudioProjectExportRows, VideoStudioProjectImportMediaFileWriter,
};
use chrono::NaiveDateTime;
use serde_json::Value;

use super::OrchestratorContext;
use crate::canvas::{canvas_db_error, video_studio_project_import_error};
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
    pub async fn get_or_create_canvas_graph(
        &self,
        project_id: i64,
        graph_id: Option<i64>,
        now: NaiveDateTime,
    ) -> Result<CanvasGraphDocument, AppError> {
        let document = self
            .canvas
            .get_or_create_canvas_graph_record(project_id, graph_id, now)
            .await?;
        canvas_graph_document(document)
    }

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

    pub async fn save_canvas_graph(
        &self,
        project_id: i64,
        expected_version: Option<i32>,
        input: CanvasGraphInput,
        now: NaiveDateTime,
    ) -> Result<CanvasGraphDocument, AppError> {
        let document = self
            .canvas
            .save_canvas_graph_record(
                project_id,
                canvas_graph_record_input(input, expected_version),
                now,
            )
            .await?;
        canvas_graph_document(document)
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

    pub async fn upsert_canvas_text_asset(
        &self,
        project_id: i64,
        input: CanvasTextAssetInput,
        now: NaiveDateTime,
    ) -> Result<CanvasTextAssetSummary, AppError> {
        let record = self
            .canvas
            .upsert_canvas_text_asset_record(
                project_id,
                CanvasTextAssetRecordInput {
                    id: input.id,
                    title: input.title,
                    text_kind: input.text_kind,
                    content: input.content,
                },
                now,
            )
            .await?;
        Ok(canvas_text_asset_summary(record))
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
    ) -> Result<Option<CanvasMediaAssetProbeRecord>, AppError> {
        self.canvas.canvas_media_asset_by_id(media_asset_id).await
    }

    /// 画布整图运行：校验后将 workflow 标为 completed（节点执行仍由 Host 推进）。
    pub(crate) async fn complete_canvas_graph_workflow_validated(
        &self,
        workflow_run_id: i64,
    ) -> Result<a4n_core::models::generation::workflow_run::Model, AppError> {
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

fn canvas_graph_record_input(
    input: CanvasGraphInput,
    expected_version: Option<i32>,
) -> CanvasGraphRecordInput {
    CanvasGraphRecordInput {
        id: input.id,
        expected_version,
        name: Some(input.name),
        graph_kind: Some(input.graph_kind),
        viewport_json: input.viewport_json.map(canvas_json_value),
        nodes: input
            .nodes
            .into_iter()
            .map(|node| CanvasNodeRecordInput {
                client_id: node.client_id,
                node_kind: node.node_kind,
                ref_type: node.ref_type,
                ref_id: node.ref_id,
                config_json: Some(canvas_json_value(node.config_json)),
                position_json: canvas_json_value(node.position_json),
                state: Some(node.state),
            })
            .collect(),
        edges: input
            .edges
            .into_iter()
            .map(|edge| CanvasEdgeRecordInput {
                source_client_id: edge.source_client_id,
                target_client_id: edge.target_client_id,
                source_port: edge.source_port,
                target_port: edge.target_port,
                role: edge.role,
                config_json: Some(canvas_json_value(edge.config_json)),
            })
            .collect(),
    }
}

fn canvas_graph_document(
    document: CanvasGraphRecordDocument,
) -> Result<CanvasGraphDocument, AppError> {
    let CanvasGraphRecordDocument {
        graph,
        nodes,
        edges,
        text_assets: _,
    } = document;
    let viewport_json = graph
        .viewport_json
        .map(|value| canvas_json_object(value, "canvas_graph.viewport_json"))
        .transpose()?;
    let nodes = nodes
        .into_iter()
        .map(|node| {
            Ok(CanvasNodeSummary {
                id: node.id,
                node_kind: node.node_kind,
                ref_type: node.ref_type,
                ref_id: node.ref_id,
                config_json: canvas_json_object(node.config_json, "canvas_node.config_json")?,
                position_json: canvas_json_object(node.position_json, "canvas_node.position_json")?,
                state: node.state,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    let edges = edges
        .into_iter()
        .map(|edge| {
            Ok(CanvasEdgeSummary {
                id: edge.id,
                source_node_id: edge.source_node_id,
                target_node_id: edge.target_node_id,
                source_port: edge.source_port,
                target_port: edge.target_port,
                role: edge.role,
                config_json: canvas_json_object(edge.config_json, "canvas_edge.config_json")?,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    Ok(CanvasGraphDocument {
        graph: CanvasGraphSummary {
            id: graph.id,
            name: graph.name,
            graph_kind: graph.graph_kind,
            version: graph.version,
            viewport_json,
        },
        nodes,
        edges,
    })
}

fn canvas_text_asset_summary(record: canvas_text_asset::Model) -> CanvasTextAssetSummary {
    CanvasTextAssetSummary {
        id: record.id,
        text_kind: record.text_kind,
        content: record.content,
    }
}

fn canvas_json_object(value: Value, field: &str) -> Result<CanvasJsonObject, AppError> {
    match value {
        Value::Object(object) => Ok(object.into_iter().collect()),
        _ => Err(AppError::InvalidState {
            current: format!("{field} 不是 JSON 对象"),
            operation: "输出 CanvasGraphDocument".to_string(),
        }),
    }
}

fn canvas_json_value(input: CanvasJsonObject) -> Value {
    Value::Object(input.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use a4n_core::models::canvas::{canvas_graph, canvas_text_asset};
    use chrono::DateTime;
    use serde_json::json;

    use super::{AppError, CanvasGraphRecordDocument, canvas_graph_document, canvas_json_object};

    #[test]
    fn canvas_graph_document_projects_only_api_contract() {
        let now = DateTime::UNIX_EPOCH.naive_utc();
        let document = canvas_graph_document(CanvasGraphRecordDocument {
            graph: canvas_graph::Model {
                id: 7,
                project_id: 11,
                name: "画布".to_string(),
                graph_kind: "video".to_string(),
                version: 3,
                viewport_json: Some(json!({ "zoom": 1 })),
                created_at: now,
                updated_at: now,
            },
            nodes: Vec::new(),
            edges: Vec::new(),
            text_assets: Vec::new(),
        })
        .expect("record document should project");

        assert_eq!(
            serde_json::to_value(document).expect("document should serialize"),
            json!({
                "graph": {
                    "id": 7,
                    "name": "画布",
                    "graph_kind": "video",
                    "version": 3,
                    "viewport_json": { "zoom": 1 }
                },
                "nodes": [],
                "edges": []
            })
        );
    }

    #[test]
    fn canvas_text_asset_summary_projects_only_api_contract() {
        let now = DateTime::UNIX_EPOCH.naive_utc();
        let summary = super::canvas_text_asset_summary(canvas_text_asset::Model {
            id: 5,
            project_id: 11,
            title: "内部标题".to_string(),
            text_kind: "script".to_string(),
            content: "正文".to_string(),
            content_hash: "hash".to_string(),
            created_at: now,
            updated_at: now,
        });

        assert_eq!(
            serde_json::to_value(summary).expect("summary should serialize"),
            json!({
                "id": 5,
                "text_kind": "script",
                "content": "正文"
            })
        );
    }

    #[test]
    fn canvas_json_object_rejects_non_object_values() {
        assert_eq!(
            canvas_json_object(json!({ "x": 1 }), "canvas_node.config_json")
                .expect("object should be accepted")
                .get("x"),
            Some(&json!(1))
        );
        assert!(matches!(
            canvas_json_object(json!(null), "canvas_node.config_json"),
            Err(AppError::InvalidState { .. })
        ));
    }
}
