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
        .start_inspiration_generation_task(user_id, &input)
        .await?;
    if !start.should_run {
        return Ok(start.handle);
    }

    let worker_state = state.clone();
    state.spawn_generation_execution(
        start.handle.clone(),
        "inspiration_generation",
        move |handle| async move {
            worker_state
                .orchestrator()
                .run_inspiration_generation_task(&worker_state, input, handle)
                .await
        },
    );
    Ok(start.handle)
}
