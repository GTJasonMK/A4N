use std::path::Path;

pub(in crate::actions::video_generation::media) fn is_provider_supported_image_content_type(
    value: &str,
) -> bool {
    let content_type = value
        .split(';')
        .next()
        .unwrap_or(value)
        .trim()
        .to_ascii_lowercase();
    matches!(
        content_type.as_str(),
        "image/jpeg"
            | "image/jpg"
            | "image/png"
            | "image/webp"
            | "image/x-icon"
            | "image/vnd.microsoft.icon"
    )
}

pub(super) fn truncate_error_body(value: &str) -> String {
    const MAX_LEN: usize = 600;
    let trimmed = value.trim();
    let mut chars = trimmed.chars();
    let preview = chars.by_ref().take(MAX_LEN).collect::<String>();
    if chars.next().is_some() {
        format!("{preview}…")
    } else {
        preview
    }
}

pub(super) fn truncate_url_for_error(value: &str) -> String {
    const MAX_LEN: usize = 240;
    let trimmed = value.trim();
    let mut chars = trimmed.chars();
    let preview = chars.by_ref().take(MAX_LEN).collect::<String>();
    if chars.next().is_some() {
        format!("{preview}…")
    } else {
        preview
    }
}

pub(in crate::actions::video_generation::media) fn encode_query_component(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

pub(in crate::actions::video_generation::media) fn image_mime_from_path(
    path: &Path,
) -> Result<&'static str, String> {
    let extension = path
        .extension()
        .and_then(|s| s.to_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            format!(
                "参考图片缺少文件扩展名，无法判断 MIME 类型：{}",
                path.display()
            )
        })?;
    match extension.to_ascii_lowercase().as_str() {
        "png" => Ok("image/png"),
        "jpg" | "jpeg" => Ok("image/jpeg"),
        "webp" => Ok("image/webp"),
        _ => Err(format!("参考图片扩展名不支持作为视频参考图：{}", extension)),
    }
}
