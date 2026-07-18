use crate::actions::video_generation::media::remote_temp_upload_configured;
use crate::actions::video_generation::{
    CLIPROXY_OPENAI_VIDEO_BACKEND, ProviderImageInputTransport,
};

pub(in crate::actions::video_generation) fn provider_image_input_transport(
    backend: &str,
) -> Result<ProviderImageInputTransport, String> {
    let override_value = match std::env::var("A4N_VIDEO_IMAGE_INPUT_TRANSPORT") {
        Ok(value) => Some(value),
        Err(std::env::VarError::NotPresent) => None,
        Err(error) => {
            return Err(format!(
                "读取 A4N_VIDEO_IMAGE_INPUT_TRANSPORT 失败：{error}"
            ));
        }
    };
    provider_image_input_transport_with_override(backend, override_value.as_deref())
}

pub(in crate::actions::video_generation) fn provider_image_input_transport_with_override(
    backend: &str,
    override_value: Option<&str>,
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
            if backend == CLIPROXY_OPENAI_VIDEO_BACKEND {
                return Err(
                    "cliproxy-openai-video 不支持 base64/data URL 视频图片输入，请设置 A4N_VIDEO_IMAGE_INPUT_TRANSPORT=remote_temp_upload 并配置 A4N_VIDEO_REMOTE_TEMP_UPLOAD_URL/A4N_VIDEO_REMOTE_TEMP_UPLOAD_TOKEN，或设置 A4N_PUBLIC_APP_URL/A4N_PUBLIC_MEDIA_BASE_URL"
                        .to_string(),
                );
            }
            Ok(ProviderImageInputTransport::InlineDataUrl)
        }
        Some("auto") | None => provider_default_image_input_transport(backend),
        Some(other) => Err(format!(
            "A4N_VIDEO_IMAGE_INPUT_TRANSPORT 无效：{other}；允许值为 auto、inline_data_url、public_url 或 remote_temp_upload"
        )),
    }
}

fn provider_default_image_input_transport(
    backend: &str,
) -> Result<ProviderImageInputTransport, String> {
    Ok(provider_default_image_input_transport_with_remote(
        backend,
        remote_temp_upload_configured()?,
    ))
}

pub(in crate::actions::video_generation) fn provider_default_image_input_transport_with_remote(
    backend: &str,
    remote_temp_configured: bool,
) -> ProviderImageInputTransport {
    if backend.trim() == CLIPROXY_OPENAI_VIDEO_BACKEND {
        if remote_temp_configured {
            return ProviderImageInputTransport::RemoteTempUpload;
        }
        return ProviderImageInputTransport::PublicUrl;
    }
    ProviderImageInputTransport::InlineDataUrl
}
