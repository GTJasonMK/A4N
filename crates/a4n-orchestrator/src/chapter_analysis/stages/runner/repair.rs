use std::sync::Mutex;

use a4n_core::chapter_analysis::CHAPTER_ANALYSIS_SERVICE_PROGRESS_TOTAL;
use a4n_core::error::AppError;
use a4n_llm::provider::ChatRequest;
use serde_json::Value;

use crate::OrchestratorContext;
use crate::structured_llm::{StructuredRepairCheckpointContext, StructuredValueRepairInput};

use super::super::super::ChapterAnalysisProgress;
use super::super::contracts::ChapterAnalysisStageSpec;
use super::schema::chapter_analysis_stage_schema;

pub(super) struct ChapterAnalysisStageRepairInput<'a, 'progress, V> {
    pub(super) request: ChatRequest,
    pub(super) response_content: &'a str,
    pub(super) progress: &'a mut ChapterAnalysisProgress<'progress>,
    pub(super) workflow_run_id: i64,
    pub(super) spec: ChapterAnalysisStageSpec,
    pub(super) validate: &'a V,
}

pub(super) async fn repair_chapter_analysis_stage_output<V>(
    context: &OrchestratorContext,
    input: ChapterAnalysisStageRepairInput<'_, '_, V>,
) -> Result<Value, AppError>
where
    V: Fn(&Value) -> Result<(), AppError>,
{
    let ChapterAnalysisStageRepairInput {
        request,
        response_content,
        progress,
        workflow_run_id,
        spec,
        validate,
    } = input;
    let parse_output = |value: &Value| {
        if !value.is_object() {
            return Err(AppError::InvalidArgument(format!(
                "{} 必须返回 JSON 对象",
                spec.context
            )));
        }
        validate(value)?;
        Ok(value.clone())
    };
    let cancel_token = progress.cancel_token;
    let repair_progress = Mutex::new(progress);
    let report_repair_progress = |message: &str| {
        let mut progress = repair_progress.lock().map_err(|_| AppError::InvalidState {
            current: "chapter_analysis_progress_poisoned".to_string(),
            operation: format!("报告{}结构纠正进度", spec.context),
        })?;
        progress.report(
            spec.progress_span.end(),
            CHAPTER_ANALYSIS_SERVICE_PROGRESS_TOTAL,
            message,
        )
    };
    let repaired = context
        .parse_validate_or_repair_structured_value(StructuredValueRepairInput {
            request,
            raw_response: response_content,
            label: spec.context,
            schema: chapter_analysis_stage_schema(spec.stage)?,
            checkpoint: Some(StructuredRepairCheckpointContext {
                workflow_run_id,
                base_call_key: spec.step_key.to_string(),
                call_order: spec.step_order,
            }),
            cancel_token,
            progress: Some(&report_repair_progress),
            parse_output: &parse_output,
        })
        .await?;
    Ok(repaired.output)
}
