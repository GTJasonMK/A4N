use a4n_core::{error::AppError, models::asset::object_profile};
use a4n_orchestrator::{OrchestratorContext, asset::ObjectProfileDraft};

pub(crate) struct AssetRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> AssetRuntimeFacade<'a> {
    pub(super) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn object_profiles_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<object_profile::Model>, AppError> {
        self.orchestrator
            .object_profiles_by_project(project_id)
            .await
    }

    pub(crate) async fn object_profile_by_id(
        &self,
        object_profile_id: i64,
    ) -> Result<object_profile::Model, AppError> {
        self.orchestrator
            .object_profile_by_id(object_profile_id)
            .await
    }

    pub(crate) async fn create_object_profile_validated(
        &self,
        project_id: i64,
        draft: ObjectProfileDraft,
    ) -> Result<object_profile::Model, AppError> {
        self.orchestrator
            .create_object_profile_validated(project_id, draft)
            .await
    }

    pub(crate) async fn update_object_profile_validated(
        &self,
        object_profile_id: i64,
        draft: ObjectProfileDraft,
    ) -> Result<object_profile::Model, AppError> {
        self.orchestrator
            .update_object_profile_validated(object_profile_id, draft)
            .await
    }

    pub(crate) async fn delete_object_profile_record_validated(
        &self,
        object_profile_id: i64,
    ) -> Result<(), AppError> {
        self.orchestrator
            .delete_object_profile_record_validated(object_profile_id)
            .await
    }
}
