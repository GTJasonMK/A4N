use super::contracts::{
    GALGAME_CHECKPOINT_BRANCH_PLAN, GALGAME_CHECKPOINT_LINES_DRAFT,
    GALGAME_CHECKPOINT_LINES_STAGED, GALGAME_CHECKPOINT_SCENES, GALGAME_CHECKPOINT_VISUAL_PLAN,
    GALGAME_PIPELINE_WORKFLOW_KIND,
};
use crate::AppState;
use a4n_orchestrator::galgame::{
    GalgamePipelineStartStage, branch_design_from_value, character_staging_from_value,
    script_adaptation_from_value, shooting_script_projection_from_value, visual_plan_from_value,
};
use a4n_orchestrator::workflow::{ListWorkflowRunsInput, WORKFLOW_STATUS_COMPLETED};
use serde_json::Value;

mod inputs;

pub(super) use inputs::GalgamePreviewCheckpointInputs;

pub(super) async fn load_galgame_preview_checkpoint_inputs(
    state: &AppState,
    project_id: i64,
    start_from_stage: Option<GalgamePipelineStartStage>,
    resume_from_checkpoint: bool,
) -> Result<GalgamePreviewCheckpointInputs, String> {
    let requires_prior_checkpoint = start_from_stage.is_some();
    if !requires_prior_checkpoint && !resume_from_checkpoint {
        return Ok(GalgamePreviewCheckpointInputs::default());
    }
    let mut inputs = GalgamePreviewCheckpointInputs {
        required_prior_stage: start_from_stage,
        ..Default::default()
    };
    let Some(run) = state
        .orchestrator
        .workflow_run_models(ListWorkflowRunsInput {
            project_id: Some(project_id),
            workflow_kind: Some(GALGAME_PIPELINE_WORKFLOW_KIND.to_string()),
            status: None,
            resumable_only: None,
            limit: Some(1),
        })
        .await
        .map_err(|err| err.to_string())?
        .into_iter()
        .next()
    else {
        return Ok(inputs);
    };
    for checkpoint in state
        .orchestrator
        .workflow_checkpoint_models(run.id)
        .await
        .map_err(|err| err.to_string())?
    {
        if checkpoint.status != WORKFLOW_STATUS_COMPLETED {
            continue;
        }
        match checkpoint.step_key.as_str() {
            GALGAME_CHECKPOINT_SCENES => {
                inputs.scenes = Some(galgame_preview_checkpoint_output(
                    GALGAME_CHECKPOINT_SCENES,
                    checkpoint.checkpoint_json.as_ref(),
                )?);
            }
            GALGAME_CHECKPOINT_LINES_DRAFT => {
                inputs.lines_draft = Some(galgame_preview_checkpoint_output(
                    GALGAME_CHECKPOINT_LINES_DRAFT,
                    checkpoint.checkpoint_json.as_ref(),
                )?);
            }
            GALGAME_CHECKPOINT_LINES_STAGED => {
                inputs.lines_staged = Some(galgame_preview_checkpoint_output(
                    GALGAME_CHECKPOINT_LINES_STAGED,
                    checkpoint.checkpoint_json.as_ref(),
                )?);
            }
            GALGAME_CHECKPOINT_VISUAL_PLAN => {
                inputs.visual_plan = Some(galgame_preview_checkpoint_output(
                    GALGAME_CHECKPOINT_VISUAL_PLAN,
                    checkpoint.checkpoint_json.as_ref(),
                )?);
            }
            GALGAME_CHECKPOINT_BRANCH_PLAN => {
                inputs.branch_plan = Some(galgame_preview_checkpoint_output(
                    GALGAME_CHECKPOINT_BRANCH_PLAN,
                    checkpoint.checkpoint_json.as_ref(),
                )?);
            }
            other => {
                return Err(format!(
                    "Galgame 管线存在未知 completed checkpoint: {other}"
                ));
            }
        }
    }
    inputs.validate_required_prior_checkpoints()?;
    Ok(inputs)
}

pub(super) fn galgame_preview_checkpoint_output(
    checkpoint_key: &str,
    checkpoint_json: Option<&Value>,
) -> Result<Value, String> {
    let object = checkpoint_json
        .and_then(Value::as_object)
        .ok_or_else(|| format!("Galgame checkpoint {checkpoint_key} 缺少 JSON 对象"))?;
    let output = object
        .get("output")
        .ok_or_else(|| format!("Galgame checkpoint {checkpoint_key} 缺少 output"))?
        .clone();
    if let Some(parsed_output) = object.get("parsed_output")
        && parsed_output != &output
    {
        return Err(format!(
            "Galgame checkpoint {checkpoint_key} 的 output 与 parsed_output 不一致"
        ));
    }
    match checkpoint_key {
        GALGAME_CHECKPOINT_SCENES => {
            shooting_script_projection_from_value(output.clone(), checkpoint_key)
                .map_err(|err| err.to_string())?;
        }
        GALGAME_CHECKPOINT_LINES_DRAFT => {
            script_adaptation_from_value(output.clone(), checkpoint_key)
                .map_err(|err| err.to_string())?;
        }
        GALGAME_CHECKPOINT_LINES_STAGED => {
            character_staging_from_value(output.clone(), checkpoint_key)
                .map_err(|err| err.to_string())?;
        }
        GALGAME_CHECKPOINT_VISUAL_PLAN => {
            visual_plan_from_value(output.clone(), checkpoint_key)
                .map_err(|err| err.to_string())?;
        }
        GALGAME_CHECKPOINT_BRANCH_PLAN => {
            branch_design_from_value(output.clone(), checkpoint_key)
                .map_err(|err| err.to_string())?;
        }
        other => {
            return Err(format!("Galgame checkpoint key 不支持: {other}"));
        }
    }
    Ok(output)
}
