use super::*;
use crate::graph::{canvas_graph_document, read_canvas_graph};

impl WorkerState {
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
