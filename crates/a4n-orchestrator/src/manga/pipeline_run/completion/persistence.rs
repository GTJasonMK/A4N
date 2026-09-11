use a4n_db_models::models::chapter::chapter_manga_prompt;
use a4n_error::AppError;

use super::super::super::pipeline_persist::MangaPipelinePersistInput;
use super::super::contracts::MangaRunContext;
use super::contracts::MangaCompletionArtifacts;

pub(super) async fn persist_completion_outputs(
    context: &MangaRunContext<'_>,
    artifacts: MangaCompletionArtifacts,
) -> Result<Vec<chapter_manga_prompt::Model>, AppError> {
    let MangaCompletionArtifacts {
        plan,
        page_prompts,
        page_traces,
    } = artifacts;

    context
        .service
        .persist_pipeline_outputs(MangaPipelinePersistInput {
            chapter_id: context.chapter_id,
            project_id: context.project_id,
            source_metadata: &context.source_metadata,
            reference_assets_metadata: &context.reference_assets_metadata,
            prompt_fingerprint: &context.prompt_fingerprint,
            plan: &plan,
            page_traces,
            page_prompts,
        })
        .await
}
