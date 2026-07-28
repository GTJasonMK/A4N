mod execution;
mod preview;

pub use execution::{preflight_manga_pipeline, run_manga_pipeline};
pub use preview::preview_manga_pipeline_requests;

pub type MangaPipelineStartStage = a4n_orchestrator::manga::MangaPipelineStartStage;

pub fn parse_manga_pipeline_start_stage(
    value: Option<String>,
) -> Result<Option<MangaPipelineStartStage>, a4n_core::error::AppError> {
    a4n_orchestrator::manga::parse_manga_pipeline_start_stage(value)
}
