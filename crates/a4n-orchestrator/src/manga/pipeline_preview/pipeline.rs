use a4n_core::error::AppError;
use serde_json::json;

use crate::shooting_script::ChapterShootingScriptSourceInput;
use a4n_core::source_spans::source_spans_for_content;

use super::super::reference_assets::{character_portraits_json, format_character_portrait_context};
use super::super::{MangaPipelineRequestPreviewInput, OrchestratorContext};
use super::contracts::{MangaPipelineRequestPreviewStep, MangaPreviewContext};
use super::early_extraction::preview_early_extraction;
use super::late_extraction::preview_late_extraction;
use super::planning::preview_planning;

impl OrchestratorContext {
    pub(crate) async fn preview_generate_panels_requests(
        &self,
        input: MangaPipelineRequestPreviewInput<'_>,
    ) -> Result<Vec<MangaPipelineRequestPreviewStep>, AppError> {
        let MangaPipelineRequestPreviewInput {
            chapter_id,
            project_id,
            prompt_renderer,
            variables,
            reference_context,
            workflow_run_id,
            resume_from_checkpoint,
            start_from_stage,
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
        let mut vars = variables.clone();
        vars.insert("chapter_content".into(), json!(chapter_content));
        vars.insert(
            "source_content_sha256".into(),
            json!(&source.content_sha256),
        );
        let character_portraits = character_portraits_json(&reference_context.character_portraits);
        vars.insert("character_portraits".into(), character_portraits.clone());
        vars.insert(
            "character_portrait_count".into(),
            json!(reference_context.character_portraits.len()),
        );
        let reference_assets = json!({
            "character_portraits": character_portraits,
            "character_portrait_summary": format_character_portrait_context(&reference_context),
        });
        let context = MangaPreviewContext {
            service: self,
            chapter_id,
            project_id,
            shooting_script,
            prompt_renderer,
            workflow_run_id,
            resume_from_checkpoint,
            start_from_stage,
            chapter_content,
            source_spans,
            variables: vars,
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
