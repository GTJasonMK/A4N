use a4n_orchestrator::video::{VideoPromptRequestPreview, VideoPromptRequestPreviewStep};

use crate::AppState;
use crate::actions::request_preview::{
    ChatRequestStepInput, ModelRequestPreview, ModelRequestPreviewStep,
    chat_request_from_preview_value, chat_request_step_for_state,
};

pub(in crate::actions::video_prompts) async fn video_prompt_preview_to_model_request_preview(
    state: &AppState,
    preview: VideoPromptRequestPreview,
) -> Result<ModelRequestPreview, String> {
    let mut steps = Vec::new();
    for step in preview.steps {
        let rebuilt_request = match step.request.as_ref() {
            Some(request) => Some(chat_request_from_preview_value(
                request,
                step.context_manifest.clone(),
            )?),
            None => None,
        };
        let converted = match rebuilt_request {
            Some(request) => {
                let mut converted = chat_request_step_for_state(
                    state,
                    ChatRequestStepInput {
                        key: step.key,
                        title: step.title,
                        prompt_name: Some(step.prompt_name),
                        state: step.state,
                        note: step.note,
                        request,
                    },
                )
                .await?;
                converted.kind = "llm_chat".to_string();
                converted
            }
            None => ModelRequestPreviewStep {
                key: step.key,
                title: step.title,
                kind: "llm_chat".to_string(),
                prompt_name: Some(step.prompt_name),
                state: step.state,
                note: step.note,
                system_prompt: step.system_prompt,
                user_prompt: step.user_prompt,
                messages: step.messages,
                request: step.request,
                context_manifest: step.context_manifest,
                context_plan: step.context_plan,
                context_pack_refs: step.context_pack_refs,
                total_chars: step.total_chars,
                char_counts: step.char_counts,
            },
        };
        steps.push(converted);
    }
    Ok(crate::actions::request_preview::model_request_preview(
        preview.workflow,
        preview.title,
        steps,
    ))
}

pub(in crate::actions::video_prompts) async fn replan_video_prompt_request_preview_for_state(
    state: &AppState,
    preview: VideoPromptRequestPreview,
) -> Result<VideoPromptRequestPreview, String> {
    let mut steps = Vec::new();
    for step in preview.steps {
        let rebuilt_request = match step.request.as_ref() {
            Some(request) => Some(chat_request_from_preview_value(
                request,
                step.context_manifest.clone(),
            )?),
            None => None,
        };
        let replanned = match rebuilt_request {
            Some(request) => {
                let converted = chat_request_step_for_state(
                    state,
                    ChatRequestStepInput {
                        key: step.key,
                        title: step.title,
                        prompt_name: Some(step.prompt_name),
                        state: step.state,
                        note: step.note,
                        request,
                    },
                )
                .await?;
                VideoPromptRequestPreviewStep {
                    key: converted.key,
                    title: converted.title,
                    prompt_name: converted.prompt_name.unwrap_or_default(),
                    state: converted.state,
                    note: converted.note,
                    system_prompt: converted.system_prompt,
                    user_prompt: converted.user_prompt,
                    messages: converted.messages,
                    request: converted.request,
                    context_manifest: converted.context_manifest,
                    context_plan: converted.context_plan,
                    context_pack_refs: converted.context_pack_refs,
                    total_chars: converted.total_chars,
                    char_counts: converted.char_counts,
                }
            }
            None => step,
        };
        steps.push(replanned);
    }
    Ok(VideoPromptRequestPreview {
        workflow: preview.workflow,
        title: preview.title,
        steps,
    })
}
