use a4n_core::error::AppError;
use a4n_db::workflow::{self, CompletedLlmCallCheckpoint, CompletedLlmCallCheckpointInput};

use crate::outline::part_contract::parse_part_outline_single_response;
use crate::outline::{OrchestratorContext, ensure_response_not_truncated};
use crate::workflow::workflow_checkpoint_contract_app_error;

use super::super::contracts::{
    GeneratedPartOutline, PartOutlineJsonInput, PartOutlineRequestInput,
};
use super::super::requests::{
    build_part_outline_request, part_outline_call_fingerprint_json,
    restored_part_outline_checkpoint,
};

impl OrchestratorContext {
    pub(in crate::outline::parts) async fn generate_part_outline_json(
        &self,
        input: PartOutlineJsonInput<'_>,
    ) -> Result<GeneratedPartOutline, AppError> {
        let PartOutlineJsonInput {
            project_id,
            part_num,
            start_ch,
            end_ch,
            chapters_per_part,
            requested_part_count,
            blueprint_context,
            previous_parts,
            source_snapshot,
            prompt_renderer,
            variables,
            workflow_run_id,
        } = input;
        let mut request = build_part_outline_request(PartOutlineRequestInput {
            project_id,
            part_num,
            start_ch,
            end_ch,
            chapters_per_part,
            requested_part_count,
            blueprint_context,
            previous_parts,
            source_snapshot,
            prompt_renderer,
            variables,
        })
        .await?;
        let call_key = format!("part_{part_num}");
        let workflow_run_id =
            workflow::required_llm_call_workflow_run_id(workflow_run_id, &call_key)
                .map_err(workflow_checkpoint_contract_app_error)?;
        let fingerprint_json = part_outline_call_fingerprint_json(
            &call_key,
            part_num,
            start_ch,
            end_ch,
            chapters_per_part,
            requested_part_count,
            &request,
            source_snapshot,
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
            restored_part_outline_checkpoint(
                checkpoint.parsed_output,
                checkpoint.response_content,
                checkpoint.finish_reason,
                request,
                part_num,
                start_ch,
                end_ch,
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
                &format!("第 {part_num} 部分部大纲"),
            )?;
            let data = parse_part_outline_single_response(&content, part_num, start_ch, end_ch)?;
            let generated = GeneratedPartOutline {
                data,
                request,
                response_content: content,
                finish_reason,
            };
            self.save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
                workflow_run_id,
                call_key,
                call_order: part_num,
                input_fingerprint_json: fingerprint_json,
                checkpoint: CompletedLlmCallCheckpoint {
                    parsed_output: generated.data.clone(),
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
