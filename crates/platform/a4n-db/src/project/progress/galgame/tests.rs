use super::status::galgame_project_progress_from_counts;
use crate::project::progress::shared::PipelineProjectState;

#[test]
fn galgame_project_progress_prioritizes_failed_pipeline_over_ready_script() {
    let pipeline = PipelineProjectState {
        failed: true,
        message: Some("LLM failed".to_string()),
        ..Default::default()
    };
    let summary = galgame_project_progress_from_counts(true, 3, 12, pipeline);

    assert_eq!(summary.label, "Galgame 管线失败");
    assert_eq!(summary.detail.as_deref(), Some("LLM failed"));
}
