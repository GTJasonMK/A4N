use a4n_domain::chapter::ChapterRecord;
use a4n_error::AppError;

use crate::OrchestratorContext;

pub(super) async fn importable_chapters(
    context: &OrchestratorContext,
    project_id: i64,
) -> Result<Vec<ChapterRecord>, AppError> {
    Ok(context
        .import_analysis_chapter_candidates(project_id)
        .await?
        .into_iter()
        .filter(ChapterRecord::has_available_content)
        .collect())
}
