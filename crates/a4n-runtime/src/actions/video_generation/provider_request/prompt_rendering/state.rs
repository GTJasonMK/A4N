use super::plan::build_provider_prompt_plan;
use crate::AppState;
use crate::actions::prompt_records::render_prompt_template;
use crate::actions::video_generation::ProviderPromptImageBinding;
use a4n_core::models::chapter::chapter_video_prompt;

pub(in crate::actions::video_generation) async fn build_video_generation_provider_prompt_with_binding_for_state(
    state: &AppState,
    prompt: &chapter_video_prompt::Model,
    image_binding: ProviderPromptImageBinding,
) -> Result<String, String> {
    build_video_generation_provider_prompt_inner_for_state(state, prompt, image_binding, false)
        .await
}

pub(in crate::actions::video_generation) async fn build_video_generation_provider_preview_prompt_with_binding_for_state(
    state: &AppState,
    prompt: &chapter_video_prompt::Model,
    image_binding: ProviderPromptImageBinding,
) -> Result<String, String> {
    build_video_generation_provider_prompt_inner_for_state(state, prompt, image_binding, true).await
}

async fn build_video_generation_provider_prompt_inner_for_state(
    state: &AppState,
    prompt: &chapter_video_prompt::Model,
    image_binding: ProviderPromptImageBinding,
    allow_preview_placeholders: bool,
) -> Result<String, String> {
    let plan = build_provider_prompt_plan(prompt, image_binding, allow_preview_placeholders)?;
    render_prompt_template(
        state,
        plan.prompt_name,
        plan.variables,
        Some(prompt.project_id),
    )
    .await
    .map(|value| value.trim().to_string())
    .map_err(|err| {
        format!(
            "视频 provider 提示词模板 {} 渲染失败：{err}",
            plan.prompt_name
        )
    })
}
