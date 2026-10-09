use super::*;
use crate::graph::{
    apply_operation, canvas_graph_document, canvas_graph_record_input, check_version, graph_input,
    resource,
};

impl WorkerState {
    pub(super) async fn save_graph(
        &self,
        request: WorkerEnvelope<SaveCanvasGraphInput>,
    ) -> Result<CanvasGraphSaveResult, WorkerFailure> {
        let user_id = user_id(&request)?;
        request.require_project_scope(request.payload.project_id)?;
        self.verify_project_permission(
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        let input = request.payload;
        let record_input = canvas_graph_record_input(input.graph, input.expected_version);
        let saved = a4n_db::canvas::save_canvas_graph_record(
            self.database.connection(),
            input.project_id,
            record_input,
            Utc::now().naive_utc(),
        )
        .await
        .map_err(canvas_db_error)?;
        Ok(CanvasGraphSaveResult {
            document: canvas_graph_document(saved.document)?,
            node_ids: saved.node_ids,
        })
    }

    pub(super) async fn edit_graph(
        &self,
        request: WorkerEnvelope<EditCanvasGraphInput>,
    ) -> Result<CanvasGraphEditResult, WorkerFailure> {
        let user_id = user_id(&request)?;
        request.require_project_scope(request.payload.project_id)?;
        self.verify_project_permission(
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        let input = request.payload;
        if input.graph_id <= 0
            || input.expected_version <= 0
            || !(1..=20).contains(&input.operations.len())
        {
            return Err(WorkerFailure::invalid_request(
                "画布局部修改需要有效画布、版本和 1..20 个操作",
            ));
        }
        let original_record = a4n_db::canvas::get_or_create_canvas_graph_record(
            self.database.connection(),
            input.project_id,
            Some(input.graph_id),
            Utc::now().naive_utc(),
        )
        .await
        .map_err(canvas_db_error)?;
        let original = canvas_graph_document(original_record)?;
        check_version(&original, Some(input.expected_version))?;
        let mut edited = original.clone();
        for operation in input.operations {
            apply_operation(&mut edited, operation)?;
        }
        let changed = edited != original;
        let saved = if changed {
            let record_input =
                canvas_graph_record_input(graph_input(edited), Some(input.expected_version));
            let document = a4n_db::canvas::save_canvas_graph_record(
                self.database.connection(),
                input.project_id,
                record_input,
                Utc::now().naive_utc(),
            )
            .await
            .map_err(canvas_db_error)?;
            canvas_graph_document(document.document)?
        } else {
            original.clone()
        };
        Ok(CanvasGraphEditResult {
            resource: resource(&saved),
            changed,
            changed_nodes: saved
                .nodes
                .iter()
                .filter(|node| !original.nodes.contains(node))
                .cloned()
                .collect(),
            added_edges: saved
                .edges
                .iter()
                .filter(|edge| !original.edges.contains(edge))
                .cloned()
                .collect(),
            removed_edge_ids: original
                .edges
                .iter()
                .filter(|edge| {
                    !saved
                        .edges
                        .iter()
                        .any(|saved_edge| saved_edge.id == edge.id)
                })
                .map(|edge| edge.id)
                .collect(),
        })
    }

    pub(super) async fn save_text_asset(
        &self,
        request: WorkerEnvelope<SaveCanvasTextAssetInput>,
    ) -> Result<CanvasTextAssetSummary, WorkerFailure> {
        let user_id = user_id(&request)?;
        request.require_project_scope(request.payload.project_id)?;
        self.verify_project_permission(
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        let input = request.payload;
        if input.text_asset.id.is_some() && input.text_asset.expected_version.is_none() {
            return Err(WorkerFailure::invalid_request(
                "更新文本素材必须提供 expected_version",
            ));
        }
        let record = a4n_db::canvas::upsert_canvas_text_asset_record(
            self.database.connection(),
            input.project_id,
            CanvasTextAssetRecordInput {
                id: input.text_asset.id,
                expected_version: input.text_asset.expected_version,
                title: input.text_asset.title,
                text_kind: input.text_asset.text_kind,
                content: input.text_asset.content,
            },
            Utc::now().naive_utc(),
        )
        .await
        .map_err(canvas_db_error)?;
        Ok(CanvasTextAssetSummary {
            id: record.id,
            project_id: record.project_id,
            title: record.title,
            text_kind: record.text_kind,
            content: record.content,
            content_hash: record.content_hash,
            current_version: record.current_version,
            updated_at: record.updated_at.to_string(),
        })
    }
}
