use super::*;
use crate::graph::{canvas_graph_document, read_canvas_graph};

impl WorkerState {
    pub(super) async fn list_text_assets(
        &self,
        request: WorkerEnvelope<ListCanvasTextAssetsInput>,
    ) -> Result<Vec<CanvasTextAssetSummary>, WorkerFailure> {
        let user_id = user_id(&request)?;
        validate_project_scope(&request, request.payload.project_id)?;
        self.verify_project_permission(
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Viewer,
        )
        .await?;
        let assets = a4n_db::canvas::canvas_text_assets_by_project_record(
            self.database.connection(),
            request.payload.project_id,
        )
        .await
        .map_err(canvas_db_error)?;
        Ok(assets
            .into_iter()
            .map(|asset| CanvasTextAssetSummary {
                id: asset.id,
                project_id: asset.project_id,
                title: asset.title,
                text_kind: asset.text_kind,
                content: asset.content,
                content_hash: asset.content_hash,
                current_version: asset.current_version,
                updated_at: asset.updated_at.to_string(),
            })
            .collect())
    }

    pub(super) async fn list_text_asset_versions(
        &self,
        request: WorkerEnvelope<ListCanvasTextAssetVersionsInput>,
    ) -> Result<Vec<CanvasTextAssetVersionSummary>, WorkerFailure> {
        let user_id = user_id(&request)?;
        validate_project_scope(&request, request.payload.project_id)?;
        self.verify_project_permission(
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Viewer,
        )
        .await?;
        let versions = a4n_db::canvas::canvas_text_asset_versions_record(
            self.database.connection(),
            request.payload.project_id,
            request.payload.asset_id,
        )
        .await
        .map_err(canvas_db_error)?;
        Ok(versions
            .into_iter()
            .map(|version| CanvasTextAssetVersionSummary {
                version: version.version,
                content: version.content,
                content_hash: version.content_hash,
                created_at: version.created_at.to_string(),
            })
            .collect())
    }

    pub(super) async fn get_graph(
        &self,
        request: WorkerEnvelope<GetCanvasGraphInput>,
    ) -> Result<CanvasGraphDocument, WorkerFailure> {
        let user_id = user_id(&request)?;
        validate_project_scope(&request, request.payload.project_id)?;
        self.verify_project_permission(
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Viewer,
        )
        .await?;
        let document = a4n_db::canvas::get_or_create_canvas_graph_record(
            self.database.connection(),
            request.payload.project_id,
            request.payload.graph_id,
            Utc::now().naive_utc(),
        )
        .await
        .map_err(canvas_db_error)?;
        canvas_graph_document(document)
    }

    pub(super) async fn read_graph(
        &self,
        request: WorkerEnvelope<ReadCanvasGraphInput>,
    ) -> Result<CanvasGraphReadResult, WorkerFailure> {
        let user_id = user_id(&request)?;
        validate_project_scope(&request, request.payload.project_id)?;
        self.verify_project_permission(
            request.payload.project_id,
            user_id,
            ProjectAccessRole::Viewer,
        )
        .await?;
        let input = request.payload;
        if input.graph_id <= 0 || input.expected_version.is_some_and(|version| version <= 0) {
            return Err(WorkerFailure::invalid_request("画布和版本必须是正整数"));
        }
        let document = a4n_db::canvas::get_or_create_canvas_graph_record(
            self.database.connection(),
            input.project_id,
            Some(input.graph_id),
            Utc::now().naive_utc(),
        )
        .await
        .map_err(canvas_db_error)?;
        read_canvas_graph(canvas_graph_document(document)?, input)
    }
}
