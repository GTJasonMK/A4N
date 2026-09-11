use a4n_api_contract::generation_tasks::GenerationTaskHandle;
use a4n_api_contract::outline::StartOutlineGenerationInput;
use a4n_orchestrator::GenerationTaskAccessError;

use crate::AppState;

pub async fn start_outline_generation_typed(
    state: &AppState,
    user_id: i64,
    input: StartOutlineGenerationInput,
) -> Result<GenerationTaskHandle, GenerationTaskAccessError> {
    let start = state
        .outline_generation_application()
        .start_task(user_id, &input)
        .await?;
    if !start.should_run {
        return Ok(start.handle);
    }

    let worker_state = state.clone();
    state.spawn_generation_execution(
        start.handle.clone(),
        "outline_generation",
        move |handle| async move {
            worker_state
                .outline_generation_application()
                .run_task(input, handle)
                .await
        },
    );
    Ok(start.handle)
}
