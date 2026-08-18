//! 视频 provider 参考图：内容类型 / URL 编码 / 传输模式纯规则（无 DB、无 Host）。

/// 视频 provider 参考图输入传输模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderImageInputTransport {
    PublicUrl,
    InlineDataUrl,
    RemoteTempUpload,
}

/// 解析视频图片传输模式；默认直接使用本地 data URL。
pub fn provider_image_input_transport_with_override(
    override_value: Option<&str>,
) -> Result<ProviderImageInputTransport, String> {
    let override_value = override_value
        .map(|value| {
            let value = value.trim();
            if value.is_empty() {
                return Err(
                    "A4N_VIDEO_IMAGE_INPUT_TRANSPORT 不能为空；允许值为 inline_data_url、public_url 或 remote_temp_upload"
                        .to_string(),
                );
            }
            Ok(value.to_ascii_lowercase())
        })
        .transpose()?;
    match override_value.as_deref() {
        Some("public_url") => Ok(ProviderImageInputTransport::PublicUrl),
        Some("remote_temp_upload") => Ok(ProviderImageInputTransport::RemoteTempUpload),
        Some("inline_data_url") | None => Ok(ProviderImageInputTransport::InlineDataUrl),
        Some(other) => Err(format!(
            "A4N_VIDEO_IMAGE_INPUT_TRANSPORT 无效：{other}；允许值为 inline_data_url、public_url 或 remote_temp_upload"
        )),
    }
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

    #[test]
    fn content_type_accepts_jpeg_and_png() {
        assert!(is_provider_supported_image_content_type(
            "image/jpeg; charset=utf-8"
        ));
        assert!(is_provider_supported_image_content_type("IMAGE/PNG"));
        assert!(!is_provider_supported_image_content_type("image/gif"));
    }

    #[test]
    fn encode_query_leaves_unreserved() {
        assert_eq!(encode_query_component("abc-_.~"), "abc-_.~");
        assert!(encode_query_component("a b").contains('%'));
    }

    #[test]
    fn transport_override_defaults_and_rejects() {
        assert_eq!(
            provider_image_input_transport_with_override(None).unwrap(),
            ProviderImageInputTransport::InlineDataUrl
        );
        assert_eq!(
            provider_image_input_transport_with_override(Some("remote_temp_upload")).unwrap(),
            ProviderImageInputTransport::RemoteTempUpload
        );
        assert_eq!(
            provider_image_input_transport_with_override(Some("public_url")).unwrap(),
            ProviderImageInputTransport::PublicUrl
        );
        assert!(provider_image_input_transport_with_override(Some("bad")).is_err());
        assert!(provider_image_input_transport_with_override(Some("  ")).is_err());
    }
}
