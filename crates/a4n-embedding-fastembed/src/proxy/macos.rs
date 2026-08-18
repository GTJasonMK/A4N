use std::process::Command;

use super::model::SystemProxy;
use super::url::proxy_url;

pub(super) fn read_macos_system_proxy() -> Option<SystemProxy> {
    let output = Command::new("scutil").arg("--proxy").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);

    Some(SystemProxy {
        http_proxy: scutil_proxy_url(&text, "HTTPEnable", "HTTPProxy", "HTTPPort", "http"),
        https_proxy: scutil_proxy_url(
            &text,
            "HTTPSEnable",
            "HTTPSProxy",
            "HTTPSPort",
            "http",
        ),
        all_proxy: scutil_proxy_url(
            &text,
            "SOCKSEnable",
            "SOCKSProxy",
            "SOCKSPort",
            "socks5h",
        ),
        no_proxy: None,
    })
}

fn scutil_proxy_url(
    text: &str,
    enabled_key: &str,
    host_key: &str,
    port_key: &str,
    scheme: &str,
) -> Option<String> {
    if scutil_value(text, enabled_key).as_deref() != Some("1") {
        return None;
    }
    let host = scutil_value(text, host_key)?;
    let port = scutil_value(text, port_key)?.parse().ok()?;
    proxy_url(scheme, &host, port)
}

fn scutil_value(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let (left, right) = line.split_once(':')?;
        if left.trim() == key {
            Some(right.trim().to_string())
        } else {
            None
        }
    })
}
