use super::super::*;
use super::contracts::ReferenceMediaProviderRequestContext;
use super::rules::{
    ensure_reference_media_tokens, reference_media_dimensions_for_aspect_ratio,
    reference_media_preview_placeholder_url, reference_media_prompt, reference_media_urls,
};

pub(super) async fn build_reference_media_provider_request(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    input: ReferenceMediaVideoInput,
    purpose: VideoProviderRequestPurpose,
) -> Result<ReferenceMediaProviderRequestContext, String> {
    match purpose {
        VideoProviderRequestPurpose::Generate => {
            resolve_project_chapter_video_source(
                state,
                project_id,
                chapter_id,
                "生成参考媒体视频",
                true,
            )
            .await?;
        }
        VideoProviderRequestPurpose::Preview => {
            resolve_project_chapter_video_source(
                state,
                project_id,
                chapter_id,
                "预览参考媒体视频请求",
                false,
            )
            .await?;
        }
    }
    let cfg = resolve_video_config(state, input.config_id).await?;
    let mut provider_input_note = None;
    if cfg.backend != SEEDANCE2_VIDEO_BACKEND {
        let message = format!(
            "参考媒体模式必须使用 {SEEDANCE2_VIDEO_BACKEND} 视频配置，当前配置 backend={}",
            cfg.backend
        );
        if purpose == VideoProviderRequestPurpose::Generate {
            return Err(message);
        }
        append_provider_input_note(
            &mut provider_input_note,
            format!(
                "{message}；此处仅展示 Seedance 2.0 参考媒体请求结构，真实生成仍需切换到支持的配置"
            ),
        );
    }
    match purpose {
        VideoProviderRequestPurpose::Generate => {
            validate_video_duration_for_config_secs(&cfg, input.duration_secs)?;
        }
        VideoProviderRequestPurpose::Preview => {
            append_preview_duration_note(&mut provider_input_note, &cfg, input.duration_secs);
        }
    }
    let mut prompt = reference_media_prompt(&input.prompt, purpose, &mut provider_input_note)?;
    let mut image_files = reference_media_urls(
        input.image_files,
        "image_files",
        MAX_SEEDANCE2_IMAGE_FILES,
        purpose,
        &mut provider_input_note,
    )?;
    let video_files = reference_media_urls(
        input.video_files,
        "video_files",
        MAX_SEEDANCE2_VIDEO_FILES,
        purpose,
        &mut provider_input_note,
    )?;
    let audio_files = reference_media_urls(
        input.audio_files,
        "audio_files",
        MAX_SEEDANCE2_AUDIO_FILES,
        purpose,
        &mut provider_input_note,
    )?;
    if image_files.is_empty() && video_files.is_empty() && audio_files.is_empty() {
        if purpose == VideoProviderRequestPurpose::Generate {
            return Err(
                "参考媒体模式至少需要一个 image_files、video_files 或 audio_files URL".to_string(),
            );
        }
        append_provider_input_note(
            &mut provider_input_note,
            "参考媒体模式至少需要一个 image_files、video_files 或 audio_files URL；预览使用占位 image_files 展示请求结构，真实生成仍需填写可访问媒体 URL",
        );
        image_files.push(reference_media_preview_placeholder_url("image_files", 0));
    }
    for (token_prefix, count) in [
        ("image_file", image_files.len()),
        ("video_file", video_files.len()),
        ("audio_file", audio_files.len()),
    ] {
        ensure_reference_media_tokens(
            &mut prompt,
            token_prefix,
            count,
            purpose,
            &mut provider_input_note,
        )?;
    }
    let (width, height) = reference_media_dimensions_for_aspect_ratio(&input.aspect_ratio)?;
    let request = VideoGenerationRequest {
        prompt,
        negative_prompt: None,
        width,
        height,
        duration_secs: input.duration_secs,
        fps: cfg.default_fps,
        model: cfg.model.clone(),
        first_frame_image_url: None,
        last_frame_image_url: None,
        reference_image_urls: Vec::new(),
        reference_media: Some(VideoReferenceMediaRequest {
            image_files,
            video_files,
            audio_files,
        }),
    };
    Ok(ReferenceMediaProviderRequestContext {
        cfg,
        request,
        provider_input_note,
    })
}
