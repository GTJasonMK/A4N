mod content;

use a4n_api_contract::runtime_config::RuntimeWebFetchConfig;
use a4n_api_contract::web_fetch::{
    WebFetchInput, WebFetchResult, default_web_fetch_config, validate_web_fetch_config,
};
use a4n_error::AppError;
use futures::StreamExt;
use reqwest::{Response, header};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;
use url::Url;

const USER_AGENT: &str = "A4N/1.0 (+https://github.com/a4n)";
const BLOCKED_IPV4_RANGES: &[(u32, u32)] = &[
    (0x0000_0000, 8),
    (0x0a00_0000, 8),
    (0x6440_0000, 10),
    (0x7f00_0000, 8),
    (0xa9fe_0000, 16),
    (0xac10_0000, 12),
    (0xc000_0000, 24),
    (0xc000_0200, 24),
    (0xc0a8_0000, 16),
    (0xc612_0000, 15),
    (0xc633_6400, 24),
    (0xcb00_7100, 24),
    (0xe000_0000, 4),
    (0xf000_0000, 4),
];

pub async fn web_fetch(input: WebFetchInput) -> Result<WebFetchResult, AppError> {
    let url = input.normalized_url().map_err(AppError::InvalidArgument)?;
    let config = current_web_fetch_config();
    if !config.enabled {
        return Err(AppError::ConfigError("个人网页读取已禁用".into()));
    }
    validate_web_fetch_config(&config).map_err(AppError::ConfigError)?;

    fetch_url(url, &config).await
}

fn current_web_fetch_config() -> RuntimeWebFetchConfig {
    crate::runtime_config::current_snapshot()
        .and_then(|snapshot| snapshot.web_fetch)
        .unwrap_or_else(default_web_fetch_config)
}

async fn fetch_url(
    mut current_url: Url,
    config: &RuntimeWebFetchConfig,
) -> Result<WebFetchResult, AppError> {
    let mut redirects = 0;
    loop {
        let client = client_for(&current_url, config).await?;
        let response = client
            .get(current_url.clone())
            .header(
                header::ACCEPT,
                "text/html,application/xhtml+xml,text/*;q=0.9,application/json;q=0.8",
            )
            .send()
            .await
            .map_err(|error| request_error(error, &current_url))?;

        if response.status().is_redirection() {
            if redirects >= config.max_redirects {
                return Err(AppError::InvalidState {
                    current: format!("网页读取重定向次数超过 {}", config.max_redirects),
                    operation: "读取网页".into(),
                });
            }
            let location = response
                .headers()
                .get(header::LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or_else(|| AppError::InvalidState {
                    current: format!("网页读取响应 HTTP {} 缺少 Location", response.status()),
                    operation: "读取网页重定向".into(),
                })?;
            let next_url = current_url
                .join(location)
                .map_err(|error| AppError::InvalidState {
                    current: format!("网页读取重定向 URL 无效: {error}"),
                    operation: "读取网页重定向".into(),
                })?;
            validate_redirect(&current_url, &next_url)?;
            current_url = next_url;
            redirects += 1;
            continue;
        }

        return read_result(response, current_url, config).await;
    }
}

async fn client_for(
    url: &Url,
    config: &RuntimeWebFetchConfig,
) -> Result<reqwest::Client, AppError> {
    let host = url
        .host_str()
        .ok_or_else(|| AppError::InvalidArgument("网页读取 URL 缺少 host".into()))?;
    let port = url
        .port_or_known_default()
        .ok_or_else(|| AppError::InvalidArgument("网页读取 URL 缺少有效端口".into()))?;
    let address = resolve_public_address(host, port).await?;
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(config.timeout_secs as u64))
        .user_agent(USER_AGENT)
        .resolve(host, address)
        .build()
        .map_err(|error| {
            AppError::Internal(anyhow::anyhow!("网页读取 HTTP 客户端创建失败: {error}"))
        })
}

async fn resolve_public_address(host: &str, port: u16) -> Result<SocketAddr, AppError> {
    if let Ok(ip) = host.parse::<IpAddr>() {
        return public_socket(ip, port);
    }
    let mut addresses = tokio::net::lookup_host((host, port))
        .await
        .map_err(|error| AppError::Internal(anyhow::anyhow!("网页读取 DNS 解析失败: {error}")))?;
    addresses
        .find(|address| is_public_ip(address.ip()))
        .ok_or_else(|| AppError::Forbidden(format!("网页读取目标不是公开网络地址: {host}")))
}

fn public_socket(ip: IpAddr, port: u16) -> Result<SocketAddr, AppError> {
    if is_public_ip(ip) {
        Ok(SocketAddr::new(ip, port))
    } else {
        Err(AppError::Forbidden(format!(
            "网页读取目标不是公开网络地址: {ip}"
        )))
    }
}

fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => !blocked_ipv4(ip),
        IpAddr::V6(ip) => !blocked_ipv6(ip),
    }
}

fn blocked_ipv4(ip: Ipv4Addr) -> bool {
    let value = u32::from(ip);
    BLOCKED_IPV4_RANGES.iter().any(|(network, prefix)| {
        let mask = if *prefix == 0 {
            0
        } else {
            u32::MAX << (32 - prefix)
        };
        value & mask == network & mask
    })
}

fn blocked_ipv6(ip: Ipv6Addr) -> bool {
    ip.is_unspecified()
        || ip.is_loopback()
        || ip.is_multicast()
        || ip.is_unique_local()
        || ip.is_unicast_link_local()
        || ip.segments()[0] == 0x2001 && ip.segments()[1] == 0x0db8
        || ip.to_ipv4_mapped().is_some_and(blocked_ipv4)
}

fn validate_redirect(current: &Url, next: &Url) -> Result<(), AppError> {
    if !matches!(next.scheme(), "http" | "https")
        || next.host_str().is_none()
        || !next.username().is_empty()
        || next.password().is_some()
    {
        return Err(AppError::Forbidden("网页读取重定向目标无效或不安全".into()));
    }
    if current.scheme() != next.scheme()
        || current.host_str() != next.host_str()
        || current.port_or_known_default() != next.port_or_known_default()
    {
        return Err(AppError::Forbidden(
            "网页读取不自动跟随跨源重定向，请单独读取目标 URL".into(),
        ));
    }
    Ok(())
}

async fn read_result(
    response: Response,
    final_url: Url,
    config: &RuntimeWebFetchConfig,
) -> Result<WebFetchResult, AppError> {
    let status = response.status();
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let (bytes, byte_truncated) = read_body(response, config.max_response_bytes).await?;
    let (kind, charset) = content::classify_content_type(content_type.as_deref())?;
    let decoded = content::decode_body(&bytes, charset)?;
    let (body, body_truncated) = content::render_body(kind, &decoded, config.max_body_chars)?;
    let body_truncated = byte_truncated || body_truncated;
    let (content, output_truncated) = content::format_fetch_output(
        &final_url,
        status,
        body,
        body_truncated,
        config.max_output_chars,
    );
    Ok(WebFetchResult {
        url: final_url.to_string(),
        status_code: status.as_u16(),
        content,
        truncated: output_truncated,
    })
}

async fn read_body(response: Response, max_bytes: u64) -> Result<(Vec<u8>, bool), AppError> {
    if response
        .content_length()
        .is_some_and(|length| length > max_bytes)
    {
        return Err(AppError::InvalidState {
            current: format!("网页响应超过 {max_bytes} 字节"),
            operation: "读取网页".into(),
        });
    }
    let mut stream = response.bytes_stream();
    let mut body = Vec::new();
    let mut truncated = false;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk
            .map_err(|error| AppError::Internal(anyhow::anyhow!("网页响应读取失败: {error}")))?;
        let remaining = max_bytes.saturating_sub(body.len() as u64) as usize;
        if chunk.len() > remaining {
            body.extend_from_slice(&chunk[..remaining]);
            truncated = true;
            break;
        }
        body.extend_from_slice(&chunk);
    }
    Ok((body, truncated))
}

fn request_error(error: reqwest::Error, url: &Url) -> AppError {
    if error.is_timeout() {
        AppError::Timeout(format!("网页读取请求超过限制: {url}"))
    } else {
        AppError::Internal(anyhow::anyhow!("网页读取请求失败 {url}: {error}"))
    }
}

#[cfg(test)]
mod tests {
    use super::content::{classify_content_type, format_fetch_output, render_body};
    use super::{blocked_ipv4, content::BodyKind, is_public_ip};
    use reqwest::StatusCode;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
    use url::Url;

    #[test]
    fn blocks_private_and_loopback_addresses_but_accepts_public_examples() {
        for ip in [
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            IpAddr::V6(Ipv6Addr::LOCALHOST),
        ] {
            assert!(!is_public_ip(ip));
        }
        assert!(is_public_ip(IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34))));
        assert!(blocked_ipv4(Ipv4Addr::new(169, 254, 169, 254)));
    }

    #[test]
    fn classifies_text_html_and_rejects_binary_content() {
        assert!(classify_content_type(Some("text/html; charset=utf-8")).is_ok());
        assert!(classify_content_type(Some("application/octet-stream")).is_err());
    }

    #[test]
    fn formats_untrusted_truncated_content_with_source_url() {
        let (output, truncated) = format_fetch_output(
            &Url::parse("https://example.com/article").unwrap(),
            StatusCode::OK,
            "content".into(),
            true,
            200,
        );
        assert!(output.contains("External web content follows"));
        assert!(output.contains("Content truncated"));
        assert!(output.contains("example.com/article"));
        assert!(truncated);
    }

    #[test]
    fn removes_active_html_content_before_markdown_conversion() {
        let (markdown, truncated) = render_body(
            BodyKind::Html,
            "<script>alert(1)</script><div hidden>secret</div><h1>Title</h1><p>Body</p>",
            200,
        )
        .unwrap();
        assert!(!truncated);
        assert!(markdown.contains("# Title"));
        assert!(markdown.contains("Body"));
        assert!(!markdown.contains("alert"));
        assert!(!markdown.contains("secret"));
    }
}
