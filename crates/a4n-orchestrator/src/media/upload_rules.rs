use std::path::Path;

use a4n_core::media::canonical_audio_mime;
use base64::Engine as _;

pub const MAX_IMPORT_AUDIO_BYTES: usize = 256 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedUploadedAudio {
    pub file_name: String,
    pub mime_type: &'static str,
    pub extension: &'static str,
    pub bytes: Vec<u8>,
}

pub fn required_upload_file_name(file_name: &str, label: &str) -> Result<String, String> {
    let trimmed = file_name.trim();
    if trimmed.is_empty() {
        return Err(format!("{label}不能为空"));
    }
    if trimmed.contains('/') || trimmed.contains('\\') {
        return Err(format!("{label}不能包含路径"));
    }
    let name = Path::new(trimmed)
        .file_name()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{label}无效"))?;
    if name != trimmed {
        return Err(format!("{label}不能包含路径"));
    }
    Ok(name.to_string())
}

pub fn decode_uploaded_audio_data(
    file_name: &str,
    mime_type: &str,
    data_base64: &str,
) -> Result<DecodedUploadedAudio, String> {
    let file_name = required_upload_file_name(file_name, "上传音频文件名")?;
    let (mime_type, extension) = canonical_audio_mime(mime_type)?;
    let actual_extension = Path::new(&file_name)
        .extension()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "上传音频文件名缺少扩展名".to_string())?;
    if !actual_extension.eq_ignore_ascii_case(extension) {
        return Err(format!(
            "上传音频文件扩展名与 MIME 不一致，需要 .{extension} 文件"
        ));
    }
    let bytes = decode_uploaded_bytes(data_base64, "上传音频", MAX_IMPORT_AUDIO_BYTES)?;
    Ok(DecodedUploadedAudio {
        file_name,
        mime_type,
        extension,
        bytes,
    })
}

pub fn decode_uploaded_bytes(
    data_base64: &str,
    label: &str,
    max_bytes: usize,
) -> Result<Vec<u8>, String> {
    let raw = data_base64.trim();
    if raw.is_empty() {
        return Err(format!("{label}数据不能为空"));
    }
    let max_base64_len = max_bytes.div_ceil(3) * 4;
    if raw.len() > max_base64_len {
        return Err(format!("{label}不能超过 {}MB", max_bytes / 1024 / 1024));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(raw)
        .map_err(|error| format!("{label} base64 解码失败：{error}"))?;
    if bytes.is_empty() {
        return Err(format!("{label}文件不能为空"));
    }
    if bytes.len() > max_bytes {
        return Err(format!("{label}不能超过 {}MB", max_bytes / 1024 / 1024));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uploaded_audio_requires_plain_matching_file_name() {
        assert!(decode_uploaded_audio_data("dir/voice.mp3", "audio/mpeg", "SUQz").is_err());
        assert!(decode_uploaded_audio_data("voice.wav", "audio/mpeg", "SUQz").is_err());
        let source = decode_uploaded_audio_data(" voice.mp3 ", "audio/mp3", "SUQz").unwrap();
        assert_eq!(source.file_name, "voice.mp3");
        assert_eq!(source.mime_type, "audio/mpeg");
    }
}
