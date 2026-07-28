mod object_profiles;

pub use a4n_db::asset::{AssetDbError, ObjectProfileRecordInput, ObjectProfileUpsertRecordResult};
pub use object_profiles::{
    OBJECT_TYPES, ObjectProfileDraft, normalize_object_name, normalize_object_profile_record_input,
    normalize_object_profile_record_inputs, normalize_object_type_value,
};

use a4n_core::error::AppError;
use a4n_core::models::asset::object_profile;

use crate::OrchestratorContext;

impl OrchestratorContext {
    pub async fn create_object_profile_record(
        &self,
        project_id: i64,
        input: ObjectProfileRecordInput,
    ) -> Result<object_profile::Model, AppError> {
        a4n_db::asset::create_object_profile_record(self.database.connection(), project_id, input)
            .await
            .map_err(asset_error)
    }

    pub async fn update_object_profile_record(
        &self,
        object_profile_id: i64,
        input: ObjectProfileRecordInput,
    ) -> Result<object_profile::Model, AppError> {
        a4n_db::asset::update_object_profile_record(
            self.database.connection(),
            object_profile_id,
            input,
        )
        .await
        .map_err(asset_error)
    }

    pub async fn upsert_object_profiles_with_generation_run_record(
        &self,
        project_id: i64,
        inputs: Vec<ObjectProfileRecordInput>,
        generation_run: a4n_db::generation::RecordCompletedGenerationRunInput,
    ) -> Result<ObjectProfileUpsertRecordResult, AppError> {
        a4n_db::asset::upsert_object_profiles_with_generation_run_record(
            self.database.connection(),
            project_id,
            inputs,
            generation_run,
        )
        .await
        .map_err(asset_error)
    }
}

fn asset_error(error: AssetDbError) -> AppError {
    match error {
        AssetDbError::Database(error) => AppError::Database(error),
        AssetDbError::NotFound(message) => AppError::NotFound(message),
        AssetDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
    }
}
