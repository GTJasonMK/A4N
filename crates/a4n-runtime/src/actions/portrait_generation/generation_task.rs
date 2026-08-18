use a4n_api_contract::generation_tasks::GenerationTaskHandle;
use a4n_api_contract::portrait::StartPortraitGenerationInput;
use a4n_orchestrator::GenerationTaskAccessError;

use crate::{AppState, actions::portrait_generation::host_port::RuntimePortraitGenerationHostPort};

pub async fn start_portrait_generation_typed(
    state: &AppState,
    user_id: i64,
    input: StartPortraitGenerationInput,
) -> Result<GenerationTaskHandle, GenerationTaskAccessError> {
    let start = state
        .orchestrator()
        .start_portrait_generation_task(user_id, &input)
        .await?;
    if !start.should_run {
        return Ok(start.handle);
    }

    let worker_state = state.clone();
    state.spawn_image_generation_execution(
        start.handle.clone(),
        "portrait_generation",
        move |handle| async move {
            let prompt_renderer = &worker_state;
            let host_port = RuntimePortraitGenerationHostPort::new(&worker_state);
            worker_state
                .orchestrator()
                .run_portrait_generation_task(input, handle, prompt_renderer, &host_port)
                .await
        },
    );
    Ok(start.handle)
}
