use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageUploadFormat {
    pub extension: &'static str,
    pub accepted_extensions: &'static [&'static str],
    pub mime_type: &'static str,
    pub accepted_upload_mime_types: &'static [&'static str],
    pub browser_mime_types: &'static [&'static str],
}

impl ImageUploadFormat {
    pub const PNG: Self = Self {
        extension: "png",
        accepted_extensions: &["png"],
        mime_type: "image/png",
        accepted_upload_mime_types: &["image/png"],
        browser_mime_types: &["image/png"],
    };
    pub const JPEG: Self = Self {
        extension: "jpg",
        accepted_extensions: &["jpg", "jpeg"],
        mime_type: "image/jpeg",
        accepted_upload_mime_types: &["image/jpeg"],
        browser_mime_types: &["image/jpeg"],
    };
    pub const WEBP: Self = Self {
        extension: "webp",
        accepted_extensions: &["webp"],
        mime_type: "image/webp",
        accepted_upload_mime_types: &["image/webp"],
        browser_mime_types: &["image/webp"],
    };
}

pub const IMAGE_UPLOAD_MEDIA_TYPES: &[ImageUploadFormat] = &[
    ImageUploadFormat::PNG,
    ImageUploadFormat::JPEG,
    ImageUploadFormat::WEBP,
];

pub fn image_upload_format_from_extension(extension: &str) -> Option<&'static ImageUploadFormat> {
    IMAGE_UPLOAD_MEDIA_TYPES.iter().find(|format| {
        format
            .accepted_extensions
            .iter()
            .any(|accepted| extension.eq_ignore_ascii_case(accepted))
    })
}

pub fn image_upload_format_from_mime(mime_type: &str) -> Option<&'static ImageUploadFormat> {
    let mime_type = mime_type.trim();
    IMAGE_UPLOAD_MEDIA_TYPES.iter().find(|format| {
        format
            .accepted_upload_mime_types
            .iter()
            .any(|accepted| mime_type.eq_ignore_ascii_case(accepted))
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoMediaType {
    pub extension: &'static str,
    pub mime_type: &'static str,
    pub accepted_upload_mime_types: &'static [&'static str],
}

pub const VIDEO_MEDIA_TYPES: &[VideoMediaType] = &[
    VideoMediaType {
        extension: "mp4",
        mime_type: "video/mp4",
        accepted_upload_mime_types: &["video/mp4", "application/mp4"],
    },
    VideoMediaType {
        extension: "webm",
        mime_type: "video/webm",
        accepted_upload_mime_types: &["video/webm"],
    },
    VideoMediaType {
        extension: "mov",
        mime_type: "video/quicktime",
        accepted_upload_mime_types: &["video/quicktime", "video/mov"],
    },
    VideoMediaType {
        extension: "m4v",
        mime_type: "video/mp4",
        accepted_upload_mime_types: &["video/x-m4v", "video/mp4", "application/mp4"],
    },
];

#[derive(Debug, Clone, PartialEq)]
pub struct VideoFileProperties {
    pub width: i32,
    pub height: i32,
    pub duration_secs: f32,
    pub fps: i32,
    pub file_size: i64,
}

impl VideoFileProperties {
    pub fn validate(&self) -> Result<(), String> {
        if self.width <= 0 || self.height <= 0 || self.fps <= 0 || self.file_size <= 0 {
            return Err("视频实际尺寸、帧率和文件大小必须大于 0".into());
        }
        if !self.duration_secs.is_finite() || self.duration_secs <= 0.0 {
            return Err("视频实际时长必须是大于 0 的有限数".into());
        }
        Ok(())
    }
}

/// Facts inspected from a generated media file before publication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedMediaArtifactFile {
    pub content_type: String,
    pub sha256: String,
    pub byte_size: u64,
}

/// Decoded image facts passed from media parsing to the image publication
/// owner. The parser owns validation; this pure value carries no image crate
/// or file-system implementation dependency.
#[derive(Debug)]
pub struct DecodedUploadedImage {
    pub bytes: Vec<u8>,
    pub extension: &'static str,
    pub mime_type: &'static str,
    pub width: i32,
    pub height: i32,
    pub source_file_name: String,
}

/// Decoded audio facts passed from upload validation to the owning workflow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedUploadedAudio {
    pub file_name: String,
    pub mime_type: &'static str,
    pub extension: &'static str,
    pub bytes: Vec<u8>,
}

pub fn media_mime_from_path(path: &Path) -> Result<&'static str, String> {
    let extension = media_file_extension(path, "媒体文件")?;
    media_mime_from_extension(&extension)
        .ok_or_else(|| format!("媒体文件扩展名不支持：{extension}"))
}

pub fn image_mime_from_path(path: &Path) -> Result<&'static str, String> {
    let extension = media_file_extension(path, "图片")?;
    image_upload_format_from_extension(&extension)
        .map(|format| format.mime_type)
        .ok_or_else(|| format!("图片扩展名不支持：{extension}"))
}

pub fn video_mime_from_path(path: &Path) -> Result<&'static str, String> {
    let extension = media_file_extension(path, "视频文件")?;
    video_mime_from_extension(&extension)
        .ok_or_else(|| format!("视频文件扩展名不支持：{extension}"))
}

pub fn canonical_video_extension(value: &str) -> Option<&'static str> {
    let value = value.trim();
    VIDEO_MEDIA_TYPES
        .iter()
        .find(|media_type| value.eq_ignore_ascii_case(media_type.extension))
        .map(|media_type| media_type.extension)
}

pub fn canonical_video_mime(value: &str) -> Option<(&'static str, &'static str)> {
    let value = value.trim();
    VIDEO_MEDIA_TYPES
        .iter()
        .find(|media_type| value.eq_ignore_ascii_case(media_type.mime_type))
        .map(|media_type| (media_type.mime_type, media_type.extension))
}

pub fn video_upload_mime_matches(extension: &str, mime_type: &str) -> bool {
    let mime_type = mime_type.trim();
    VIDEO_MEDIA_TYPES.iter().any(|media_type| {
        media_type.extension == extension
            && media_type
                .accepted_upload_mime_types
                .iter()
                .any(|accepted| mime_type.eq_ignore_ascii_case(accepted))
    })
}

pub fn canonical_audio_mime(value: &str) -> Result<(&'static str, &'static str), String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "audio/mpeg" | "audio/mp3" => Ok(("audio/mpeg", "mp3")),
        "audio/wav" | "audio/x-wav" => Ok(("audio/wav", "wav")),
        "audio/ogg" | "application/ogg" => Ok(("audio/ogg", "ogg")),
        "audio/mp4" | "audio/x-m4a" => Ok(("audio/mp4", "m4a")),
        "audio/aac" => Ok(("audio/aac", "aac")),
        "audio/flac" | "audio/x-flac" => Ok(("audio/flac", "flac")),
        other => Err(format!("音频 MIME 不支持：{other}")),
    }
}

fn media_file_extension(path: &Path, label: &str) -> Result<String, String> {
    path.extension()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| {
            format!(
                "{label}缺少文件扩展名，无法确定 MIME 类型：{}",
                path.display()
            )
        })
}

fn media_mime_from_extension(extension: &str) -> Option<&'static str> {
    if let Some(format) = image_upload_format_from_extension(extension) {
        return Some(format.mime_type);
    }
    match extension {
        "apng" => Some("image/apng"),
        "avif" => Some("image/avif"),
        "gif" => Some("image/gif"),
        "svg" => Some("image/svg+xml"),
        "mp3" => Some("audio/mpeg"),
        "m4a" => Some("audio/mp4"),
        "wav" => Some("audio/wav"),
        "ogg" => Some("audio/ogg"),
        "aac" => Some("audio/aac"),
        "flac" => Some("audio/flac"),
        _ => video_mime_from_extension(extension),
    }
}

fn video_mime_from_extension(extension: &str) -> Option<&'static str> {
    VIDEO_MEDIA_TYPES
        .iter()
        .find(|media_type| media_type.extension == extension)
        .map(|media_type| media_type.mime_type)
}
