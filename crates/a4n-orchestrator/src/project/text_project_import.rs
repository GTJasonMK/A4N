use a4n_core::{error::AppError, models::project::novel_project};
use serde_json::Value;

use crate::{OrchestratorContext, chapter_content::ChapterDerivedCleanupHostPort};

impl OrchestratorContext {
    pub async fn run_import_text_project_validated(
        &self,
        host_port: &dyn ChapterDerivedCleanupHostPort,
        user_id: i64,
        root: Value,
    ) -> Result<novel_project::Model, AppError> {
        let import_result = self
            .import_text_project_from_json_validated(user_id, root)
            .await?;
        if !import_result.has_preserved_chapter_tags {
            for chapter_id in import_result.postprocess_chapter_ids.iter().copied() {
                self.postprocess_chapter_after_content_change_validated(host_port, chapter_id)
                    .await?;
            }
        }
        Ok(import_result.project)
    }
}
