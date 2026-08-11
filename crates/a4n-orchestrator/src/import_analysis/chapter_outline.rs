use a4n_core::models::chapter::chapter;

use crate::OrchestratorContext;

pub async fn importable_chapters(
    context: &OrchestratorContext,
    project_id: i64,
) -> Result<Vec<chapter::Model>, String> {
    context
        .import_analysis_chapter_candidates(project_id)
        .await
        .map_err(|err| err.to_string())
        .map(|chapters| {
            chapters
                .into_iter()
                .filter(|chapter| !chapter.content.trim().is_empty())
                .collect()
        })
}
