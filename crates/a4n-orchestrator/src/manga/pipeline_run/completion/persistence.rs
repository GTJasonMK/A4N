use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_manga_prompt;

use super::super::super::pipeline_json::validate_page_drawing_prompts;
use super::super::super::pipeline_persist::MangaPipelinePersistInput;
use super::super::contracts::MangaRunContext;
use super::contracts::MangaCompletionArtifacts;

pub(super) async fn persist_completion_outputs(
    context: &MangaRunContext<'_>,
    artifacts: MangaCompletionArtifacts,
) -> Result<Vec<chapter_manga_prompt::Model>, AppError> {
    let MangaCompletionArtifacts {
        plan_json,
        dp_json,
        page_traces,
    } = artifacts;
    let page_prompts = validate_page_drawing_prompts(&dp_json, "manga_page_drawing_prompt")?;

    context
        .service
        .persist_pipeline_outputs(MangaPipelinePersistInput {
            chapter_id: context.chapter_id,
            project_id: context.project_id,
            source_metadata: &context.source_metadata,
            reference_assets_metadata: &context.reference_assets_metadata,
            prompt_fingerprint: &context.prompt_fingerprint,
            plan_json: &plan_json,
            page_traces,
            page_prompts,
        })
        .await
}
