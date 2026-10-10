use a4n_api_contract::temp_media::TEMP_MEDIA_FORMATS;
use a4n_domain::media::image_upload_format_from_extension;
use std::ffi::OsStr;
use std::path::Path;

pub(super) fn temp_media_artifact_extensions() -> impl Iterator<Item = &'static str> {
    TEMP_MEDIA_FORMATS.iter().map(|format| format.extension)
}

pub(super) fn normalize_temp_media_type(
    value: &str,
) -> Result<(&'static str, &'static str), String> {
    let normalized = value.trim().to_ascii_lowercase();
    TEMP_MEDIA_FORMATS
        .iter()
        .find(|format| {
            format
                .accepted_mime_types
                .iter()
                .any(|accepted| accepted.eq_ignore_ascii_case(&normalized))
        })
        .map(|format| (format.mime_type, format.extension))
        .ok_or_else(|| format!("临时媒体 MIME 不支持：{normalized}"))
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
    if let Some(extension) = extension {
        let image_extension_matches = image_upload_format_from_extension(expected_extension)
            .is_some_and(|format| {
                format
                    .accepted_extensions
                    .iter()
                    .any(|accepted| *accepted == extension)
            });
        if extension != expected_extension && !image_extension_matches {
            return Err(format!(
                "临时媒体 file_name 扩展名与 MIME 不一致：期望 {expected_extension}，实际 {extension}"
            ));
        }
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
    fn temp_media_maps_only_contract_mime_types_and_extensions() {
        for format in TEMP_MEDIA_FORMATS {
            for mime_type in format.accepted_mime_types {
                assert_eq!(
                    normalize_temp_media_type(mime_type),
                    Ok((format.mime_type, format.extension))
                );
            }
            assert!(
                format
                    .browser_mime_types
                    .iter()
                    .all(|mime_type| { format.accepted_mime_types.contains(mime_type) })
            );
        }
        assert!(normalize_temp_media_type("application/ogg").is_err());
        assert!(normalize_temp_media_type("video/x-m4v").is_err());
        assert_eq!(
            temp_media_artifact_extensions().collect::<Vec<_>>(),
            TEMP_MEDIA_FORMATS
                .iter()
                .map(|format| format.extension)
                .collect::<Vec<_>>()
        );
        assert!(validate_temp_media_file_name("0123456789abcdef0123456789abcdef.mp4").is_ok());
        assert!(validate_temp_media_file_name("0123456789abcdef0123456789abcdef.m4v").is_err());
    }
}
