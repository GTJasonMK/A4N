use super::super::{ProposeOptimizationPatchesInput, preview::chat_step};
use crate::{
    AppState,
    actions::request_preview::{ModelRequestPreview, chat_status_preview_for_state},
};
use a4n_api_contract::{
    generation_tasks::GenerationTaskHandle, optimize::StartOptimizePatchGenerationInput,
};
use a4n_orchestrator::GenerationTaskAccessError;

pub async fn start_optimize_patch_generation_typed(
    state: &AppState,
    user_id: i64,
    input: StartOptimizePatchGenerationInput,
) -> Result<GenerationTaskHandle, GenerationTaskAccessError> {
    let start = state
        .optimize_patch_generation_application()
        .start_patch_generation(user_id, &input)
        .await?;
    if !start.should_run {
        return Ok(start.handle);
    }
    let worker_state = state.clone();
    state.spawn_generation_execution(
        start.handle.clone(),
        "optimize_patch",
        move |handle| async move {
            let prompt_renderer = &worker_state;
            worker_state
                .optimize_patch_generation_application()
                .run_patch_generation(prompt_renderer, user_id, input, handle)
                .await
        },
    );
    Ok(start.handle)
}

pub async fn preview_propose_optimization_patches_request(
    state: &AppState,
    user_id: i64,
    input: ProposeOptimizationPatchesInput,
) -> Result<ModelRequestPreview, String> {
    let prompt_renderer = state;
    let request = state
        .optimize_preview_application()
        .preview_propose_patches(user_id, input, prompt_renderer)
        .await
        .map_err(|err| err.to_string())?;
    chat_status_preview_for_state(
        state,
        "optimization_patch_generation",
        "内容优化局部补丁请求预览",
        vec![chat_step(
            "optimization_patch_generation",
            "局部补丁生成 LLM 请求",
            Some("content_optimization_patch"),
            "ready",
            None,
            request,
        )],
        "llm_chat",
    )
    .await
}
