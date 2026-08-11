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
        .start_portrait_generation_task(user_id, input.clone())
        .await?;
    if !start.should_run {
        return Ok(start.handle);
    }

    let worker_state = state.clone();
    let handle = start.handle.clone();
    state.spawn_image_generation_execution(handle.task_id.clone(), async move {
        let prompt_renderer = &worker_state;
        let host_port = RuntimePortraitGenerationHostPort::new(&worker_state);
        if let Err(error) = worker_state
            .orchestrator()
            .run_portrait_generation_task(input, handle.clone(), prompt_renderer, &host_port)
            .await
        {
            tracing::error!(
                task_id = %handle.task_id,
                attempt = handle.attempt,
                error = %error,
                "人物立绘 generation worker 失败"
            );
        }
    });
    Ok(start.handle)
}
