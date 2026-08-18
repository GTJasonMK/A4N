use a4n_core::media::canonical_audio_mime;
use base64::Engine as _;
use std::ffi::OsStr;
use std::path::Path;

pub(super) const TEMP_MEDIA_ARTIFACT_EXTENSIONS: &[&str] = &[
    "png", "jpg", "webp", "mp4", "mov", "webm", "mp3", "wav", "m4a", "ogg",
];

pub(super) fn decode_base64_media(value: &str) -> Result<Vec<u8>, String> {
    base64::engine::general_purpose::STANDARD
        .decode(value.trim())
        .map_err(|err| format!("临时媒体 data_base64 无效：{err}"))
}

pub(super) fn normalize_temp_media_type(
    value: &str,
) -> Result<(&'static str, &'static str), String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "image/png" => Ok(("image/png", "png")),
        "image/jpeg" => Ok(("image/jpeg", "jpg")),
        "image/webp" => Ok(("image/webp", "webp")),
        "video/mp4" => Ok(("video/mp4", "mp4")),
        "video/quicktime" => Ok(("video/quicktime", "mov")),
        "video/webm" => Ok(("video/webm", "webm")),
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
    if TEMP_MEDIA_ARTIFACT_EXTENSIONS.contains(&extension.as_str()) {
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
