use crate::AppState;
use a4n_api_contract::generation_tasks::GenerationTaskHandle;
use a4n_api_contract::inspiration::StartInspirationGenerationInput;
use a4n_orchestrator::GenerationTaskAccessError;

pub async fn start_inspiration_generation_typed(
    state: &AppState,
    user_id: i64,
    input: StartInspirationGenerationInput,
) -> Result<GenerationTaskHandle, GenerationTaskAccessError> {
    let start = state
        .orchestrator()
        .start_inspiration_generation_task(user_id, input.clone())
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
            .run_inspiration_generation_task(
                a4n_orchestrator::inspiration::InspirationGenerationApplicationContext {
                    prompt_renderer: prompt_renderer,
                },
                input,
                handle.clone(),
            )
            .await
        {
            tracing::error!(
                task_id = %handle.task_id,
                attempt = handle.attempt,
                error = %error,
                "灵感 generation worker 失败"
            );
        }
    });
    Ok(start.handle)
}
