use crate::OrchestratorContext;
use a4n_core::error::AppError;

impl OrchestratorContext {
    pub async fn snapshot_references_by_profile(
        &self,
        profile_id: i64,
        start_chapter: Option<i32>,
        end_chapter: Option<i32>,
    ) -> Result<Vec<a4n_core::models::character::snapshot_reference::Model>, AppError> {
        a4n_db::character::snapshot_references_by_profile(
            self.db.connection(),
            profile_id,
            start_chapter,
            end_chapter,
        )
        .await
        .map_err(AppError::Database)
    }

    pub async fn snapshot_reference_by_chapter(
        &self,
        profile_id: i64,
        chapter_number: i32,
    ) -> Result<Option<a4n_core::models::character::snapshot_reference::Model>, AppError> {
        a4n_db::character::snapshot_reference_by_chapter(
            self.db.connection(),
            profile_id,
            chapter_number,
        )
        .await
        .map_err(AppError::Database)
    }
}
