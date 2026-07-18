use a4n_core::error::AppError;

use super::super::{CharacterPortraitRollbackResult, RecordCompletedGenerationRunInput};
use super::errors::{character_portrait_rollback_error, character_profile_create_error};
use crate::OrchestratorContext;

impl OrchestratorContext {
    pub async fn create_profile(
        &self,
        project_id: i64,
        name: &str,
        base_attributes: serde_json::Value,
    ) -> Result<a4n_core::models::character::protagonist_profile::Model, AppError> {
        a4n_db::character::create_profile(self.db.connection(), project_id, name, base_attributes)
            .await
            .map_err(character_profile_create_error)
    }

    pub async fn create_profile_with_generation_run(
        &self,
        project_id: i64,
        name: &str,
        base_attributes: serde_json::Value,
        generation_run: RecordCompletedGenerationRunInput,
    ) -> Result<a4n_core::models::character::protagonist_profile::Model, AppError> {
        a4n_db::character::create_profile_with_generation_run(
            self.db.connection(),
            project_id,
            name,
            base_attributes,
            generation_run,
        )
        .await
        .map_err(character_profile_create_error)
    }

    pub async fn rollback_failed_portrait_creation_records(
        &self,
        portrait_id: i64,
        image_path: &str,
    ) -> Result<CharacterPortraitRollbackResult, AppError> {
        a4n_db::character::rollback_failed_portrait_creation_records(
            self.db.connection(),
            portrait_id,
            image_path,
        )
        .await
        .map_err(character_portrait_rollback_error)
    }
}
