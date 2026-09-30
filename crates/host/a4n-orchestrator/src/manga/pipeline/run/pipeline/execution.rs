use std::collections::HashMap;

use a4n_api_contract::manga::MangaPagePromptSourceSummary;
use a4n_db_models::models::chapter::chapter_manga_prompt;
use a4n_domain::source_spans::source_spans_for_content;
use a4n_error::AppError;

use a4n_domain::media_authoring::AdaptationTarget;

use super::super::super::super::reference_assets::manga_reference_assets_json;
use super::super::super::super::{
    MangaGeneratePagePromptsInput, MangaPipelineStage, OrchestratorContext,
};
use super::super::super::prompt::manga_pipeline_prompt_fingerprint;
use super::super::completion::complete_pipeline;
use super::super::contracts::MangaRunContext;
use super::super::early_extraction::run_early_extraction;
use super::super::late_extraction::run_late_extraction;
use super::super::page_slots::run_page_slots;

impl OrchestratorContext {
    pub(crate) async fn generate_page_prompts(
        &self,
        input: MangaGeneratePagePromptsInput<'_>,
    ) -> Result<Vec<chapter_manga_prompt::Model>, AppError> {
        let MangaGeneratePagePromptsInput {
            user_id,
            chapter_id,
            chapter_number,
            project_id,
            prompt_renderer,
            character_portraits,
            task_id,
            workflow_run_id,
            resume_from_checkpoint,
            stop_after,
            image_size,
            image_generator,
            cancel_token,
            progress_reporter,
            stream_reporter,
        } = input;
        let source = a4n_db::project::resolve_chapter_source(
            self.database.connection(),
            project_id,
            chapter_id,
        )
        .await
        .map_err(crate::shooting_script::errors::resolve_source_error)?;
        let report_adaptation_delta = move |delta: &str| {
            if let Some(reporter) = stream_reporter {
                reporter("adaptation_plan", delta);
            }
        };
        let adaptation = self
            .ensure_adaptation_plan(
                &source,
                AdaptationTarget::Manga,
                workflow_run_id,
                "manga_pipeline",
                prompt_renderer,
                Some(&report_adaptation_delta),
                cancel_token,
            )
            .await?;
        if source.source_chapter_id != chapter_id {
            return Err(AppError::InvalidArgument(format!(
                "漫画章节 ID 与统一来源解析结果不一致: requested={chapter_id}, resolved={}",
                source.source_chapter_id
            )));
        }
        let chapter_content = source.content.clone();
        let source_spans =
            source_spans_for_content(chapter_id, source.chapter_number, &chapter_content);
        let source_metadata = MangaPagePromptSourceSummary {
            root_project_id: source.root_project_id,
            source_chapter_id: source.source_chapter_id,
            source_snapshot_id: source.source_snapshot_id,
            source_snapshot_chapter_id: source.source_snapshot_chapter_id,
            source_revision_key: source.source_revision_key.clone(),
            content_sha256: source.content_sha256.clone(),
            adaptation_plan_id: adaptation.record.id,
            adaptation_plan_revision: adaptation.record.revision,
            adaptation_plan_sha256: adaptation.record.sha256.clone(),
        };
        let variables = HashMap::new();
        let reference_assets = manga_reference_assets_json(&character_portraits);
        let reference_assets_metadata = reference_assets.clone();
        let prompt_fingerprint = serde_json::json!({
            "pipeline_contract": "manga_page_pipeline_v2",
            "prompts": manga_pipeline_prompt_fingerprint(
                prompt_renderer,
                project_id,
                &variables,
            )
            .await?,
            "reference_assets": reference_assets_metadata,
        });
        let context = MangaRunContext {
            service: self,
            chapter_id,
            chapter_number,
            project_id,
            user_id,
            adaptation,
            prompt_renderer,
            task_id,
            workflow_run_id,
            resume_from_checkpoint,
            stop_after,
            image_size,
            image_generator,
            cancel_token,
            progress_reporter,
            stream_reporter,
            chapter_content,
            source_spans,
            variables,
            reference_assets,
            source_metadata,
            reference_assets_metadata,
            prompt_fingerprint,
        };
        context
            .advance_stage(MangaPipelineStage::CharacterBinding)
            .await?;
        let early = run_early_extraction(&context).await?;
        let extraction = run_late_extraction(&context, early).await?;
        let page_slots = run_page_slots(&context, &extraction.extraction_summary).await?;
        complete_pipeline(&context, extraction, page_slots).await
    }
}
