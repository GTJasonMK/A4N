use std::collections::HashMap;

use a4n_domain::source_spans::source_spans_for_content;
use a4n_error::AppError;

use a4n_domain::media_authoring::AdaptationTarget;

use super::super::super::reference_assets::manga_reference_assets_json;
use super::super::super::{MangaPipelineRequestPreviewInput, OrchestratorContext};
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
            character_portraits,
            workflow_run_id,
            resume_from_checkpoint,
            stop_after,
        } = input;
        let source = a4n_db::project::resolve_chapter_source(
            self.database.connection(),
            project_id,
            chapter_id,
        )
        .await
        .map_err(crate::shooting_script::errors::resolve_source_error)?;
        let Some(adaptation) = self
            .load_current_adaptation(&source, AdaptationTarget::Manga)
            .await?
        else {
            let request = self
                .preview_initial_adaptation(&source, AdaptationTarget::Manga, prompt_renderer)
                .await?;
            return Ok(vec![MangaPipelineRequestPreviewStep::ready(
                "adaptation_plan",
                "漫画改编计划",
                "chapter_media_adaptation_plan",
                None,
                request,
            )]);
        };
        let chapter_content = source.content.clone();
        let source_spans =
            source_spans_for_content(chapter_id, source.chapter_number, &chapter_content);
        let reference_assets = manga_reference_assets_json(&character_portraits);
        let context = MangaPreviewContext {
            service: self,
            chapter_id,
            project_id,
            adaptation,
            prompt_renderer,
            workflow_run_id,
            resume_from_checkpoint,
            stop_after,
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
