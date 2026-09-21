use a4n_db::workflow;
use a4n_db_models::models::chapter::chapter;
use a4n_domain::source_spans::source_spans_for_chapters;
use a4n_error::AppError;
use a4n_prompt::rendering::PromptRenderer;

use super::super::{ChapterAnalysisProgress, OrchestratorContext};
use super::contracts::ChapterAnalysisRun;
use super::finalization::FinalizeChapterAnalysisInput;
use crate::workflow::workflow_checkpoint_contract_app_error;

impl OrchestratorContext {
    pub(super) async fn analyze_chapter(
        &self,
        chapter: &chapter::Model,
        prompt_renderer: &dyn PromptRenderer,
        progress: &mut ChapterAnalysisProgress<'_>,
        workflow_run_id: Option<i64>,
    ) -> Result<ChapterAnalysisRun, AppError> {
        let workflow_run_id =
            workflow::required_llm_call_workflow_run_id(workflow_run_id, "chapter_analysis")
                .map_err(workflow_checkpoint_contract_app_error)?;
        let source_spans = source_spans_for_chapters(std::slice::from_ref(chapter));
        let facts = self
            .execute_chapter_analysis_facts_stage(
                chapter,
                prompt_renderer,
                &source_spans,
                progress,
                workflow_run_id,
            )
            .await?;
        let summary = self
            .execute_chapter_analysis_summary_stage(
                chapter,
                prompt_renderer,
                &facts.output,
                progress,
                workflow_run_id,
            )
            .await?;
        let character_states = self
            .execute_chapter_analysis_character_state_stage(
                chapter,
                prompt_renderer,
                &facts.output,
                progress,
                workflow_run_id,
            )
            .await?;
        let foreshadowing_events = self
            .execute_chapter_analysis_foreshadowing_stage(
                chapter,
                prompt_renderer,
                &facts.output,
                progress,
                workflow_run_id,
            )
            .await?;
        let analysis = self
            .finalize_chapter_analysis(
                FinalizeChapterAnalysisInput {
                    chapter,
                    facts: facts.output,
                    summary: summary.output,
                    character_states: character_states.output,
                    foreshadowing_events: foreshadowing_events.output,
                    source_spans: &source_spans,
                    workflow_run_id,
                },
                progress,
            )
            .await?;
        Ok(ChapterAnalysisRun {
            analysis,
            stage_traces: vec![
                facts.trace,
                summary.trace,
                character_states.trace,
                foreshadowing_events.trace,
            ],
        })
    }
}
