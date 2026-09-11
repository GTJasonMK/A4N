mod visual;

use a4n_error::AppError;

use super::contracts::{
    GalgamePipelineEarlyOutputs, GalgamePipelineExecution, GalgamePipelineLateOutputs,
};

pub(super) async fn run_galgame_pipeline_late_stages(
    execution: &GalgamePipelineExecution<'_>,
    early: &GalgamePipelineEarlyOutputs,
) -> Result<GalgamePipelineLateOutputs, AppError> {
    let visual_plan = visual::run_visual_planning_stage(execution, early).await?;
    Ok(GalgamePipelineLateOutputs { visual_plan })
}
