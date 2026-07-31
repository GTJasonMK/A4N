//! 视频 provider 参考图：MIME / 内容类型 / URL 编码 / 传输模式纯规则（无 DB、无 Host）。

use std::path::Path;

/// 视频 provider 参考图输入传输模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderImageInputTransport {
    PublicUrl,
    InlineDataUrl,
    RemoteTempUpload,
}

/// 解析 `A4N_VIDEO_IMAGE_INPUT_TRANSPORT` 覆盖值（纯规则，不读环境）。
///
/// `None` / `"auto"` → 默认 `InlineDataUrl`。
pub fn parse_provider_image_input_transport_override(
    override_value: Option<&str>,
) -> Result<ProviderImageInputTransport, String> {
    let override_value = override_value
        .map(|value| {
            let value = value.trim();
            if value.is_empty() {
                return Err(
                    "A4N_VIDEO_IMAGE_INPUT_TRANSPORT 不能为空；允许值为 auto、inline_data_url 或 public_url"
                        .to_string(),
                );
            }
            Ok(value.to_ascii_lowercase())
        })
        .transpose()?;
    match override_value.as_deref() {
        Some("public_url") => Ok(ProviderImageInputTransport::PublicUrl),
        Some("inline_data_url") => Ok(ProviderImageInputTransport::InlineDataUrl),
        Some("auto") | None => Ok(ProviderImageInputTransport::InlineDataUrl),
        Some(other) => Err(format!(
            "A4N_VIDEO_IMAGE_INPUT_TRANSPORT 无效：{other}；允许值为 auto、inline_data_url 或 public_url"
        )),
    }
}

/// 按 backend、显式覆盖和远端临时上传能力解析图片传输模式。
pub fn provider_image_input_transport_with_override(
    backend: &str,
    override_value: Option<&str>,
    remote_temp_configured: bool,
    cliproxy_backend: &str,
) -> Result<ProviderImageInputTransport, String> {
    let backend = backend.trim();
    let override_value = override_value
        .map(|value| {
            let value = value.trim();
            if value.is_empty() {
                return Err(
                    "A4N_VIDEO_IMAGE_INPUT_TRANSPORT 不能为空；允许值为 auto、inline_data_url、public_url 或 remote_temp_upload"
                        .to_string(),
                );
            }
            Ok(value.to_ascii_lowercase())
        })
        .transpose()?;
    match override_value.as_deref() {
        Some("public_url") => Ok(ProviderImageInputTransport::PublicUrl),
        Some("remote_temp_upload") => Ok(ProviderImageInputTransport::RemoteTempUpload),
        Some("inline_data_url") => {
            if backend == cliproxy_backend {
                return Err(
                    "cliproxy-openai-video 不支持 base64/data URL 视频图片输入，请设置 A4N_VIDEO_IMAGE_INPUT_TRANSPORT=remote_temp_upload 并配置 A4N_VIDEO_REMOTE_TEMP_UPLOAD_URL/A4N_VIDEO_REMOTE_TEMP_UPLOAD_TOKEN，或设置 A4N_PUBLIC_APP_URL/A4N_PUBLIC_MEDIA_BASE_URL"
                        .to_string(),
                );
            }
            Ok(ProviderImageInputTransport::InlineDataUrl)
        }
        Some("auto") | None => Ok(provider_default_image_input_transport(
            backend,
            remote_temp_configured,
            cliproxy_backend,
        )),
        Some(other) => Err(format!(
            "A4N_VIDEO_IMAGE_INPUT_TRANSPORT 无效：{other}；允许值为 auto、inline_data_url、public_url 或 remote_temp_upload"
        )),
    }
}

pub fn provider_default_image_input_transport(
    backend: &str,
    remote_temp_configured: bool,
    cliproxy_backend: &str,
) -> ProviderImageInputTransport {
    if backend.trim() == cliproxy_backend {
        if remote_temp_configured {
            return ProviderImageInputTransport::RemoteTempUpload;
        }
        return ProviderImageInputTransport::PublicUrl;
    }
    ProviderImageInputTransport::InlineDataUrl
}

/// 是否为视频 provider 可接受的图片 Content-Type。
pub fn is_provider_supported_image_content_type(value: &str) -> bool {
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

/// 截断 URL 以便错误信息展示。
pub fn truncate_url_for_error(value: &str) -> String {
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

/// 百分号编码查询参数分量。
pub fn encode_query_component(value: &str) -> String {
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

/// 由文件路径扩展名推断视频参考图 MIME。
pub fn image_mime_from_path(path: &Path) -> Result<&'static str, String> {
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

pub fn truncate_error_body(value: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn content_type_accepts_jpeg_and_png() {
        assert!(is_provider_supported_image_content_type(
            "image/jpeg; charset=utf-8"
        ));
        assert!(is_provider_supported_image_content_type("IMAGE/PNG"));
        assert!(!is_provider_supported_image_content_type("image/gif"));
    }

    #[test]
    fn mime_from_path_maps_common_extensions() {
        assert_eq!(
            image_mime_from_path(Path::new("a/b/c.PNG")).unwrap(),
            "image/png"
        );
        assert_eq!(
            image_mime_from_path(&PathBuf::from("x.webp")).unwrap(),
            "image/webp"
        );
        assert!(image_mime_from_path(Path::new("noext")).is_err());
    }

    #[test]
    fn encode_query_leaves_unreserved() {
        assert_eq!(encode_query_component("abc-_.~"), "abc-_.~");
        assert!(encode_query_component("a b").contains('%'));
    }

    #[test]
    fn transport_override_defaults_and_rejects() {
        assert_eq!(
            parse_provider_image_input_transport_override(None).unwrap(),
            ProviderImageInputTransport::InlineDataUrl
        );
        assert_eq!(
            parse_provider_image_input_transport_override(Some("public_url")).unwrap(),
            ProviderImageInputTransport::PublicUrl
        );
        assert!(parse_provider_image_input_transport_override(Some("bad")).is_err());
        assert!(parse_provider_image_input_transport_override(Some("  ")).is_err());
    }
}
