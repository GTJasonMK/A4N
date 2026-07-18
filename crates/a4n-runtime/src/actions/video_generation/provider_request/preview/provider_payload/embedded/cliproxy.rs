use super::super::super::super::*;
use super::super::super::notes::{
    append_optional_input_note, append_video_request_prompt_length_note,
};

pub(super) fn cliproxy_preview_payload(
    context: &ChapterVideoProviderRequestContext,
) -> Result<(&'static str, &'static str, String, Value), String> {
    let request = CliProxyOpenAiVideoProvider::request_body_preview_with_options(
        &context.request,
        context.cfg.provider_options_json.as_ref(),
    )
    .map_err(|e| e.to_string())?;
    let grok_mode = cliproxy_grok_reference_mode(&context.cfg)?;
    let mut note = match grok_mode {
        CliproxyGrokReferenceMode::Reference => format!(
            "CLIProxy `/openai/v1/videos` 请求；Grok 输入模式：{}；所有输入图片均作为 reference_images 发送，最多 {} 张，当前绑定参考图：{}，实际发送参考图参数：{}；不会发送 input_reference 或尾帧图片槽；真实生成必须使用公网媒体 URL，auto 模式会优先使用已配置的 remote_temp_upload。",
            cliproxy_grok_reference_mode_label(grok_mode),
            MAX_CLIPROXY_VIDEO_REFERENCE_IMAGES,
            context.provider_reference_image_count,
            context.request.reference_image_urls.len()
        ),
        CliproxyGrokReferenceMode::FirstFrame => format!(
            "CLIProxy `/openai/v1/videos` 请求；Grok 输入模式：{}；首帧图片会作为 input_reference.image_url 发送：{}；尾帧图片不会作为 provider 图片参数发送：{}；当前绑定提示词参考图：{}，实际发送参考图参数：{}；真实生成必须使用公网媒体 URL，auto 模式会优先使用已配置的 remote_temp_upload。",
            cliproxy_grok_reference_mode_label(grok_mode),
            context.first_frame_image_url_available,
            context.last_frame_image_url_available,
            context.prompt_reference_image_count,
            context.request.reference_image_urls.len()
        ),
    };
    append_optional_input_note(&mut note, context.provider_input_note.as_deref());
    append_video_request_prompt_length_note(&mut note, &context.request);
    return Ok((
        "video_generation_cliproxy_openai",
        "CLIProxy OpenAI /videos 请求",
        note,
        request,
    ));
}
