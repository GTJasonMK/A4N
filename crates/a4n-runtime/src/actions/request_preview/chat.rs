use super::*;

mod budget;
mod step;

pub(crate) use budget::chat_request_step_for_state;
pub(crate) use step::chat_request_step_with_budget;

pub(crate) fn model_request_preview(
    workflow: impl Into<String>,
    title: impl Into<String>,
    steps: Vec<ModelRequestPreviewStep>,
) -> ModelRequestPreview {
    let ready_step_count = steps
        .iter()
        .filter(|step| step.state == "ready" || step.state == "preview_placeholder")
        .count();
    let total_chars = steps.iter().map(|step| step.total_chars).sum();
    ModelRequestPreview {
        workflow: workflow.into(),
        title: title.into(),
        steps,
        ready_step_count,
        total_chars,
    }
}
