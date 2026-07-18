use super::super::checkpoints::load_galgame_preview_checkpoint_inputs;
use super::super::context::load_galgame_pipeline_prompt_context;
use super::super::contracts::StartGalgamePipelineInput;
use super::steps::{GalgamePreviewStepInputs, build_galgame_preview_steps};
use crate::AppState;
use crate::actions::access;
use crate::actions::request_preview::{ModelRequestPreview, model_request_preview};
use a4n_orchestrator::galgame::{galgame_shooting_script_projection, parse_galgame_start_stage};

pub async fn preview_galgame_pipeline_request(
    state: &AppState,
    user_id: i64,
    input: StartGalgamePipelineInput,
) -> Result<ModelRequestPreview, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let start_from_stage = parse_galgame_start_stage(input.start_from_stage.as_deref())
        .map_err(|err| err.to_string())?;
    let context = load_galgame_pipeline_prompt_context(state, input.project_id).await?;
    let checkpoint_inputs = load_galgame_preview_checkpoint_inputs(
        state,
        input.project_id,
        start_from_stage,
        input.resume_from_checkpoint,
    )
    .await?;
    let shooting_script_projection_input = checkpoint_inputs
        .shooting_script_projection_input()?
        .unwrap_or(
            serde_json::to_value(
                galgame_shooting_script_projection(&context).map_err(|err| err.to_string())?,
            )
            .map_err(|err| err.to_string())?,
        );
    let branch_design_input = checkpoint_inputs.branch_design_input()?;
    let script_adaptation_input = checkpoint_inputs.script_adaptation_input()?;
    let character_staging_input = checkpoint_inputs.character_staging_input()?;
    let steps = build_galgame_preview_steps(
        state,
        GalgamePreviewStepInputs::new(
            start_from_stage,
            &context,
            &shooting_script_projection_input,
            branch_design_input.as_ref(),
            script_adaptation_input.as_ref(),
            character_staging_input.as_ref(),
        ),
    )
    .await?;
    Ok(model_request_preview(
        "galgame_pipeline",
        "Galgame 七阶段管线请求预览",
        steps,
    ))
}
