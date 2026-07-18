use super::super::CliproxyGrokReferenceMode;
use super::reject_generation_or_note_preview;
use crate::actions::video_generation::{
    CLIPROXY_OPENAI_VIDEO_BACKEND, MAX_CLIPROXY_VIDEO_REFERENCE_IMAGES, VideoProviderRequestPurpose,
};

pub(in crate::actions::video_generation) fn enforce_cliproxy_video_media_contract(
    backend: &str,
    model: &str,
    grok_mode: CliproxyGrokReferenceMode,
    bound_reference_image_count: usize,
    has_first_frame_image: bool,
    duration_secs: i32,
    purpose: VideoProviderRequestPurpose,
    note: &mut Option<String>,
) -> Result<(), String> {
    if backend.trim() != CLIPROXY_OPENAI_VIDEO_BACKEND {
        return Ok(());
    }
    if grok_mode == CliproxyGrokReferenceMode::FirstFrame {
        return Ok(());
    }
    if bound_reference_image_count > 0 && !cliproxy_grok_model_supports_reference_images(model) {
        let message = format!(
            "CLIProxy / Grok 视频模型 {model} 不支持 reference_images；当前请求需要发送 {bound_reference_image_count} 张参考图。请改用 grok-imagine-video，或切换为 Grok 首帧模式并使用首帧串联流程。"
        );
        reject_generation_or_note_preview(purpose, note, message, " 当前仅展示请求结构。")?;
    }
    if has_first_frame_image && bound_reference_image_count > 0 {
        let message = format!(
            "CLIProxy / Grok 视频使用首帧 input_reference 时不能同时发送 reference_images；当前请求还绑定了 {bound_reference_image_count} 张参考图。请移除参考图，或切换到支持首帧加参考图的视频 provider。"
        );
        reject_generation_or_note_preview(purpose, note, message, " 当前仅展示请求结构。")?;
    }
    if bound_reference_image_count > MAX_CLIPROXY_VIDEO_REFERENCE_IMAGES {
        let message = format!(
            "CLIProxy / Grok 视频最多支持 {MAX_CLIPROXY_VIDEO_REFERENCE_IMAGES} 张参考图，当前 {bound_reference_image_count} 张"
        );
        reject_generation_or_note_preview(purpose, note, message, "；真实生成会拒绝该输入。")?;
    }
    if bound_reference_image_count > 0 && duration_secs > 10 {
        let message =
            "CLIProxy / Grok 视频使用参考图时最大时长为 10 秒，请降低时长或移除参考图".to_string();
        reject_generation_or_note_preview(purpose, note, message, "；真实生成会拒绝该输入。")?;
    }
    Ok(())
}

fn cliproxy_grok_model_supports_reference_images(model: &str) -> bool {
    !cliproxy_grok_model_is_image_to_video_only(model)
}

fn cliproxy_grok_model_is_image_to_video_only(model: &str) -> bool {
    matches!(
        cliproxy_grok_model_base(model).as_deref(),
        Some("grok-imagine-video-1.5")
            | Some("grok-imagine-video-1.5-preview")
            | Some("grok-imagine-video-1.5-2026-05-30")
    )
}

fn cliproxy_grok_model_base(model: &str) -> Option<String> {
    let model = model.trim().to_ascii_lowercase();
    if model.is_empty() {
        return None;
    }
    Some(
        model
            .rsplit('/')
            .next()
            .unwrap_or(model.as_str())
            .trim()
            .to_string(),
    )
}
