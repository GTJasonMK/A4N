use a4n_core::error::AppError;
use a4n_db::workflow::{self, CompletedLlmCallCheckpointInput};
use serde_json::json;

use crate::outline::part_contract::{
    parse_part_outline_single_response, part_outline_single_response_schema,
};
use crate::outline::{OrchestratorContext, ensure_response_not_truncated};
use crate::structured_llm::{StructuredRepairCheckpointContext, StructuredValueRepairInput};
use crate::workflow::{
    workflow_checkpoint_contract_app_error, workflow_read_app_error, workflow_write_app_error,
};

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
        )?;
        crate::workflow::attach_llm_call_checkpoint_trace(
            &mut request,
            workflow_run_id,
            &call_key,
            &fingerprint_json,
        )?;
        workflow::mark_workflow_current_step(
            self.database.connection(),
            workflow_run_id,
            &call_key,
        )
        .await
        .map_err(workflow_write_app_error)?;
        if let Some(checkpoint) = workflow::load_completed_llm_call_checkpoint(
            self.database.connection(),
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
            return restored_part_outline_checkpoint(
                checkpoint_json,
                request,
                part_num,
                start_ch,
                end_ch,
            );
        }
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
        let parse_output = |value: &serde_json::Value| {
            parse_part_outline_single_response(
                &serde_json::to_string(value)?,
                part_num,
                start_ch,
                end_ch,
            )
        };
        let repaired = self
            .parse_validate_or_repair_structured_value(StructuredValueRepairInput {
                request: request.clone(),
                raw_response: &content,
                label: "部大纲",
                schema: part_outline_single_response_schema(),
                checkpoint: Some(StructuredRepairCheckpointContext {
                    workflow_run_id,
                    base_call_key: call_key.clone(),
                    call_order: part_num,
                }),
                cancel_token: None,
                progress: None,
                parse_output: &parse_output,
            })
            .await?;
        let repair_artifact = repaired.artifact_summary();
        let data = repaired.output;
        let generated = GeneratedPartOutline {
            data,
            request,
            response_content: content,
            finish_reason,
        };
        workflow::save_completed_llm_call_checkpoint(
            self.database.connection(),
            CompletedLlmCallCheckpointInput {
                workflow_run_id,
                call_key,
                call_order: part_num,
                input_fingerprint_json: fingerprint_json,
                checkpoint_json: json!({
                    "output": generated.data.clone(),
                    "parsed_output": generated.data.clone(),
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
