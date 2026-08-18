use std::path::Path;

pub fn media_mime_from_path(path: &Path) -> Result<&'static str, String> {
    let extension = media_file_extension(path, "媒体文件")?;
    media_mime_from_extension(&extension)
        .ok_or_else(|| format!("媒体文件扩展名不支持：{extension}"))
}

pub fn image_mime_from_path(path: &Path) -> Result<&'static str, String> {
    let extension = media_file_extension(path, "图片")?;
    if !matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "webp") {
        return Err(format!("图片扩展名不支持：{extension}"));
    }
    media_mime_from_extension(&extension)
        .ok_or_else(|| format!("图片扩展名不支持：{extension}"))
}

pub fn video_mime_from_path(path: &Path) -> Result<&'static str, String> {
    let extension = media_file_extension(path, "视频文件")?;
    if !matches!(extension.as_str(), "mp4" | "m4v" | "webm" | "mov") {
        return Err(format!("视频文件扩展名不支持：{extension}"));
    }
    media_mime_from_extension(&extension)
        .ok_or_else(|| format!("视频文件扩展名不支持：{extension}"))
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
    match extension {
        "apng" => Some("image/apng"),
        "avif" => Some("image/avif"),
        "gif" => Some("image/gif"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "png" => Some("image/png"),
        "svg" => Some("image/svg+xml"),
        "webp" => Some("image/webp"),
        "mp4" | "m4v" => Some("video/mp4"),
        "webm" => Some("video/webm"),
        "mov" => Some("video/quicktime"),
        "mp3" => Some("audio/mpeg"),
        "m4a" => Some("audio/mp4"),
        "wav" => Some("audio/wav"),
        "ogg" => Some("audio/ogg"),
        "aac" => Some("audio/aac"),
        "flac" => Some("audio/flac"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_image_and_video_mime_require_supported_extensions() {
        assert_eq!(
            image_mime_from_path(Path::new("image.PNG")),
            Ok("image/png")
        );
        assert!(image_mime_from_path(Path::new("image.gif")).is_err());
        assert_eq!(
            video_mime_from_path(Path::new("clip.M4V")),
            Ok("video/mp4")
        );
        assert!(video_mime_from_path(Path::new("clip.avi")).is_err());
    }

    #[test]
    fn generic_media_mime_rejects_unknown_extensions() {
        assert_eq!(
            media_mime_from_path(Path::new("image.AVIF")),
            Ok("image/avif")
        );
        assert_eq!(
            media_mime_from_path(Path::new("voice.flac")),
            Ok("audio/flac")
        );
        assert!(media_mime_from_path(Path::new("archive.bin")).is_err());
    }

    #[test]
    fn audio_mime_is_canonical_and_extension_bound() {
        assert_eq!(
            canonical_audio_mime("audio/x-wav"),
            Ok(("audio/wav", "wav"))
        );
        assert_eq!(
            canonical_audio_mime("audio/x-flac"),
            Ok(("audio/flac", "flac"))
        );
        assert!(canonical_audio_mime("application/octet-stream").is_err());
    }
}
