use super::*;

impl WorkerState {
    pub(super) async fn history(
        &self,
        request: WorkerEnvelope<InspirationHistoryInput>,
    ) -> Result<Vec<InspirationMessageSummary>, WorkerFailure> {
        validate_request(&request)?;
        let user_id = user_id(&request)?;
        let project_id = scoped_project_id(&request, request.payload.project_id)?;
        a4n_db::collaboration::verify_project_permission(
            self.database.connection(),
            project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await
        .map_err(collaboration_error)?;
        let messages = a4n_db::inspiration::inspiration_messages(
            self.database.connection(),
            project_id,
            Some(a4n_db::inspiration::INSPIRATION_HISTORY_LIMIT),
        )
        .await
        .map_err(inspiration_error)?;
        messages
            .into_iter()
            .map(message_summary)
            .collect::<Result<_, _>>()
            .map_err(WorkerFailure::internal)
    }
}
