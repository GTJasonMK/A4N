use super::*;

impl WorkerState {
    pub(super) async fn set_primary(
        &self,
        request: WorkerEnvelope<PortraitRecordInput>,
    ) -> Result<(), WorkerFailure> {
        validate_request(&request, PORTRAIT_SET_PRIMARY_OPERATION)?;
        let portrait_id =
            WorkerFailure::require_positive_id(request.payload.portrait_id, "portrait_id")?;
        let portrait = a4n_db::character::character_portrait_record_by_id(
            self.database.connection(),
            portrait_id,
        )
        .await
        .map_err(a4n_worker_db_adapter::map_database_error)?
        .ok_or_else(|| WorkerFailure::not_found("立绘不存在"))?;
        let user_id = request.principal.require_user_id()?;
        verify_permission(
            self,
            portrait.project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        a4n_db::character::set_primary_character_portrait_record(
            self.database.connection(),
            portrait_id,
        )
        .await
        .map_err(portrait_record_error)
    }
}
