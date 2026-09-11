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
        .inspiration_generation_application()
        .start_generation(user_id, &input)
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
                .inspiration_generation_application()
                .run_generation(&worker_state, input, handle)
                .await
        },
    );
    Ok(start.handle)
}
