//! Local Server, sidecar and endpoint runtime helpers.

use super::*;

pub(super) fn run_migration(database_url: &str, source_root: Option<&Path>) -> Result<()> {
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

pub(super) fn spawn_server(
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

pub(super) fn bundled_release_manifest() -> Option<PathBuf> {
    let executable = std::env::current_exe().ok()?;
    let path = executable.parent()?.parent()?.join("release-manifest.json");
    path.is_file().then_some(path)
}

pub(super) fn configure_onnxruntime_environment(
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

pub(super) fn onnxruntime_library_dir(sidecar_bin_dir: Option<&Path>) -> Result<PathBuf> {
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

    let root = if let Some(root) = a4n_config::optional_non_empty_env_path("A4N_ONNXRUNTIME_ROOT")?
    {
        root
    } else {
        let data_home = match a4n_config::optional_non_empty_env_path("XDG_DATA_HOME")? {
            Some(root) => root,
            None => a4n_config::optional_non_empty_env_path("HOME")?
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

pub(super) fn onnxruntime_version() -> &'static str {
    include_str!("../../../../../tools/onnxruntime/VERSION").trim()
}

pub(super) fn prepend_env_path(name: &str, directory: &Path) -> Result<OsString> {
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
pub(super) fn onnxruntime_library_name() -> &'static str {
    "libonnxruntime.so.1"
}

#[cfg(target_os = "macos")]
pub(super) fn onnxruntime_library_name() -> &'static str {
    "libonnxruntime.dylib"
}

#[cfg(windows)]
pub(super) fn onnxruntime_library_name() -> &'static str {
    "onnxruntime.dll"
}

#[cfg(target_os = "linux")]
pub(super) fn runtime_loader_path_var() -> &'static str {
    "LD_LIBRARY_PATH"
}

#[cfg(target_os = "macos")]
pub(super) fn runtime_loader_path_var() -> &'static str {
    "DYLD_LIBRARY_PATH"
}

#[cfg(windows)]
pub(super) fn runtime_loader_path_var() -> &'static str {
    "PATH"
}

pub(super) fn spawn_qdrant(
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

pub(super) fn resolve_or_build_sidecars(source_root: Option<&Path>) -> Result<PathBuf> {
    if let Some(directory) = a4n_config::optional_non_empty_env_path(SIDECAR_BIN_DIR_ENV)? {
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
    let mut command = Command::new("cargo");
    command.arg("build").arg("-q");
    for service in a4n_process_catalog::service_catalog()? {
        command.arg("-p").arg(service.binary);
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

pub(super) fn validate_sidecar_directory(directory: &Path) -> Result<()> {
    let missing = service_catalog()?
        .into_iter()
        .filter(|service| !directory.join(executable_name(&service.launcher)).is_file())
        .map(|service| service.name)
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        bail!(
            "sidecar 目录缺少独立 service 二进制 {}: {}",
            directory.display(),
            missing.join(", ")
        );
    }
    Ok(())
}

pub(super) fn resolve_or_build_binary(
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

pub(super) fn sibling_binary(name: &str) -> Option<PathBuf> {
    let executable = executable_name(name);
    let sibling = std::env::current_exe().ok()?.parent()?.join(executable);
    sibling.is_file().then_some(sibling)
}

pub(super) fn source_root() -> Result<Option<PathBuf>> {
    if let Some(path) = a4n_config::optional_non_empty_env_path("A4N_SOURCE_ROOT")? {
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

pub(super) fn is_source_root(path: &Path) -> bool {
    path.join("Cargo.toml").is_file() && path.join("crates/host/a4n-server").is_dir()
}

pub(super) fn target_debug_dir(root: &Path) -> Result<PathBuf> {
    Ok(
        match a4n_config::optional_non_empty_env_path("CARGO_TARGET_DIR")? {
            Some(path) => {
                if path.is_absolute() {
                    path.join("debug")
                } else {
                    root.join(path).join("debug")
                }
            }
            None => root.join("target/debug"),
        },
    )
}

pub(super) fn load_state(path: &Path) -> Result<Option<LocalState>> {
    let _lock = lock_config(path)?;
    load_state_unlocked(path)
}

pub(super) fn load_state_unlocked(path: &Path) -> Result<Option<LocalState>> {
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

pub(super) async fn inspect_local_state(state: &LocalState) -> LocalStateInspection {
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

pub(super) fn binary_marker(binary: &Path) -> Result<String> {
    binary
        .file_name()
        .filter(|name| !name.is_empty())
        .map(|name| name.to_string_lossy().into_owned())
        .ok_or_else(|| anyhow::anyhow!("可执行文件路径缺少文件名: {}", binary.display()))
}

pub(super) async fn terminate_owned(pid: u32, marker: &str, force: bool) -> Result<()> {
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

pub(super) async fn wait_process_exit(pid: u32) -> bool {
    for _ in 0..50 {
        if !process_alive(pid) {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    false
}

#[cfg(unix)]
pub(super) fn terminate_process(pid: u32, force: bool) -> Result<()> {
    let signal = if force { "-KILL" } else { "-TERM" };
    let output = Command::new("kill")
        .args([signal, &pid.to_string()])
        .output()
        .context("调用 kill 失败")?;
    finish_terminate_process(pid, output)
}

#[cfg(windows)]
pub(super) fn terminate_process(pid: u32, force: bool) -> Result<()> {
    let mut command = Command::new("taskkill");
    command.args(["/PID", &pid.to_string(), "/T"]);
    if force {
        command.arg("/F");
    }
    let output = command.output().context("调用 taskkill 失败")?;
    finish_terminate_process(pid, output)
}

pub(super) fn finish_terminate_process(pid: u32, output: std::process::Output) -> Result<()> {
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

pub(super) fn process_matches(pid: u32, marker: &str) -> bool {
    process_command_line(pid).is_some_and(|command| command.contains(marker))
}

#[cfg(target_os = "linux")]
pub(super) fn process_command_line(pid: u32) -> Option<String> {
    let bytes = fs::read(format!("/proc/{pid}/cmdline")).ok()?;
    non_empty_command_line(&bytes)
}

#[cfg(all(unix, not(target_os = "linux")))]
pub(super) fn process_command_line(pid: u32) -> Option<String> {
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
pub(super) fn process_command_line(pid: u32) -> Option<String> {
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

pub(super) fn non_empty_command_line(bytes: &[u8]) -> Option<String> {
    let command = String::from_utf8_lossy(bytes).replace('\0', " ");
    (!command.trim().is_empty()).then_some(command)
}

pub(super) fn process_alive(pid: u32) -> bool {
    pid != 0 && process_command_line(pid).is_some()
}

pub(super) fn server_endpoint(api_base: &str) -> Result<SocketAddr> {
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

pub(super) fn database_endpoint(database_url: &str) -> Result<SocketAddr> {
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

pub(super) fn endpoint_from_url(value: &str) -> Result<SocketAddr> {
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

pub(super) fn qdrant_http_port(grpc_port: u16) -> Result<u16> {
    grpc_port
        .checked_sub(1)
        .filter(|port| *port > 0)
        .ok_or_else(|| anyhow::anyhow!("Qdrant gRPC 端口必须大于 1"))
}

pub(super) fn resolve_socket(host: &str, port: u16) -> Result<SocketAddr> {
    use std::net::ToSocketAddrs;
    (host, port)
        .to_socket_addrs()
        .context("解析服务地址失败")?
        .next()
        .ok_or_else(|| anyhow::anyhow!("无法解析服务地址: {host}:{port}"))
}

pub(super) async fn tcp_reachable(endpoint: SocketAddr, timeout: Duration) -> bool {
    tokio::time::timeout(timeout, TcpStream::connect(endpoint))
        .await
        .is_ok_and(|result| result.is_ok())
}

pub(super) async fn wait_tcp(
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

pub(super) async fn wait_server(
    api_base: &str,
    child: &mut std::process::Child,
    timeout: Duration,
) -> bool {
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

pub(super) async fn server_healthy(api_base: &str) -> bool {
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

pub(super) fn log_stdio(path: &Path) -> Result<(Stdio, Stdio)> {
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

pub(super) fn tail_lines(path: &Path, lines: usize) -> Result<Vec<String>> {
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

pub(super) fn required_env(key: &str) -> Result<String> {
    non_empty_env_string(key)?.ok_or_else(|| anyhow::anyhow!("缺少 {key}"))
}

pub(super) fn push_check(
    checks: &mut Vec<Value>,
    name: &str,
    status: &str,
    detail: impl Into<Value>,
) {
    checks.push(json!({ "name": name, "status": status, "detail": detail.into() }));
}

pub(super) fn executable_name(name: &str) -> OsString {
    #[cfg(windows)]
    {
        format!("{name}.exe").into()
    }
    #[cfg(not(windows))]
    {
        name.into()
    }
}
