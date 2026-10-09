use crate::state::AppState;

use super::super::{
    chat::{chat_request_step_for_state, model_request_preview},
    contracts::{ChatRequestStepInput, ModelRequestPreview, ModelRequestPreviewStep},
};
use super::model::{ChatStatusPreviewStep, status_step};

pub(crate) async fn chat_status_preview_for_state(
    state: &AppState,
    workflow: impl Into<String>,
    title: impl Into<String>,
    steps: impl IntoIterator<Item = ChatStatusPreviewStep>,
    default_status_kind: &'static str,
) -> Result<ModelRequestPreview, String> {
    Ok(model_request_preview(
        workflow,
        title,
        chat_status_preview_steps_for_state(state, steps, default_status_kind).await?,
    ))
}

async fn chat_status_preview_steps_for_state(
    state: &AppState,
    steps: impl IntoIterator<Item = ChatStatusPreviewStep>,
    default_status_kind: &'static str,
) -> Result<Vec<ModelRequestPreviewStep>, String> {
    let mut result = Vec::new();
    for step in steps {
        let preview_step = match step.request {
            Some(request) => {
                chat_request_step_for_state(
                    state,
                    ChatRequestStepInput {
                        key: step.key,
                        title: step.title,
                        prompt_name: step.prompt_name,
                        prompt_assembly: None,
                        state: step.state,
                        note: step.note,
                        request,
                    },
                )
                .await?
            }
            None => status_step(
                step.key,
                step.title,
                step.kind.unwrap_or_else(|| default_status_kind.to_string()),
                step.prompt_name,
                step.state,
                step.note,
            ),
        };
        result.push(preview_step);
    }
    Ok(result)
}
