use super::super::{CliproxyGrokReferenceMode, ProviderGenerationWorkflow};
use super::reject_generation_or_note_preview;
use crate::actions::video_generation::{
    CLIPROXY_OPENAI_VIDEO_BACKEND, MAX_CLIPROXY_VIDEO_PROMPT_CHARS, NEWAPI_VIDEO_BACKEND,
    VideoProviderRequestPurpose,
};

pub(in crate::actions::video_generation) fn enforce_provider_prompt_length_contract(
    backend: &str,
    prompt: &str,
    purpose: VideoProviderRequestPurpose,
    note: &mut Option<String>,
) -> Result<(), String> {
    if backend.trim() != CLIPROXY_OPENAI_VIDEO_BACKEND {
        return Ok(());
    }
    let prompt_chars = prompt.chars().count();
    let prompt_bytes = prompt.len();
    if prompt_chars <= MAX_CLIPROXY_VIDEO_PROMPT_CHARS {
        return Ok(());
    }
    let message = format!(
        "CLIProxy/Grok 视频 prompt 超过 {MAX_CLIPROXY_VIDEO_PROMPT_CHARS} 字符限制：实际 {prompt_chars} 字符 / {prompt_bytes} UTF-8 字节；请缩短 Visual/Motion Prompt 后再生成"
    );
    reject_generation_or_note_preview(purpose, note, message, " 当前仅展示请求结构。")
}

pub(in crate::actions::video_generation) fn enforce_provider_image_binding_contract(
    backend: &str,
    workflow: ProviderGenerationWorkflow,
    grok_mode: CliproxyGrokReferenceMode,
    prompt_reference_image_count: usize,
    purpose: VideoProviderRequestPurpose,
    note: &mut Option<String>,
) -> Result<(), String> {
    let backend = backend.trim();
    if backend == NEWAPI_VIDEO_BACKEND && workflow == ProviderGenerationWorkflow::Storyboard {
        reject_generation_or_note_preview(
            purpose,
            note,
            "New API 视频接口不支持 reference_images，不能用于分镜模式；请切换到支持参考图输入的视频 provider。"
                .to_string(),
            " 当前仅展示请求结构。",
        )?;
    } else if backend == NEWAPI_VIDEO_BACKEND && !workflow.uses_first_frame_chain() {
        reject_generation_or_note_preview(
            purpose,
            note,
            "New API 视频接口只有一个首帧图片输入槽，只能作为视频起始帧使用；不能用于首尾帧模式，否则尾帧图片会被静默丢弃。请改用首帧串联模式，或切换到支持尾帧/参考图输入的视频 provider。"
                .to_string(),
            " 当前仅展示请求结构。",
        )?;
    }
    if backend == CLIPROXY_OPENAI_VIDEO_BACKEND
        && grok_mode == CliproxyGrokReferenceMode::FirstFrame
        && workflow == ProviderGenerationWorkflow::Storyboard
    {
        reject_generation_or_note_preview(
            purpose,
            note,
            "CLIProxy / Grok 视频接口当前是首帧模式，只有一个 input_reference 输入槽，不能用于分镜模式；请切换到 Grok 参考模式，或切换到支持 reference_images 的视频 provider。"
                .to_string(),
            " 当前仅展示请求结构。",
        )?;
    } else if backend == CLIPROXY_OPENAI_VIDEO_BACKEND
        && grok_mode == CliproxyGrokReferenceMode::FirstFrame
        && !workflow.uses_first_frame_chain()
    {
        reject_generation_or_note_preview(
            purpose,
            note,
            "CLIProxy / Grok 视频接口当前是首帧模式，只有一个 input_reference 输入槽，只能作为视频起始帧使用；不能用于首尾帧模式，否则尾帧图片会被静默丢弃。请改用首帧串联模式，切换到 Grok 参考模式，或切换到支持尾帧输入的视频 provider。"
                .to_string(),
            " 当前仅展示请求结构。",
        )?;
    }
    if backend != NEWAPI_VIDEO_BACKEND {
        return Ok(());
    }
    if prompt_reference_image_count > 0 {
        reject_generation_or_note_preview(
            purpose,
            note,
            format!(
                "New API 视频接口不支持提示词参考图输入；当前视频提示词绑定了 {prompt_reference_image_count} 张参考图。已阻止生成以避免参考图被丢弃，请移除参考图，或切换到支持参考图输入的视频 provider。"
            ),
            " 当前仅展示请求结构。",
        )?;
    }
    Ok(())
}

pub(in crate::actions::video_generation) fn ensure_newapi_first_frame_image_supported(
    image_url: Option<&str>,
) -> Result<(), String> {
    let Some(image_url) = image_url.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(());
    };
    let Some(mime) = data_url_mime(image_url)? else {
        return Ok(());
    };
    match mime.as_str() {
        "image/png" | "image/jpeg" | "image/jpg" => Ok(()),
        other => Err(format!(
            "New API 视频首帧 image 仅支持 PNG/JPEG data URL，当前首帧是 {other}；请先使用 PNG/JPEG 图片作为首帧。素材参考图、WebP/GIF 或参考板不能放进 New API 的首帧 image 字段"
        )),
    }
}

fn data_url_mime(value: &str) -> Result<Option<String>, String> {
    let value = value.trim();
    let Some(rest) = value.strip_prefix("data:") else {
        return Ok(None);
    };
    let Some((mime, _)) = rest.split_once(";base64,") else {
        return Err(
            "New API 视频首帧 image data URL 格式无效，必须是 data:image/png;base64,... 或 data:image/jpeg;base64,..."
                .to_string(),
        );
    };
    let mime = mime.trim().to_ascii_lowercase();
    if mime.is_empty() {
        return Err("New API 视频首帧 image data URL 缺少 MIME 类型".to_string());
    }
    Ok(Some(mime))
}
