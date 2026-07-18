use super::{CharacterAttributeDeletionRequestInput, DeletionMarkListStatus, character_db_error};
use crate::OrchestratorContext;
use a4n_core::error::AppError;

impl OrchestratorContext {
    pub async fn behavior_records_by_profile(
        &self,
        profile_id: i64,
        chapter_from: Option<i32>,
        chapter_to: Option<i32>,
        limit: Option<u64>,
    ) -> Result<Vec<a4n_core::models::character::protagonist_behavior_record::Model>, AppError>
    {
        a4n_db::character::behavior_records_by_profile(
            self.db.connection(),
            profile_id,
            chapter_from,
            chapter_to,
            limit,
        )
        .await
        .map_err(AppError::Database)
    }

    pub async fn implicit_review_behavior_records(
        &self,
        profile_id: i64,
    ) -> Result<Vec<a4n_core::models::character::protagonist_behavior_record::Model>, AppError>
    {
        a4n_db::character::implicit_review_behavior_records(self.db.connection(), profile_id)
            .await
            .map_err(AppError::Database)
    }

    pub async fn deletion_marks_by_profile(
        &self,
        profile_id: i64,
        category: Option<&str>,
        status: DeletionMarkListStatus,
        deletion_threshold: i32,
    ) -> Result<Vec<a4n_core::models::character::protagonist_deletion_mark::Model>, AppError> {
        a4n_db::character::deletion_marks_by_profile(
            self.db.connection(),
            profile_id,
            category,
            status,
            deletion_threshold,
        )
        .await
        .map_err(AppError::Database)
    }

    pub async fn request_attribute_deletion(
        &self,
        input: CharacterAttributeDeletionRequestInput,
    ) -> Result<a4n_core::models::character::protagonist_deletion_mark::Model, AppError> {
        a4n_db::character::request_attribute_deletion(self.db.connection(), input)
            .await
            .map_err(character_db_error)
    }
    pub async fn reset_deletion_marks(
        &self,
        profile_id: i64,
        category: &str,
        key: &str,
    ) -> Result<u64, AppError> {
        a4n_db::character::reset_deletion_marks_in(self.db.connection(), profile_id, category, key)
            .await
            .map_err(character_db_error)
    }

    pub async fn behavior_records_before_chapter(
        &self,
        profile_id: i64,
        start_chapter: i32,
    ) -> Result<Vec<a4n_core::models::character::protagonist_behavior_record::Model>, AppError>
    {
        a4n_db::character::behavior_records_before_chapter(
            self.db.connection(),
            profile_id,
            start_chapter,
        )
        .await
        .map_err(AppError::Database)
    }
}
