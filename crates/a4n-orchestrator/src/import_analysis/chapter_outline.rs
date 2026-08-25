use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter;

use crate::OrchestratorContext;

pub(super) async fn importable_chapters(
    context: &OrchestratorContext,
    project_id: i64,
) -> Result<Vec<chapter::Model>, AppError> {
    Ok(context
        .import_analysis_chapter_candidates(project_id)
        .await?
        .into_iter()
        .filter(chapter::Model::has_available_content)
        .collect())
}
