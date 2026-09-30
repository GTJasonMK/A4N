use std::{env, net::SocketAddr};

const WORKER_ID_ENV: &str = "A4N_AGENT_WORKER_ID";

pub(super) fn control_addr() -> anyhow::Result<SocketAddr> {
    if let Some(value) = env::var_os("A4N_CONTROL_ADDR") {
        let value = value
            .into_string()
            .map_err(|_| anyhow::anyhow!("A4N_CONTROL_ADDR 必须是有效 UTF-8 地址"))?;
        let address = value
            .parse()
            .map_err(|error| anyhow::anyhow!("A4N_CONTROL_ADDR 无效: {error}"))?;
        return ensure_nonzero_port(address);
    }
    let web_addr = env::var("A4N_WEB_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8080".into())
        .parse::<SocketAddr>()
        .map_err(|error| anyhow::anyhow!("A4N_WEB_ADDR 无效: {error}"))?;
    let port = web_addr
        .port()
        .checked_add(1)
        .filter(|port| *port != 0)
        .unwrap_or(18080);
    ensure_nonzero_port(format!("127.0.0.1:{port}").parse()?)
}

fn ensure_nonzero_port(address: SocketAddr) -> anyhow::Result<SocketAddr> {
    if address.port() == 0 {
        anyhow::bail!("A4N_CONTROL_ADDR 不允许使用端口 0");
    }
    Ok(address)
}

pub(super) fn worker_id() -> anyhow::Result<String> {
    match a4n_config::optional_non_empty_env_string(WORKER_ID_ENV)? {
        Some(value) => Ok(value),
        None => Ok(format!("a4n-agent:{}", std::process::id())),
    }
}
