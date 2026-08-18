use a4n_api_contract::{generation_tasks::GenerationTaskHandle, video::StartVideoGenerationInput};
use a4n_orchestrator::GenerationTaskAccessError;

use crate::{AppState, actions::video_generation::provider_host::RuntimeVideoProviderHostPort};

pub async fn start_video_generation_typed(
    state: &AppState,
    user_id: i64,
    input: StartVideoGenerationInput,
) -> Result<GenerationTaskHandle, GenerationTaskAccessError> {
    let start = state
        .orchestrator()
        .start_video_generation_task(user_id, &input)
        .await?;
    if !start.should_run {
        return Ok(start.handle);
    }

    let worker_state = state.clone();
    if input.operation.is_prompt_generation() {
        state.spawn_generation_execution(
            start.handle.clone(),
            "video_prompt_generation",
            move |handle| async move {
                let runtime_video_config = crate::runtime_config::current_video_model();
                let prompt_renderer = &worker_state;
                worker_state
                    .orchestrator()
                    .run_video_prompt_generation_task(
                        user_id,
                        input,
                        handle,
                        a4n_orchestrator::video::VideoPromptApplicationContext {
                            runtime_video_config: runtime_video_config.as_ref(),
                            prompt_renderer: prompt_renderer,
                        },
                    )
                    .await
            },
        );
    } else {
        state.spawn_video_generation_execution(
            start.handle.clone(),
            "video_generation",
            move |handle| async move {
                let host_port = RuntimeVideoProviderHostPort::new(&worker_state);
                worker_state
                    .orchestrator()
                    .run_video_generation_task(user_id, input, handle, &host_port)
                    .await
            },
        );
    }
    Ok(start.handle)
}
