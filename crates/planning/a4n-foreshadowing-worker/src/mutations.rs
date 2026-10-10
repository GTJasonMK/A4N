use super::*;

impl WorkerState {
    pub(super) async fn create(
        &self,
        request: WorkerEnvelope<CreateInput>,
    ) -> Result<ForeshadowingMutationResult, WorkerFailure> {
        validate_request(&request, FORESHADOWING_CREATE_OPERATION)?;
        let user_id = request.principal.require_user_id()?;
        let project_id = request.require_project_scope(request.payload.0.project_id)?;
        let input = request.payload.0;
        self.verify_editor(project_id, user_id).await?;
        let record = a4n_db::project::create_foreshadowing_read_record(
            self.database.connection(),
            project_id,
            input.record,
        )
        .await
        .map_err(foreshadowing_error)?;
        Ok(ForeshadowingMutationResult {
            project_id,
            foreshadowing_id: record.id,
        })
    }

    pub(super) async fn update(
        &self,
        request: WorkerEnvelope<UpdateInput>,
    ) -> Result<ForeshadowingMutationResult, WorkerFailure> {
        validate_request(&request, FORESHADOWING_UPDATE_OPERATION)?;
        let user_id = request.principal.require_user_id()?;
        let project_id = request.require_project_scope(request.payload.0.project_id)?;
        let input = request.payload.0;
        self.verify_editor(project_id, user_id).await?;
        let record = a4n_db::project::update_foreshadowing_read_record(
            self.database.connection(),
            project_id,
            input.foreshadowing_id,
            &input.expected_updated_at,
            input.record,
        )
        .await
        .map_err(foreshadowing_error)?;
        Ok(ForeshadowingMutationResult {
            project_id,
            foreshadowing_id: record.id,
        })
    }

    pub(super) async fn delete(
        &self,
        request: WorkerEnvelope<DeleteInput>,
    ) -> Result<ForeshadowingMutationResult, WorkerFailure> {
        validate_request(&request, FORESHADOWING_DELETE_OPERATION)?;
        let user_id = request.principal.require_user_id()?;
        let project_id = request.require_project_scope(request.payload.0.project_id)?;
        let input = request.payload.0;
        self.verify_editor(project_id, user_id).await?;
        let record = a4n_db::project::delete_foreshadowing_record(
            self.database.connection(),
            project_id,
            input.foreshadowing_id,
            &input.expected_updated_at,
        )
        .await
        .map_err(foreshadowing_error)?;
        Ok(ForeshadowingMutationResult {
            project_id,
            foreshadowing_id: record.id,
        })
    }

    async fn verify_editor(&self, project_id: i64, user_id: i64) -> Result<(), WorkerFailure> {
        a4n_db::collaboration::verify_project_permission(
            self.database.connection(),
            project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Editor,
        )
        .await
        .map(|_| ())
        .map_err(collaboration_error)
    }
}
