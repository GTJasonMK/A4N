use a4n_chapter_analysis::{CHAPTER_ANALYSIS_SERVICE_PROGRESS_TOTAL, CHAPTER_ANALYSIS_STEP_FINAL};
use a4n_domain::chapter::ChapterRecord;
use a4n_domain::source_spans::SourceSpan;
use a4n_error::AppError;
use serde_json::{Value, json};

use super::super::owner::{
    assemble_staged_chapter_analysis, materialize_analysis_source_spans, normalize_analysis,
};
use super::super::{ChapterAnalysisProgress, OrchestratorContext};

pub(super) struct FinalizeChapterAnalysisInput<'a> {
    pub(super) chapter: &'a ChapterRecord,
    pub(super) facts: Value,
    pub(super) summary: Value,
    pub(super) character_states: Value,
    pub(super) foreshadowing_events: Value,
    pub(super) source_spans: &'a [SourceSpan],
    pub(super) workflow_run_id: i64,
}

impl OrchestratorContext {
    pub(super) async fn finalize_chapter_analysis(
        &self,
        input: FinalizeChapterAnalysisInput<'_>,
        progress: &mut ChapterAnalysisProgress<'_>,
    ) -> Result<Value, AppError> {
        let FinalizeChapterAnalysisInput {
            chapter,
            facts,
            summary,
            character_states,
            foreshadowing_events,
            source_spans,
            workflow_run_id,
        } = input;
        self.mark_workflow_current_step(workflow_run_id, CHAPTER_ANALYSIS_STEP_FINAL)
            .await?;
        progress.report(
            96,
            CHAPTER_ANALYSIS_SERVICE_PROGRESS_TOTAL,
            "正在组装章节结构化分析结果",
        )?;
        let parsed = assemble_staged_chapter_analysis(
            facts,
            summary,
            character_states,
            foreshadowing_events,
        )?;
        let materialized = materialize_analysis_source_spans(parsed, source_spans)?;
        let analysis = normalize_analysis(materialized, chapter)?;
        self.save_completed_chapter_analysis_final_checkpoint(
            workflow_run_id,
            CHAPTER_ANALYSIS_STEP_FINAL,
            5,
            json!({
                "schema_version": 1,
                "stage": "chapter_analysis.final",
                "chapter_id": chapter.id,
                "chapter_number": chapter.chapter_number,
                "analysis": &analysis,
            }),
            analysis.clone(),
        )
        .await?;
        progress.report(
            CHAPTER_ANALYSIS_SERVICE_PROGRESS_TOTAL,
            CHAPTER_ANALYSIS_SERVICE_PROGRESS_TOTAL,
            "章节结构化分析结果已生成",
        )?;
        Ok(analysis)
    }
}
