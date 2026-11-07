use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::Duration;

use a4n_api_contract::command_manifest::HOST_IDENTITY;
use a4n_process_catalog::service_catalog;
use a4n_service_protocol::{MANAGED_SIDECARS_ENV, RELEASE_MANIFEST_ENV, SIDECAR_BIN_DIR_ENV};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use tokio::net::TcpStream;
use url::Url;

mod runtime;
mod state;

use runtime::*;
use state::{
    DEFAULT_QDRANT_URL, LOCAL_STATE_SCHEMA_VERSION, LocalPaths, LocalState, LocalStateInspection,
};
pub(crate) use state::{DoctorOutcome, StartOptions};

use crate::config::{lock_config, write_private_json_unlocked};

pub(crate) async fn doctor(
    api_base: &str,
    config_path: &Path,
    strict: bool,
) -> Result<DoctorOutcome> {
    let mut checks = Vec::new();
    let mut errors = 0_u32;
    let mut warnings = 0_u32;
    let paths = LocalPaths::new(config_path);
    let local_state = load_state(&paths.state)?;
    let local_inspection = match local_state.as_ref() {
        Some(state) => Some(inspect_local_state(state).await),
        None => None,
    };

    match std::env::var("A4N_DATABASE_URL") {
        Ok(value) if !value.trim().is_empty() => match database_endpoint(&value) {
            Ok(endpoint) => {
                if tcp_reachable(endpoint, Duration::from_secs(2)).await {
                    push_check(&mut checks, "postgres", "ok", format!("{endpoint} 可连接"));
                } else {
                    errors += 1;
                    push_check(
                        &mut checks,
                        "postgres",
                        "error",
                        format!("{endpoint} 不可连接；CLI 不会自动启动 PostgreSQL/Docker"),
                    );
                }
            }
            Err(error) => {
                errors += 1;
                push_check(&mut checks, "postgres", "error", error.to_string());
            }
        },
        _ => {
            errors += 1;
            push_check(&mut checks, "postgres", "error", "缺少 A4N_DATABASE_URL");
        }
    }

    if std::env::var("A4N_JWT_SECRET").is_ok_and(|value| !value.trim().is_empty()) {
        push_check(&mut checks, "jwt", "ok", "A4N_JWT_SECRET 已设置");
    } else {
        errors += 1;
        push_check(&mut checks, "jwt", "error", "缺少 A4N_JWT_SECRET");
    }

    let configured_server_healthy = match (local_state.as_ref(), local_inspection.as_ref()) {
        (Some(state), Some(inspection)) if state.server_url == api_base => {
            inspection.server_healthy
        }
        _ => server_healthy(api_base).await,
    };
    if configured_server_healthy {
        push_check(&mut checks, "server", "ok", format!("{api_base} 可用"));
    } else {
        warnings += 1;
        push_check(
            &mut checks,
            "server",
            "warning",
            format!("{api_base} 尚未启动"),
        );
    }

    match source_root()? {
        Some(root) => {
            push_check(
                &mut checks,
                "runtime-binaries",
                "ok",
                format!("可从 workspace 构建: {}", root.display()),
            );
        }
        None if sibling_binary("a4n-server").is_some()
            && sibling_binary("a4n-migration").is_some() =>
        {
            push_check(
                &mut checks,
                "runtime-binaries",
                "ok",
                "a4n-server 与 a4n-migration 位于 CLI 同目录",
            );
        }
        None => {
            errors += 1;
            push_check(
                &mut checks,
                "runtime-binaries",
                "error",
                "未找到 workspace 或 CLI 同目录的 a4n-server/a4n-migration",
            );
        }
    }

    if let Some(url) = a4n_config::optional_non_empty_env_string("A4N_QDRANT_URL")? {
        match endpoint_from_url(&url) {
            Ok(endpoint) if tcp_reachable(endpoint, Duration::from_secs(2)).await => {
                push_check(&mut checks, "qdrant", "ok", format!("{endpoint} 可连接"));
            }
            Ok(endpoint) => {
                warnings += 1;
                push_check(
                    &mut checks,
                    "qdrant",
                    "warning",
                    format!("{endpoint} 不可连接"),
                );
            }
            Err(error) => {
                errors += 1;
                push_check(&mut checks, "qdrant", "error", error.to_string());
            }
        }
    } else {
        warnings += 1;
        push_check(
            &mut checks,
            "qdrant",
            "warning",
            "A4N_QDRANT_URL 未设置；需要 RAG 时 local start 会尝试启动已安装 Qdrant",
        );
    }

    match (local_state.as_ref(), local_inspection.as_ref()) {
        (Some(state), Some(inspection)) => {
            let status = inspection.status();
            match status {
                "error" => errors += 1,
                "warning" => warnings += 1,
                _ => {}
            }
            push_check(
                &mut checks,
                "local-state",
                status,
                json!({
                    "owned": true,
                    "server_pid": state.server_pid,
                    "server_process_alive": inspection.server_process_alive,
                    "server_identity_matches": inspection.server_identity_matches,
                    "server_healthy": inspection.server_healthy,
                    "server": state.server_url,
                    "qdrant_pid": state.qdrant_pid,
                    "qdrant_process_alive": inspection.qdrant_process_alive,
                    "qdrant_identity_matches": inspection.qdrant_identity_matches,
                }),
            );
        }
        (None, None) => push_check(&mut checks, "local-state", "ok", json!({ "owned": false })),
        _ => unreachable!("local state and inspection must have matching presence"),
    }

    Ok(DoctorOutcome {
        report: json!({
            "healthy": errors == 0 && (!strict || warnings == 0),
            "strict": strict,
            "errors": errors,
            "warnings": warnings,
            "checks": checks,
        }),
        healthy: errors == 0 && (!strict || warnings == 0),
    })
}

pub(crate) async fn start(
    api_base: &str,
    config_path: &Path,
    options: StartOptions,
) -> Result<Value> {
    let endpoint = server_endpoint(api_base)?;
    let paths = LocalPaths::new(config_path);
    let _state_lock = lock_config(&paths.state)?;
    if let Some(state) = load_state_unlocked(&paths.state)? {
        if process_alive(state.server_pid) {
            if !process_matches(state.server_pid, &state.server_marker) {
                bail!(
                    "local state 中的 Server PID {} 已被其他进程占用；拒绝覆盖 owned state",
                    state.server_pid
                );
            }
            if state.server_url != api_base {
                bail!(
                    "local state 已拥有其他 Server 地址 {}；请先执行 local stop",
                    state.server_url
                );
            }
            if !server_healthy(&state.server_url).await {
                bail!(
                    "local state 中的 Server 进程仍存在但未通过 A4N 健康检查；请查看日志 {} 后执行 local stop",
                    state.server_log.display()
                );
            }
            if let (Some(pid), Some(marker)) = (state.qdrant_pid, state.qdrant_marker.as_deref())
                && (!process_alive(pid) || !process_matches(pid, marker))
            {
                bail!("local state 中的 Qdrant 进程已失效；请执行 local stop 后重试");
            }
            return Ok(json!({
                "running": true,
                "owned": true,
                "already_running": true,
                "server_pid": state.server_pid,
                "server": state.server_url,
                "log": state.server_log,
            }));
        }
        stop_state(&paths, &state, false).await?;
    }
    if server_healthy(api_base).await {
        return Ok(json!({
            "running": true,
            "owned": false,
            "already_running": true,
            "server": api_base,
        }));
    }

    let database_url = required_env("A4N_DATABASE_URL")?;
    required_env("A4N_JWT_SECRET")?;
    let source_root = source_root()?;
    validate_start_source_workspace(options.full, source_root.as_deref())?;
    run_migration(&database_url, source_root.as_deref())?;

    let server_binary = resolve_or_build_binary(
        "a4n-server",
        source_root.as_deref(),
        if options.full {
            Some("embedded-capabilities")
        } else {
            None
        },
    )?;
    let sidecar_bin_dir = if options.managed_sidecars {
        Some(resolve_or_build_sidecars(source_root.as_deref())?)
    } else {
        None
    };

    let mut qdrant_pid = None;
    let mut qdrant_marker = None;
    let mut qdrant_url = a4n_config::optional_non_empty_env_string("A4N_QDRANT_URL")?;
    if options.qdrant && qdrant_url.is_none() {
        qdrant_url = Some(DEFAULT_QDRANT_URL.into());
    }
    if options.qdrant {
        let endpoint = endpoint_from_url(
            qdrant_url
                .as_deref()
                .expect("Qdrant URL must exist when local Qdrant is enabled"),
        )?;
        if !tcp_reachable(endpoint, Duration::from_secs(2)).await {
            if !endpoint.ip().is_loopback() {
                bail!("远程 Qdrant {endpoint} 不可连接");
            }
            let qdrant_binary = source_root
                .as_deref()
                .map(|root| {
                    root.join("tools/qdrant/bin")
                        .join(executable_name("qdrant"))
                })
                .filter(|path| path.is_file())
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "需要本地 Qdrant，但未安装；请运行 ./scripts/dev/setup-qdrant.sh 或设置 A4N_QDRANT_URL"
                    )
                })?;
            let marker = binary_marker(&qdrant_binary)?;
            let mut child = spawn_qdrant(&qdrant_binary, endpoint, &paths)?;
            qdrant_pid = Some(child.id());
            qdrant_marker = Some(marker);
            qdrant_url = Some(format!("http://{endpoint}"));
            if !wait_tcp(endpoint, &mut child, Duration::from_secs(20)).await {
                stop_owned_processes(
                    None,
                    Some((
                        child.id(),
                        qdrant_marker
                            .as_deref()
                            .expect("spawned Qdrant must have a process marker"),
                    )),
                    true,
                )
                .await
                .context("Qdrant 启动失败后清理 owned 进程失败")?;
                bail!("Qdrant 启动超时；日志: {}", paths.qdrant_log.display());
            }
        }
    } else if options.managed_sidecars || options.full {
        let url = qdrant_url.as_deref().ok_or_else(|| {
            anyhow::anyhow!(
                "managed/full profile 使用 --no-qdrant 时必须设置可连接的 A4N_QDRANT_URL"
            )
        })?;
        let endpoint = endpoint_from_url(url)?;
        if !tcp_reachable(endpoint, Duration::from_secs(2)).await {
            bail!("A4N_QDRANT_URL {endpoint} 不可连接");
        }
    }

    let server_marker = binary_marker(&server_binary)?;
    let mut child = match spawn_server(
        &server_binary,
        endpoint,
        source_root.as_deref(),
        &paths,
        options,
        sidecar_bin_dir.as_deref(),
        qdrant_url.as_deref(),
    ) {
        Ok(child) => child,
        Err(error) => {
            if let (Some(pid), Some(marker)) = (qdrant_pid, qdrant_marker.as_deref())
                && let Err(cleanup_error) =
                    stop_owned_processes(None, Some((pid, marker)), true).await
            {
                bail!("启动 A4N Server 失败: {error:#}；同时清理 Qdrant 失败: {cleanup_error:#}");
            }
            return Err(error);
        }
    };
    let state = LocalState {
        schema_version: LOCAL_STATE_SCHEMA_VERSION,
        server_pid: child.id(),
        server_marker,
        server_url: api_base.to_string(),
        server_log: paths.server_log.clone(),
        qdrant_pid,
        qdrant_marker,
        qdrant_log: qdrant_pid.map(|_| paths.qdrant_log.clone()),
    };
    if let Err(error) = write_private_json_unlocked(&paths.state, &state) {
        if let Err(cleanup_error) = stop_owned_processes(
            Some((state.server_pid, &state.server_marker)),
            state.qdrant_pid.zip(state.qdrant_marker.as_deref()),
            true,
        )
        .await
        {
            bail!("写入 local state 失败: {error:#}；同时清理 owned 进程失败: {cleanup_error:#}");
        }
        return Err(error);
    }

    if !wait_server(api_base, &mut child, Duration::from_secs(60)).await {
        if let Err(cleanup_error) = stop_owned_processes(
            Some((state.server_pid, &state.server_marker)),
            state.qdrant_pid.zip(state.qdrant_marker.as_deref()),
            true,
        )
        .await
        {
            bail!(
                "A4N Server 启动失败或超时；日志: {}；owned state 已保留，清理进程失败: {cleanup_error:#}",
                paths.server_log.display()
            );
        }
        if let Err(cleanup_error) = fs::remove_file(&paths.state)
            .with_context(|| format!("删除失败的 local state: {}", paths.state.display()))
        {
            bail!(
                "A4N Server 启动失败或超时；日志: {}；同时清理 local state 失败: {cleanup_error:#}",
                paths.server_log.display()
            );
        }
        bail!(
            "A4N Server 启动失败或超时；日志: {}",
            paths.server_log.display()
        );
    }

    Ok(json!({
        "running": true,
        "owned": true,
        "server": api_base,
        "server_pid": state.server_pid,
        "server_log": state.server_log,
        "qdrant_pid": state.qdrant_pid,
        "qdrant_log": state.qdrant_log,
        "managed_sidecars": options.managed_sidecars,
        "profile": if options.full { "full" } else { "sidecar" },
    }))
}

pub(crate) async fn status(api_base: &str, config_path: &Path) -> Result<Value> {
    let paths = LocalPaths::new(config_path);
    let state = load_state(&paths.state)?;
    Ok(match state {
        Some(state) => {
            let inspection = inspect_local_state(&state).await;
            json!({
                "running": inspection.running(),
                "owned": true,
                "server": state.server_url,
                "server_pid": state.server_pid,
                "server_process_alive": inspection.server_process_alive,
                "server_identity_matches": inspection.server_identity_matches,
                "server_log": state.server_log,
                "qdrant_pid": state.qdrant_pid,
                "qdrant_process_alive": inspection.qdrant_process_alive,
                "qdrant_identity_matches": inspection.qdrant_identity_matches,
                "qdrant_log": state.qdrant_log,
                "state_file": paths.state,
            })
        }
        None => json!({
            "running": server_healthy(api_base).await,
            "owned": false,
            "server": api_base,
            "state_file": paths.state,
        }),
    })
}

pub(crate) async fn stop(config_path: &Path, force: bool) -> Result<Value> {
    let paths = LocalPaths::new(config_path);
    let _state_lock = lock_config(&paths.state)?;
    let Some(state) = load_state_unlocked(&paths.state)? else {
        return Ok(json!({ "stopped": true, "owned": false, "already_stopped": true }));
    };
    stop_state(&paths, &state, force).await?;
    Ok(json!({
        "stopped": true,
        "owned": true,
        "server_pid": state.server_pid,
        "qdrant_pid": state.qdrant_pid,
        "state_removed": true,
    }))
}

async fn stop_state(paths: &LocalPaths, state: &LocalState, force: bool) -> Result<()> {
    stop_owned_processes(
        Some((state.server_pid, &state.server_marker)),
        state.qdrant_pid.zip(state.qdrant_marker.as_deref()),
        force,
    )
    .await?;
    fs::remove_file(&paths.state)
        .with_context(|| format!("删除 local state 失败: {}", paths.state.display()))?;
    Ok(())
}

async fn stop_owned_processes(
    server: Option<(u32, &str)>,
    qdrant: Option<(u32, &str)>,
    force: bool,
) -> Result<()> {
    let mut failures = Vec::new();
    for (name, process) in [("Server", server), ("Qdrant", qdrant)] {
        let Some((pid, marker)) = process else {
            continue;
        };
        if process_alive(pid)
            && let Err(error) = terminate_owned(pid, marker, force).await
        {
            failures.push(format!("{name} PID {pid}: {error:#}"));
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        bail!(failures.join("；"))
    }
}

pub(crate) fn logs(config_path: &Path, lines: usize) -> Result<Value> {
    if !(1..=10_000).contains(&lines) {
        bail!("--lines 必须在 1 到 10000 之间");
    }
    let paths = LocalPaths::new(config_path);
    Ok(json!({
        "server_log": paths.server_log,
        "server": tail_lines(&paths.server_log, lines)?,
        "qdrant_log": paths.qdrant_log,
        "qdrant": tail_lines(&paths.qdrant_log, lines)?,
    }))
}

pub(crate) fn migrate() -> Result<Value> {
    let database_url = required_env("A4N_DATABASE_URL")?;
    let source_root = source_root()?;
    run_migration(&database_url, source_root.as_deref())?;
    Ok(json!({ "migrated": true }))
}

fn validate_start_source_workspace(full: bool, source_root: Option<&Path>) -> Result<()> {
    if full && source_root.is_none() {
        bail!("local start --full 需要 A4N 源码 workspace；发布包请使用默认 sidecar profile");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::process::Command;

    use tempfile::TempDir;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    use super::{
        LocalPaths, LocalState, LocalStateInspection, StartOptions,
        configure_onnxruntime_environment, database_endpoint, endpoint_from_url,
        load_state_unlocked, log_stdio, non_empty_command_line, onnxruntime_library_name,
        qdrant_http_port, runtime_loader_path_var, server_endpoint, server_healthy, spawn_server,
        stop, tail_lines, validate_start_source_workspace,
    };
    use crate::local::MANAGED_SIDECARS_ENV;

    #[test]
    fn full_local_start_requires_source_workspace() {
        let error = validate_start_source_workspace(true, None).unwrap_err();
        assert_eq!(
            error.to_string(),
            "local start --full 需要 A4N 源码 workspace；发布包请使用默认 sidecar profile"
        );
        assert!(validate_start_source_workspace(false, None).is_ok());
        assert!(validate_start_source_workspace(true, Some(Path::new("workspace"))).is_ok());
    }

    #[test]
    fn local_endpoints_reject_remote_server_and_parse_postgres() {
        assert!(server_endpoint("https://example.com/api").is_err());
        assert!(server_endpoint("https://127.0.0.1:8080/api").is_err());
        assert_eq!(
            server_endpoint("http://127.0.0.1:8080/api").unwrap().port(),
            8080
        );
        assert_eq!(
            database_endpoint("postgres://user:pass@127.0.0.1/db")
                .unwrap()
                .port(),
            5432
        );
        assert!(endpoint_from_url("ftp://127.0.0.1:6334").is_err());
        assert!(endpoint_from_url("http://user:secret@127.0.0.1:6334").is_err());
        assert!(endpoint_from_url("http://127.0.0.1:6334?token=secret").is_err());
        assert_eq!(
            endpoint_from_url("http://127.0.0.1:6334").unwrap().port(),
            6334
        );
    }

    #[test]
    fn owned_local_state_reports_stale_and_mismatched_processes() {
        let status = |server_process_alive,
                      server_identity_matches,
                      server_healthy,
                      qdrant_owned,
                      qdrant_process_alive,
                      qdrant_identity_matches| {
            LocalStateInspection {
                server_process_alive,
                server_identity_matches,
                server_healthy,
                qdrant_owned,
                qdrant_process_alive,
                qdrant_identity_matches,
            }
            .status()
        };
        assert_eq!(status(true, true, true, false, false, false), "ok");
        assert_eq!(status(true, true, true, true, true, true), "ok");
        assert_eq!(status(false, false, false, false, false, false), "warning");
        assert_eq!(status(true, false, false, false, false, false), "error");
        assert_eq!(status(true, true, true, true, false, false), "warning");
        assert_eq!(status(true, true, true, true, true, false), "error");
    }

    #[test]
    fn qdrant_http_port_rejects_saturating_fallbacks() {
        assert_eq!(qdrant_http_port(6334).unwrap(), 6333);
        assert!(qdrant_http_port(1).is_err());
        assert!(qdrant_http_port(0).is_err());
    }

    #[test]
    fn blank_process_output_is_not_treated_as_a_live_process() {
        assert!(non_empty_command_line(b"\r\n").is_none());
        assert_eq!(
            non_empty_command_line(b"a4n-server\0--flag\0").as_deref(),
            Some("a4n-server --flag ")
        );
    }

    #[tokio::test]
    async fn server_health_requires_the_a4n_meta_identity() {
        let (server, task) = meta_server("other-service").await;
        assert!(!server_healthy(&server).await);
        task.await.unwrap();

        let (server, task) = meta_server("a4n-server").await;
        assert!(server_healthy(&server).await);
        task.await.unwrap();
    }

    #[test]
    fn bundled_onnxruntime_is_added_to_the_server_loader_path() {
        let directory = TempDir::new().unwrap();
        std::fs::write(
            directory.path().join(onnxruntime_library_name()),
            b"runtime",
        )
        .unwrap();
        let mut command = Command::new("a4n-server");
        configure_onnxruntime_environment(&mut command, Some(directory.path())).unwrap();
        let loader_path = command
            .get_envs()
            .find(|(key, _)| *key == runtime_loader_path_var())
            .and_then(|(_, value)| value)
            .unwrap();
        assert_eq!(
            std::env::split_paths(loader_path).next(),
            Some(directory.path().to_path_buf())
        );
    }

    #[test]
    fn log_tail_handles_utf8_truncation_and_logs_remain_private() {
        const MAX_BYTES: usize = 2 * 1024 * 1024;
        let directory = TempDir::new().unwrap();
        let path = directory.path().join("server.log");
        let prefix = "头\n".as_bytes();
        let suffix = b"\nkeep\nlast\n";
        let mut contents = prefix.to_vec();
        contents.extend(std::iter::repeat_n(
            b'x',
            MAX_BYTES + 1 - prefix.len() - suffix.len(),
        ));
        contents.extend_from_slice(suffix);
        std::fs::write(&path, contents).unwrap();

        assert_eq!(tail_lines(&path, 2).unwrap(), ["keep", "last"]);

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        }
        drop(log_stdio(&path).unwrap());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }

    #[test]
    fn local_state_rejects_schema_unknown_fields_and_partial_qdrant_identity() {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join("local.json");
        let valid = serde_json::json!({
            "schema_version": 1,
            "server_pid": 123,
            "server_marker": "a4n-server",
            "server_url": "http://127.0.0.1:8080/api",
            "server_log": directory.path().join("server.log"),
            "qdrant_pid": null,
            "qdrant_marker": null,
            "qdrant_log": null
        });

        fs::write(&path, serde_json::to_vec(&valid).unwrap()).unwrap();
        assert!(load_state_unlocked(&path).unwrap().is_some());

        let mut invalid_schema = valid.clone();
        invalid_schema["schema_version"] = serde_json::json!(99);
        fs::write(&path, serde_json::to_vec(&invalid_schema).unwrap()).unwrap();
        assert!(load_state_unlocked(&path).is_err());

        let mut unknown = valid.clone();
        unknown["unexpected"] = serde_json::json!(true);
        fs::write(&path, serde_json::to_vec(&unknown).unwrap()).unwrap();
        assert!(load_state_unlocked(&path).is_err());

        let mut partial_qdrant = valid;
        partial_qdrant["qdrant_pid"] = serde_json::json!(1234);
        fs::write(&path, serde_json::to_vec(&partial_qdrant).unwrap()).unwrap();
        assert!(load_state_unlocked(&path).is_err());
    }

    #[tokio::test]
    async fn stale_state_is_removed_but_pid_identity_mismatch_is_never_killed() {
        let directory = TempDir::new().unwrap();
        let config = directory.path().join("cli.json");
        let paths = LocalPaths::new(&config);
        let stale = LocalState {
            schema_version: 1,
            server_pid: u32::MAX,
            server_marker: "a4n-server".into(),
            server_url: "http://127.0.0.1:8080/api".into(),
            server_log: paths.server_log.clone(),
            qdrant_pid: None,
            qdrant_marker: None,
            qdrant_log: None,
        };
        fs::write(&paths.state, serde_json::to_vec(&stale).unwrap()).unwrap();
        let stopped = stop(&config, false).await.unwrap();
        assert_eq!(stopped["state_removed"], true);
        assert!(!paths.state.exists());

        let occupied = LocalState {
            server_pid: std::process::id(),
            server_marker: "marker-that-cannot-match-this-test-process".into(),
            ..stale
        };
        fs::write(&paths.state, serde_json::to_vec(&occupied).unwrap()).unwrap();
        let error = stop(&config, true).await.unwrap_err();
        assert!(format!("{error:#}").contains("拒绝终止"));
        assert!(paths.state.exists());
    }

    #[cfg(unix)]
    #[test]
    fn start_profiles_pass_expected_server_environment() {
        use std::os::unix::fs::PermissionsExt;

        let directory = TempDir::new().unwrap();
        let script = directory.path().join("fake-server.sh");
        let sidecars = directory.path().join("sidecars");
        fs::create_dir_all(&sidecars).unwrap();
        fs::write(sidecars.join(onnxruntime_library_name()), b"runtime").unwrap();
        let script_body = |env_path: &Path| format!("#!/bin/sh\nenv > '{}'\n", env_path.display());

        for (name, options, qdrant_url, expected_profile, expected_managed) in [
            (
                "managed",
                StartOptions {
                    managed_sidecars: true,
                    qdrant: true,
                    full: false,
                },
                Some("http://127.0.0.1:6334"),
                "sidecar",
                "1",
            ),
            (
                "external",
                StartOptions {
                    managed_sidecars: false,
                    qdrant: false,
                    full: false,
                },
                None,
                "sidecar",
                "0",
            ),
            (
                "external-qdrant",
                StartOptions {
                    managed_sidecars: true,
                    qdrant: false,
                    full: false,
                },
                Some("http://127.0.0.1:6334"),
                "sidecar",
                "1",
            ),
            (
                "full",
                StartOptions {
                    managed_sidecars: false,
                    qdrant: true,
                    full: true,
                },
                Some("http://127.0.0.1:6334"),
                "full",
                "0",
            ),
        ] {
            let env_path = directory.path().join(format!("{name}.env"));
            fs::write(&script, script_body(&env_path)).unwrap();
            fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
            let paths = LocalPaths::new(&directory.path().join(format!("{name}.json")));
            let mut child = spawn_server(
                &script,
                "127.0.0.1:39000".parse().unwrap(),
                None,
                &paths,
                options,
                Some(&sidecars),
                qdrant_url,
            )
            .unwrap();
            let status = child.wait().unwrap();
            assert!(status.success());
            let env = fs::read_to_string(env_path).unwrap();
            assert!(env.contains(&format!("A4N_RUNTIME_PROFILE={expected_profile}")));
            assert!(env.contains(&format!("{MANAGED_SIDECARS_ENV}={expected_managed}")));
            match qdrant_url {
                Some(url) => assert!(env.contains(&format!("A4N_QDRANT_URL={url}"))),
                None => {
                    assert!(!env.contains("A4N_QDRANT_URL="));
                    assert!(!env.contains("A4N_QDRANT_COLLECTION="));
                }
            }
            if options.managed_sidecars || options.full {
                assert!(env.contains("ORT_LIB_PATH="));
                assert!(env.contains(&format!("{}=", runtime_loader_path_var())));
            }
        }
    }

    async fn meta_server(host: &'static str) -> (String, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0_u8; 1024];
            let read = stream.read(&mut request).await.unwrap();
            assert!(String::from_utf8_lossy(&request[..read]).starts_with("GET /api/meta "));
            let body = format!(r#"{{"host":"{host}"}}"#);
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).await.unwrap();
        });
        (format!("http://{address}/api"), task)
    }
}
