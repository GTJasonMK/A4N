use a4n_core::error::AppError;

use super::super::{
    CharacterPortraitCreateInput, PortraitNamedPromptProfileRows, PortraitPromptProfileRows,
};
use super::errors::character_portrait_record_error;
use crate::OrchestratorContext;

impl OrchestratorContext {
    pub async fn character_portrait_by_id(
        &self,
        portrait_id: i64,
    ) -> Result<Option<a4n_core::models::character::character_portrait::Model>, AppError> {
        a4n_db::character::character_portrait_by_id(self.db.connection(), portrait_id)
            .await
            .map_err(AppError::Database)
    }

    pub async fn create_character_portrait_record(
        &self,
        input: CharacterPortraitCreateInput,
    ) -> Result<a4n_core::models::character::character_portrait::Model, AppError> {
        a4n_db::character::create_character_portrait_record(self.db.connection(), input)
            .await
            .map_err(AppError::Database)
    }

    pub async fn character_portrait_records_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<a4n_core::models::character::character_portrait::Model>, AppError> {
        a4n_db::character::character_portrait_records_by_project(self.db.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    pub async fn primary_character_portrait_record(
        &self,
        profile_id: i64,
    ) -> Result<Option<a4n_core::models::character::character_portrait::Model>, AppError> {
        a4n_db::character::primary_character_portrait_record(self.db.connection(), profile_id)
            .await
            .map_err(AppError::Database)
    }

    pub async fn create_character_portrait_for_profile(
        &self,
        input: CharacterPortraitCreateInput,
    ) -> Result<a4n_core::models::character::character_portrait::Model, AppError> {
        a4n_db::character::create_character_portrait_for_profile(self.db.connection(), input)
            .await
            .map_err(character_portrait_record_error)
    }

    pub async fn set_primary_character_portrait_record(
        &self,
        portrait_id: i64,
    ) -> Result<(), AppError> {
        a4n_db::character::set_primary_character_portrait_record(self.db.connection(), portrait_id)
            .await
            .map_err(character_portrait_record_error)
    }

    pub async fn portrait_prompt_profile_rows(
        &self,
        project_id: i64,
        profile_id: i64,
    ) -> Result<PortraitPromptProfileRows, AppError> {
        a4n_db::character::portrait_prompt_profile_rows(
            self.db.connection(),
            project_id,
            profile_id,
        )
        .await
        .map_err(character_portrait_record_error)
    }

    pub async fn portrait_named_prompt_profile_rows(
        &self,
        project_id: i64,
        character_name: &str,
    ) -> Result<PortraitNamedPromptProfileRows, AppError> {
        a4n_db::character::portrait_named_prompt_profile_rows(
            self.db.connection(),
            project_id,
            character_name,
        )
        .await
        .map_err(character_portrait_record_error)
    }

    pub async fn portrait_reference_images_by_ids(
        &self,
        project_id: i64,
        image_ids: &[i64],
    ) -> Result<Vec<a4n_core::models::ai::generated_image::Model>, AppError> {
        a4n_db::character::portrait_reference_images_by_ids(
            self.db.connection(),
            project_id,
            image_ids,
        )
        .await
        .map_err(character_portrait_record_error)
    }
}
