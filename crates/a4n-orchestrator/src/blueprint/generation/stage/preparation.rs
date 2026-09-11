use a4n_domain::project::blueprint_stage_call_key;
use a4n_error::AppError;
use a4n_prompt::blueprint_generation::{BlueprintStageRequestInput, build_blueprint_stage_request};

use crate::blueprint::OrchestratorContext;

use super::super::checkpoint::blueprint_stage_call_fingerprint_json;
use super::contracts::{BlueprintStageRunInput, PreparedBlueprintStage};
use super::rules::ensure_not_cancelled;

pub(super) async fn prepare_blueprint_stage<'a>(
    context: &OrchestratorContext,
    input: BlueprintStageRunInput<'a>,
) -> Result<PreparedBlueprintStage<'a>, AppError> {
    let BlueprintStageRunInput {
        project_id,
        spec,
        publisher,
        session,
        checkpoint_writer,
        cancel_token,
        prompt_renderer,
        base_prompt,
        user_payload_json,
        previous_context,
        workflow_run_id,
    } = input;
    ensure_not_cancelled(&cancel_token)?;
    let mut request = build_blueprint_stage_request(BlueprintStageRequestInput {
        prompt_renderer,
        project_id,
        base_prompt,
        user_payload_json,
        stage_key: spec.key,
        stage_name: spec.title,
        output_schema: spec.output_schema,
        previous_context,
    })
    .await?;
    let call_key = blueprint_stage_call_key(spec.key);
    let fingerprint_json = blueprint_stage_call_fingerprint_json(spec, &request)?;
    crate::workflow::attach_llm_call_checkpoint_trace(
        &mut request,
        workflow_run_id,
        &call_key,
        &fingerprint_json,
    )?;
    context
        .mark_workflow_current_step(workflow_run_id, &call_key)
        .await?;

    Ok(PreparedBlueprintStage {
        spec,
        publisher,
        session,
        checkpoint_writer,
        cancel_token,
        request,
        workflow_run_id,
        fingerprint_json,
    })
}
