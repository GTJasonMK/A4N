//! 参考媒体请求纯校验（无 DB、无 Host IO）。
//!
//! `Generate` 模式硬失败；`Preview` 模式返回占位与提示 note。

use a4n_core::{
    models::ai::video_gen_config,
    video_execution::{VideoReferenceMediaBindingV1, VideoReferenceMediaRecipeV1},
};
use a4n_video::{VideoGenerationRequest, VideoMediaInput};
use base64::Engine as _;
use sha2::{Digest, Sha256};

use super::{
    CLIPROXY_OPENAI_VIDEO_BACKEND, MAX_CLIPROXY_VIDEO_REFERENCE_IMAGES, SEEDANCE2_VIDEO_BACKEND,
    validate_video_duration_for_config_secs,
};

pub(crate) const MAX_SEEDANCE2_IMAGE_FILES: usize = 9;
pub(crate) const MAX_SEEDANCE2_VIDEO_FILES: usize = 3;
pub(crate) const MAX_SEEDANCE2_AUDIO_FILES: usize = 3;

pub use a4n_api_contract::video::ReferenceMediaVideoInput;

/// 生成 reservation 边界的纯输入校验；不触碰 DB、provider 或文件系统。
pub fn validate_reference_media_start_input(
    input: &ReferenceMediaVideoInput,
) -> Result<(), String> {
    validate_reference_media_uploads(&input.image_uploads, "image", MAX_SEEDANCE2_IMAGE_FILES)?;
    validate_reference_media_uploads(&input.video_uploads, "video", MAX_SEEDANCE2_VIDEO_FILES)?;
    validate_reference_media_uploads(&input.audio_uploads, "audio", MAX_SEEDANCE2_AUDIO_FILES)?;
    Ok(())
}

fn validate_reference_media_uploads(
    uploads: &[a4n_api_contract::video::ReferenceMediaUploadInput],
    kind: &str,
    max_count: usize,
) -> Result<(), String> {
    if uploads.len() > max_count {
        return Err(format!("{kind}_uploads 最多允许 {max_count} 个文件"));
    }
    for upload in uploads {
        if upload.file_name.trim().is_empty()
            || upload.mime_type.trim().is_empty()
            || upload.data_base64.trim().is_empty()
        {
            return Err(format!(
                "{kind}_uploads 的 file_name、mime_type 和 data_base64 不能为空"
            ));
        }
        if !upload
            .mime_type
            .trim()
            .to_ascii_lowercase()
            .starts_with(&format!("{kind}/"))
        {
            return Err(format!("{kind}_uploads 的 MIME 必须以 {kind}/ 开头"));
        }
    }
    Ok(())
}

pub(crate) struct PreparedReferenceMediaVideoRequest {
    pub(crate) request: VideoGenerationRequest,
    pub(crate) provider_input_note: Option<String>,
}

pub(crate) struct ResolvedReferenceMediaVideoInput {
    pub(crate) prompt: String,
    pub(crate) image_files: Vec<String>,
    pub(crate) video_files: Vec<String>,
    pub(crate) audio_files: Vec<String>,
    pub(crate) duration_secs: i32,
    pub(crate) aspect_ratio: String,
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

/// 画幅 → 参考媒体视频宽高。
pub fn reference_media_dimensions_for_aspect_ratio(
    aspect_ratio: &str,
) -> Result<(i32, i32), String> {
    reference_media_dimensions_for_short_edge(aspect_ratio, 720)
}

fn reference_media_dimensions_for_short_edge(
    aspect_ratio: &str,
    short_edge: i32,
) -> Result<(i32, i32), String> {
    if short_edge <= 0 {
        return Err("参考媒体视频分辨率短边必须大于 0".to_string());
    }
    let short_edge = even_dimension(short_edge);
    match aspect_ratio.trim() {
        "21:9" => Ok((scaled_even_dimension(short_edge, 21, 9)?, short_edge)),
        "16:9" => Ok((scaled_even_dimension(short_edge, 16, 9)?, short_edge)),
        "4:3" => Ok((scaled_even_dimension(short_edge, 4, 3)?, short_edge)),
        "1:1" => Ok((short_edge, short_edge)),
        "3:4" => Ok((short_edge, scaled_even_dimension(short_edge, 4, 3)?)),
        "9:16" => Ok((short_edge, scaled_even_dimension(short_edge, 16, 9)?)),
        other => Err(format!(
            "参考媒体视频模式不支持画幅 {other}，请选择 21:9、16:9、4:3、1:1、3:4 或 9:16"
        )),
    }
}

fn reference_media_dimensions_for_config(
    aspect_ratio: &str,
    config: &video_gen_config::Model,
) -> Result<(i32, i32), String> {
    reference_media_dimensions_for_short_edge(
        aspect_ratio,
        config.default_width.min(config.default_height),
    )
}

fn even_dimension(value: i32) -> i32 {
    value.max(2) & !1
}

fn scaled_even_dimension(short_edge: i32, numerator: i32, denominator: i32) -> Result<i32, String> {
    let rounded = (i64::from(short_edge) * i64::from(numerator) + i64::from(denominator / 2))
        / i64::from(denominator);
    i32::try_from(rounded)
        .map(even_dimension)
        .map_err(|_| "参考媒体视频分辨率超出 i32 范围".to_string())
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
                    "{field}[{index}] 必须以小写 http:// 或 https:// 开头，并且可被当前视频模型访问"
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

pub(crate) fn reference_media_prompt_from_recipe(recipe: &VideoReferenceMediaRecipeV1) -> String {
    let mut lines = Vec::new();
    for (prefix, rows) in [
        ("image_file", recipe.image_rows.as_slice()),
        ("video_file", recipe.video_rows.as_slice()),
        ("audio_file", recipe.audio_rows.as_slice()),
    ] {
        lines.extend(
            rows.iter()
                .enumerate()
                .map(|(index, row)| format!("- @{prefix}_{}：{}", index + 1, row.usage.trim())),
        );
    }
    let base = recipe.prompt.trim();
    if lines.is_empty() {
        return base.to_string();
    }
    if base.is_empty() {
        format!("参考媒体绑定：\n{}", lines.join("\n"))
    } else {
        format!("{base}\n\n参考媒体绑定：\n{}", lines.join("\n"))
    }
}

pub(crate) fn reference_media_upload_slot_count(rows: &[VideoReferenceMediaBindingV1]) -> usize {
    rows.iter().filter(|row| row.url.trim().is_empty()).count()
}

pub(crate) fn validate_reference_media_upload_identity(
    rows: &[VideoReferenceMediaBindingV1],
    uploads: &[a4n_api_contract::video::ReferenceMediaUploadInput],
    field: &str,
) -> Result<(), String> {
    let local_rows = rows
        .iter()
        .filter(|row| row.url.trim().is_empty())
        .collect::<Vec<_>>();
    if local_rows.len() != uploads.len() {
        return Err(format!(
            "{field} 需要 {} 个本地文件，实际提交 {} 个",
            local_rows.len(),
            uploads.len()
        ));
    }
    for (index, (row, upload)) in local_rows.into_iter().zip(uploads).enumerate() {
        let expected_sha256 = row
            .upload_sha256
            .as_deref()
            .map(str::trim)
            .filter(|value| value.len() == 64)
            .ok_or_else(|| format!("{field}[{index}] 缺少本地文件 SHA-256，请重新选择文件"))?;
        let expected_file_name = row
            .upload_file_name
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| format!("{field}[{index}] 缺少本地文件名，请重新选择文件"))?;
        let expected_mime = row
            .upload_mime_type
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| format!("{field}[{index}] 缺少本地文件 MIME，请重新选择文件"))?;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(upload.data_base64.trim())
            .map_err(|error| format!("{field}[{index}] data_base64 无效：{error}"))?;
        let actual_sha256 = format!("{:x}", Sha256::digest(bytes));
        if upload.file_name.trim() != expected_file_name
            || upload.mime_type.trim() != expected_mime
            || actual_sha256 != expected_sha256
        {
            return Err(format!(
                "{field}[{index}] 本地文件与保存片段方案时选择的文件不一致，请重新保存方案"
            ));
        }
    }
    Ok(())
}

pub(crate) fn resolve_reference_media_urls(
    rows: &[VideoReferenceMediaBindingV1],
    uploaded_urls: Vec<String>,
    field: &str,
    max_count: usize,
) -> Result<Vec<String>, String> {
    if rows.len() > max_count {
        return Err(format!("{field} 最多允许 {max_count} 个输入"));
    }
    if rows.is_empty() && !uploaded_urls.is_empty() {
        return Err(format!("{field} 没有本地上传槽，不能提交上传文件"));
    }
    let expected_uploads = reference_media_upload_slot_count(rows);
    if uploaded_urls.len() != expected_uploads {
        return Err(format!(
            "{field} 需要 {expected_uploads} 个本地文件，实际提交 {} 个；刷新页面后请重新选择本地文件",
            uploaded_urls.len()
        ));
    }
    let mut uploaded = uploaded_urls.into_iter();
    let mut urls = Vec::with_capacity(rows.len());
    for (index, row) in rows.iter().enumerate() {
        if row.usage.trim().is_empty() {
            return Err(format!("{field}[{index}] 缺少用途描述"));
        }
        let url = row.url.trim();
        if url.is_empty() {
            urls.push(
                uploaded
                    .next()
                    .expect("upload count was validated before resolving slots"),
            );
        } else {
            urls.push(url.to_string());
        }
    }
    Ok(urls)
}

pub(crate) fn build_reference_media_video_request(
    config: &video_gen_config::Model,
    input: ResolvedReferenceMediaVideoInput,
    mode: VideoProviderValidationMode,
) -> Result<PreparedReferenceMediaVideoRequest, String> {
    let ResolvedReferenceMediaVideoInput {
        prompt,
        image_files,
        video_files,
        audio_files,
        duration_secs,
        aspect_ratio,
    } = input;
    let mut provider_input_note = None;
    let uses_seedance = config.backend == SEEDANCE2_VIDEO_BACKEND;
    let uses_cliproxy_references = config.backend == CLIPROXY_OPENAI_VIDEO_BACKEND;
    if !uses_seedance && !uses_cliproxy_references {
        let message = format!(
            "参考媒体模式必须使用 {SEEDANCE2_VIDEO_BACKEND} 或 {CLIPROXY_OPENAI_VIDEO_BACKEND} 视频配置，当前配置 backend={}",
            config.backend
        );
        match mode {
            VideoProviderValidationMode::Generate => return Err(message),
            VideoProviderValidationMode::Preview => append_note(
                &mut provider_input_note,
                format!("{message}；此处仅展示参考媒体请求结构，真实生成仍需切换到支持的配置"),
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
    let max_image_files = if uses_cliproxy_references {
        MAX_CLIPROXY_VIDEO_REFERENCE_IMAGES
    } else {
        MAX_SEEDANCE2_IMAGE_FILES
    };
    let (mut image_files, note) =
        reference_media_urls(image_files, "image_files", max_image_files, mode)?;
    append_optional_note(&mut provider_input_note, note);
    let (video_files, note) =
        reference_media_urls(video_files, "video_files", MAX_SEEDANCE2_VIDEO_FILES, mode)?;
    append_optional_note(&mut provider_input_note, note);
    let (audio_files, note) =
        reference_media_urls(audio_files, "audio_files", MAX_SEEDANCE2_AUDIO_FILES, mode)?;
    append_optional_note(&mut provider_input_note, note);
    if uses_cliproxy_references && (!video_files.is_empty() || !audio_files.is_empty()) {
        append_optional_note(
            &mut provider_input_note,
            validation_error_or_preview_note(
                mode,
                format!(
                    "cliproxy-openai-video 参考媒体模式只支持图片，当前包含 {} 个视频和 {} 个音频",
                    video_files.len(),
                    audio_files.len()
                ),
                "；预览只会展示图片参考请求，真实生成会失败",
            )?,
        );
    }
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
    if uses_cliproxy_references && image_files.is_empty() {
        append_optional_note(
            &mut provider_input_note,
            validation_error_or_preview_note(
                mode,
                "cliproxy-openai-video 参考媒体模式至少需要一张图片".to_string(),
                "；预览使用占位图片展示请求结构，真实生成会失败",
            )?,
        );
        image_files.push(reference_media_preview_placeholder_url("image_files", 0));
    }
    if uses_cliproxy_references && image_files.len() > MAX_CLIPROXY_VIDEO_REFERENCE_IMAGES {
        image_files.truncate(MAX_CLIPROXY_VIDEO_REFERENCE_IMAGES);
    }
    for (token_prefix, count) in [
        ("image_file", image_files.len()),
        ("video_file", video_files.len()),
        ("audio_file", audio_files.len()),
    ] {
        let note = ensure_reference_media_tokens(&mut prompt, token_prefix, count, mode)?;
        append_optional_note(&mut provider_input_note, note);
    }
    let (width, height) = reference_media_dimensions_for_config(&aspect_ratio, config)?;
    if uses_cliproxy_references && (!video_files.is_empty() || !audio_files.is_empty()) {
        return Err("CLIProxy / Grok 视频参考模式不支持视频或音频素材".to_string());
    }
    Ok(PreparedReferenceMediaVideoRequest {
        request: VideoGenerationRequest {
            prompt,
            negative_prompt: None,
            width,
            height,
            duration_secs,
            fps: config.default_fps,
            model: config.model.clone(),
            media: VideoMediaInput::References {
                images: image_files,
                videos: video_files,
                audio: audio_files,
            },
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
    let VideoMediaInput::References {
        images,
        videos,
        audio,
    } = &request.media
    else {
        return Err("参考媒体请求要求 media.type=references".to_string());
    };
    let mut note = match config.backend.as_str() {
        SEEDANCE2_VIDEO_BACKEND => format!(
            "{source_label}；config_id={}，backend={}；Seedance 2.0 参考媒体模式使用 functionMode=omni_reference；image_files={}，video_files={}，audio_files={}；prompt 必须用 @image_file_N / @video_file_N / @audio_file_N 明确绑定每个媒体用途。",
            config.id,
            config.backend,
            images.len(),
            videos.len(),
            audio.len()
        ),
        CLIPROXY_OPENAI_VIDEO_BACKEND => format!(
            "{source_label}；config_id={}，backend={}；图片参考媒体按顺序映射为 provider reference_images；image_files={}，video_files=0，audio_files=0。",
            config.id,
            config.backend,
            images.len()
        ),
        backend => format!(
            "{source_label}；config_id={}，backend={backend}；当前后端不支持真实参考媒体生成，此请求仅用于预览结构。",
            config.id
        ),
    };
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
        assert_eq!(
            reference_media_dimensions_for_aspect_ratio("4:3").unwrap(),
            (960, 720)
        );
        assert_eq!(
            reference_media_dimensions_for_aspect_ratio("1:1").unwrap(),
            (720, 720)
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
    fn configured_full_hd_quality_scales_reference_media_dimensions() {
        let mut config = video_config(CLIPROXY_OPENAI_VIDEO_BACKEND);
        config.default_width = 1920;
        config.default_height = 1080;

        assert_eq!(
            reference_media_dimensions_for_config("16:9", &config).unwrap(),
            (1920, 1080)
        );
        assert_eq!(
            reference_media_dimensions_for_config("9:16", &config).unwrap(),
            (1080, 1920)
        );
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
        .err()
        .expect("cliproxy video/audio references must fail");
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
            ResolvedReferenceMediaVideoInput {
                prompt,
                image_files: vec!["https://example.test/ref.png".to_string()],
                video_files: Vec::new(),
                audio_files: Vec::new(),
                duration_secs: 8,
                aspect_ratio: "16:9".to_string(),
            },
            VideoProviderValidationMode::Generate,
        )
        .unwrap();
        assert_eq!(
            (prepared.request.width, prepared.request.height),
            (1280, 720)
        );
        let a4n_video::VideoMediaInput::References { images, .. } = &prepared.request.media else {
            panic!("expected reference media");
        };
        assert_eq!(images, &["https://example.test/ref.png"]);
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

    #[test]
    fn request_builder_maps_cliproxy_image_references() {
        let prompt = format!(
            "{} @image_file_1 只作为环境光、色调和写实质感参考。",
            "固定机位保持场景方向、动作、构图和节奏连续，".repeat(8)
        );
        let mut config = video_config(CLIPROXY_OPENAI_VIDEO_BACKEND);
        config.model = "bc/se-2.0".to_string();
        let prepared = build_reference_media_video_request(
            &config,
            ResolvedReferenceMediaVideoInput {
                prompt,
                image_files: vec!["https://example.test/ref.png".to_string()],
                video_files: Vec::new(),
                audio_files: Vec::new(),
                duration_secs: 8,
                aspect_ratio: "16:9".to_string(),
            },
            VideoProviderValidationMode::Generate,
        )
        .unwrap();

        assert_eq!(
            prepared.request.media.reference_images(),
            ["https://example.test/ref.png"]
        );
        let note = reference_media_provider_request_note(
            "当前表单",
            &config,
            &prepared.request,
            prepared.provider_input_note.as_deref(),
        )
        .unwrap();
        assert!(note.contains("reference_images"));
    }

    #[test]
    fn request_builder_rejects_cliproxy_video_or_audio_references() {
        let config = video_config(CLIPROXY_OPENAI_VIDEO_BACKEND);
        let error = match build_reference_media_video_request(
            &config,
            ResolvedReferenceMediaVideoInput {
                prompt: "保持人物、场景、动作、光影、构图和节奏连续。".repeat(12),
                image_files: vec!["https://example.test/ref.png".to_string()],
                video_files: vec!["https://example.test/ref.mp4".to_string()],
                audio_files: Vec::new(),
                duration_secs: 8,
                aspect_ratio: "16:9".to_string(),
            },
            VideoProviderValidationMode::Generate,
        ) {
            Ok(_) => panic!("cliproxy video/audio references must fail"),
            Err(error) => error,
        };

        assert!(error.contains("只支持图片"));
    }

    #[test]
    fn request_builder_uses_cliproxy_image_limit() {
        let config = video_config(CLIPROXY_OPENAI_VIDEO_BACKEND);
        let error = build_reference_media_video_request(
            &config,
            ResolvedReferenceMediaVideoInput {
                prompt: "保持人物、场景、动作、光影、构图和节奏连续。".repeat(12),
                image_files: (0..8)
                    .map(|index| format!("https://example.test/ref-{index}.png"))
                    .collect(),
                video_files: Vec::new(),
                audio_files: Vec::new(),
                duration_secs: 8,
                aspect_ratio: "16:9".to_string(),
            },
            VideoProviderValidationMode::Generate,
        )
        .err()
        .expect("cliproxy image count over the provider limit must fail");

        assert!(error.contains("最多允许 7 个 URL"));
    }
}
