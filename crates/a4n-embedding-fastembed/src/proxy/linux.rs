use std::process::Command;

use super::model::SystemProxy;
use super::url::proxy_url;

pub(super) fn read_gnome_system_proxy() -> Option<SystemProxy> {
    let mode = gsettings_string("org.gnome.system.proxy", "mode")?;
    if mode != "manual" {
        return None;
    }

    let http_proxy = gsettings_proxy_url("http", "org.gnome.system.proxy.http");
    let mut https_proxy = gsettings_proxy_url("http", "org.gnome.system.proxy.https");
    if https_proxy.is_none()
        && gsettings_bool("org.gnome.system.proxy.http", "use-same-proxy") == Some(true)
    {
        https_proxy = http_proxy.clone();
    }

    Some(SystemProxy {
        http_proxy,
        https_proxy,
        all_proxy: gsettings_proxy_url("socks5h", "org.gnome.system.proxy.socks"),
        no_proxy: gsettings_ignore_hosts("org.gnome.system.proxy", "ignore-hosts"),
    })
}

fn gsettings_proxy_url(scheme: &str, schema: &str) -> Option<String> {
    let host = gsettings_string(schema, "host")?;
    let port = gsettings_u16(schema, "port")?;
    proxy_url(scheme, &host, port)
}

fn gsettings_value(schema: &str, key: &str) -> Option<String> {
    let output = Command::new("gsettings")
        .args(["get", schema, key])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn gsettings_string(schema: &str, key: &str) -> Option<String> {
    gsettings_value(schema, key).map(|value| parse_gsettings_string(&value))
}

fn gsettings_bool(schema: &str, key: &str) -> Option<bool> {
    gsettings_value(schema, key).and_then(|value| value.parse().ok())
}

fn gsettings_u16(schema: &str, key: &str) -> Option<u16> {
    gsettings_value(schema, key).and_then(|value| value.parse().ok())
}

fn gsettings_ignore_hosts(schema: &str, key: &str) -> Option<String> {
    let value = gsettings_value(schema, key)?;
    let hosts = parse_gsettings_array(&value);
    if hosts.is_empty() {
        None
    } else {
        Some(hosts.join(","))
    }
}

pub(crate) fn parse_gsettings_string(value: &str) -> String {
    value
        .trim()
        .strip_prefix('\'')
        .and_then(|value| value.strip_suffix('\''))
        .unwrap_or_else(|| value.trim())
        .to_string()
}

pub(crate) fn parse_gsettings_array(value: &str) -> Vec<String> {
    value
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(parse_gsettings_string)
        .filter(|value| !value.is_empty())
        .collect()
}
