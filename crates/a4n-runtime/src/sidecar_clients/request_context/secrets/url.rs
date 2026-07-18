use std::net::IpAddr;

pub(in crate::sidecar_clients::request_context) fn sidecar_url_allows_local_secrets(
    sidecar_url: &str,
) -> bool {
    sidecar_url_scheme_and_host(sidecar_url).is_some_and(|(scheme, host)| {
        matches!(scheme.as_str(), "http" | "https") && loopback_host_allows_local_secrets(&host)
    })
}

fn sidecar_url_scheme_and_host(sidecar_url: &str) -> Option<(String, String)> {
    let value = sidecar_url.trim();
    let (scheme, after_scheme) = value.split_once("://")?;
    let scheme = scheme.trim().to_ascii_lowercase();
    if scheme.is_empty() {
        return None;
    }
    let authority = after_scheme.split('/').next().unwrap_or(after_scheme);
    let host_port = authority.rsplit('@').next().unwrap_or(authority).trim();
    if host_port.is_empty() {
        return None;
    }
    if let Some(rest) = host_port.strip_prefix('[') {
        let (host, tail) = rest.split_once(']')?;
        if !valid_optional_port_suffix(tail) {
            return None;
        }
        let host = host.trim();
        return (!host.is_empty()).then(|| (scheme, host.to_ascii_lowercase()));
    }
    let (host, port_suffix) = host_port
        .split_once(':')
        .map(|(host, port)| (host, Some(port)))
        .unwrap_or((host_port, None));
    if let Some(port) = port_suffix
        && !valid_port(port)
    {
        return None;
    }
    let host = host.trim().trim_end_matches('.');
    (!host.is_empty()).then(|| (scheme, host.to_ascii_lowercase()))
}

fn valid_optional_port_suffix(value: &str) -> bool {
    if value.is_empty() {
        return true;
    }
    value.strip_prefix(':').is_some_and(valid_port)
}

fn valid_port(value: &str) -> bool {
    !value.is_empty() && value.parse::<u16>().is_ok()
}

fn loopback_host_allows_local_secrets(host: &str) -> bool {
    host == "localhost"
        || host.ends_with(".localhost")
        || host
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback())
}
