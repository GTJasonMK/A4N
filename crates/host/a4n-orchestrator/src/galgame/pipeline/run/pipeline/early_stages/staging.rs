use a4n_error::AppError;
use serde_json::Value;

use crate::galgame::{
    GalgameCharacterStaging, GalgamePipelineStartStage, GalgameScriptAdaptation,
    character_staging_from_value, galgame_character_staging_user_message,
};

use super::super::super::contracts::CHECKPOINT_LINES_STAGED;
use super::super::super::llm::GalgameLlmStageInput;
use super::super::super::progress::ensure_galgame_pipeline_not_cancelled;
use super::super::contracts::GalgamePipelineExecution;

pub(super) async fn run_character_staging_stage(
    execution: &GalgamePipelineExecution<'_>,
    scenes_json: &Value,
    lines_draft_json: &Value,
    script: &GalgameScriptAdaptation,
) -> Result<(Value, GalgameCharacterStaging), AppError> {
    let stage = GalgamePipelineStartStage::CharacterStaging;
    ensure_galgame_pipeline_not_cancelled(execution.service, execution.options.workflow_run_id)
        .await?;
    execution
        .service
        .report_pipeline_progress(
            execution.options.workflow_run_id,
            execution.options.task_id.as_deref(),
            execution.options.progress_reporter.as_ref(),
            stage,
            4,
            stage.message(),
        )
        .await?;
    let user_prompt = galgame_character_staging_user_message(
        lines_draft_json,
        &execution.prompt_context.speaker_profiles,
        execution.source_markers,
    )?;
    let request_user_prompt = user_prompt.clone();
    let output = execution
        .service
        .run_galgame_llm_stage(
            GalgameLlmStageInput {
                prompt_renderer: execution.prompt_renderer,
                project_id: execution.project_id,
                root_project_id: execution.prompt_context.root_project_id,
                workflow_run_id: execution.options.workflow_run_id,
                stage,
                checkpoint_key: CHECKPOINT_LINES_STAGED,
                prompt_name: "galgame_character_staging",
                user_prompt,
                request_user_prompt,
                shared_adaptation_projection: scenes_json,
                source_markers: execution.source_markers,
                restore_from_checkpoint: execution.should_restore(stage),
                restore_required: execution.restore_required(stage),
            },
            |value| {
                let parsed = character_staging_from_value(value, "galgame_character_staging")?;
                crate::galgame::pipeline::persist::validate_galgame_staging(
                    &parsed,
                    script,
                    &execution.prompt_context.speaker_profiles,
                )?;
                Ok(parsed)
            },
        )
        .await?;
    execution
        .service
        .report_pipeline_progress(
            execution.options.workflow_run_id,
            execution.options.task_id.as_deref(),
            execution.options.progress_reporter.as_ref(),
            stage,
            4,
            "已完成 Galgame 角色演出标注",
        )
        .await?;
    Ok(output)
}
