use super::super::{ProposeOptimizationPatchesInput, preview::chat_step};
use crate::{
    AppState,
    actions::request_preview::{ModelRequestPreview, chat_status_preview_for_state},
};
use a4n_api_contract::{
    generation_tasks::GenerationTaskHandle, optimize::StartOptimizePatchGenerationInput,
};
use a4n_orchestrator::{
    GenerationTaskAccessError, optimize::OptimizePatchGenerationApplicationContext,
};

pub async fn start_optimize_patch_generation_typed(
    state: &AppState,
    user_id: i64,
    input: StartOptimizePatchGenerationInput,
) -> Result<GenerationTaskHandle, GenerationTaskAccessError> {
    let start = state
        .orchestrator()
        .start_optimize_patch_generation_task(user_id, input.clone())
        .await?;
    if !start.should_run {
        return Ok(start.handle);
    }
    let worker_state = state.clone();
    let handle = start.handle.clone();
    state.spawn_generation_execution(handle.task_id.clone(), async move {
        let prompt_renderer = &worker_state;
        if let Err(error) = worker_state
            .orchestrator()
            .run_optimize_patch_generation_task(
                OptimizePatchGenerationApplicationContext {
                    prompt_renderer: prompt_renderer,
                },
                user_id,
                input,
                handle.clone(),
            )
            .await
        {
            tracing::error!(
                task_id = %handle.task_id,
                attempt = handle.attempt,
                error = %error,
                "内容优化补丁 generation worker 失败"
            );
        }
    });
    Ok(start.handle)
}

pub async fn preview_propose_optimization_patches_request(
    state: &AppState,
    user_id: i64,
    input: ProposeOptimizationPatchesInput,
) -> Result<ModelRequestPreview, String> {
    let prompt_renderer = state;
    let request = state
        .orchestrator()
        .build_propose_patches_preview_request_for_user_validated(
            user_id,
            input.session_id,
            input.issue_ids,
            prompt_renderer,
            input.agent_instruction.as_deref(),
        )
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
