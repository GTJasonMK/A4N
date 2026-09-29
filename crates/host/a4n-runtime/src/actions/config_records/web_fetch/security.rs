use a4n_error::AppError;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use url::Url;

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

pub(super) async fn resolve_public_address(host: &str, port: u16) -> Result<SocketAddr, AppError> {
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

pub(super) fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => !blocked_ipv4(ip),
        IpAddr::V6(ip) => !blocked_ipv6(ip),
    }
}

pub(super) fn blocked_ipv4(ip: Ipv4Addr) -> bool {
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

pub(super) fn validate_redirect(current: &Url, next: &Url) -> Result<(), AppError> {
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
