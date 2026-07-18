use std::process::Command;

use super::model::SystemProxy;
use super::url::proxy_url;

pub(super) fn read_windows_system_proxy() -> Option<SystemProxy> {
    let enabled = windows_registry_value("ProxyEnable")
        .is_some_and(|value| value.eq_ignore_ascii_case("0x1") || value == "1");
    if !enabled {
        return None;
    }

    let proxy_server = windows_registry_value("ProxyServer")?;
    let mut proxy = parse_windows_proxy_server(&proxy_server);
    proxy.no_proxy = windows_registry_value("ProxyOverride").and_then(|value| {
        let hosts = value
            .split(';')
            .flat_map(|value| {
                if value.trim().eq_ignore_ascii_case("<local>") {
                    vec!["localhost", "127.0.0.1", "::1"]
                } else {
                    vec![value.trim()]
                }
            })
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>();
        if hosts.is_empty() {
            None
        } else {
            Some(hosts.join(","))
        }
    });
    Some(proxy)
}

fn windows_registry_value(name: &str) -> Option<String> {
    let output = Command::new("reg")
        .args([
            "query",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings",
            "/v",
            name,
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    text.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        let key = parts.next()?;
        if !key.eq_ignore_ascii_case(name) {
            return None;
        }
        let _kind = parts.next()?;
        let value = parts.collect::<Vec<_>>().join(" ");
        (!value.trim().is_empty()).then(|| value.trim().to_string())
    })
}

fn parse_windows_proxy_server(value: &str) -> SystemProxy {
    let mut proxy = SystemProxy::default();
    if value.contains('=') {
        for item in value.split(';') {
            let Some((kind, address)) = item.split_once('=') else {
                continue;
            };
            match kind.trim().to_ascii_lowercase().as_str() {
                "http" => proxy.http_proxy = proxy_url_from_host_port("http", address),
                "https" => proxy.https_proxy = proxy_url_from_host_port("http", address),
                "socks" => proxy.all_proxy = proxy_url_from_host_port("socks5h", address),
                _ => {}
            }
        }
    } else {
        let url = proxy_url_from_host_port("http", value);
        proxy.http_proxy = url.clone();
        proxy.https_proxy = url;
    }
    proxy
}

fn proxy_url_from_host_port(scheme: &str, value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    if value.contains("://") {
        return Some(value.to_string());
    }
    let (host, port) = value.rsplit_once(':')?;
    let port = port.parse().ok()?;
    proxy_url(scheme, host, port)
}
