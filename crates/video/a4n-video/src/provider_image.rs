//! 视频 provider 参考图输入的纯规则。

use a4n_video_provider_contract::{GEMINI_VEO_VIDEO_BACKEND, ProviderImageInputTransport};

pub fn provider_image_input_transport_for_backend(
    backend: &str,
    override_value: Option<&str>,
) -> Result<Option<ProviderImageInputTransport>, String> {
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
    let transport = match override_value.as_deref() {
        Some("public_url") => Ok(Some(ProviderImageInputTransport::PublicUrl)),
        Some("remote_temp_upload") => Ok(Some(ProviderImageInputTransport::RemoteTempUpload)),
        Some("inline_data_url") => Ok(Some(ProviderImageInputTransport::InlineDataUrl)),
        Some("auto") | None => Ok(None),
        Some(other) => Err(format!(
            "A4N_VIDEO_IMAGE_INPUT_TRANSPORT 无效：{other}；允许值为 auto、inline_data_url、public_url 或 remote_temp_upload"
        )),
    }?;
    if backend == GEMINI_VEO_VIDEO_BACKEND {
        if matches!(
            transport,
            Some(
                ProviderImageInputTransport::PublicUrl
                    | ProviderImageInputTransport::RemoteTempUpload
            )
        ) {
            return Err("Gemini Veo 当前图片合同只支持 inline_data_url，不能使用 public_url 或 remote_temp_upload".into());
        }
        return Ok(Some(ProviderImageInputTransport::InlineDataUrl));
    }
    Ok(transport)
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
        let backend = a4n_video_provider_contract::CLIPROXY_OPENAI_VIDEO_BACKEND;
        assert_eq!(
            provider_image_input_transport_for_backend(backend, None).unwrap(),
            None
        );
        assert_eq!(
            provider_image_input_transport_for_backend(backend, Some("remote_temp_upload"))
                .unwrap(),
            Some(ProviderImageInputTransport::RemoteTempUpload)
        );
        assert_eq!(
            provider_image_input_transport_for_backend(backend, Some("auto")).unwrap(),
            None
        );
        assert_eq!(
            provider_image_input_transport_for_backend(backend, Some("inline_data_url")).unwrap(),
            Some(ProviderImageInputTransport::InlineDataUrl)
        );
        assert!(provider_image_input_transport_for_backend(backend, Some("bad")).is_err());
        assert_eq!(
            provider_image_input_transport_for_backend(GEMINI_VEO_VIDEO_BACKEND, Some("auto"))
                .unwrap(),
            Some(ProviderImageInputTransport::InlineDataUrl)
        );
        assert!(
            provider_image_input_transport_for_backend(
                GEMINI_VEO_VIDEO_BACKEND,
                Some("public_url")
            )
            .is_err()
        );
    }
}
