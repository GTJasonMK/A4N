use super::plan::build_provider_prompt_plan;
use crate::actions::video_generation::ProviderPromptImageBinding;
use a4n_core::models::chapter::chapter_video_prompt;
use a4n_prompt::rendering::PromptRenderer;

pub(in crate::actions::video_generation) fn build_video_generation_provider_prompt_with_binding(
    prompt_renderer: &dyn PromptRenderer,
    prompt: &chapter_video_prompt::Model,
    image_binding: ProviderPromptImageBinding,
) -> Result<String, String> {
    build_video_generation_provider_prompt_inner(prompt_renderer, prompt, image_binding, false)
}

pub(in crate::actions::video_generation) fn build_video_generation_provider_preview_prompt_with_binding(
    prompt_renderer: &dyn PromptRenderer,
    prompt: &chapter_video_prompt::Model,
    image_binding: ProviderPromptImageBinding,
) -> Result<String, String> {
    build_video_generation_provider_prompt_inner(prompt_renderer, prompt, image_binding, true)
}

fn build_video_generation_provider_prompt_inner(
    prompt_renderer: &dyn PromptRenderer,
    prompt: &chapter_video_prompt::Model,
    image_binding: ProviderPromptImageBinding,
    allow_preview_placeholders: bool,
) -> Result<String, String> {
    let plan = build_provider_prompt_plan(prompt, image_binding, allow_preview_placeholders)?;
    futures::executor::block_on(prompt_renderer.render_prompt(
        plan.prompt_name,
        &plan.variables,
        Some(prompt.project_id),
    ))
    .map(|value| value.trim().to_string())
    .map_err(|err| {
        format!(
            "视频 provider 提示词模板 {} 渲染失败：{err}",
            plan.prompt_name
        )
    })
}
