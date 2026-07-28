use a4n_core::models::chapter::chapter_outline;
use a4n_db::project::ProjectChapterOutlineUpdateInput;

use super::super::errors::project_chapter_outline_update_error_to_app_error;
use crate::OrchestratorContext;

impl OrchestratorContext {
    pub async fn update_chapter_outline_record(
        &self,
        input: ProjectChapterOutlineUpdateInput,
    ) -> Result<chapter_outline::Model, a4n_core::error::AppError> {
        a4n_db::project::update_chapter_outline_record(self.database.connection(), input)
            .await
            .map_err(project_chapter_outline_update_error_to_app_error)
    }
}
