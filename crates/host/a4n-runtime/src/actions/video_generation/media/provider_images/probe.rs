use crate::actions::video_generation::media::types::PROVIDER_IMAGE_URL_PROBE_TIMEOUT_SECS;
use a4n_video::{is_provider_supported_image_content_type, truncate_url_for_error};
use reqwest::header::{CONTENT_TYPE, RANGE};
use std::sync::OnceLock;
use std::time::Duration;

pub(super) async fn validate_provider_public_image_url(
    url: &str,
    label: &str,
) -> Result<(), String> {
    let client = provider_image_probe_client()?;
    let head_probe = probe_provider_public_image_url(client, url, label, false).await?;
    if head_probe.supported {
        return Ok(());
    }
    let get_probe = probe_provider_public_image_url(client, url, label, true).await?;
    if get_probe.supported {
        return Ok(());
    }
    Err(format!(
        "{label} 的公网 URL 无法作为视频模型图片输入：{}。请检查 A4N_PUBLIC_APP_URL 是否指向可公网访问的应用入口，且没有被 Cloudflare、DNSPod、Nginx 或登录页重定向到 HTML/JSON 页面。url={}",
        get_probe.message,
        truncate_url_for_error(url)
    ))
}

fn provider_image_probe_client() -> Result<&'static reqwest::Client, String> {
    static CLIENT: OnceLock<Result<reqwest::Client, String>> = OnceLock::new();
    CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .timeout(Duration::from_secs(PROVIDER_IMAGE_URL_PROBE_TIMEOUT_SECS))
                .build()
                .map_err(|err| format!("创建公网图片探测客户端失败：{err}"))
        })
        .as_ref()
        .map_err(Clone::clone)
}

#[derive(Debug)]
struct ProviderImageUrlProbe {
    supported: bool,
    message: String,
}

async fn probe_provider_public_image_url(
    client: &reqwest::Client,
    url: &str,
    label: &str,
    use_get: bool,
) -> Result<ProviderImageUrlProbe, String> {
    let request = if use_get {
        client.get(url).header(RANGE, "bytes=0-0")
    } else {
        client.head(url)
    };
    let response = request
        .send()
        .await
        .map_err(|err| format!("{label} 公网 URL 探测失败：{err}"))?;
    let status = response.status();
    let final_url = response.url().to_string();
    if !status.is_success() {
        return Ok(ProviderImageUrlProbe {
            supported: false,
            message: format!(
                "{} 返回 HTTP {}，最终 URL={}",
                if use_get { "GET" } else { "HEAD" },
                status.as_u16(),
                truncate_url_for_error(&final_url)
            ),
        });
    }
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    if content_type
        .as_deref()
        .is_some_and(is_provider_supported_image_content_type)
    {
        return Ok(ProviderImageUrlProbe {
            supported: true,
            message: String::new(),
        });
    }
    let content_type = content_type.unwrap_or_else(|| "<missing>".to_string());
    Ok(ProviderImageUrlProbe {
        supported: false,
        message: format!(
            "{} 返回不支持的 Content-Type：{}，最终 URL={}",
            if use_get { "GET" } else { "HEAD" },
            content_type,
            truncate_url_for_error(&final_url)
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::provider_image_probe_client;

    #[test]
    fn provider_image_probe_client_is_reused() {
        let first = provider_image_probe_client().expect("probe client should build") as *const _;
        let second =
            provider_image_probe_client().expect("probe client should be reusable") as *const _;
        assert_eq!(first, second);
    }
}
