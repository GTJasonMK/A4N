use a4n_db::workflow::{self, CompletedLlmCallCheckpoint, CompletedLlmCallCheckpointInput};
use a4n_error::AppError;

use crate::outline::chapter_contract::parse_chapter_outline_batch;
use crate::outline::{OrchestratorContext, ensure_response_not_truncated};
use crate::workflow::workflow_checkpoint_contract_app_error;

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
            prompt_assembly_call_key,
            context,
            focus_part,
            chapter_number,
            prompt_renderer,
            variables,
            workflow_run_id,
        } = input;
        let mut request = build_chapter_outline_request(ChapterOutlineRequestInput {
            project_id,
            prompt_assembly_call_key,
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
        let fingerprint_json = chapter_outline_call_fingerprint_json(
            &call_key,
            chapter_number,
            &request,
            &context.source_snapshot,
        )?;
        crate::workflow::attach_llm_call_checkpoint_trace(
            &mut request,
            workflow_run_id,
            &call_key,
            &fingerprint_json,
        )?;
        self.mark_workflow_current_step(workflow_run_id, &call_key)
            .await?;
        let generated = if let Some(checkpoint) = self
            .load_completed_llm_call_checkpoint(workflow_run_id, &call_key, &fingerprint_json)
            .await?
        {
            restored_chapter_outline_checkpoint(
                checkpoint.parsed_output,
                checkpoint.response_content,
                checkpoint.finish_reason,
                request,
                chapter_number,
            )?
        } else {
            let response =
                crate::outline::execute_outline_llm_call(self, request.clone(), &call_key).await?;

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
            let mut chapters = parse_chapter_outline_batch(&content, chapter_number, 1)?;
            let chapter_json = chapters.pop().ok_or_else(|| {
                AppError::InvalidArgument("outline.chapters 不能为空".to_string())
            })?;
            let generated = GeneratedChapterOutline {
                chapter_json,
                request,
                response_content: content,
                finish_reason,
            };
            self.save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
                workflow_run_id,
                call_key,
                call_order: chapter_number,
                input_fingerprint_json: fingerprint_json,
                checkpoint: CompletedLlmCallCheckpoint {
                    parsed_output: generated.chapter_json.clone(),
                    response_content: generated.response_content.clone(),
                    usage: response_usage_json,
                    finish_reason: generated.finish_reason.clone(),
                    artifacts_json: None,
                },
            })
            .await?;
            generated
        };
        Ok(generated)
    }
}
