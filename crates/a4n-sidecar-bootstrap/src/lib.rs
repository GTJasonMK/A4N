//! Shared process bootstrap rules for A4N sidecar binaries.

mod event_stream;

use std::{env::VarError, net::SocketAddr};

use anyhow::{Context, anyhow};
use tracing_subscriber::EnvFilter;

pub use event_stream::{
    service_event_line, service_event_ndjson_response, single_service_event_stream,
};

pub fn init_tracing(log_env: &str, default_filter: &str) -> anyhow::Result<()> {
    let filter_text = env_value_or_default(log_env, default_filter)?;
    let filter = EnvFilter::try_new(&filter_text)
        .with_context(|| format!("{log_env} 不是有效日志过滤表达式"))?;
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .try_init()
        .map_err(|error| anyhow!("初始化 sidecar 日志系统失败: {error}"))?;
    Ok(())
}

pub fn socket_addr(addr_env: &str, default_addr: &str) -> anyhow::Result<SocketAddr> {
    let value = env_value_or_default(addr_env, default_addr)?;
    value
        .parse()
        .map_err(|error| anyhow!("{addr_env} 不是有效监听地址 {value:?}: {error}"))
}

fn env_value_or_default(key: &str, default_value: &str) -> anyhow::Result<String> {
    resolve_env_value(key, std::env::var(key), default_value).map_err(anyhow::Error::msg)
}

fn resolve_env_value(
    key: &str,
    configured: Result<String, VarError>,
    default_value: &str,
) -> Result<String, String> {
    let value = match configured {
        Ok(value) => value,
        Err(VarError::NotPresent) => default_value.to_string(),
        Err(error) => return Err(format!("读取 {key} 失败: {error}")),
    };
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{key} 不能为空"));
    }
    Ok(value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_value_uses_default() {
        assert_eq!(
            resolve_env_value("A4N_SERVICE_ADDR", Err(VarError::NotPresent), "127.0.0.1:1")
                .unwrap(),
            "127.0.0.1:1"
        );
    }

    #[test]
    fn configured_value_is_trimmed() {
        assert_eq!(
            resolve_env_value("A4N_SERVICE_ADDR", Ok(" 127.0.0.1:2 ".into()), "unused").unwrap(),
            "127.0.0.1:2"
        );
    }

    #[test]
    fn configured_blank_value_is_rejected() {
        let error = resolve_env_value("A4N_SERVICE_ADDR", Ok("  ".into()), "127.0.0.1:1")
            .expect_err("configured blank value must fail");
        assert!(error.contains("不能为空"));
    }

    #[test]
    fn invalid_socket_addr_is_rejected() {
        let value =
            resolve_env_value("A4N_SERVICE_ADDR", Ok("not-an-address".into()), "unused").unwrap();
        let error = value
            .parse::<SocketAddr>()
            .expect_err("invalid address must fail");
        assert!(!error.to_string().is_empty());
    }
}
