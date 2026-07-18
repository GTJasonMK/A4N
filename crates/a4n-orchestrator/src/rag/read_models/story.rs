use a4n_core::{
    error::AppError,
    models::{
        chapter::{chapter, chapter_outline, part_outline},
        project::novel_blueprint,
    },
};

use crate::OrchestratorContext;

impl OrchestratorContext {
    pub async fn chapters_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<chapter::Model>, AppError> {
        a4n_db::rag::chapters_for_rag_ingest(self.db.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    pub async fn chapter_outlines_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<chapter_outline::Model>, AppError> {
        a4n_db::rag::chapter_outlines_for_rag_ingest(self.db.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    pub async fn part_outlines_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<part_outline::Model>, AppError> {
        a4n_db::rag::part_outlines_for_rag_ingest(self.db.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    pub async fn novel_blueprints_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<novel_blueprint::Model>, AppError> {
        a4n_db::rag::novel_blueprints_for_rag_ingest(self.db.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }
}
