use a4n_db_models::models::chapter::chapter;
use a4n_domain::source_spans::SourceSpan;
use a4n_error::AppError;
use a4n_prompt::rendering::PromptRenderer;
use serde_json::Value;

use super::super::owner::{
    build_chapter_character_state_request, build_chapter_fact_extraction_request,
    build_chapter_foreshadowing_events_request, build_chapter_summary_synthesis_request,
    chapter_foreshadowing_resolution_context, validate_chapter_character_state,
    validate_chapter_fact_extraction, validate_chapter_foreshadowing_events,
    validate_chapter_summary_synthesis,
};
use super::super::{ChapterAnalysisProgress, OrchestratorContext};
use super::contracts::{
    CHARACTER_STATE_STAGE, ExecutedChapterAnalysisStage, FACTS_STAGE, FORESHADOWING_STAGE,
    SUMMARY_STAGE,
};

impl OrchestratorContext {
    pub(in crate::chapter_analysis) async fn execute_chapter_analysis_facts_stage(
        &self,
        chapter: &chapter::Model,
        prompt_renderer: &dyn PromptRenderer,
        source_spans: &[SourceSpan],
        progress: &mut ChapterAnalysisProgress<'_>,
        workflow_run_id: i64,
    ) -> Result<ExecutedChapterAnalysisStage, AppError> {
        let request =
            build_chapter_fact_extraction_request(chapter, prompt_renderer, source_spans).await?;
        self.execute_chapter_analysis_stage(
            request,
            progress,
            workflow_run_id,
            FACTS_STAGE,
            |output| validate_chapter_fact_extraction(output, source_spans),
        )
        .await
    }

    pub(in crate::chapter_analysis) async fn execute_chapter_analysis_summary_stage(
        &self,
        chapter: &chapter::Model,
        prompt_renderer: &dyn PromptRenderer,
        facts: &Value,
        progress: &mut ChapterAnalysisProgress<'_>,
        workflow_run_id: i64,
    ) -> Result<ExecutedChapterAnalysisStage, AppError> {
        let request =
            build_chapter_summary_synthesis_request(chapter, prompt_renderer, facts).await?;
        self.execute_chapter_analysis_stage(
            request,
            progress,
            workflow_run_id,
            SUMMARY_STAGE,
            validate_chapter_summary_synthesis,
        )
        .await
    }

    pub(in crate::chapter_analysis) async fn execute_chapter_analysis_character_state_stage(
        &self,
        chapter: &chapter::Model,
        prompt_renderer: &dyn PromptRenderer,
        facts: &Value,
        progress: &mut ChapterAnalysisProgress<'_>,
        workflow_run_id: i64,
    ) -> Result<ExecutedChapterAnalysisStage, AppError> {
        let request =
            build_chapter_character_state_request(chapter, prompt_renderer, facts).await?;
        self.execute_chapter_analysis_stage(
            request,
            progress,
            workflow_run_id,
            CHARACTER_STATE_STAGE,
            validate_chapter_character_state,
        )
        .await
    }

    pub(in crate::chapter_analysis) async fn execute_chapter_analysis_foreshadowing_stage(
        &self,
        chapter: &chapter::Model,
        prompt_renderer: &dyn PromptRenderer,
        facts: &Value,
        progress: &mut ChapterAnalysisProgress<'_>,
        workflow_run_id: i64,
    ) -> Result<ExecutedChapterAnalysisStage, AppError> {
        let foreshadowing = self
            .foreshadowing_items_for_rag_ingest(chapter.project_id)
            .await?;
        let resolution_context =
            chapter_foreshadowing_resolution_context(chapter.chapter_number, &foreshadowing);
        let request = build_chapter_foreshadowing_events_request(
            chapter,
            prompt_renderer,
            facts,
            &resolution_context,
        )
        .await?;
        self.execute_chapter_analysis_stage(
            request,
            progress,
            workflow_run_id,
            FORESHADOWING_STAGE,
            |output| validate_chapter_foreshadowing_events(output, facts, &resolution_context),
        )
        .await
    }
}
