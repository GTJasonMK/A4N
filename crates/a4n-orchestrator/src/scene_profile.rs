use crate::OrchestratorContext;
use a4n_core::error::AppError;
pub use a4n_db::scene_profile::{
    SceneEntityStateRecordInput, SceneOccurrenceRecordInput, SceneProfileRecordDraft,
    SceneProfileRecordInput, SceneSyncRecordInput, SceneSyncRecordResult,
    SceneTransitionRecordInput,
};
use std::collections::BTreeMap;

mod contracts;
mod mapping;
mod rules;

pub use contracts::{
    SCENE_ENTITY_STATE_KEYS, SCENE_ENTITY_TYPES, SCENE_TRANSITION_MOVEMENT_TYPES, SCENE_TYPES,
    SceneEntityStateDraft, SceneOccurrenceDraft, SceneProfileDraft, SceneSyncInput,
    SceneSyncResult, SceneTransitionDraft,
};
pub use mapping::{
    normalize_scene_profile_record_input, normalize_scene_sync_record_input,
    scene_sync_result_from_record,
};
pub use rules::{
    normalize_movement_type_value, normalize_scene_entity_state_key_value,
    normalize_scene_entity_type_value, normalize_scene_type_value,
};

#[cfg(test)]
mod tests;

impl OrchestratorContext {
    pub async fn scene_profiles_by_project(
        &self,
        project_id: i64,
    ) -> Result<Vec<a4n_core::models::scene::scene_profile::Model>, AppError> {
        a4n_db::scene_profile::scene_profiles_by_project(self.db.connection(), project_id)
            .await
            .map_err(scene_profile_db_error)
    }

    pub async fn scene_profile_by_id(
        &self,
        scene_profile_id: i64,
    ) -> Result<a4n_core::models::scene::scene_profile::Model, AppError> {
        a4n_db::scene_profile::scene_profile_by_id(self.db.connection(), scene_profile_id)
            .await
            .map_err(scene_profile_db_error)
    }

    pub async fn create_scene_profile_record(
        &self,
        project_id: i64,
        input: SceneProfileRecordInput,
    ) -> Result<a4n_core::models::scene::scene_profile::Model, AppError> {
        a4n_db::scene_profile::create_scene_profile_record_checked(
            self.db.connection(),
            project_id,
            input,
        )
        .await
        .map_err(scene_profile_db_error)
    }

    pub async fn update_scene_profile_record(
        &self,
        scene_profile_id: i64,
        input: SceneProfileRecordInput,
    ) -> Result<a4n_core::models::scene::scene_profile::Model, AppError> {
        a4n_db::scene_profile::update_scene_profile_record_checked(
            self.db.connection(),
            scene_profile_id,
            input,
        )
        .await
        .map_err(scene_profile_db_error)
    }

    pub async fn delete_scene_profile_records(
        &self,
        scene_profile_id: i64,
    ) -> Result<(), AppError> {
        a4n_db::scene_profile::delete_scene_profile_records(self.db.connection(), scene_profile_id)
            .await
            .map_err(scene_profile_db_error)
    }

    pub async fn replace_chapter_scene_sync_with_generation_run_records(
        &self,
        project_id: i64,
        chapter_id: i64,
        chapter_number: i32,
        input: SceneSyncRecordInput,
        generation_run: a4n_db::generation::RecordCompletedGenerationRunInput,
    ) -> Result<SceneSyncRecordResult, AppError> {
        a4n_db::scene_profile::replace_chapter_scene_sync_with_generation_run_records(
            self.db.connection(),
            project_id,
            chapter_id,
            chapter_number,
            input,
            generation_run,
        )
        .await
        .map_err(scene_profile_db_error)
    }

    pub async fn scene_occurrence_count(
        &self,
        project_id: i64,
        scene_profile_id: i64,
    ) -> Result<u64, AppError> {
        a4n_db::scene_profile::scene_occurrence_count(
            self.db.connection(),
            project_id,
            scene_profile_id,
        )
        .await
        .map_err(scene_profile_db_error)
    }

    pub async fn scene_occurrence_counts_by_profile(
        &self,
        project_id: i64,
        profile_ids: &[i64],
    ) -> Result<BTreeMap<i64, u64>, AppError> {
        a4n_db::scene_profile::scene_occurrence_counts_by_profile(
            self.db.connection(),
            project_id,
            profile_ids,
        )
        .await
        .map_err(scene_profile_db_error)
    }
}

pub(crate) fn scene_profile_db_error(
    error: a4n_db::scene_profile::SceneProfileDbError,
) -> AppError {
    match error {
        a4n_db::scene_profile::SceneProfileDbError::Database(error) => AppError::Database(error),
        a4n_db::scene_profile::SceneProfileDbError::NotFound(message) => {
            AppError::NotFound(message)
        }
        a4n_db::scene_profile::SceneProfileDbError::InvalidArgument(message) => {
            AppError::InvalidArgument(message)
        }
    }
}
