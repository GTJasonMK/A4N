use std::process::Command;

use super::model::SystemProxy;
use super::url::proxy_url;

pub(super) fn read_macos_system_proxy() -> Option<SystemProxy> {
    let output = Command::new("scutil").arg("--proxy").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let http_enabled = scutil_value(&text, "HTTPEnable").is_some_and(|value| value == "1");
    let https_enabled = scutil_value(&text, "HTTPSEnable").is_some_and(|value| value == "1");
    let socks_enabled = scutil_value(&text, "SOCKSEnable").is_some_and(|value| value == "1");

    Some(SystemProxy {
        http_proxy: if http_enabled {
            proxy_url(
                "http",
                &scutil_value(&text, "HTTPProxy").unwrap_or_default(),
                scutil_value(&text, "HTTPPort")
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(0),
            )
        } else {
            None
        },
        https_proxy: if https_enabled {
            proxy_url(
                "http",
                &scutil_value(&text, "HTTPSProxy").unwrap_or_default(),
                scutil_value(&text, "HTTPSPort")
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(0),
            )
        } else {
            None
        },
        all_proxy: if socks_enabled {
            proxy_url(
                "socks5h",
                &scutil_value(&text, "SOCKSProxy").unwrap_or_default(),
                scutil_value(&text, "SOCKSPort")
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(0),
            )
        } else {
            None
        },
        no_proxy: None,
    })
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
