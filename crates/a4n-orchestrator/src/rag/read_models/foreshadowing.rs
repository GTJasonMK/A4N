use a4n_core::{error::AppError, models::character::foreshadowing_index};

use crate::OrchestratorContext;

impl OrchestratorContext {
    pub async fn foreshadowing_items_by_ids(
        &self,
        project_id: i64,
        foreshadowing_ids: &[i64],
    ) -> Result<Vec<foreshadowing_index::Model>, AppError> {
        a4n_db::rag::foreshadowing_items_by_ids(self.db.connection(), project_id, foreshadowing_ids)
            .await
            .map_err(AppError::Database)
    }

    pub async fn foreshadowing_items_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<foreshadowing_index::Model>, AppError> {
        a4n_db::rag::foreshadowing_items_for_rag_ingest(self.db.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    pub async fn pending_foreshadowing_before_chapter(
        &self,
        project_id: i64,
        chapter_number: i32,
        limit: u64,
    ) -> Result<Vec<foreshadowing_index::Model>, AppError> {
        a4n_db::rag::pending_foreshadowing_before_chapter(
            self.db.connection(),
            project_id,
            chapter_number,
            limit,
        )
        .await
        .map_err(AppError::Database)
    }

    pub async fn pending_foreshadowing_for_project(
        &self,
        project_id: i64,
        limit: u64,
    ) -> Result<Vec<foreshadowing_index::Model>, AppError> {
        a4n_db::rag::pending_foreshadowing_for_project(self.db.connection(), project_id, limit)
            .await
            .map_err(AppError::Database)
    }
}
