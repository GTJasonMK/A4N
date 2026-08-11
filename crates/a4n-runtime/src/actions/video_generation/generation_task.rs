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
        .start_video_generation_task(user_id, input.clone())
        .await?;
    if !start.should_run {
        return Ok(start.handle);
    }

    let handle = start.handle.clone();
    let worker_state = state.clone();
    if input.operation.is_prompt_generation() {
        state.spawn_generation_execution(handle.task_id.clone(), async move {
            let runtime_video_config = crate::runtime_config::current_video_model();
            let prompt_renderer = &worker_state;
            if let Err(error) = worker_state
                .orchestrator()
                .run_video_prompt_generation_task(
                    user_id,
                    input,
                    handle.clone(),
                    a4n_orchestrator::video::VideoPromptApplicationContext {
                        runtime_video_config: runtime_video_config.as_ref(),
                        prompt_renderer: prompt_renderer,
                    },
                )
                .await
            {
                tracing::error!(
                    task_id = %handle.task_id,
                    attempt = handle.attempt,
                    error = %error,
                    "视频提示词 generation worker 失败"
                );
            }
        });
    } else {
        state.spawn_video_generation_execution(handle.task_id.clone(), async move {
            let host_port = RuntimeVideoProviderHostPort::new(&worker_state);
            if let Err(error) = worker_state
                .orchestrator()
                .run_video_generation_task(user_id, input, handle.clone(), &host_port)
                .await
            {
                tracing::error!(
                    task_id = %handle.task_id,
                    attempt = handle.attempt,
                    error = %error,
                    "视频 generation worker 失败"
                );
            }
        });
    }
    Ok(start.handle)
}
