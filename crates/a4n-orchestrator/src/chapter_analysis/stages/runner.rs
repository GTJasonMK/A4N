mod call;

use a4n_core::chapter_analysis::CHAPTER_ANALYSIS_SERVICE_PROGRESS_TOTAL;
use a4n_core::error::AppError;
use a4n_llm::provider::ChatRequest;
use a4n_prompt::chapter_analysis::chapter_analysis_stage_fingerprint;
use serde_json::Value;

use self::call::chat_chapter_analysis_stage;
use super::super::checkpoint::{
    parse_chapter_analysis_stage_output, restored_chapter_analysis_stage_output,
};
use super::super::execution::ChapterAnalysisStageTrace;
use super::super::{ChapterAnalysisProgress, OrchestratorContext};
use super::contracts::{ChapterAnalysisStageSpec, ExecutedChapterAnalysisStage};

impl OrchestratorContext {
    pub(super) async fn execute_chapter_analysis_stage(
        &self,
        request: ChatRequest,
        progress: &mut ChapterAnalysisProgress<'_>,
        workflow_run_id: i64,
        spec: ChapterAnalysisStageSpec,
        validate: impl Fn(&Value) -> Result<(), AppError>,
    ) -> Result<ExecutedChapterAnalysisStage, AppError> {
        self.mark_workflow_current_step(workflow_run_id, spec.step_key)
            .await?;
        let fingerprint = chapter_analysis_stage_fingerprint(spec.stage, &request)?;
        let output = if let Some(checkpoint) = self
            .load_completed_llm_call_checkpoint(workflow_run_id, spec.step_key, &fingerprint)
            .await?
        {
            let output = restored_chapter_analysis_stage_output(
                &checkpoint.parsed_output,
                &checkpoint.response_content,
                spec.context,
                &validate,
            )?;
            progress.report(
                spec.restored_progress,
                CHAPTER_ANALYSIS_SERVICE_PROGRESS_TOTAL,
                &format!("{}已复用断点", spec.context),
            )?;
            output
        } else {
            let response_content =
                chat_chapter_analysis_stage(self, request.clone(), spec, progress).await?;
            let output =
                parse_chapter_analysis_stage_output(&response_content, spec.context, &validate)?;
            self.save_completed_chapter_analysis_stage_checkpoint(
                workflow_run_id,
                spec.step_key,
                spec.step_order,
                fingerprint,
                response_content,
                output.clone(),
            )
            .await?;
            output
        };
        Ok(ExecutedChapterAnalysisStage {
            output: output.clone(),
            trace: ChapterAnalysisStageTrace {
                stage: spec.stage,
                request,
                output,
            },
        })
    }
}
