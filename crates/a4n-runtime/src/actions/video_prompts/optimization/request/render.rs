use super::{
    super::contracts::{
        CHAPTER_VIDEO_PROMPT_OPTIMIZATION_PROMPT_NAME,
        CHAPTER_VIDEO_PROMPT_OPTIMIZATION_USER_PROMPT_NAME,
    },
    builder::build_video_prompt_optimization_request_with_prompts,
    payload::build_video_prompt_optimization_dynamic_payload,
};
use crate::{AppState, actions::prompt_records::render_prompt_template};
use a4n_core::models::chapter::chapter_video_prompt;
use a4n_llm::provider::ChatRequest;
use a4n_orchestrator::video::PromptOutputLanguage;
#[cfg(test)]
use a4n_prompt::rendering::PromptRenderer;
#[cfg(test)]
use serde_json::Value;
use serde_json::json;
use std::collections::HashMap;

pub(in crate::actions::video_prompts::optimization) async fn build_video_prompt_optimization_request_for_state(
    state: &AppState,
    prompt: &chapter_video_prompt::Model,
    instruction: &str,
    prompt_output_language: PromptOutputLanguage,
) -> Result<ChatRequest, String> {
    let dynamic_payload = build_video_prompt_optimization_dynamic_payload(
        prompt,
        instruction,
        prompt_output_language,
    );
    let payload = serde_json::to_string_pretty(&dynamic_payload)
        .map_err(|err| format!("视频提示词优化请求 JSON 序列化失败：{err}"))?;
    let variables = HashMap::from([("payload".to_string(), json!(payload))]);
    let system_prompt = render_prompt_template(
        state,
        CHAPTER_VIDEO_PROMPT_OPTIMIZATION_PROMPT_NAME,
        variables.clone(),
        Some(prompt.project_id),
    )
    .await
    .map(|value| value.trim().to_string())
    .map_err(|err| {
        format!("视频提示词模板 {CHAPTER_VIDEO_PROMPT_OPTIMIZATION_PROMPT_NAME} 渲染失败：{err}")
    })?;
    let user_prompt = render_prompt_template(
        state,
        CHAPTER_VIDEO_PROMPT_OPTIMIZATION_USER_PROMPT_NAME,
        variables,
        Some(prompt.project_id),
    )
    .await
    .map(|value| value.trim().to_string())
    .map_err(|err| {
        format!(
            "视频提示词模板 {CHAPTER_VIDEO_PROMPT_OPTIMIZATION_USER_PROMPT_NAME} 渲染失败：{err}"
        )
    })?;
    build_video_prompt_optimization_request_with_prompts(
        system_prompt,
        user_prompt,
        prompt,
        instruction,
        prompt_output_language,
    )
}

#[cfg(test)]
pub(in crate::actions::video_prompts) fn build_video_prompt_optimization_request(
    prompt_renderer: &dyn PromptRenderer,
    prompt: &chapter_video_prompt::Model,
    instruction: &str,
    prompt_output_language: PromptOutputLanguage,
) -> Result<ChatRequest, String> {
    let dynamic_payload = build_video_prompt_optimization_dynamic_payload(
        prompt,
        instruction,
        prompt_output_language,
    );
    let payload = serde_json::to_string_pretty(&dynamic_payload)
        .map_err(|err| format!("视频提示词优化请求 JSON 序列化失败：{err}"))?;
    let variables = HashMap::from([("payload".to_string(), json!(payload))]);
    let system_prompt = render_video_prompt_template(
        prompt_renderer,
        CHAPTER_VIDEO_PROMPT_OPTIMIZATION_PROMPT_NAME,
        prompt.project_id,
        &variables,
    )?;
    let user_prompt = render_video_prompt_template(
        prompt_renderer,
        CHAPTER_VIDEO_PROMPT_OPTIMIZATION_USER_PROMPT_NAME,
        prompt.project_id,
        &variables,
    )?;
    build_video_prompt_optimization_request_with_prompts(
        system_prompt,
        user_prompt,
        prompt,
        instruction,
        prompt_output_language,
    )
}

#[cfg(test)]
fn render_video_prompt_template(
    prompt_renderer: &dyn PromptRenderer,
    prompt_name: &str,
    project_id: i64,
    variables: &HashMap<String, Value>,
) -> Result<String, String> {
    futures::executor::block_on(prompt_renderer.render_prompt(
        prompt_name,
        variables,
        Some(project_id),
    ))
    .map(|value| value.trim().to_string())
    .map_err(|err| format!("视频提示词模板 {prompt_name} 渲染失败：{err}"))
}
