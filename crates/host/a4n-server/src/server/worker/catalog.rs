use super::super::*;

pub(crate) fn catalog_worker_specs(control_addr: SocketAddr) -> anyhow::Result<Vec<WorkerSpec>> {
    a4n_process_catalog::stateful_worker_catalog()?
        .iter()
        .map(|entry| {
            let metadata = stateful_worker_catalog_entry_ref(&entry.name).ok_or_else(|| {
                anyhow::anyhow!("进程目录缺少 {} 的 stateful worker 元数据", entry.name)
            })?;
            let address_value =
                env::var(&metadata.addr_env).unwrap_or_else(|_| metadata.default_addr.clone());
            let address = address_value
                .parse::<SocketAddr>()
                .map_err(|error| anyhow::anyhow!("{} 无效: {error}", metadata.addr_env))?;
            let address = ensure_nonzero_port(&metadata.addr_env, address)?;
            if metadata.local_only && !address.ip().is_loopback() {
                anyhow::bail!("{} 必须绑定 loopback 地址", metadata.addr_env);
            }
            Ok(WorkerSpec {
                metadata,
                binary: child_binary(&managed_worker_env(metadata, "BINARY"), &metadata.launcher)?,
                address,
                control_addr,
                token: worker_token_for_env(&managed_worker_env(metadata, "TOKEN"))?,
            })
        })
        .collect()
}

pub(crate) fn managed_worker_env(
    metadata: &a4n_process_catalog::StatefulWorkerCatalogEntry,
    suffix: &str,
) -> String {
    format!(
        "A4N_{}_{}",
        metadata.name.replace('-', "_").to_ascii_uppercase(),
        suffix
    )
}

pub(crate) fn binary_fingerprint(path: &Path) -> anyhow::Result<BinaryFingerprint> {
    let metadata = std::fs::metadata(path)
        .map_err(|error| anyhow::anyhow!("读取 worker 二进制 {} 失败: {error}", path.display()))?;
    let modified = metadata.modified().map_err(|error| {
        anyhow::anyhow!(
            "读取 worker 二进制 {} 修改时间失败: {error}",
            path.display()
        )
    })?;
    Ok(BinaryFingerprint {
        modified,
        length: metadata.len(),
    })
}

pub(crate) fn optional_binary_fingerprint(
    path: Option<&Path>,
) -> anyhow::Result<Option<BinaryFingerprint>> {
    let Some(path) = path else {
        return Ok(None);
    };
    match std::fs::metadata(path) {
        Ok(metadata) => Ok(Some(BinaryFingerprint {
            modified: metadata.modified().map_err(|error| {
                anyhow::anyhow!(
                    "读取 reload marker {} 修改时间失败: {error}",
                    path.display()
                )
            })?,
            length: metadata.len(),
        })),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(anyhow::anyhow!(
            "读取 reload marker {} 失败: {error}",
            path.display()
        )),
    }
}

pub(crate) fn web_addr() -> anyhow::Result<SocketAddr> {
    env::var("A4N_WEB_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8080".to_string())
        .parse::<SocketAddr>()
        .map_err(|error| anyhow::anyhow!("A4N_WEB_ADDR 无效: {error}"))
}

pub(crate) fn control_addr() -> anyhow::Result<SocketAddr> {
    if let Some(value) = env::var_os("A4N_CONTROL_ADDR") {
        let value = value
            .into_string()
            .map_err(|_| anyhow::anyhow!("A4N_CONTROL_ADDR 必须是有效 UTF-8 地址"))?;
        let address = value
            .parse()
            .map_err(|error| anyhow::anyhow!("A4N_CONTROL_ADDR 无效: {error}"))?;
        ensure_nonzero_port("A4N_CONTROL_ADDR", address)
    } else {
        control_addr_from_web(web_addr()?)
    }
}

pub(crate) fn control_addr_from_web(web_addr: SocketAddr) -> anyhow::Result<SocketAddr> {
    if web_addr.port() == 0 {
        return Ok(SocketAddr::from(([127, 0, 0, 1], 18080)));
    }
    let control_port = web_addr
        .port()
        .checked_add(1)
        .filter(|port| *port != 0)
        .unwrap_or(18080);
    Ok(SocketAddr::from(([127, 0, 0, 1], control_port)))
}

pub(crate) fn ensure_nonzero_port(name: &str, address: SocketAddr) -> anyhow::Result<SocketAddr> {
    if address.port() == 0 {
        anyhow::bail!("{name} 不允许使用端口 0；Edge 必须知道 Control 的固定地址");
    }
    Ok(address)
}

pub(crate) fn control_binary() -> anyhow::Result<PathBuf> {
    child_binary(CONTROL_BINARY_ENV, "a4n-control")
}

pub(crate) fn edge_binary() -> anyhow::Result<PathBuf> {
    child_binary(EDGE_BINARY_ENV, "a4n-edge")
}

pub(crate) fn child_binary(env_name: &str, name: &str) -> anyhow::Result<PathBuf> {
    if let Some(path) = env::var_os(env_name).map(PathBuf::from) {
        if path.is_file() {
            return Ok(path);
        }
        anyhow::bail!("{env_name} 指向的文件不存在: {}", path.display());
    }

    if let Some(path) = sibling_binary(name) {
        return Ok(path);
    }

    anyhow::bail!("未找到 {name}；请设置 {env_name} 或将其放在 a4n-server 同目录")
}

pub(crate) fn sibling_binary(name: &str) -> Option<PathBuf> {
    let executable = env::current_exe().ok()?;
    let parent = executable.parent()?;
    let path = parent.join(executable_name(name));
    path.is_file().then_some(path)
}

#[cfg(windows)]
pub(crate) fn executable_name(name: &str) -> PathBuf {
    format!("{name}.exe").into()
}

#[cfg(not(windows))]
pub(crate) fn executable_name(name: &str) -> PathBuf {
    name.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_environment_names_follow_catalog_entries() {
        let entries =
            a4n_process_catalog::stateful_worker_catalog().expect("stateful worker catalog");
        let workers = entries
            .iter()
            .filter(|worker| worker.name.ends_with("-record-worker"))
            .collect::<Vec<_>>();
        for name in [
            "video-record-worker",
            "manga-record-worker",
            "canvas-record-worker",
        ] {
            assert!(workers.iter().any(|worker| worker.name == name));
        }

        let video = workers
            .iter()
            .find(|worker| worker.name == "video-record-worker")
            .expect("video record worker");
        assert_eq!(
            managed_worker_env(video, "BINARY"),
            "A4N_VIDEO_RECORD_WORKER_BINARY"
        );
        assert_eq!(
            managed_worker_env(video, "TOKEN"),
            "A4N_VIDEO_RECORD_WORKER_TOKEN"
        );
        assert_eq!(
            video.url_env.as_deref().expect("video URL env"),
            "A4N_VIDEO_RECORD_WORKER_URL"
        );
    }
}
