use a4n_domain::{
    ProviderError as AppError,
    media::{canonical_video_extension, canonical_video_mime},
};

pub(crate) fn normalize_video_format_str(raw: &str) -> Result<String, AppError> {
    let format = raw.trim().trim_start_matches('.');
    canonical_video_extension(format)
        .map(str::to_string)
        .ok_or_else(|| {
            AppError::Internal(format!("不支持的视频格式：{}", format.to_ascii_lowercase()))
        })
}

/// 从 URL 路径推断视频扩展名（忽略 query 与 fragment）。
pub(crate) fn video_format_from_url(url: &str) -> Option<String> {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    let last = path.rsplit('/').next().unwrap_or(path);
    let dot = last.rfind('.')?;
    let ext = last[dot + 1..].to_ascii_lowercase();
    canonical_video_extension(&ext)
        .filter(|extension| *extension == ext.as_str())
        .map(str::to_string)
}

pub(crate) fn video_format_from_content_type(content_type: &str) -> Option<String> {
    let media_type = content_type
        .split(';')
        .next()
        .unwrap_or(content_type)
        .trim()
        .to_ascii_lowercase();
    canonical_video_mime(&media_type)
        .map(|(_, extension)| extension.to_string())
        .or_else(|| (media_type == "video/x-m4v").then(|| "m4v".into()))
}

pub(crate) fn video_format_from_content_type_or_bytes(
    content_type: Option<&str>,
    bytes: &[u8],
) -> Option<String> {
    content_type
        .and_then(video_format_from_content_type)
        .or_else(|| detect_video_format_from_bytes(bytes).ok())
}

/// 用字节魔数嗅探 MP4 / WebM；MOV 与 MP4 共享 `ftyp` 容器，统一返回 `mp4`。
pub(crate) fn detect_video_format_from_bytes(bytes: &[u8]) -> Result<String, AppError> {
    if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" {
        return Ok("mp4".into());
    }
    if bytes.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]) {
        return Ok("webm".into());
    }
    Err(AppError::Internal(
        "无法识别视频字节格式（仅支持 mp4 / webm 魔数嗅探，请在响应中显式给出 format 或 content_type）"
            .to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 最小可解码 MP4 ftyp 头：4 字节大小 + "ftyp" + brand + minor + compatible（共 16 字节）
    fn minimal_mp4_bytes() -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&[0, 0, 0, 16]);
        buf.extend_from_slice(b"ftypmp42");
        buf.extend_from_slice(&[0, 0, 0, 0]);
        buf.extend_from_slice(b"mp42");
        buf
    }

    #[test]
    fn detect_video_format_recognizes_mp4_ftyp() {
        assert_eq!(
            detect_video_format_from_bytes(&minimal_mp4_bytes()).unwrap(),
            "mp4"
        );
    }

    #[test]
    fn detect_video_format_recognizes_webm_ebml() {
        let bytes = [0x1A, 0x45, 0xDF, 0xA3, 0x01, 0x02, 0x03, 0x04];
        assert_eq!(detect_video_format_from_bytes(&bytes).unwrap(), "webm");
    }

    #[test]
    fn detect_video_format_rejects_unknown() {
        assert!(detect_video_format_from_bytes(b"not-a-video").is_err());
    }

    #[test]
    fn video_format_from_url_handles_query_and_fragment() {
        assert_eq!(
            video_format_from_url("https://e.com/x.mp4?token=abc"),
            Some("mp4".into())
        );
        assert_eq!(
            video_format_from_url("https://e.com/path/x.WebM#t=1"),
            Some("webm".into())
        );
        assert_eq!(
            video_format_from_url("https://e.com/path/x.MOV#t=1"),
            Some("mov".into())
        );
        assert_eq!(
            video_format_from_url("https://e.com/path/x.M4V#t=1"),
            Some("m4v".into())
        );
        assert_eq!(video_format_from_url("https://e.com/streaming"), None);
        assert_eq!(video_format_from_url("https://e.com/x.gif"), None);
        assert_eq!(video_format_from_url("https://e.com/x.mp4 "), None);
    }

    #[test]
    fn normalize_video_format_str_uses_the_domain_video_format_set() {
        assert_eq!(normalize_video_format_str(" .M4V ").unwrap(), "m4v");
        assert!(normalize_video_format_str("avi").is_err());
    }

    #[test]
    fn video_format_from_content_type_handles_parameters() {
        assert_eq!(
            video_format_from_content_type("video/mp4; charset=utf-8"),
            Some("mp4".into())
        );
        assert_eq!(
            video_format_from_content_type("video/quicktime"),
            Some("mov".into())
        );
        assert_eq!(
            video_format_from_content_type("video/x-m4v"),
            Some("m4v".into())
        );
        assert_eq!(video_format_from_content_type("video/mov"), None);
        assert_eq!(video_format_from_content_type("application/json"), None);
    }

    #[test]
    fn video_format_from_content_type_or_bytes_rejects_unknown_data() {
        assert_eq!(
            video_format_from_content_type_or_bytes(Some("application/json"), b"not-a-video"),
            None
        );
        assert_eq!(
            video_format_from_content_type_or_bytes(Some("video/webm"), b"not-a-video"),
            Some("webm".into())
        );
    }
}
