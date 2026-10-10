use a4n_api_contract::temp_media::TEMP_MEDIA_VIDEO_EXTENSIONS;
use a4n_domain::media::{canonical_audio_mime, canonical_video_mime};
use std::ffi::OsStr;
use std::path::Path;

const TEMP_MEDIA_IMAGE_ARTIFACT_EXTENSIONS: &[&str] = &["png", "jpg", "webp"];
const TEMP_MEDIA_AUDIO_ARTIFACT_EXTENSIONS: &[&str] = &["mp3", "wav", "m4a", "ogg"];

pub(super) fn temp_media_artifact_extensions() -> impl Iterator<Item = &'static str> {
    TEMP_MEDIA_IMAGE_ARTIFACT_EXTENSIONS
        .iter()
        .copied()
        .chain(TEMP_MEDIA_VIDEO_EXTENSIONS.iter().copied())
        .chain(TEMP_MEDIA_AUDIO_ARTIFACT_EXTENSIONS.iter().copied())
}

pub(super) fn normalize_temp_media_type(
    value: &str,
) -> Result<(&'static str, &'static str), String> {
    let normalized = value.trim().to_ascii_lowercase();
    if let Some((mime_type, extension)) = canonical_video_mime(&normalized)
        && TEMP_MEDIA_VIDEO_EXTENSIONS.contains(&extension)
    {
        return Ok((mime_type, extension));
    }
    match normalized.as_str() {
        "image/png" => Ok(("image/png", "png")),
        "image/jpeg" => Ok(("image/jpeg", "jpg")),
        "image/webp" => Ok(("image/webp", "webp")),
        other => {
            if !matches!(
                other,
                "audio/mpeg"
                    | "audio/mp3"
                    | "audio/wav"
                    | "audio/x-wav"
                    | "audio/mp4"
                    | "audio/x-m4a"
                    | "audio/ogg"
            ) {
                return Err(format!("临时媒体 MIME 不支持：{other}"));
            }
            canonical_audio_mime(other).map_err(|_| format!("临时媒体 MIME 不支持：{other}"))
        }
    }
}

pub(super) fn validate_upload_file_name_hint(
    file_name: &str,
    expected_extension: &str,
) -> Result<(), String> {
    let trimmed = file_name.trim();
    if trimmed.is_empty() {
        return Err("临时媒体 file_name 不能为空".to_string());
    }
    if trimmed.contains('/') || trimmed.contains('\\') {
        return Err("临时媒体 file_name 不能包含路径分隔符".to_string());
    }
    let path = Path::new(trimmed);
    if path.components().count() != 1 {
        return Err("临时媒体 file_name 不能包含路径分隔符".to_string());
    }
    let extension = path
        .extension()
        .and_then(OsStr::to_str)
        .map(str::to_ascii_lowercase);
    if let Some(extension) = extension
        && extension != expected_extension
        && !(expected_extension == "jpg" && extension == "jpeg")
    {
        return Err(format!(
            "临时媒体 file_name 扩展名与 MIME 不一致：期望 {expected_extension}，实际 {extension}"
        ));
    }
    Ok(())
}

pub(super) fn validate_media_magic(mime_type: &str, bytes: &[u8]) -> Result<(), String> {
    let is_iso_media = bytes.len() >= 12 && &bytes[4..8] == b"ftyp";
    let ok = match mime_type {
        "image/png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "image/jpeg" => bytes.starts_with(&[0xFF, 0xD8, 0xFF]),
        "image/webp" => bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP",
        "video/mp4" | "video/quicktime" | "audio/mp4" => is_iso_media,
        "video/webm" => bytes.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]),
        "audio/mpeg" => {
            bytes.starts_with(b"ID3")
                || (bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] & 0xE0 == 0xE0)
        }
        "audio/wav" => bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WAVE",
        "audio/ogg" => bytes.starts_with(b"OggS"),
        _ => false,
    };
    if ok {
        Ok(())
    } else {
        Err("临时媒体内容与 MIME 类型不一致".to_string())
    }
}

pub(super) fn validate_temp_media_file_name(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    let Some((uuid, extension)) = trimmed.rsplit_once('.') else {
        return Err("临时媒体文件名无效".to_string());
    };
    if uuid.len() != 32 || !uuid.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("临时媒体文件名无效".to_string());
    }
    let extension = extension.to_ascii_lowercase();
    if temp_media_artifact_extensions().any(|supported| supported == extension.as_str()) {
        Ok(trimmed.to_string())
    } else {
        Err("临时媒体文件扩展名不支持".to_string())
    }
}

pub(super) fn validate_temp_media_artifact_id(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.len() != 32 || !trimmed.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("临时媒体 artifact_id 无效".to_string());
    }
    Ok(trimmed.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temp_media_uses_the_shared_video_mime_and_extension_contract() {
        assert_eq!(
            normalize_temp_media_type(" VIDEO/MP4 "),
            Ok(("video/mp4", "mp4"))
        );
        assert_eq!(
            normalize_temp_media_type("video/quicktime"),
            Ok(("video/quicktime", "mov"))
        );
        assert_eq!(
            normalize_temp_media_type("video/webm"),
            Ok(("video/webm", "webm"))
        );
        for mime_type in ["video/x-m4v", "application/mp4", "video/mov"] {
            assert!(normalize_temp_media_type(mime_type).is_err());
        }
        assert!(validate_temp_media_file_name("0123456789abcdef0123456789abcdef.mp4").is_ok());
        assert!(validate_temp_media_file_name("0123456789abcdef0123456789abcdef.m4v").is_err());
    }
}
