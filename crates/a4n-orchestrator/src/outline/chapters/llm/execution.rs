use a4n_core::error::AppError;
use a4n_db::workflow::{self, CompletedLlmCallCheckpointInput};
use serde_json::json;

use crate::outline::chapter_contract::{
    chapter_outline_batch_response_schema, parse_chapter_outline_batch,
};
use crate::outline::{OrchestratorContext, ensure_response_not_truncated};
use crate::structured_llm::{StructuredRepairCheckpointContext, StructuredValueRepairInput};
use crate::workflow::{
    workflow_checkpoint_contract_app_error, workflow_read_app_error, workflow_write_app_error,
};

use super::super::contracts::{
    ChapterOutlineRequestInput, GeneratedChapterOutline, SingleChapterOutlineJsonInput,
};
use super::super::requests::build_chapter_outline_request;
use super::checkpoint::{
    chapter_outline_call_fingerprint_json, restored_chapter_outline_checkpoint,
};

impl OrchestratorContext {
    pub(in crate::outline::chapters) async fn generate_single_chapter_outline_json(
        &self,
        input: SingleChapterOutlineJsonInput<'_>,
    ) -> Result<GeneratedChapterOutline, AppError> {
        let SingleChapterOutlineJsonInput {
            project_id,
            context,
            focus_part,
            chapter_number,
            prompt_renderer,
            variables,
            workflow_run_id,
        } = input;
        let mut request = build_chapter_outline_request(ChapterOutlineRequestInput {
            project_id,
            context,
            focus_part,
            start_chapter: chapter_number,
            count: 1,
            end_chapter: chapter_number,
            prompt_renderer,
            variables,
        })
        .await?;
        let call_key = format!("chapter_{chapter_number}");
        let workflow_run_id =
            workflow::required_llm_call_workflow_run_id(workflow_run_id, &call_key)
                .map_err(workflow_checkpoint_contract_app_error)?;
        let fingerprint_json =
            chapter_outline_call_fingerprint_json(&call_key, chapter_number, &request)?;
        let input_fingerprint_sha256 =
            workflow::llm_call_checkpoint_fingerprint_sha256(&fingerprint_json)
                .map_err(workflow_checkpoint_contract_app_error)?;
        if let Some(trace_context) = request.trace_context.as_mut() {
            trace_context.workflow_run_id = Some(workflow_run_id);
            trace_context.call_key = Some(call_key.clone());
            trace_context.input_fingerprint_sha256 = Some(input_fingerprint_sha256);
        }
        workflow::mark_workflow_current_step(self.db.connection(), workflow_run_id, &call_key)
            .await
            .map_err(workflow_write_app_error)?;
        if let Some(checkpoint) = workflow::load_completed_llm_call_checkpoint(
            self.db.connection(),
            workflow_run_id,
            &call_key,
            &fingerprint_json,
        )
        .await
        .map_err(workflow_read_app_error)?
        {
            let checkpoint_json =
                workflow::required_llm_call_checkpoint_json(checkpoint, &call_key)
                    .map_err(workflow_checkpoint_contract_app_error)?;
            return restored_chapter_outline_checkpoint(checkpoint_json, request, chapter_number);
        }
        let response = self.llm.chat(request.clone()).await?;

        let response_usage_json = response
            .usage
            .as_ref()
            .map(serde_json::to_value)
            .transpose()?;
        let content = response.message.content;
        let finish_reason = response.finish_reason;
        ensure_response_not_truncated(
            finish_reason.as_deref(),
            &format!("第 {chapter_number} 章章纲"),
        )?;
        let parse_output = |value: &serde_json::Value| {
            parse_chapter_outline_batch(&serde_json::to_string(value)?, chapter_number, 1)
        };
        let repaired = self
            .parse_validate_or_repair_structured_value(StructuredValueRepairInput {
                request: request.clone(),
                raw_response: &content,
                label: "章节大纲",
                schema: chapter_outline_batch_response_schema(),
                checkpoint: Some(StructuredRepairCheckpointContext {
                    workflow_run_id,
                    base_call_key: call_key.clone(),
                    call_order: chapter_number,
                }),
                cancel_token: None,
                progress: None,
                parse_output: &parse_output,
            })
            .await?;
        let repair_artifact = repaired.artifact_summary();
        let mut chapters = repaired.output;
        let chapter_json = chapters
            .pop()
            .ok_or_else(|| AppError::InvalidArgument("outline.chapters 不能为空".to_string()))?;
        let generated = GeneratedChapterOutline {
            chapter_json,
            request,
            response_content: content,
            finish_reason,
        };
        workflow::save_completed_llm_call_checkpoint(
            self.db.connection(),
            CompletedLlmCallCheckpointInput {
                workflow_run_id,
                call_key,
                call_order: chapter_number,
                input_fingerprint_json: fingerprint_json,
                checkpoint_json: json!({
                    "output": generated.chapter_json.clone(),
                    "parsed_output": generated.chapter_json.clone(),
                    "response_content": generated.response_content.clone(),
                    "usage": response_usage_json,
                    "finish_reason": generated.finish_reason.clone(),
                }),
                artifacts_json: Some(json!({
                    "structured_repair": repair_artifact,
                })),
            },
        )
        .await
        .map_err(workflow_write_app_error)?;
        Ok(generated)
    }
}
