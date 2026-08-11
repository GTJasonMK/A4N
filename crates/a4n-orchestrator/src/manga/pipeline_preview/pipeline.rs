use std::collections::HashMap;

use a4n_core::error::AppError;

use crate::shooting_script::ChapterShootingScriptSourceInput;
use a4n_core::source_spans::source_spans_for_content;

use super::super::reference_assets::manga_reference_assets_json;
use super::super::{MangaPipelineRequestPreviewInput, OrchestratorContext};
use super::contracts::{MangaPipelineRequestPreviewStep, MangaPreviewContext};
use super::early_extraction::preview_early_extraction;
use super::late_extraction::preview_late_extraction;
use super::planning::preview_planning;

impl OrchestratorContext {
    pub(crate) async fn preview_generate_page_prompt_requests(
        &self,
        input: MangaPipelineRequestPreviewInput<'_>,
    ) -> Result<Vec<MangaPipelineRequestPreviewStep>, AppError> {
        let MangaPipelineRequestPreviewInput {
            chapter_id,
            project_id,
            prompt_renderer,
            reference_context,
            workflow_run_id,
            resume_from_checkpoint,
        } = input;
        let resolved = self
            .require_current_chapter_shooting_script(ChapterShootingScriptSourceInput {
                project_id,
                source_chapter_id: chapter_id,
            })
            .await?;
        let source = resolved.source;
        let shooting_script = resolved.shooting_script;
        let chapter_content = source.content.clone();
        let source_spans =
            source_spans_for_content(chapter_id, source.chapter_number, &chapter_content);
        let reference_assets = manga_reference_assets_json(&reference_context.character_portraits);
        let context = MangaPreviewContext {
            service: self,
            chapter_id,
            project_id,
            shooting_script,
            prompt_renderer,
            workflow_run_id,
            resume_from_checkpoint,
            chapter_content,
            source_spans,
            variables: HashMap::new(),
            reference_assets,
        };

        let mut steps = Vec::new();
        let Some(early) = preview_early_extraction(&context, &mut steps).await? else {
            return Ok(steps);
        };
        let Some(extraction) = preview_late_extraction(&context, &mut steps, early).await? else {
            return Ok(steps);
        };
        preview_planning(&context, &mut steps, extraction).await?;
        Ok(steps)
    }
}
