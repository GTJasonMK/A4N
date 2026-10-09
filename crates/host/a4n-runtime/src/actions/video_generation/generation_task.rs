use std::collections::HashMap;

use a4n_api_contract::{
    generation_tasks::{GenerationTaskHandle, GenerationTaskStartError},
    video::StartVideoGenerationInput,
};
use a4n_error::AppError;
use a4n_provider_contract::VideoProviderConfig;
use a4n_video_host_contract::VideoPromptGenerationHostPort;
use async_trait::async_trait;
use serde_json::Value;

use crate::{AppState, actions::video_generation::provider_host::RuntimeVideoProviderHostPort};

struct RuntimeVideoPromptGenerationHostPort<'state> {
    state: &'state AppState,
}

#[async_trait]
impl VideoPromptGenerationHostPort for RuntimeVideoPromptGenerationHostPort<'_> {
    async fn render_prompt(
        &self,
        template_name: &str,
        variables: &HashMap<String, Value>,
        project_id: Option<i64>,
    ) -> Result<String, AppError> {
        a4n_prompt::rendering::PromptRenderer::render_prompt(
            self.state,
            template_name,
            variables,
            project_id,
        )
        .await
    }

    async fn render_prompt_for_call(
        &self,
        call_key: &str,
        template_name: &str,
        variables: &HashMap<String, Value>,
        project_id: Option<i64>,
    ) -> Result<String, AppError> {
        a4n_prompt::rendering::PromptRenderer::render_prompt_for_call(
            self.state,
            call_key,
            template_name,
            variables,
            project_id,
        )
        .await
    }

    fn runtime_video_config(&self) -> Option<VideoProviderConfig> {
        crate::runtime_config::current_video_provider_config()
    }
}

pub async fn start_video_generation_typed(
    state: &AppState,
    user_id: i64,
    input: StartVideoGenerationInput,
) -> Result<GenerationTaskHandle, GenerationTaskStartError> {
    let start = state
        .video_generation_application()
        .start_task(user_id, &input)
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
                let prompt_host = RuntimeVideoPromptGenerationHostPort {
                    state: &worker_state,
                };
                worker_state
                    .video_generation_application()
                    .run_prompt_task(user_id, input, handle, &prompt_host)
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
                    .video_generation_application()
                    .run_task(user_id, input, handle, &host_port)
                    .await
            },
        );
    }
    Ok(start.handle)
}
