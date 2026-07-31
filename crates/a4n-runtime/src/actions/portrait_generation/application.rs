use crate::{AppState, actions::access, prompt_renderer::RuntimePromptRenderer};
use a4n_api_contract::{
    generation_tasks::GenerationStartRequestMeta,
    portrait::{PortraitGenerationOperation, StartPortraitGenerationInput},
};
use a4n_orchestrator::portrait::{
    GeneratePortraitInput, GeneratePortraitPromptInput, PortraitSummary, UploadPortraitInput,
};
use a4n_service_protocol::GenerationTaskStatus;

use super::host_port::RuntimePortraitGenerationHostPort;

pub async fn generate_portrait_prompt(
    state: &AppState,
    user_id: i64,
    input: GeneratePortraitPromptInput,
) -> Result<String, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let variables =
        crate::actions::template_variables::parse_template_variables(input.variables.as_ref())?;
    let prompt_renderer = RuntimePromptRenderer::new(state);
    state
        .character()
        .generate_portrait_prompt_validated(input, &prompt_renderer, &variables)
        .await
        .map_err(|error| error.to_string())
}

pub(crate) async fn generate_portrait_inline(
    state: &AppState,
    user_id: i64,
    input: GeneratePortraitInput,
) -> Result<PortraitSummary, String> {
    let project_id = input.project_id;
    let input = StartPortraitGenerationInput {
        meta: GenerationStartRequestMeta {
            start_request_id: uuid::Uuid::new_v4().to_string(),
            retry_of_task_id: None,
        },
        project_id,
        operation: PortraitGenerationOperation::Generate {
            character_name: input.character_name,
            style: input.style,
            expressions: input.expressions,
            pose: input.pose,
            reference_image_ids: input.reference_image_ids,
        },
    };
    let start = state
        .portrait()
        .start_portrait_generation_task(user_id, input.clone())
        .await
        .map_err(|error| error.to_string())?;
    let prompt_renderer = RuntimePromptRenderer::new(state);
    let host_port = RuntimePortraitGenerationHostPort::new(state);
    if start.should_run {
        state
            .portrait()
            .run_portrait_generation_task(input, start.handle.clone(), &prompt_renderer, &host_port)
            .await?;
    }
    let task = state
        .generation_tasks()
        .task_summary(user_id, &start.handle.task_id, start.handle.attempt)
        .await
        .map_err(|error| error.to_string())?;
    if task.handle.status != GenerationTaskStatus::Completed {
        return Err(format!(
            "人物立绘 generation task 未完成：{:?}",
            task.handle.status
        ));
    }
    let result = task
        .result_ref
        .filter(|result| result.kind == "portrait")
        .ok_or_else(|| "人物立绘 generation task 缺少 portrait result_ref".to_string())?;
    let portrait_id = result.id.parse::<i64>().map_err(|_| {
        format!(
            "人物立绘 generation task 返回无效 portrait id：{}",
            result.id
        )
    })?;
    let portrait = state
        .character()
        .character_portrait_by_id(portrait_id)
        .await
        .map_err(|error| error.to_string())?
        .filter(|portrait| portrait.project_id == project_id)
        .ok_or_else(|| format!("项目 #{project_id} 不存在生成的立绘 #{portrait_id}"))?;
    Ok(super::to_summary(&portrait))
}

pub async fn upload_portrait(
    state: &AppState,
    user_id: i64,
    input: UploadPortraitInput,
) -> Result<PortraitSummary, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let host_port = RuntimePortraitGenerationHostPort::new(state);
    state
        .character()
        .upload_portrait_validated(input, &host_port)
        .await
        .map_err(|error| error.to_string())
}
