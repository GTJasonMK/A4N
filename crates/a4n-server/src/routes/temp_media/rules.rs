use base64::Engine as _;
use std::ffi::OsStr;
use std::path::Path;

pub(super) fn decode_base64_image(value: &str) -> Result<Vec<u8>, String> {
    let trimmed = value.trim();
    let payload = trimmed
        .strip_prefix("data:")
        .and_then(|data_uri| data_uri.split_once(',').map(|(_, payload)| payload))
        .unwrap_or(trimmed);
    base64::engine::general_purpose::STANDARD
        .decode(payload)
        .map_err(|err| format!("临时媒体 data_base64 无效：{err}"))
}

pub(super) fn normalize_image_mime(value: &str) -> Result<&'static str, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "image/png" => Ok("image/png"),
        "image/jpeg" | "image/jpg" => Ok("image/jpeg"),
        "image/webp" => Ok("image/webp"),
        other => Err(format!("临时媒体 MIME 不支持：{other}")),
    }
}

pub(super) fn extension_for_mime(mime_type: &str) -> &'static str {
    match mime_type {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/webp" => "webp",
        _ => "bin",
    }
}

pub(super) fn validate_upload_file_name_hint(
    file_name: &str,
    expected_extension: &str,
) -> Result<(), String> {
    let trimmed = file_name.trim();
    if trimmed.is_empty() {
        return Ok(());
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

pub(super) fn validate_image_magic(mime_type: &str, bytes: &[u8]) -> Result<(), String> {
    let ok = match mime_type {
        "image/png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "image/jpeg" => bytes.starts_with(&[0xFF, 0xD8, 0xFF]),
        "image/webp" => bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP",
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
    match extension.to_ascii_lowercase().as_str() {
        "png" | "jpg" | "jpeg" | "webp" => Ok(trimmed.to_string()),
        _ => Err("临时媒体文件扩展名不支持".to_string()),
    }
}
