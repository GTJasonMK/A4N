//! 参考媒体请求纯校验（无 DB、无 Host IO）。

use a4n_domain::VideoAspectRatio;
use a4n_provider_contract::VideoProviderConfig;
use a4n_video::{VideoGenerationRequest, VideoMediaInput};
use a4n_video_contract::{VideoReferenceMediaBindingV1, VideoReferenceMediaRecipeV1};
use base64::Engine as _;
use sha2::{Digest, Sha256};

use super::{
    CLIPROXY_OPENAI_VIDEO_BACKEND, MAX_CLIPROXY_VIDEO_REFERENCE_IMAGES, SEEDANCE2_VIDEO_BACKEND,
    validate_video_duration_for_provider_config_secs,
};

pub(crate) const MAX_SEEDANCE2_IMAGE_FILES: usize = 9;
pub(crate) const MAX_SEEDANCE2_VIDEO_FILES: usize = 3;
pub(crate) const MAX_SEEDANCE2_AUDIO_FILES: usize = 3;

use a4n_api_contract::video::ReferenceMediaVideoInput;

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

pub(crate) struct ResolvedReferenceMediaVideoInput {
    pub(crate) prompt: String,
    pub(crate) image_files: Vec<String>,
    pub(crate) video_files: Vec<String>,
    pub(crate) audio_files: Vec<String>,
    pub(crate) duration_secs: i32,
    pub(crate) aspect_ratio: String,
}

/// 与 provider visual / 参考媒体 prompt 下限一致（单一来源）。
pub const MIN_PROVIDER_VISUAL_PROMPT_CHARS: usize = 100;

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
    let ratio = aspect_ratio.parse::<VideoAspectRatio>().map_err(|_| {
        format!(
            "参考媒体视频模式不支持画幅 {aspect_ratio}，请选择 {}",
            VideoAspectRatio::supported_values()
        )
    })?;
    let (ratio_width, ratio_height) = ratio.components();
    if ratio_width >= ratio_height {
        Ok((
            scaled_even_dimension(short_edge, ratio_width, ratio_height)?,
            short_edge,
        ))
    } else {
        Ok((
            short_edge,
            scaled_even_dimension(short_edge, ratio_height, ratio_width)?,
        ))
    }
}

fn reference_media_dimensions_for_config(
    aspect_ratio: &str,
    config: &VideoProviderConfig,
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

/// 校验参考媒体导入时长（1..=60 秒）与画幅。
pub fn validate_reference_media_import_options(
    duration_secs: i32,
    aspect_ratio: &str,
) -> Result<(), String> {
    if !(1..=60).contains(&duration_secs) {
        return Err("参考媒体视频时长必须在 1 到 60 秒之间".to_string());
    }
    reference_media_dimensions_for_aspect_ratio(aspect_ratio).map(|_| ())
}

fn reference_media_prompt(value: &str) -> Result<String, String> {
    let prompt = value.trim();
    if prompt.chars().count() >= MIN_PROVIDER_VISUAL_PROMPT_CHARS {
        return Ok(prompt.to_string());
    }
    Err(format!(
        "参考媒体视频 prompt 过短，至少需要 {} 个字符，并明确写出每个参考媒体的用途",
        MIN_PROVIDER_VISUAL_PROMPT_CHARS
    ))
}

fn reference_media_urls(
    values: Vec<String>,
    field: &str,
    max_count: usize,
) -> Result<Vec<String>, String> {
    if values.len() > max_count {
        return Err(format!("{field} 最多允许 {max_count} 个 URL"));
    }
    let mut normalized = Vec::with_capacity(values.len());
    for (index, value) in values.into_iter().enumerate() {
        let value = value.trim().to_string();
        if value.starts_with("https://") || value.starts_with("http://") {
            normalized.push(value);
            continue;
        }
        if value.is_empty() {
            return Err(format!("{field}[{index}] 不能为空"));
        }
        return Err(format!(
            "{field}[{index}] 必须以小写 http:// 或 https:// 开头，并且可被当前视频模型访问"
        ));
    }
    Ok(normalized)
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

pub(crate) struct ValidatedReferenceMediaUpload {
    pub(crate) file_name: String,
    pub(crate) mime_type: String,
    pub(crate) bytes: Vec<u8>,
    pub(crate) sha256: String,
}

pub(crate) fn validated_reference_media_uploads(
    rows: &[VideoReferenceMediaBindingV1],
    uploads: &[a4n_api_contract::video::ReferenceMediaUploadInput],
    field: &str,
) -> Result<Vec<ValidatedReferenceMediaUpload>, String> {
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
    let mut validated = Vec::with_capacity(uploads.len());
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
        let actual_sha256 = format!("{:x}", Sha256::digest(&bytes));
        if upload.file_name.trim() != expected_file_name
            || upload.mime_type.trim() != expected_mime
            || actual_sha256 != expected_sha256
        {
            return Err(format!(
                "{field}[{index}] 本地文件与保存片段方案时选择的文件不一致，请重新保存方案"
            ));
        }
        validated.push(ValidatedReferenceMediaUpload {
            file_name: expected_file_name.to_string(),
            mime_type: expected_mime.to_string(),
            bytes,
            sha256: actual_sha256,
        });
    }
    Ok(validated)
}

pub(crate) fn resolve_reference_media_urls(
    rows: &[VideoReferenceMediaBindingV1],
    uploaded_urls: Vec<String>,
    field: &str,
) -> Result<Vec<String>, String> {
    if rows.is_empty() && !uploaded_urls.is_empty() {
        return Err(format!("{field} 没有本地上传槽，不能提交上传文件"));
    }
    let expected_uploads = rows.iter().filter(|row| row.url.trim().is_empty()).count();
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
    config: &VideoProviderConfig,
    input: ResolvedReferenceMediaVideoInput,
) -> Result<VideoGenerationRequest, String> {
    let ResolvedReferenceMediaVideoInput {
        prompt,
        image_files,
        video_files,
        audio_files,
        duration_secs,
        aspect_ratio,
    } = input;
    let (max_image_files, image_only) = match config.backend.as_str() {
        SEEDANCE2_VIDEO_BACKEND => (MAX_SEEDANCE2_IMAGE_FILES, false),
        CLIPROXY_OPENAI_VIDEO_BACKEND => (MAX_CLIPROXY_VIDEO_REFERENCE_IMAGES, true),
        backend => {
            return Err(format!(
                "参考媒体模式必须使用 {SEEDANCE2_VIDEO_BACKEND} 或 {CLIPROXY_OPENAI_VIDEO_BACKEND} 视频配置，当前配置 backend={backend}"
            ));
        }
    };
    validate_video_duration_for_provider_config_secs(config, duration_secs)?;
    let prompt = reference_media_prompt(&prompt)?;
    let image_files = reference_media_urls(image_files, "image_files", max_image_files)?;
    let video_files = reference_media_urls(video_files, "video_files", MAX_SEEDANCE2_VIDEO_FILES)?;
    let audio_files = reference_media_urls(audio_files, "audio_files", MAX_SEEDANCE2_AUDIO_FILES)?;
    if image_files.is_empty() && video_files.is_empty() && audio_files.is_empty() {
        return Err(
            "参考媒体模式至少需要一个 image_files、video_files 或 audio_files URL".to_string(),
        );
    }
    if image_only && (!video_files.is_empty() || !audio_files.is_empty()) {
        return Err("CLIProxy / Grok 视频参考模式不支持视频或音频素材".to_string());
    }
    let (width, height) = reference_media_dimensions_for_config(&aspect_ratio, config)?;
    Ok(VideoGenerationRequest {
        prompt,
        negative_prompt: None,
        width,
        height,
        duration_secs,
        fps: config.default_fps,
        model: config.model.clone(),
        media: VideoMediaInput::References {
            images: image_files.into_iter().map(Into::into).collect(),
            videos: video_files,
            audio: audio_files,
        },
    })
}

pub(crate) fn reference_media_provider_request_note(
    source_label: &str,
    config: &VideoProviderConfig,
    request: &VideoGenerationRequest,
) -> Result<String, String> {
    let VideoMediaInput::References {
        images,
        videos,
        audio,
    } = &request.media
    else {
        return Err("参考媒体请求要求 media.type=references".to_string());
    };
    match config.backend.as_str() {
        SEEDANCE2_VIDEO_BACKEND => Ok(format!(
            "{source_label}；config_id={}，backend={}；Seedance 2.0 使用 /videos 的 input_reference；image_files={}，video_files={}，audio_files={}；参考媒体按输入顺序发送，并在 prompt 中明确每个媒体用途。",
            config.id,
            config.backend,
            images.len(),
            videos.len(),
            audio.len()
        )),
        CLIPROXY_OPENAI_VIDEO_BACKEND => Ok(format!(
            "{source_label}；config_id={}，backend={}；图片参考媒体按顺序映射为 provider reference_images；image_files={}，video_files=0，audio_files=0。",
            config.id,
            config.backend,
            images.len()
        )),
        backend => Err(format!("参考媒体请求不支持 backend={backend}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn video_config(backend: &str) -> VideoProviderConfig {
        let (min_duration_secs, max_duration_secs) = match backend {
            SEEDANCE2_VIDEO_BACKEND => (4, 15),
            CLIPROXY_OPENAI_VIDEO_BACKEND => (1, 15),
            _ => (1, 60),
        };
        VideoProviderConfig {
            id: 9,
            name: "reference media".to_string(),
            backend: backend.to_string(),
            base_url: Some("https://example.test".to_string()),
            api_key: Some("key".to_string()),
            model: "seedance-2-0-pro".to_string(),
            max_concurrent: 1,
            default_width: 1280,
            default_height: 720,
            default_duration_secs: 8,
            min_duration_secs,
            max_duration_secs,
            duration_mode: "range".to_string(),
            supported_durations_json: None,
            default_fps: 24,
            is_default: true,
            pricing: Default::default(),
        }
    }

    #[test]
    fn dimensions_and_import_options() {
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
        validate_reference_media_import_options(8, "16:9").unwrap();
        assert!(validate_reference_media_import_options(8, " 16:9 ").is_err());
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
    fn rejects_short_prompt() {
        let err = reference_media_prompt("短").unwrap_err();
        assert!(err.contains("prompt 过短"));
    }

    #[test]
    fn urls_require_lowercase_http_scheme() {
        let err = reference_media_urls(vec!["HTTPS://x".into()], "image_files", 9)
            .expect_err("uppercase URL scheme must fail");
        assert!(err.contains("小写 http://"));
    }

    #[test]
    fn validates_saved_local_upload_identity_once() {
        let bytes = b"validated local image";
        let sha256 = format!("{:x}", Sha256::digest(bytes));
        let rows = vec![VideoReferenceMediaBindingV1 {
            url: String::new(),
            usage: "人物外观参考".to_string(),
            upload_sha256: Some(sha256.clone()),
            upload_file_name: Some("reference.png".to_string()),
            upload_mime_type: Some("image/png".to_string()),
        }];
        let uploads = vec![a4n_api_contract::video::ReferenceMediaUploadInput {
            file_name: "reference.png".to_string(),
            mime_type: "image/png".to_string(),
            data_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
        }];

        let validated = validated_reference_media_uploads(&rows, &uploads, "image").unwrap();
        assert_eq!(validated.len(), 1);
        assert_eq!(validated[0].sha256, sha256);
        assert_eq!(validated[0].bytes, bytes);
    }

    #[test]
    fn request_builder_uses_one_seedance_reference_media_contract() {
        let prompt = format!(
            "{} @image_file_1 用作人物外观与服装一致性参考。",
            "镜头保持人物、场景、动作、光影、构图和节奏连续，".repeat(8)
        );
        let request = build_reference_media_video_request(
            &video_config(SEEDANCE2_VIDEO_BACKEND),
            ResolvedReferenceMediaVideoInput {
                prompt,
                image_files: vec!["https://example.test/ref.png".to_string()],
                video_files: Vec::new(),
                audio_files: Vec::new(),
                duration_secs: 8,
                aspect_ratio: "16:9".to_string(),
            },
        )
        .unwrap();
        assert_eq!((request.width, request.height), (1280, 720));
        let a4n_video::VideoMediaInput::References { images, .. } = &request.media else {
            panic!("expected reference media");
        };
        assert_eq!(
            images,
            &[a4n_video::VideoMediaSource::from(
                "https://example.test/ref.png"
            )]
        );
        let note = reference_media_provider_request_note(
            "当前表单",
            &video_config(SEEDANCE2_VIDEO_BACKEND),
            &request,
        )
        .unwrap();
        assert!(note.contains("config_id=9"));
        assert!(note.contains("image_files=1"));
        assert!(note.contains("input_reference"));
    }

    #[test]
    fn request_builder_maps_cliproxy_image_references() {
        let prompt = format!(
            "{} @image_file_1 只作为环境光、色调和写实质感参考。",
            "固定机位保持场景方向、动作、构图和节奏连续，".repeat(8)
        );
        let mut config = video_config(CLIPROXY_OPENAI_VIDEO_BACKEND);
        config.model = "bc/se-2.0".to_string();
        let request = build_reference_media_video_request(
            &config,
            ResolvedReferenceMediaVideoInput {
                prompt,
                image_files: vec!["https://example.test/ref.png".to_string()],
                video_files: Vec::new(),
                audio_files: Vec::new(),
                duration_secs: 8,
                aspect_ratio: "16:9".to_string(),
            },
        )
        .unwrap();

        assert_eq!(
            request.media.reference_images(),
            [a4n_video::VideoMediaSource::from(
                "https://example.test/ref.png"
            )]
        );
        let note = reference_media_provider_request_note("当前表单", &config, &request).unwrap();
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
        ) {
            Ok(_) => panic!("cliproxy video/audio references must fail"),
            Err(error) => error,
        };

        assert!(error.contains("不支持视频或音频素材"));
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
        )
        .expect_err("cliproxy image count over the provider limit must fail");

        assert!(error.contains("最多允许 7 个 URL"));
    }
}
