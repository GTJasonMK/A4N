use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::Duration;

use a4n_api_contract::command_manifest::HOST_IDENTITY;
use a4n_service_protocol::{MANAGED_SIDECARS_ENV, RELEASE_MANIFEST_ENV, SIDECAR_BIN_DIR_ENV};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::net::TcpStream;
use url::Url;

use crate::config::{
    lock_config, non_empty_env_path, non_empty_env_string, write_private_json_unlocked,
};

const LOCAL_STATE_SCHEMA_VERSION: u32 = 1;
const DEFAULT_QDRANT_URL: &str = "http://127.0.0.1:6334";

#[derive(Clone, Copy)]
pub(crate) struct StartOptions {
    pub(crate) managed_sidecars: bool,
    pub(crate) qdrant: bool,
    pub(crate) full: bool,
}

pub(crate) struct DoctorOutcome {
    pub(crate) report: Value,
    pub(crate) healthy: bool,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct LocalState {
    schema_version: u32,
    server_pid: u32,
    server_marker: String,
    server_url: String,
    server_log: PathBuf,
    qdrant_pid: Option<u32>,
    qdrant_marker: Option<String>,
    qdrant_log: Option<PathBuf>,
}

struct LocalPaths {
    state: PathBuf,
    server_log: PathBuf,
    qdrant_log: PathBuf,
    qdrant_root: PathBuf,
}

struct LocalStateInspection {
    server_process_alive: bool,
    server_identity_matches: bool,
    server_healthy: bool,
    qdrant_owned: bool,
    qdrant_process_alive: bool,
    qdrant_identity_matches: bool,
}

impl LocalStateInspection {
    fn running(&self) -> bool {
        self.server_identity_matches && self.server_healthy
    }

    fn status(&self) -> &'static str {
        if self.server_process_alive && !self.server_identity_matches
            || self.qdrant_process_alive && !self.qdrant_identity_matches
        {
            "error"
        } else if !self.server_process_alive
            || !self.server_healthy
            || self.qdrant_owned && (!self.qdrant_process_alive || !self.qdrant_identity_matches)
        {
            "warning"
        } else {
            "ok"
        }
    }
}

impl LocalPaths {
    fn new(config_path: &Path) -> Self {
        let root = config_path
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        Self {
            state: root.join("local.json"),
            server_log: root.join("local-server.log"),
            qdrant_log: root.join("local-qdrant.log"),
            qdrant_root: root.join("qdrant"),
        }
    }
}

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

    if let Some(url) = non_empty_env_string("A4N_QDRANT_URL")? {
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
    if options.full && source_root.is_none() {
        bail!("local start --full 需要 A4N 源码 workspace；发布包请使用默认 sidecar profile");
    }
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
    let mut qdrant_url = non_empty_env_string("A4N_QDRANT_URL")?;
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

fn run_migration(database_url: &str, source_root: Option<&Path>) -> Result<()> {
    let binary = resolve_or_build_binary("a4n-migration", source_root, None)?;
    let output = Command::new(&binary)
        .env("A4N_DATABASE_URL", database_url)
        .output()
        .with_context(|| format!("启动数据库迁移器失败: {}", binary.display()))?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr);
        let mut secrets = vec![database_url.to_string()];
        if let Some(password) = Url::parse(database_url)
            .ok()
            .and_then(|url| url.password().map(str::to_string))
            .filter(|password| !password.is_empty())
        {
            secrets.push(password);
        }
        bail!(
            "数据库迁移失败: {}",
            crate::client::redact_text_values(message.trim(), &secrets)
        );
    }
    Ok(())
}

fn spawn_server(
    binary: &Path,
    endpoint: SocketAddr,
    source_root: Option<&Path>,
    paths: &LocalPaths,
    options: StartOptions,
    sidecar_bin_dir: Option<&Path>,
    qdrant_url: Option<&str>,
) -> Result<std::process::Child> {
    let (stdout, stderr) = log_stdio(&paths.server_log)?;
    let mut command = Command::new(binary);
    command
        .env("A4N_WEB_ADDR", endpoint.to_string())
        .env(
            "A4N_RUNTIME_PROFILE",
            if options.full { "full" } else { "sidecar" },
        )
        .env(
            MANAGED_SIDECARS_ENV,
            if options.managed_sidecars { "1" } else { "0" },
        )
        .stdout(stdout)
        .stderr(stderr)
        .stdin(Stdio::null());
    if let Some(root) = source_root {
        command.current_dir(root).env("A4N_PORTABLE_HOME", root);
    } else if let Some(root) = paths.state.parent() {
        let data_dir = std::path::absolute(root.join("data"))
            .with_context(|| format!("无法解析 local 数据目录: {}", root.display()))?;
        command.current_dir(root).env("A4N_DATA_DIR", data_dir);
    }
    if let Some(directory) = sidecar_bin_dir {
        command.env(SIDECAR_BIN_DIR_ENV, directory);
    }
    if let Some(url) = qdrant_url {
        command.env("A4N_QDRANT_URL", url);
    }
    if std::env::var_os(RELEASE_MANIFEST_ENV).is_none()
        && let Some(path) = bundled_release_manifest()
    {
        command.env(RELEASE_MANIFEST_ENV, path);
    }
    if options.managed_sidecars || options.full {
        configure_onnxruntime_environment(&mut command, sidecar_bin_dir)?;
    }
    command
        .spawn()
        .with_context(|| format!("启动 A4N Server 失败: {}", binary.display()))
}

fn bundled_release_manifest() -> Option<PathBuf> {
    let executable = std::env::current_exe().ok()?;
    let path = executable.parent()?.parent()?.join("release-manifest.json");
    path.is_file().then_some(path)
}

fn configure_onnxruntime_environment(
    command: &mut Command,
    sidecar_bin_dir: Option<&Path>,
) -> Result<()> {
    let library_dir = onnxruntime_library_dir(sidecar_bin_dir)?;
    let loader_path = prepend_env_path(runtime_loader_path_var(), &library_dir)?;
    command
        .env(runtime_loader_path_var(), loader_path)
        .env("ORT_LIB_PATH", &library_dir)
        .env("ORT_PREFER_DYNAMIC_LINK", "1");
    if std::env::var_os("A4N_ONNXRUNTIME_ROOT").is_none()
        && library_dir.file_name().and_then(|value| value.to_str()) == Some("lib")
        && let Some(root) = library_dir.parent()
    {
        command.env("A4N_ONNXRUNTIME_ROOT", root);
    }
    Ok(())
}

fn onnxruntime_library_dir(sidecar_bin_dir: Option<&Path>) -> Result<PathBuf> {
    if let Some(directory) = sidecar_bin_dir
        && directory.join(onnxruntime_library_name()).is_file()
    {
        return Ok(directory.to_path_buf());
    }
    if let Ok(executable) = std::env::current_exe()
        && let Some(contents) = executable.parent().and_then(Path::parent)
    {
        let resources = contents.join("Resources");
        if resources.join(onnxruntime_library_name()).is_file() {
            return Ok(resources);
        }
    }

    let root = if let Some(root) = non_empty_env_path("A4N_ONNXRUNTIME_ROOT")? {
        root
    } else {
        let data_home = match non_empty_env_path("XDG_DATA_HOME")? {
            Some(root) => root,
            None => non_empty_env_path("HOME")?
                .map(|root| root.join(".local/share"))
                .ok_or_else(|| {
                    anyhow::anyhow!("无法确定 ONNX Runtime 目录；请设置 A4N_ONNXRUNTIME_ROOT")
                })?,
        };
        data_home
            .join("a4n/onnxruntime")
            .join(onnxruntime_version())
    };
    let version_path = root.join("VERSION_NUMBER");
    let version = fs::read_to_string(&version_path)
        .with_context(|| format!("读取 ONNX Runtime 版本失败: {}", version_path.display()))?
        .trim()
        .to_string();
    let library_dir = root.join("lib");
    if version != onnxruntime_version() || !library_dir.join(onnxruntime_library_name()).is_file() {
        bail!(
            "缺少 ONNX Runtime {} 动态库: {}；请先运行 ./scripts/dev/setup-onnxruntime.sh",
            onnxruntime_version(),
            root.display()
        );
    }
    Ok(library_dir)
}

fn onnxruntime_version() -> &'static str {
    include_str!("../../../tools/onnxruntime/VERSION").trim()
}

fn prepend_env_path(name: &str, directory: &Path) -> Result<OsString> {
    let mut paths = vec![directory.to_path_buf()];
    if let Some(current) = std::env::var_os(name) {
        for path in std::env::split_paths(&current) {
            if !paths.contains(&path) {
                paths.push(path);
            }
        }
    }
    std::env::join_paths(paths).with_context(|| format!("构造 {name} 失败"))
}

#[cfg(target_os = "linux")]
fn onnxruntime_library_name() -> &'static str {
    "libonnxruntime.so.1"
}

#[cfg(target_os = "macos")]
fn onnxruntime_library_name() -> &'static str {
    "libonnxruntime.dylib"
}

#[cfg(windows)]
fn onnxruntime_library_name() -> &'static str {
    "onnxruntime.dll"
}

#[cfg(target_os = "linux")]
fn runtime_loader_path_var() -> &'static str {
    "LD_LIBRARY_PATH"
}

#[cfg(target_os = "macos")]
fn runtime_loader_path_var() -> &'static str {
    "DYLD_LIBRARY_PATH"
}

#[cfg(windows)]
fn runtime_loader_path_var() -> &'static str {
    "PATH"
}

fn spawn_qdrant(
    binary: &Path,
    endpoint: SocketAddr,
    paths: &LocalPaths,
) -> Result<std::process::Child> {
    fs::create_dir_all(paths.qdrant_root.join("storage"))?;
    fs::create_dir_all(paths.qdrant_root.join("snapshots"))?;
    let http_port = qdrant_http_port(endpoint.port())?;
    let (stdout, stderr) = log_stdio(&paths.qdrant_log)?;
    Command::new(binary)
        .current_dir(&paths.qdrant_root)
        .env("QDRANT__SERVICE__HOST", endpoint.ip().to_string())
        .env("QDRANT__SERVICE__GRPC_PORT", endpoint.port().to_string())
        .env("QDRANT__SERVICE__HTTP_PORT", http_port.to_string())
        .env(
            "QDRANT__STORAGE__STORAGE_PATH",
            paths.qdrant_root.join("storage"),
        )
        .env(
            "QDRANT__STORAGE__SNAPSHOTS_PATH",
            paths.qdrant_root.join("snapshots"),
        )
        .stdout(stdout)
        .stderr(stderr)
        .stdin(Stdio::null())
        .spawn()
        .with_context(|| format!("启动 Qdrant 失败: {}", binary.display()))
}

fn resolve_or_build_sidecars(source_root: Option<&Path>) -> Result<PathBuf> {
    if let Some(directory) = non_empty_env_path(SIDECAR_BIN_DIR_ENV)? {
        validate_sidecar_directory(&directory)?;
        return Ok(directory);
    }
    if source_root.is_none()
        && let Ok(executable) = std::env::current_exe()
        && let Some(parent) = executable.parent()
    {
        let services = parent.join("services");
        let directory = if services.is_dir() {
            services
        } else {
            parent.to_path_buf()
        };
        validate_sidecar_directory(&directory)?;
        return Ok(directory);
    }
    let root = source_root.ok_or_else(|| {
        anyhow::anyhow!("未找到 managed sidecar 二进制；请设置 {SIDECAR_BIN_DIR_ENV}")
    })?;
    let directory = target_debug_dir(root)?;
    let catalog = a4n_service_protocol::service_catalog()?;
    let mut command = Command::new("cargo");
    command.arg("build").arg("-q");
    for service in &catalog {
        command.arg("-p").arg(&service.binary);
    }
    configure_onnxruntime_environment(&mut command, None)?;
    let output = command
        .current_dir(root)
        .output()
        .context("构建 managed sidecar 失败")?;
    if !output.status.success() {
        bail!(
            "构建 managed sidecar 失败: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    validate_sidecar_directory(&directory)?;
    Ok(directory)
}

fn validate_sidecar_directory(directory: &Path) -> Result<()> {
    let missing = a4n_service_protocol::service_catalog()?
        .into_iter()
        .filter(|service| !directory.join(executable_name(&service.binary)).is_file())
        .map(|service| service.binary)
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        bail!(
            "sidecar 目录缺少可执行文件 {}: {}",
            directory.display(),
            missing.join(", ")
        );
    }
    Ok(())
}

fn resolve_or_build_binary(
    name: &str,
    source_root: Option<&Path>,
    feature: Option<&str>,
) -> Result<PathBuf> {
    if source_root.is_none()
        && feature.is_none()
        && let Some(binary) = sibling_binary(name)
    {
        return Ok(binary);
    }
    if feature.is_some() && source_root.is_none() {
        bail!("构建带 feature 的 {name} 需要 A4N 源码 workspace");
    }
    let root = source_root
        .ok_or_else(|| anyhow::anyhow!("未找到可执行文件 {name}，也未找到 A4N workspace"))?;
    let binary = target_debug_dir(root)?.join(executable_name(name));
    let mut command = Command::new("cargo");
    command.arg("build").arg("-q").arg("-p").arg(name);
    if let Some(feature) = feature {
        command.arg("--features").arg(feature);
        configure_onnxruntime_environment(&mut command, None)?;
    } else if name == "a4n-server" {
        command.arg("--no-default-features");
    }
    let output = command
        .current_dir(root)
        .output()
        .with_context(|| format!("构建 {name} 失败"))?;
    if !output.status.success() {
        bail!(
            "构建 {name} 失败: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    if !binary.is_file() {
        bail!("构建完成后仍未找到可执行文件: {}", binary.display());
    }
    Ok(binary)
}

fn sibling_binary(name: &str) -> Option<PathBuf> {
    let executable = executable_name(name);
    let sibling = std::env::current_exe().ok()?.parent()?.join(executable);
    sibling.is_file().then_some(sibling)
}

fn source_root() -> Result<Option<PathBuf>> {
    if let Some(path) = non_empty_env_path("A4N_SOURCE_ROOT")? {
        if is_source_root(&path) {
            return Ok(Some(path));
        }
        bail!("A4N_SOURCE_ROOT 不是 A4N workspace: {}", path.display());
    }
    if let Ok(current) = std::env::current_dir() {
        for path in current.ancestors() {
            if is_source_root(path) {
                return Ok(Some(path.to_path_buf()));
            }
        }
    }
    Ok(None)
}

fn is_source_root(path: &Path) -> bool {
    path.join("Cargo.toml").is_file() && path.join("crates/a4n-server").is_dir()
}

fn target_debug_dir(root: &Path) -> Result<PathBuf> {
    Ok(match non_empty_env_path("CARGO_TARGET_DIR")? {
        Some(path) => {
            if path.is_absolute() {
                path.join("debug")
            } else {
                root.join(path).join("debug")
            }
        }
        None => root.join("target/debug"),
    })
}

fn load_state(path: &Path) -> Result<Option<LocalState>> {
    let _lock = lock_config(path)?;
    load_state_unlocked(path)
}

fn load_state_unlocked(path: &Path) -> Result<Option<LocalState>> {
    if !path.exists() {
        return Ok(None);
    }
    let contents = fs::read_to_string(path)
        .with_context(|| format!("读取 local state 失败: {}", path.display()))?;
    let state: LocalState = serde_json::from_str(&contents)
        .with_context(|| format!("local state 无效: {}", path.display()))?;
    if state.schema_version != LOCAL_STATE_SCHEMA_VERSION {
        bail!("local state schema 不兼容");
    }
    if state.server_pid == 0
        || state.server_marker.trim().is_empty()
        || state.server_url.trim().is_empty()
        || state.qdrant_pid.is_some() != state.qdrant_marker.is_some()
        || state.qdrant_pid.is_some() != state.qdrant_log.is_some()
    {
        bail!("local state 内容不一致");
    }
    Ok(Some(state))
}

async fn inspect_local_state(state: &LocalState) -> LocalStateInspection {
    let server_process_alive = process_alive(state.server_pid);
    let server_identity_matches =
        server_process_alive && process_matches(state.server_pid, &state.server_marker);
    let qdrant_owned = state.qdrant_pid.is_some();
    let qdrant_process_alive = state.qdrant_pid.is_some_and(process_alive);
    let qdrant_identity_matches = qdrant_process_alive
        && state
            .qdrant_pid
            .zip(state.qdrant_marker.as_deref())
            .is_some_and(|(pid, marker)| process_matches(pid, marker));
    LocalStateInspection {
        server_process_alive,
        server_identity_matches,
        server_healthy: server_healthy(&state.server_url).await,
        qdrant_owned,
        qdrant_process_alive,
        qdrant_identity_matches,
    }
}

fn binary_marker(binary: &Path) -> Result<String> {
    binary
        .file_name()
        .filter(|name| !name.is_empty())
        .map(|name| name.to_string_lossy().into_owned())
        .ok_or_else(|| anyhow::anyhow!("可执行文件路径缺少文件名: {}", binary.display()))
}

async fn terminate_owned(pid: u32, marker: &str, force: bool) -> Result<()> {
    if !process_matches(pid, marker) {
        bail!("拒绝终止 PID {pid}：进程身份与 local state 不匹配");
    }
    terminate_process(pid, false)?;
    if wait_process_exit(pid).await {
        return Ok(());
    }
    if !force {
        bail!("PID {pid} 未在 5 秒内退出；确认后使用 local stop --force");
    }
    if !process_alive(pid) {
        return Ok(());
    }
    if !process_matches(pid, marker) {
        bail!("拒绝强制终止 PID {pid}：PID 已被其他进程复用");
    }
    terminate_process(pid, true)?;
    if wait_process_exit(pid).await {
        Ok(())
    } else {
        bail!("强制终止 PID {pid} 后进程仍未退出")
    }
}

async fn wait_process_exit(pid: u32) -> bool {
    for _ in 0..50 {
        if !process_alive(pid) {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    false
}

#[cfg(unix)]
fn terminate_process(pid: u32, force: bool) -> Result<()> {
    let signal = if force { "-KILL" } else { "-TERM" };
    let output = Command::new("kill")
        .args([signal, &pid.to_string()])
        .output()
        .context("调用 kill 失败")?;
    finish_terminate_process(pid, output)
}

#[cfg(windows)]
fn terminate_process(pid: u32, force: bool) -> Result<()> {
    let mut command = Command::new("taskkill");
    command.args(["/PID", &pid.to_string(), "/T"]);
    if force {
        command.arg("/F");
    }
    let output = command.output().context("调用 taskkill 失败")?;
    finish_terminate_process(pid, output)
}

fn finish_terminate_process(pid: u32, output: std::process::Output) -> Result<()> {
    if output.status.success() || !process_alive(pid) {
        return Ok(());
    }
    let detail = if output.stderr.is_empty() {
        &output.stdout
    } else {
        &output.stderr
    };
    let detail = String::from_utf8_lossy(detail);
    let detail = detail.trim();
    if detail.is_empty() {
        bail!("无法终止 PID {pid}");
    }
    bail!(
        "无法终止 PID {pid}: {}",
        detail.chars().take(4096).collect::<String>()
    )
}

fn process_matches(pid: u32, marker: &str) -> bool {
    process_command_line(pid).is_some_and(|command| command.contains(marker))
}

#[cfg(target_os = "linux")]
fn process_command_line(pid: u32) -> Option<String> {
    let bytes = fs::read(format!("/proc/{pid}/cmdline")).ok()?;
    non_empty_command_line(&bytes)
}

#[cfg(all(unix, not(target_os = "linux")))]
fn process_command_line(pid: u32) -> Option<String> {
    let output = Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "command="])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| non_empty_command_line(&output.stdout))
        .flatten()
}

#[cfg(windows)]
fn process_command_line(pid: u32) -> Option<String> {
    let output = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &format!(
                "[Console]::OutputEncoding = [Text.UTF8Encoding]::new(); (Get-CimInstance Win32_Process -Filter 'ProcessId = {pid}').CommandLine"
            ),
        ])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| non_empty_command_line(&output.stdout))
        .flatten()
}

fn non_empty_command_line(bytes: &[u8]) -> Option<String> {
    let command = String::from_utf8_lossy(bytes).replace('\0', " ");
    (!command.trim().is_empty()).then_some(command)
}

fn process_alive(pid: u32) -> bool {
    pid != 0 && process_command_line(pid).is_some()
}

fn server_endpoint(api_base: &str) -> Result<SocketAddr> {
    let url = Url::parse(api_base).context("Server URL 无效")?;
    if url.scheme() != "http" {
        bail!("local Server URL 必须使用 http://");
    }
    let host = url
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("Server URL 缺少 host"))?;
    let ip: IpAddr = host
        .parse()
        .or_else(|_| match host {
            "localhost" => Ok(IpAddr::from([127, 0, 0, 1])),
            _ => Err(()),
        })
        .map_err(|_| anyhow::anyhow!("local 只允许 loopback Server，当前 host: {host}"))?;
    if !ip.is_loopback() {
        bail!("local 只允许 loopback Server，当前 host: {host}");
    }
    Ok(SocketAddr::new(
        ip,
        url.port_or_known_default()
            .expect("http Server URL must have a known default port"),
    ))
}

fn database_endpoint(database_url: &str) -> Result<SocketAddr> {
    let url = Url::parse(database_url).context("A4N_DATABASE_URL 无效")?;
    if !matches!(url.scheme(), "postgres" | "postgresql") {
        bail!("A4N_DATABASE_URL 必须使用 postgres:// 或 postgresql://");
    }
    let host = url
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("A4N_DATABASE_URL 缺少 host"))?;
    let port = url.port().unwrap_or(5432);
    resolve_socket(host, port)
}

fn endpoint_from_url(value: &str) -> Result<SocketAddr> {
    let url = Url::parse(value).context("服务 URL 无效")?;
    if !matches!(url.scheme(), "http" | "https") {
        bail!("服务 URL 必须使用 http:// 或 https://");
    }
    if !url.username().is_empty() || url.password().is_some() {
        bail!("服务 URL 不能包含用户名或密码");
    }
    if url.query().is_some() || url.fragment().is_some() {
        bail!("服务 URL 不能包含查询参数或 fragment");
    }
    let host = url
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("服务 URL 缺少 host"))?;
    resolve_socket(
        host,
        url.port_or_known_default()
            .expect("http/https service URL must have a known default port"),
    )
}

fn qdrant_http_port(grpc_port: u16) -> Result<u16> {
    grpc_port
        .checked_sub(1)
        .filter(|port| *port > 0)
        .ok_or_else(|| anyhow::anyhow!("Qdrant gRPC 端口必须大于 1"))
}

fn resolve_socket(host: &str, port: u16) -> Result<SocketAddr> {
    use std::net::ToSocketAddrs;
    (host, port)
        .to_socket_addrs()
        .context("解析服务地址失败")?
        .next()
        .ok_or_else(|| anyhow::anyhow!("无法解析服务地址: {host}:{port}"))
}

async fn tcp_reachable(endpoint: SocketAddr, timeout: Duration) -> bool {
    tokio::time::timeout(timeout, TcpStream::connect(endpoint))
        .await
        .is_ok_and(|result| result.is_ok())
}

async fn wait_tcp(
    endpoint: SocketAddr,
    child: &mut std::process::Child,
    timeout: Duration,
) -> bool {
    let started = std::time::Instant::now();
    while started.elapsed() < timeout {
        if !matches!(child.try_wait(), Ok(None)) {
            return false;
        }
        if tcp_reachable(endpoint, Duration::from_millis(300)).await {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    false
}

async fn wait_server(api_base: &str, child: &mut std::process::Child, timeout: Duration) -> bool {
    let started = std::time::Instant::now();
    while started.elapsed() < timeout {
        if !matches!(child.try_wait(), Ok(None)) {
            return false;
        }
        if server_healthy(api_base).await {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
    false
}

async fn server_healthy(api_base: &str) -> bool {
    static CLIENT: OnceLock<Option<reqwest::Client>> = OnceLock::new();
    let Some(client) = CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .timeout(Duration::from_secs(1))
                .build()
                .ok()
        })
        .as_ref()
    else {
        return false;
    };
    let Ok(response) = client
        .get(format!("{}/meta", api_base.trim_end_matches('/')))
        .send()
        .await
    else {
        return false;
    };
    response.status().is_success()
        && response
            .json::<Value>()
            .await
            .is_ok_and(|value| value["host"] == HOST_IDENTITY)
}

fn log_stdio(path: &Path) -> Result<(Stdio, Stdio)> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options
        .open(path)
        .with_context(|| format!("打开日志文件失败: {}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .with_context(|| format!("设置日志文件权限失败: {}", path.display()))?;
    }
    let stderr = file.try_clone()?;
    Ok((Stdio::from(file), Stdio::from(stderr)))
}

fn tail_lines(path: &Path, lines: usize) -> Result<Vec<String>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    const MAX_BYTES: u64 = 2 * 1024 * 1024;
    let mut file = File::open(path)?;
    let length = file.metadata()?.len();
    let start = length.saturating_sub(MAX_BYTES);
    let starts_mid_line = if start > 0 {
        let mut previous = [0_u8; 1];
        file.seek(SeekFrom::Start(start - 1))?;
        file.read_exact(&mut previous)?;
        previous[0] != b'\n'
    } else {
        false
    };
    file.seek(SeekFrom::Start(start))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    if starts_mid_line {
        let first_complete_line = bytes
            .iter()
            .position(|byte| *byte == b'\n')
            .map(|index| index + 1)
            .unwrap_or(bytes.len());
        bytes.drain(..first_complete_line);
    }
    let text = String::from_utf8_lossy(&bytes);
    Ok(text
        .lines()
        .rev()
        .take(lines)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(str::to_string)
        .collect())
}

fn required_env(key: &str) -> Result<String> {
    non_empty_env_string(key)?.ok_or_else(|| anyhow::anyhow!("缺少 {key}"))
}

fn push_check(checks: &mut Vec<Value>, name: &str, status: &str, detail: impl Into<Value>) {
    checks.push(json!({ "name": name, "status": status, "detail": detail.into() }));
}

fn executable_name(name: &str) -> OsString {
    #[cfg(windows)]
    {
        format!("{name}.exe").into()
    }
    #[cfg(not(windows))]
    {
        name.into()
    }
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
        stop, tail_lines,
    };
    use crate::local::MANAGED_SIDECARS_ENV;

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
                None => assert!(!env.contains("A4N_QDRANT_URL=")),
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
