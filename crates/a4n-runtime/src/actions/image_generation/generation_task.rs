use a4n_api_contract::generation_tasks::GenerationTaskHandle;
use a4n_api_contract::image_generation::StartImageGenerationInput;
use a4n_orchestrator::GenerationTaskAccessError;

use crate::{AppState, actions::image_generation::host_port::RuntimeImageGenerationHostPort};

pub async fn start_image_generation_typed(
    state: &AppState,
    user_id: i64,
    input: StartImageGenerationInput,
) -> Result<GenerationTaskHandle, GenerationTaskAccessError> {
    let start = state
        .image_generation_task_application()
        .start_task(user_id, &input)
        .await?;
    if !start.should_run {
        return Ok(start.handle);
    }

    let worker_state = state.clone();
    state.spawn_image_generation_execution(
        start.handle.clone(),
        "image_generation",
        move |handle| async move {
            let host_port = RuntimeImageGenerationHostPort::new(&worker_state);
            worker_state
                .image_generation_task_application()
                .run_task(user_id, input, handle, &host_port)
                .await
        },
    );
    Ok(start.handle)
}
