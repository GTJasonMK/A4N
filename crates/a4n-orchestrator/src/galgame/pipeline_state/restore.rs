use a4n_core::error::AppError;

use super::stage::{GalgamePipelineStartStage, galgame_pipeline_stage_keys};

pub fn parse_galgame_start_stage(
    value: Option<&str>,
) -> Result<Option<GalgamePipelineStartStage>, AppError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    GalgamePipelineStartStage::parse(value)
        .map(Some)
        .ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "未知 Galgame 管线起始阶段: {value}; 可用阶段: {}",
                galgame_pipeline_stage_keys().join(", ")
            ))
        })
}

pub fn should_restore_galgame_stage_checkpoint(
    resume_from_checkpoint: bool,
    start_from_stage: Option<GalgamePipelineStartStage>,
    stage_order: i32,
) -> bool {
    if let Some(start_stage) = start_from_stage {
        return stage_order < start_stage.order();
    }
    resume_from_checkpoint
}
