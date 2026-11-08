mod call;

use a4n_chapter_analysis::CHAPTER_ANALYSIS_SERVICE_PROGRESS_TOTAL;
use a4n_db::workflow::{CompletedLlmCallCheckpoint, CompletedLlmCallCheckpointInput};
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatRequest;
use a4n_prompt::chapter_analysis::chapter_analysis_stage_fingerprint;
use serde_json::Value;

use self::call::chat_chapter_analysis_stage;
use super::super::checkpoint::{
    parse_chapter_analysis_stage_output, restored_chapter_analysis_stage_output,
};
use super::super::execution::ChapterAnalysisStageTrace;
use super::super::{ChapterAnalysisProgress, OrchestratorContext};
use super::contracts::{ChapterAnalysisStageSpec, ExecutedChapterAnalysisStage};
use crate::source_markers::SourceMarkerMap;

impl OrchestratorContext {
    pub(super) async fn execute_chapter_analysis_stage(
        &self,
        request: ChatRequest,
        progress: &mut ChapterAnalysisProgress<'_>,
        workflow_run_id: i64,
        spec: ChapterAnalysisStageSpec,
        source_markers: &SourceMarkerMap,
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
                source_markers,
                &validate,
            )?;
            progress.report(
                spec.restored_progress,
                CHAPTER_ANALYSIS_SERVICE_PROGRESS_TOTAL,
                &format!("{}已复用断点", spec.context),
            )?;
            output
        } else {
            let response =
                chat_chapter_analysis_stage(self, request.clone(), spec, progress).await?;
            let (response, output) = self
                .validate_llm_response(
                    &request,
                    crate::structured_llm::structured_chat_response(response),
                    spec.context,
                    None,
                    |raw| {
                        parse_chapter_analysis_stage_output(
                            raw,
                            spec.context,
                            source_markers,
                            &validate,
                        )
                    },
                )
                .await?;
            self.save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
                workflow_run_id,
                call_key: spec.step_key.to_string(),
                call_order: spec.step_order,
                input_fingerprint_json: fingerprint,
                checkpoint: CompletedLlmCallCheckpoint {
                    parsed_output: output.clone(),
                    response_content: response.message.content,
                    usage: response
                        .usage
                        .as_ref()
                        .map(serde_json::to_value)
                        .transpose()?,
                    finish_reason: response.finish_reason,
                    artifacts_json: None,
                },
            })
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
