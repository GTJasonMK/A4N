use a4n_domain::chapter::ChapterRecord;
use a4n_domain::source_spans::{SourceSpan, source_spans_for_chapters};
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
use crate::source_markers::SourceMarkerMap;

impl OrchestratorContext {
    pub(in crate::chapter_analysis) async fn execute_chapter_analysis_facts_stage(
        &self,
        chapter: &ChapterRecord,
        prompt_renderer: &dyn PromptRenderer,
        source_spans: &[SourceSpan],
        progress: &mut ChapterAnalysisProgress<'_>,
        workflow_run_id: i64,
    ) -> Result<ExecutedChapterAnalysisStage, AppError> {
        let request = build_chapter_fact_extraction_request(chapter, prompt_renderer).await?;
        let source_markers = SourceMarkerMap::from_source_spans(source_spans)?;
        self.execute_chapter_analysis_stage(
            request,
            progress,
            workflow_run_id,
            FACTS_STAGE,
            &source_markers,
            |output| validate_chapter_fact_extraction(output, source_spans),
        )
        .await
    }

    pub(in crate::chapter_analysis) async fn execute_chapter_analysis_summary_stage(
        &self,
        chapter: &ChapterRecord,
        prompt_renderer: &dyn PromptRenderer,
        facts: &Value,
        progress: &mut ChapterAnalysisProgress<'_>,
        workflow_run_id: i64,
    ) -> Result<ExecutedChapterAnalysisStage, AppError> {
        let request =
            build_chapter_summary_synthesis_request(chapter, prompt_renderer, facts).await?;
        let source_markers = chapter_source_markers(chapter)?;
        self.execute_chapter_analysis_stage(
            request,
            progress,
            workflow_run_id,
            SUMMARY_STAGE,
            &source_markers,
            validate_chapter_summary_synthesis,
        )
        .await
    }

    pub(in crate::chapter_analysis) async fn execute_chapter_analysis_character_state_stage(
        &self,
        chapter: &ChapterRecord,
        prompt_renderer: &dyn PromptRenderer,
        facts: &Value,
        progress: &mut ChapterAnalysisProgress<'_>,
        workflow_run_id: i64,
    ) -> Result<ExecutedChapterAnalysisStage, AppError> {
        let request =
            build_chapter_character_state_request(chapter, prompt_renderer, facts).await?;
        let source_markers = chapter_source_markers(chapter)?;
        self.execute_chapter_analysis_stage(
            request,
            progress,
            workflow_run_id,
            CHARACTER_STATE_STAGE,
            &source_markers,
            validate_chapter_character_state,
        )
        .await
    }

    pub(in crate::chapter_analysis) async fn execute_chapter_analysis_foreshadowing_stage(
        &self,
        chapter: &ChapterRecord,
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
        let source_markers = chapter_source_markers(chapter)?;
        self.execute_chapter_analysis_stage(
            request,
            progress,
            workflow_run_id,
            FORESHADOWING_STAGE,
            &source_markers,
            |output| validate_chapter_foreshadowing_events(output, facts, &resolution_context),
        )
        .await
    }
}

fn chapter_source_markers(chapter: &ChapterRecord) -> Result<SourceMarkerMap, AppError> {
    let spans = source_spans_for_chapters(std::slice::from_ref(chapter));
    SourceMarkerMap::from_source_spans(&spans)
}
