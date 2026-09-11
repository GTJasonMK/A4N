//! 视频 provider 参考图输入的纯规则。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderImageInputTransport {
    PublicUrl,
    InlineDataUrl,
    RemoteTempUpload,
}

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

pub fn truncate_url_for_error(value: &str) -> String {
    truncate_chars(value, 240)
}

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
    truncate_chars(value, 600)
}

fn truncate_chars(value: &str, max_len: usize) -> String {
    let trimmed = value.trim();
    let mut chars = trimmed.chars();
    let preview = chars.by_ref().take(max_len).collect::<String>();
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
    fn content_type_and_query_rules_are_stable() {
        assert!(is_provider_supported_image_content_type(
            "image/jpeg; charset=utf-8"
        ));
        assert!(!is_provider_supported_image_content_type("image/gif"));
        assert_eq!(encode_query_component("abc-_.~"), "abc-_.~");
        assert!(encode_query_component("a b").contains('%'));
    }

    #[test]
    fn transport_defaults_and_invalid_values_are_explicit() {
        assert_eq!(
            provider_image_input_transport_with_override(None).unwrap(),
            ProviderImageInputTransport::InlineDataUrl
        );
        assert_eq!(
            provider_image_input_transport_with_override(Some("remote_temp_upload")).unwrap(),
            ProviderImageInputTransport::RemoteTempUpload
        );
        assert!(provider_image_input_transport_with_override(Some("bad")).is_err());
    }
}
