//! Galgame 媒体绑定/音频导入纯规则（无 DB、无 Host 路径 IO）。

use std::path::Path;

use base64::Engine as _;

use crate::media::strip_data_url_prefix;
use crate::project::required_text;

/// 导入音频体积上限（字节）。
pub const MAX_IMPORT_AUDIO_BYTES: usize = 256 * 1024 * 1024;

/// 已解码的上传音频源。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedAudioSource {
    pub file_name: String,
    pub mime_type: &'static str,
    pub extension: &'static str,
    pub bytes: Vec<u8>,
}

impl ImportedAudioSource {
    /// 从上传字段解析音频（base64 + MIME + 扩展名一致性）。
    pub fn from_upload(
        file_name: &str,
        mime_type: &str,
        data_base64: &str,
    ) -> Result<Self, String> {
        let file_name = required_text(file_name, "音频文件名")?.to_string();
        let (mime_type, extension) = supported_audio_mime(mime_type)?;
        let raw = strip_data_url_prefix(data_base64).trim();
        if raw.is_empty() {
            return Err("音频文件数据不能为空".to_string());
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(raw)
            .map_err(|err| format!("音频文件 base64 解码失败：{err}"))?;
        if bytes.is_empty() {
            return Err("音频文件不能为空".to_string());
        }
        if bytes.len() > MAX_IMPORT_AUDIO_BYTES {
            return Err(format!(
                "音频文件超过 {}MB",
                MAX_IMPORT_AUDIO_BYTES / 1024 / 1024
            ));
        }
        if !file_name_matches_audio_extension(&file_name, extension) {
            return Err(format!(
                "音频文件扩展名与 MIME 不一致，需要 .{} 文件",
                extension
            ));
        }
        Ok(Self {
            file_name,
            mime_type,
            extension,
            bytes,
        })
    }
}

/// 支持的音频 MIME → 规范 MIME + 扩展名。
pub fn supported_audio_mime(mime_type: &str) -> Result<(&'static str, &'static str), String> {
    match mime_type.trim().to_ascii_lowercase().as_str() {
        "audio/mpeg" | "audio/mp3" => Ok(("audio/mpeg", "mp3")),
        "audio/wav" | "audio/x-wav" => Ok(("audio/wav", "wav")),
        "audio/ogg" | "application/ogg" => Ok(("audio/ogg", "ogg")),
        other => Err(format!("Galgame 音频格式不支持: {other}")),
    }
}

fn file_name_matches_audio_extension(file_name: &str, extension: &str) -> bool {
    Path::new(file_name)
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case(extension))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_supported_audio_mime() {
        assert_eq!(
            supported_audio_mime("audio/mp3").unwrap(),
            ("audio/mpeg", "mp3")
        );
    }

    #[test]
    fn imported_audio_rejects_empty_and_accepts_ogg() {
        assert!(ImportedAudioSource::from_upload("a.ogg", "audio/ogg", "").is_err());
        let raw = base64::engine::general_purpose::STANDARD.encode(b"ogg-bytes");
        let source = ImportedAudioSource::from_upload("a.ogg", "audio/ogg", &raw).unwrap();
        assert_eq!(source.extension, "ogg");
        assert_eq!(source.bytes, b"ogg-bytes");
    }
}
