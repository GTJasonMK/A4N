//! Seedance 参考媒体请求纯校验（无 DB、无 Host IO）。
//!
//! `Generate` 模式硬失败；`Preview` 模式返回占位与提示 note。

use a4n_core::models::ai::video_gen_config;
use a4n_video::{VideoGenerationRequest, VideoReferenceMediaRequest};

use super::{SEEDANCE2_VIDEO_BACKEND, validate_video_duration_for_config_secs};

const MAX_SEEDANCE2_IMAGE_FILES: usize = 9;
const MAX_SEEDANCE2_VIDEO_FILES: usize = 3;
const MAX_SEEDANCE2_AUDIO_FILES: usize = 3;

pub use a4n_api_contract::video::ReferenceMediaVideoInput;

/// 生成 reservation 边界的纯输入校验；不触碰 DB、provider 或文件系统。
pub fn validate_reference_media_start_input(
    input: &ReferenceMediaVideoInput,
) -> Result<(), String> {
    reference_media_prompt(&input.prompt, VideoProviderValidationMode::Generate)?;
    reference_media_urls(
        input.image_files.clone(),
        "image_files",
        MAX_SEEDANCE2_IMAGE_FILES,
        VideoProviderValidationMode::Generate,
    )?;
    for (kind, upload_count, url_count, max_count) in [
        (
            "图片",
            input.image_uploads.len(),
            input.image_files.len(),
            MAX_SEEDANCE2_IMAGE_FILES,
        ),
        (
            "视频",
            input.video_uploads.len(),
            input.video_files.len(),
            MAX_SEEDANCE2_VIDEO_FILES,
        ),
        (
            "音频",
            input.audio_uploads.len(),
            input.audio_files.len(),
            MAX_SEEDANCE2_AUDIO_FILES,
        ),
    ] {
        if upload_count + url_count > max_count {
            return Err(format!("{kind}参考媒体最多允许 {max_count} 个输入"));
        }
    }
    for upload in input
        .image_uploads
        .iter()
        .chain(&input.video_uploads)
        .chain(&input.audio_uploads)
    {
        if upload.file_name.trim().is_empty()
            || upload.mime_type.trim().is_empty()
            || upload.data_base64.trim().is_empty()
        {
            return Err("本地参考媒体的 file_name、mime_type 和 data_base64 不能为空".to_string());
        }
    }
    reference_media_urls(
        input.video_files.clone(),
        "video_files",
        MAX_SEEDANCE2_VIDEO_FILES,
        VideoProviderValidationMode::Generate,
    )?;
    reference_media_urls(
        input.audio_files.clone(),
        "audio_files",
        MAX_SEEDANCE2_AUDIO_FILES,
        VideoProviderValidationMode::Generate,
    )?;
    validate_reference_media_import_options(input.duration_secs, &input.aspect_ratio)?;
    if input.image_files.is_empty()
        && input.image_uploads.is_empty()
        && input.video_files.is_empty()
        && input.video_uploads.is_empty()
        && input.audio_files.is_empty()
        && input.audio_uploads.is_empty()
    {
        return Err(
            "参考媒体模式至少需要一个 image_files、video_files 或 audio_files URL".to_string(),
        );
    }
    Ok(())
}

pub(crate) struct PreparedReferenceMediaVideoRequest {
    pub(crate) request: VideoGenerationRequest,
    pub(crate) provider_input_note: Option<String>,
}

/// 请求校验模式：生成硬失败 / 预览软化。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoProviderValidationMode {
    Generate,
    Preview,
}

/// 与 provider visual / 参考媒体 prompt 下限一致（单一来源）。
pub const MIN_PROVIDER_VISUAL_PROMPT_CHARS: usize = 100;

/// Generate 硬失败；Preview 返回可合并的 note。
pub fn validation_error_or_preview_note(
    mode: VideoProviderValidationMode,
    message: String,
    preview_suffix: &str,
) -> Result<Option<String>, String> {
    match mode {
        VideoProviderValidationMode::Generate => Err(message),
        VideoProviderValidationMode::Preview => Ok(Some(format!("{message}{preview_suffix}"))),
    }
}

/// 预览占位 prompt（仅结构展示）。
pub const REFERENCE_MEDIA_PREVIEW_PROMPT_PLACEHOLDER: &str = "【请求预览占位】当前参考媒体 prompt 未完整填写。真实生成前需要补全整体画面、动作、风格、镜头调度和每个参考媒体的用途；此占位仅用于展示将发送给视频模型的请求结构。";

/// 预览 URL 占位。
pub fn reference_media_preview_placeholder_url(field: &str, index: usize) -> String {
    format!("https://example.invalid/a4n-preview/{field}/{}", index + 1)
}

/// 画幅 → Seedance 2.0 参考媒体宽高。
pub fn reference_media_dimensions_for_aspect_ratio(
    aspect_ratio: &str,
) -> Result<(i32, i32), String> {
    match aspect_ratio.trim() {
        "21:9" => Ok((1680, 720)),
        "16:9" => Ok((1280, 720)),
        "4:3" => Ok((1024, 768)),
        "1:1" => Ok((1024, 1024)),
        "3:4" => Ok((768, 1024)),
        "9:16" => Ok((720, 1280)),
        other => Err(format!(
            "Seedance 2.0 参考媒体模式不支持画幅 {other}，请选择 21:9、16:9、4:3、1:1、3:4 或 9:16"
        )),
    }
}

/// 参考媒体导入选项（时长 + 画幅）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceMediaImportOptions {
    pub aspect_ratio: String,
}

/// 校验参考媒体导入时长（1..=60 秒）与画幅。
pub fn validate_reference_media_import_options(
    duration_secs: i32,
    aspect_ratio: &str,
) -> Result<ReferenceMediaImportOptions, String> {
    if !(1..=60).contains(&duration_secs) {
        return Err("参考媒体视频时长必须在 1 到 60 秒之间".to_string());
    }
    let aspect_ratio = aspect_ratio.trim();
    reference_media_dimensions_for_aspect_ratio(aspect_ratio)?;
    Ok(ReferenceMediaImportOptions {
        aspect_ratio: aspect_ratio.to_string(),
    })
}

/// 规范化参考媒体 prompt；Preview 时可能附带 note。
pub fn reference_media_prompt(
    value: &str,
    mode: VideoProviderValidationMode,
) -> Result<(String, Option<String>), String> {
    let prompt = value.trim();
    if prompt.chars().count() >= MIN_PROVIDER_VISUAL_PROMPT_CHARS {
        return Ok((prompt.to_string(), None));
    }
    match mode {
        VideoProviderValidationMode::Generate => Err(format!(
            "参考媒体视频 prompt 过短，至少需要 {} 个字符，并明确写出每个 @image_file_N / @video_file_N / @audio_file_N 的用途",
            MIN_PROVIDER_VISUAL_PROMPT_CHARS
        )),
        VideoProviderValidationMode::Preview => {
            let note = format!(
                "参考媒体视频 prompt 过短，至少需要 {} 个字符；预览使用占位 prompt 展示请求结构，真实生成仍需补全",
                MIN_PROVIDER_VISUAL_PROMPT_CHARS
            );
            let prompt = if prompt.is_empty() {
                REFERENCE_MEDIA_PREVIEW_PROMPT_PLACEHOLDER.to_string()
            } else {
                format!("{prompt}\n\n{REFERENCE_MEDIA_PREVIEW_PROMPT_PLACEHOLDER}")
            };
            Ok((prompt, Some(note)))
        }
    }
}

/// 规范化参考媒体 URL 列表。
pub fn reference_media_urls(
    values: Vec<String>,
    field: &str,
    max_count: usize,
    mode: VideoProviderValidationMode,
) -> Result<(Vec<String>, Option<String>), String> {
    let mut notes = Vec::new();
    if values.len() > max_count {
        match mode {
            VideoProviderValidationMode::Generate => {
                return Err(format!("{field} 最多允许 {max_count} 个 URL"));
            }
            VideoProviderValidationMode::Preview => {
                notes.push(format!(
                    "{field} 选择了 {} 个 URL，超过真实生成上限 {max_count} 个；预览仍展示请求结构，真实生成会失败",
                    values.len()
                ));
            }
        }
    }
    let mut normalized = Vec::with_capacity(values.len());
    for (index, value) in values.into_iter().enumerate() {
        let value = value.trim().to_string();
        if value.starts_with("https://") || value.starts_with("http://") {
            normalized.push(value);
            continue;
        }
        match mode {
            VideoProviderValidationMode::Generate => {
                if value.is_empty() {
                    return Err(format!("{field}[{index}] 不能为空"));
                }
                return Err(format!(
                    "{field}[{index}] 必须以小写 http:// 或 https:// 开头，并且可被 Seedance 2.0 服务访问"
                ));
            }
            VideoProviderValidationMode::Preview => {
                let reason = if value.is_empty() {
                    "为空"
                } else {
                    "未以小写 http:// 或 https:// 开头"
                };
                notes.push(format!(
                    "{field}[{index}] {reason}；预览使用占位 URL 展示请求结构，真实生成仍需填写以小写 http:// 或 https:// 开头且可访问的媒体 URL"
                ));
                normalized.push(reference_media_preview_placeholder_url(field, index));
            }
        }
    }
    let note = if notes.is_empty() {
        None
    } else {
        Some(notes.join("；"))
    };
    Ok((normalized, note))
}

/// 确保 prompt 显式引用每个绑定 token。
pub fn ensure_reference_media_tokens(
    prompt: &mut String,
    token_prefix: &str,
    count: usize,
    mode: VideoProviderValidationMode,
) -> Result<Option<String>, String> {
    let missing = (1..=count)
        .map(|index| format!("@{token_prefix}_{index}"))
        .filter(|token| !prompt.contains(token.as_str()))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        return Ok(None);
    }
    match mode {
        VideoProviderValidationMode::Generate => Err(format!(
            "参考媒体 prompt 缺少 {} 的用途描述；每个 {token_prefix} 条目都必须在 prompt 中显式引用",
            missing[0]
        )),
        VideoProviderValidationMode::Preview => {
            let note = format!(
                "参考媒体 prompt 缺少 {} 的用途描述；预览追加 token 占位说明，真实生成仍需显式引用每个媒体 token",
                missing.join("、")
            );
            prompt.push_str("\n\n参考媒体绑定占位：\n");
            for token in &missing {
                prompt.push_str(&format!(
                    "- {token}：请求预览占位用途；真实生成前需要填写该媒体的主体、细节和约束。\n"
                ));
            }
            Ok(Some(note))
        }
    }
}

pub(crate) fn build_reference_media_video_request(
    config: &video_gen_config::Model,
    input: ReferenceMediaVideoInput,
    mode: VideoProviderValidationMode,
) -> Result<PreparedReferenceMediaVideoRequest, String> {
    let ReferenceMediaVideoInput {
        segment_prompt_id: _,
        shot_number: _,
        prompt,
        image_files,
        image_uploads: _,
        video_files,
        video_uploads: _,
        audio_files,
        audio_uploads: _,
        duration_secs,
        aspect_ratio,
        config_id: _,
    } = input;
    let mut provider_input_note = None;
    if config.backend != SEEDANCE2_VIDEO_BACKEND {
        let message = format!(
            "参考媒体模式必须使用 {SEEDANCE2_VIDEO_BACKEND} 视频配置，当前配置 backend={}",
            config.backend
        );
        match mode {
            VideoProviderValidationMode::Generate => return Err(message),
            VideoProviderValidationMode::Preview => append_note(
                &mut provider_input_note,
                format!(
                    "{message}；此处仅展示 Seedance 2.0 参考媒体请求结构，真实生成仍需切换到支持的配置"
                ),
            ),
        }
    }
    match mode {
        VideoProviderValidationMode::Generate => {
            validate_video_duration_for_config_secs(config, duration_secs)?;
        }
        VideoProviderValidationMode::Preview => {
            if let Err(error) = validate_video_duration_for_config_secs(config, duration_secs) {
                append_note(
                    &mut provider_input_note,
                    format!(
                        "当前视频配置不支持 {duration_secs} 秒；预览仍展示请求结构，真实生成会失败：{error}"
                    ),
                );
            }
        }
    }
    let (mut prompt, note) = reference_media_prompt(&prompt, mode)?;
    append_optional_note(&mut provider_input_note, note);
    let (mut image_files, note) =
        reference_media_urls(image_files, "image_files", MAX_SEEDANCE2_IMAGE_FILES, mode)?;
    append_optional_note(&mut provider_input_note, note);
    let (video_files, note) =
        reference_media_urls(video_files, "video_files", MAX_SEEDANCE2_VIDEO_FILES, mode)?;
    append_optional_note(&mut provider_input_note, note);
    let (audio_files, note) =
        reference_media_urls(audio_files, "audio_files", MAX_SEEDANCE2_AUDIO_FILES, mode)?;
    append_optional_note(&mut provider_input_note, note);
    if image_files.is_empty() && video_files.is_empty() && audio_files.is_empty() {
        match mode {
            VideoProviderValidationMode::Generate => {
                return Err(
                    "参考媒体模式至少需要一个 image_files、video_files 或 audio_files URL"
                        .to_string(),
                );
            }
            VideoProviderValidationMode::Preview => {
                append_note(
                    &mut provider_input_note,
                    "参考媒体模式至少需要一个 image_files、video_files 或 audio_files URL；预览使用占位 image_files 展示请求结构，真实生成仍需填写可访问媒体 URL",
                );
                image_files.push(reference_media_preview_placeholder_url("image_files", 0));
            }
        }
    }
    for (token_prefix, count) in [
        ("image_file", image_files.len()),
        ("video_file", video_files.len()),
        ("audio_file", audio_files.len()),
    ] {
        let note = ensure_reference_media_tokens(&mut prompt, token_prefix, count, mode)?;
        append_optional_note(&mut provider_input_note, note);
    }
    let (width, height) = reference_media_dimensions_for_aspect_ratio(&aspect_ratio)?;
    Ok(PreparedReferenceMediaVideoRequest {
        request: VideoGenerationRequest {
            prompt,
            negative_prompt: None,
            width,
            height,
            duration_secs,
            fps: config.default_fps,
            model: config.model.clone(),
            first_frame_image_url: None,
            last_frame_image_url: None,
            reference_image_urls: Vec::new(),
            reference_media: Some(VideoReferenceMediaRequest {
                image_files,
                video_files,
                audio_files,
            }),
        },
        provider_input_note,
    })
}

pub(crate) fn reference_media_provider_request_note(
    source_label: &str,
    config: &video_gen_config::Model,
    request: &VideoGenerationRequest,
    provider_input_note: Option<&str>,
) -> Result<String, String> {
    let reference_media = request
        .reference_media
        .as_ref()
        .ok_or_else(|| "参考媒体请求缺少 reference_media".to_string())?;
    let mut note = format!(
        "{source_label}；config_id={}，backend={}；Seedance 2.0 参考媒体模式使用 functionMode=omni_reference；image_files={}，video_files={}，audio_files={}；prompt 必须用 @image_file_N / @video_file_N / @audio_file_N 明确绑定每个媒体用途。",
        config.id,
        config.backend,
        reference_media.image_files.len(),
        reference_media.video_files.len(),
        reference_media.audio_files.len()
    );
    if let Some(provider_input_note) = provider_input_note {
        note.push('；');
        note.push_str(provider_input_note);
    }
    Ok(note)
}

fn append_optional_note(note: &mut Option<String>, addition: Option<String>) {
    if let Some(addition) = addition {
        append_note(note, addition);
    }
}

fn append_note(note: &mut Option<String>, addition: impl AsRef<str>) {
    let addition = addition.as_ref().trim();
    if addition.is_empty() {
        return;
    }
    match note {
        Some(note) if !note.trim().is_empty() => {
            note.push('；');
            note.push_str(addition);
        }
        _ => *note = Some(addition.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn video_config(backend: &str) -> video_gen_config::Model {
        let now = chrono::Utc::now().naive_utc();
        video_gen_config::Model {
            id: 9,
            name: "reference media".to_string(),
            backend: backend.to_string(),
            base_url: Some("https://example.test".to_string()),
            api_key_encrypted: Some("key".to_string()),
            model: "seedance-2-0-pro".to_string(),
            max_concurrent: 1,
            default_width: 1280,
            default_height: 720,
            default_duration_secs: 8,
            min_duration_secs: 1,
            max_duration_secs: 60,
            duration_mode: "range".to_string(),
            supported_durations_json: None,
            default_fps: 24,
            provider_options_json: None,
            is_default: true,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn dimensions_and_placeholder() {
        assert_eq!(
            reference_media_dimensions_for_aspect_ratio("16:9").unwrap(),
            (1280, 720)
        );
        assert!(reference_media_dimensions_for_aspect_ratio("2:1").is_err());
        assert_eq!(
            reference_media_preview_placeholder_url("image_files", 0),
            "https://example.invalid/a4n-preview/image_files/1"
        );
        let opts = validate_reference_media_import_options(8, " 16:9 ").unwrap();
        assert_eq!(opts.aspect_ratio, "16:9");
        assert!(validate_reference_media_import_options(0, "16:9").is_err());
        assert!(validate_reference_media_import_options(8, "2:1").is_err());
    }

    #[test]
    fn generate_rejects_short_prompt() {
        let err = reference_media_prompt("短", VideoProviderValidationMode::Generate).unwrap_err();
        assert!(err.contains("prompt 过短"));
    }

    #[test]
    fn preview_pads_short_prompt() {
        let (prompt, note) =
            reference_media_prompt("", VideoProviderValidationMode::Preview).unwrap();
        assert!(prompt.contains(REFERENCE_MEDIA_PREVIEW_PROMPT_PLACEHOLDER));
        assert!(note.unwrap().contains("prompt 过短"));
    }

    #[test]
    fn urls_scheme_and_tokens() {
        let err = reference_media_urls(
            vec!["HTTPS://x".into()],
            "image_files",
            9,
            VideoProviderValidationMode::Generate,
        )
        .unwrap_err();
        assert!(err.contains("小写 http://"));

        let mut prompt = "足够长".repeat(40);
        let note = ensure_reference_media_tokens(
            &mut prompt,
            "image_file",
            1,
            VideoProviderValidationMode::Preview,
        )
        .unwrap()
        .unwrap();
        assert!(prompt.contains("@image_file_1"));
        assert!(note.contains("@image_file_1"));
    }

    #[test]
    fn request_builder_uses_one_seedance_reference_media_contract() {
        let prompt = format!(
            "{} @image_file_1 用作人物外观与服装一致性参考。",
            "镜头保持人物、场景、动作、光影、构图和节奏连续，".repeat(8)
        );
        let prepared = build_reference_media_video_request(
            &video_config(SEEDANCE2_VIDEO_BACKEND),
            ReferenceMediaVideoInput {
                segment_prompt_id: 1,
                shot_number: 1,
                prompt,
                image_files: vec!["https://example.test/ref.png".to_string()],
                image_uploads: Vec::new(),
                video_files: Vec::new(),
                video_uploads: Vec::new(),
                audio_files: Vec::new(),
                audio_uploads: Vec::new(),
                duration_secs: 8,
                aspect_ratio: "16:9".to_string(),
                config_id: Some(9),
            },
            VideoProviderValidationMode::Generate,
        )
        .unwrap();
        assert_eq!(
            (prepared.request.width, prepared.request.height),
            (1280, 720)
        );
        assert_eq!(
            prepared
                .request
                .reference_media
                .as_ref()
                .unwrap()
                .image_files,
            ["https://example.test/ref.png"]
        );
        let note = reference_media_provider_request_note(
            "当前表单",
            &video_config(SEEDANCE2_VIDEO_BACKEND),
            &prepared.request,
            prepared.provider_input_note.as_deref(),
        )
        .unwrap();
        assert!(note.contains("config_id=9"));
        assert!(note.contains("image_files=1"));
    }
}
