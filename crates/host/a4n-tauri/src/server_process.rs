use a4n_config::app_config::{AppConfig, portable_home_dir};
use a4n_process_catalog::StatefulWorkerCatalogEntry;
use a4n_worker_client::{HttpWorkerClient, WorkerHealthStatus};
use reqwest::StatusCode;
use std::collections::VecDeque;
use std::io::{self, BufRead, BufReader, Write};
use std::net::SocketAddr;
use std::net::TcpListener;
#[cfg(target_os = "linux")]
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Manager};

const SERVER_BINARY_ENV: &str = "A4N_SERVER_BINARY";
const CONTROL_BINARY_ENV: &str = "A4N_CONTROL_BINARY";
const EDGE_BINARY_ENV: &str = "A4N_EDGE_BINARY";
const MIGRATION_BINARY_ENV: &str = "A4N_MIGRATION_BINARY";
const NATIVE_WORKER_READY_TIMEOUT: Duration = Duration::from_secs(30);
const NATIVE_WORKER_RETRY_INTERVAL: Duration = Duration::from_millis(100);
const SERVER_STDERR_TAIL_LIMIT: usize = 8 * 1024;

pub(crate) struct LocalServerProcess {
    child: Mutex<Option<Child>>,
    address_directory: tempfile::TempDir,
    stderr_tail: Arc<Mutex<VecDeque<u8>>>,
    stderr_reader: Mutex<Option<std::thread::JoinHandle<()>>>,
}

pub(crate) struct NativeWorkerSettings {
    workers: Vec<NativeWorkerSettingsEntry>,
    reservations: Vec<TcpListener>,
}

struct NativeWorkerSettingsEntry {
    metadata: StatefulWorkerCatalogEntry,
    address: SocketAddr,
    url: String,
    url_env: String,
    token: String,
    token_env: String,
}

#[derive(Default)]
pub(crate) struct NativeWorkerProcesses {
    children: Mutex<Vec<(String, Child)>>,
    control_task: Option<tauri::async_runtime::JoinHandle<()>>,
}

impl NativeWorkerSettings {
    pub(crate) fn prepare() -> Result<(Self, TcpListener), Box<dyn std::error::Error>> {
        let catalog = a4n_process_catalog::stateful_worker_catalog()?;
        let entries = &catalog;
        let control = TcpListener::bind("127.0.0.1:0")?;
        let mut reservations = Vec::with_capacity(entries.len());
        let mut workers = Vec::with_capacity(entries.len());

        for metadata in entries {
            if !metadata.local_only {
                return Err(io::Error::other(format!(
                    "Native worker {} 必须是 local-only",
                    metadata.name
                ))
                .into());
            }
            if metadata.addr_env.trim().is_empty() {
                return Err(io::Error::other(format!(
                    "Native worker {} 缺少地址环境变量",
                    metadata.name
                ))
                .into());
            }
            let url_env = metadata
                .url_env
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| {
                    io::Error::other(format!("Native worker {} 缺少 URL 环境变量", metadata.name))
                })?;
            let listener = TcpListener::bind("127.0.0.1:0")?;
            let address = listener.local_addr()?;
            reservations.push(listener);
            workers.push(NativeWorkerSettingsEntry {
                token_env: stateful_worker_env(&metadata.name, "TOKEN"),
                url: format!("http://{address}"),
                url_env: url_env.to_string(),
                token: uuid::Uuid::new_v4().to_string(),
                metadata: metadata.clone(),
                address,
            });
        }

        // SAFETY: `prepare` is called before Tauri, logging, and worker threads start.
        unsafe { std::env::set_var("A4N_CONTROL_ADDR", control.local_addr()?.to_string()) };
        for worker in &workers {
            unsafe {
                std::env::set_var(&worker.metadata.addr_env, worker.address.to_string());
                std::env::set_var(&worker.url_env, &worker.url);
                std::env::set_var(&worker.token_env, &worker.token);
            }
        }
        Ok((
            Self {
                workers,
                reservations,
            },
            control,
        ))
    }

    pub(crate) fn split_agent(self) -> (Self, Self) {
        let (agent, workers): (Vec<_>, Vec<_>) = self
            .workers
            .into_iter()
            .zip(self.reservations)
            .partition(|(worker, _)| worker.metadata.name == "agent-worker");
        let (workers, reservations) = workers.into_iter().unzip();
        let ordinary = Self {
            workers,
            reservations,
        };
        let (workers, reservations) = agent.into_iter().unzip();
        (
            ordinary,
            Self {
                workers,
                reservations,
            },
        )
    }

    pub(crate) async fn start(
        self,
        app: &AppHandle,
        config: &AppConfig,
    ) -> Result<NativeWorkerProcesses, Box<dyn std::error::Error>> {
        let processes = NativeWorkerProcesses::default();
        let mut reservations = self.reservations.into_iter();
        for worker in self.workers {
            let binary_env = stateful_worker_binary_env(&worker.metadata.name);
            let binary = resolve_binary(app, &worker.metadata.binary, &binary_env)?;
            let mut command = Command::new(&binary);
            // Agent health reports a process identity, not the catalog service name.
            let worker_id = if worker.metadata.name == "agent-worker" {
                let identity = format!("a4n-agent:{}", uuid::Uuid::new_v4());
                command.env("A4N_AGENT_WORKER_ID", &identity);
                identity
            } else {
                worker.metadata.name.clone()
            };
            command
                .env(&worker.metadata.addr_env, worker.address.to_string())
                .env(&worker.token_env, &worker.token)
                .env("A4N_DATABASE_URL", &config.database_url)
                .env("A4N_DATA_DIR", &config.data_dir)
                .env("A4N_CACHE_DIR", &config.cache_dir)
                .env("A4N_TEMP_DIR", &config.temp_dir)
                .env("A4N_LOG_DIR", &config.log_dir)
                .stdin(Stdio::null())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit());
            configure_parent_death_signal(&mut command);

            // ponytail: release each reserved port immediately before bind; retry if a rare race occurs.
            drop(reservations.next());
            let mut child = command.spawn().map_err(|error| {
                io::Error::other(format!(
                    "启动 Native worker {} 失败（{}）: {error}",
                    worker.metadata.name,
                    binary.display()
                ))
            })?;
            let client = HttpWorkerClient::new(&worker.url, worker_id)
                .with_auth_header(a4n_worker_contract::worker_auth_header(
                    &worker.metadata.name,
                ))
                .with_auth_token(worker.token.clone());
            if let Err(error) = wait_for_native_worker(&mut child, &client, &worker.metadata).await
            {
                if let Err(stop_error) = terminate_child(&mut child) {
                    tracing::error!(
                        worker = %worker.metadata.name,
                        error = %stop_error,
                        "Native worker readiness 失败后停止进程失败"
                    );
                }
                return Err(error);
            }
            tracing::info!(
                worker = %worker.metadata.name,
                address = %worker.address,
                "Native worker 启动并通过健康检查"
            );
            processes
                .children
                .lock()
                .map_err(|_| io::Error::other("获取 Native worker 进程锁失败"))?
                .push((worker.metadata.name, child));
        }
        Ok(processes)
    }
}

impl NativeWorkerProcesses {
    pub(crate) fn stop_agent(&self) {
        let Ok(mut children) = self.children.lock() else {
            tracing::error!("获取 Native worker 进程锁失败");
            return;
        };
        if let Some(index) = children.iter().position(|(name, _)| name == "agent-worker") {
            let (name, child) = children.remove(index);
            stop_native_child(name, child);
        }
    }

    pub(crate) fn serve_agent_host(
        &mut self,
        listener: TcpListener,
        state: &a4n_runtime::AppState,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let token = a4n_config::required_non_empty_env_string("A4N_AGENT_WORKER_TOKEN")?;
        let router = a4n_control::native_agent_router(state.clone(), token)?;
        listener.set_nonblocking(true)?;
        let listener =
            tauri::async_runtime::block_on(async { tokio::net::TcpListener::from_std(listener) })?;
        self.control_task = Some(tauri::async_runtime::spawn(async move {
            if let Err(error) = axum::serve(listener, router).await {
                tracing::error!(%error, "Native Agent Host callback listener 退出");
            }
        }));
        Ok(())
    }

    pub(crate) fn append(&mut self, mut other: Self) -> Result<(), io::Error> {
        self.children
            .get_mut()
            .map_err(|_| io::Error::other("获取 Native worker 进程锁失败"))?
            .append(
                other
                    .children
                    .get_mut()
                    .map_err(|_| io::Error::other("获取 Native worker 进程锁失败"))?,
            );
        Ok(())
    }

    pub(crate) fn stop(&self) {
        let Ok(mut children) = self.children.lock() else {
            tracing::error!("获取 Native worker 进程锁失败");
            return;
        };
        // Also preserve dependency order when unwinding a failed startup.
        for (name, child) in children.drain(..).rev() {
            stop_native_child(name, child);
        }
        if let Some(task) = &self.control_task {
            task.abort();
        }
    }
}

fn stop_native_child(name: String, mut child: Child) {
    if let Err(error) = terminate_child(&mut child)
        && error.kind() != io::ErrorKind::InvalidInput
    {
        tracing::warn!(worker = %name, %error, "停止 Native worker 失败");
    }
}

impl Drop for NativeWorkerProcesses {
    fn drop(&mut self) {
        self.stop();
    }
}

async fn wait_for_native_worker(
    child: &mut Child,
    client: &HttpWorkerClient,
    metadata: &StatefulWorkerCatalogEntry,
) -> Result<(), Box<dyn std::error::Error>> {
    let deadline = Instant::now() + NATIVE_WORKER_READY_TIMEOUT;
    loop {
        if let Some(status) = child.try_wait()? {
            return Err(io::Error::other(format!(
                "Native worker {} 提前退出: {status}",
                metadata.name
            ))
            .into());
        }
        let last_error = match client.health().await {
            Ok(health) if health.status == WorkerHealthStatus::Healthy => {
                let capabilities = client.capabilities().await?;
                validate_native_worker_capabilities(metadata, &capabilities)?;
                return Ok(());
            }
            Ok(health) => format!("worker health status={:?}", health.status),
            Err(error) => error.to_string(),
        };
        if Instant::now() >= deadline {
            return Err(io::Error::other(format!(
                "等待 Native worker {} 就绪超时: {last_error}",
                metadata.name
            ))
            .into());
        }
        tokio::time::sleep(NATIVE_WORKER_RETRY_INTERVAL).await;
    }
}

fn validate_native_worker_capabilities(
    metadata: &StatefulWorkerCatalogEntry,
    capabilities: &a4n_worker_client::WorkerCapabilities,
) -> Result<(), io::Error> {
    let missing = metadata.capabilities.iter().find(|expected| {
        !capabilities.capabilities.iter().any(|reported| {
            reported.name == expected.as_str()
                && reported.enabled
                && reported.version == metadata.capability_version.as_str()
        })
    });
    if let Some(missing) = missing {
        return Err(io::Error::other(format!(
            "Native worker {} 未提供预期 capability {}@{}",
            metadata.name, missing, metadata.capability_version
        )));
    }
    Ok(())
}

impl LocalServerProcess {
    pub(crate) async fn start(
        app: &AppHandle,
        config: &AppConfig,
    ) -> Result<(Self, String), Box<dyn std::error::Error>> {
        let configured_address = a4n_config::optional_non_empty_env_string("A4N_WEB_ADDR")?
            .as_deref()
            .map(parse_local_server_address)
            .transpose()?;
        // A private directory gives this launch its own address record, including Edge restarts.
        let address_directory = tempfile::Builder::new()
            .prefix("a4n-http-")
            .tempdir_in(&config.temp_dir)?;

        let server = resolve_binary(app, "a4n-server", SERVER_BINARY_ENV)?;
        let control = resolve_binary(app, "a4n-control", CONTROL_BINARY_ENV)?;
        let edge = resolve_binary(app, "a4n-edge", EDGE_BINARY_ENV)?;
        let migration = resolve_binary(app, "a4n-migration", MIGRATION_BINARY_ENV)?;
        let worker_binaries = a4n_process_catalog::stateful_worker_catalog()?
            .into_iter()
            .map(|worker| {
                let env_name = stateful_worker_binary_env(&worker.name);
                resolve_binary(app, &worker.binary, &env_name).map(|binary| (env_name, binary))
            })
            .collect::<Result<Vec<_>, _>>()?;
        run_migration(&migration, &config.database_url)?;

        let mut command = Command::new(&server);
        // The Server main returns startup failures through stderr; retain a sanitized tail for the Tauri setup error.
        command
            .env(
                "A4N_WEB_LISTEN_FILE",
                address_directory.path().join("http.addr"),
            )
            .env(CONTROL_BINARY_ENV, &control)
            .env(EDGE_BINARY_ENV, &edge)
            .env("A4N_RUNTIME_PROFILE", "sidecar")
            .env("A4N_MANAGED_SIDECARS", "1")
            .env("A4N_PORTABLE_HOME", server_portable_home())
            .env("A4N_DATABASE_URL", &config.database_url)
            .env("A4N_DATA_DIR", &config.data_dir)
            .env("A4N_CACHE_DIR", &config.cache_dir)
            .env("A4N_TEMP_DIR", &config.temp_dir)
            .env("A4N_LOG_DIR", &config.log_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::piped());
        for (env_name, binary) in worker_binaries {
            command.env(env_name, binary);
        }
        if let Some(jwt_secret) = config.security.jwt_secret.as_deref() {
            command.env("A4N_JWT_SECRET", jwt_secret);
        }
        if let Some(address) = configured_address {
            command.env("A4N_WEB_ADDR", address.to_string());
        }
        configure_parent_death_signal(&mut command);

        let mut child = command
            .spawn()
            .map_err(|error| format!("启动 A4N Server 失败: {}: {error}", server.display()))?;
        let Some(stderr) = child.stderr.take() else {
            if let Err(error) = terminate_child(&mut child) {
                tracing::warn!(%error, "停止 stderr 未能捕获的 A4N Server 失败");
            }
            return Err(io::Error::other("A4N Server stderr 未开启捕获").into());
        };
        let stderr_tail = Arc::new(Mutex::new(VecDeque::with_capacity(
            SERVER_STDERR_TAIL_LIMIT,
        )));
        let stderr_reader = match spawn_server_stderr_reader(stderr, config, stderr_tail.clone()) {
            Ok(reader) => reader,
            Err(error) => {
                if let Err(stop_error) = terminate_child(&mut child) {
                    tracing::warn!(%stop_error, "启动 stderr 捕获失败后停止 A4N Server 失败");
                }
                return Err(format!("启动 A4N Server stderr 捕获失败: {error}").into());
            }
        };
        let process = Self {
            child: Mutex::new(Some(child)),
            address_directory,
            stderr_tail,
            stderr_reader: Mutex::new(Some(stderr_reader)),
        };
        let api_base_url = match wait_until_healthy(&process).await {
            Ok(api_base_url) => api_base_url,
            Err(error) => {
                process.stop();
                return match process.server_stderr() {
                    Ok(stderr) if stderr.trim().is_empty() => Err(error),
                    Ok(stderr) => Err(format!(
                        "{error}; A4N Server stderr（已脱敏，末尾最多 {SERVER_STDERR_TAIL_LIMIT} 字节）:\n{}",
                        stderr.trim()
                    )
                    .into()),
                    Err(capture_error) => Err(format!(
                        "{error}; 读取 A4N Server stderr 失败: {capture_error}"
                    )
                    .into()),
                };
            }
        };

        tracing::info!(url = %api_base_url, "A4N HTTP-only Server 启动完成");
        Ok((process, api_base_url))
    }

    fn server_stderr(&self) -> Result<String, io::Error> {
        let reader = self
            .stderr_reader
            .lock()
            .map_err(|_| io::Error::other("获取 A4N Server stderr 捕获锁失败"))?
            .take();
        if let Some(reader) = reader {
            for _ in 0..20 {
                if reader.is_finished() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            if reader.is_finished() {
                reader
                    .join()
                    .map_err(|_| io::Error::other("A4N Server stderr 捕获线程异常退出"))?;
            }
        }
        let tail = self
            .stderr_tail
            .lock()
            .map_err(|_| io::Error::other("获取 A4N Server stderr 内容锁失败"))?;
        Ok(String::from_utf8_lossy(&tail.iter().copied().collect::<Vec<_>>()).into_owned())
    }

    pub(crate) fn stop(&self) {
        let Ok(mut child) = self.child.lock() else {
            tracing::error!("获取 A4N Server 子进程锁失败");
            return;
        };
        let Some(mut child_process) = child.take() else {
            return;
        };
        if let Err(error) = terminate_child(&mut child_process)
            && error.kind() != std::io::ErrorKind::InvalidInput
        {
            tracing::warn!(%error, "停止 A4N Server 失败");
        }
    }
}

fn spawn_server_stderr_reader(
    stderr: std::process::ChildStderr,
    config: &AppConfig,
    tail: Arc<Mutex<VecDeque<u8>>>,
) -> Result<std::thread::JoinHandle<()>, io::Error> {
    let secrets = server_startup_secrets(config);

    std::thread::Builder::new()
        .name("a4n-server-stderr".to_string())
        .spawn(move || {
            let mut reader = BufReader::new(stderr);
            let stderr_output = io::stderr();
            let mut line = Vec::new();
            loop {
                line.clear();
                match reader.read_until(b'\n', &mut line) {
                    Ok(0) => break,
                    Ok(_) => {}
                    Err(error) => {
                        let message = redact_server_stderr(
                            &format!("读取 A4N Server stderr 失败: {error}\n"),
                            &secrets,
                        );
                        let _ = stderr_output.lock().write_all(message.as_bytes());
                        append_server_stderr_tail(&tail, message.as_bytes());
                        break;
                    }
                }
                let message = redact_server_stderr(&String::from_utf8_lossy(&line), &secrets);
                let _ = stderr_output.lock().write_all(message.as_bytes());
                append_server_stderr_tail(&tail, message.as_bytes());
            }
        })
}

fn server_startup_secrets(config: &AppConfig) -> Vec<String> {
    let mut secrets = std::env::vars_os()
        .filter_map(|(name, value)| {
            let name = name.to_string_lossy().to_ascii_uppercase();
            let is_secret = [
                "TOKEN",
                "SECRET",
                "PASSWORD",
                "API_KEY",
                "DATABASE_URL",
                "_KEY",
            ]
            .iter()
            .any(|suffix| name.ends_with(suffix));
            let value = value.to_string_lossy().into_owned();
            (is_secret && !value.is_empty()).then_some(value)
        })
        .collect::<Vec<_>>();
    secrets.extend([config.database_url.clone()]);
    secrets.extend(config.security.jwt_secret.clone());
    secrets.retain(|secret| !secret.is_empty());
    secrets.sort_unstable_by_key(|secret| std::cmp::Reverse(secret.len()));
    secrets.dedup();
    secrets
}

fn redact_server_stderr(message: &str, secrets: &[String]) -> String {
    let mut message = a4n_service_protocol::redact_sensitive_message(message);
    for secret in secrets {
        message = message.replace(secret, "[REDACTED]");
    }
    message
}

fn append_server_stderr_tail(tail: &Mutex<VecDeque<u8>>, bytes: &[u8]) {
    let Ok(mut tail) = tail.lock() else {
        return;
    };
    for byte in bytes {
        if tail.len() == SERVER_STDERR_TAIL_LIMIT {
            tail.pop_front();
        }
        tail.push_back(*byte);
    }
}

#[cfg(target_os = "linux")]
fn configure_parent_death_signal(command: &mut Command) {
    // Keep the child server from surviving a Tauri dev rebuild or abrupt exit.
    unsafe {
        command.pre_exec(|| {
            unsafe extern "C" {
                fn prctl(option: i32, ...) -> i32;
            }
            const PR_SET_PDEATHSIG: i32 = 1;
            const SIGTERM: i32 = 15;
            if prctl(PR_SET_PDEATHSIG, SIGTERM) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

#[cfg(not(target_os = "linux"))]
fn configure_parent_death_signal(_: &mut Command) {}

impl Drop for LocalServerProcess {
    fn drop(&mut self) {
        self.stop();
        let _ = self.server_stderr();
    }
}

fn parse_local_server_address(value: &str) -> Result<SocketAddr, Box<dyn std::error::Error>> {
    let address: SocketAddr = value.trim().parse()?;
    if !address.ip().is_loopback() {
        return Err(format!("桌面本地 Server 必须使用 loopback 地址: {address}").into());
    }
    Ok(address)
}

fn read_server_address(path: &Path) -> Result<Option<SocketAddr>, Box<dyn std::error::Error>> {
    let value = match std::fs::read_to_string(path) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let address = parse_local_server_address(&value)?;
    if address.port() == 0 {
        return Err("Server 尚未发布实际监听端口".into());
    }
    Ok(Some(address))
}

async fn wait_until_healthy(
    process: &LocalServerProcess,
) -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(1))
        .build()?;
    let address_file = process.address_directory.path().join("http.addr");
    let mut last_error = None;
    for _ in 0..200 {
        let exit_status = {
            let mut child = process
                .child
                .lock()
                .map_err(|_| "获取 A4N Server 子进程锁失败")?;
            let child = child.as_mut().ok_or("A4N Server 子进程状态丢失")?;
            child
                .try_wait()
                .map_err(|error| format!("读取 A4N Server 子进程状态失败: {error}"))?
        };
        if let Some(status) = exit_status {
            return Err(
                format!("A4N Server 在健康检查通过前退出（{status}）；请检查 Server 日志").into(),
            );
        }
        if let Some(address) = read_server_address(&address_file)? {
            let api_base_url = format!("http://{address}/api");
            match client.get(format!("{api_base_url}/health")).send().await {
                Ok(response) => {
                    let status = response.status();
                    if status == StatusCode::OK && response.text().await?.trim() == "ok" {
                        return Ok(api_base_url);
                    }
                    return Err(
                        format!("{api_base_url}/health 返回无效健康响应（{status}）").into(),
                    );
                }
                Err(error) => last_error = Some(error.to_string()),
            }
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
    Err(format!("A4N Server 启动超时；请检查 Server 日志；最后连接错误: {last_error:?}").into())
}

pub(crate) fn run_native_migration(
    app: &AppHandle,
    config: &AppConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let binary = resolve_binary(app, "a4n-migration", MIGRATION_BINARY_ENV)?;
    run_migration(&binary, &config.database_url)
}

fn run_migration(binary: &Path, database_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(binary)
        .env("A4N_DATABASE_URL", database_url)
        .env_remove("A4N_WEB_ADDR")
        .stdout(Stdio::inherit())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| format!("启动数据库迁移器失败: {}: {error}", binary.display()))?;
    if output.status.success() {
        return Ok(());
    }
    let database_url = std::env::var("A4N_DATABASE_URL").unwrap_or_default();
    let detail = String::from_utf8_lossy(&output.stderr);
    let detail = if database_url.is_empty() {
        detail.into_owned()
    } else {
        detail.replace(&database_url, "[REDACTED]")
    };
    Err(format!("数据库迁移失败: {}", detail.trim()).into())
}

fn terminate_child(child: &mut Child) -> Result<(), std::io::Error> {
    #[cfg(unix)]
    {
        let status = Command::new("kill")
            .args(["-TERM", &child.id().to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        if status.success() {
            for _ in 0..50 {
                if child.try_wait()?.is_some() {
                    return Ok(());
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
    child.kill()?;
    let _ = child.wait();
    Ok(())
}

fn resolve_binary(
    app: &AppHandle,
    name: &str,
    env_name: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut directories = Vec::new();
    if let Some(path) = std::env::var_os(env_name).map(PathBuf::from) {
        if path.is_file() {
            return Ok(path);
        }
        directories.push(path);
    }
    if let Ok(executable) = std::env::current_exe()
        && let Some(parent) = executable.parent()
    {
        directories.push(parent.to_path_buf());
    }
    if let Ok(resource_dir) = app.path().resource_dir() {
        directories.push(resource_dir.clone());
        directories.push(resource_dir.join("bin"));
    }

    resolve_binary_in_directories(name, env_name, directories)
}

fn stateful_worker_binary_env(name: &str) -> String {
    stateful_worker_env(name, "BINARY")
}

fn stateful_worker_env(name: &str, suffix: &str) -> String {
    format!(
        "A4N_{}_{}",
        name.replace('-', "_").to_ascii_uppercase(),
        suffix
    )
}

fn resolve_binary_in_directories(
    name: &str,
    env_name: &str,
    directories: impl IntoIterator<Item = PathBuf>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    for directory in directories {
        for candidate in [directory.join(name), directory.join(format!("{name}.exe"))] {
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let file_name = entry.file_name().to_string_lossy().into_owned();
            if (file_name == name
                || file_name == format!("{name}.exe")
                || file_name.starts_with(&format!("{name}-")))
                && entry.path().is_file()
            {
                return Ok(entry.path());
            }
        }
    }

    Err(format!("未找到 {name}；请构建 Server，或设置 {env_name}").into())
}

fn server_portable_home() -> PathBuf {
    if let Some(path) = std::env::var_os("A4N_PORTABLE_HOME") {
        return PathBuf::from(path);
    }
    if let Ok(current_dir) = std::env::current_dir()
        && let Some(root) = current_dir
            .ancestors()
            .find(|path| path.join("prompts/_registry.yaml").is_file())
    {
        return root.to_path_buf();
    }
    portable_home_dir()
}

#[cfg(test)]
mod tests {
    use super::{
        parse_local_server_address, read_server_address, resolve_binary_in_directories,
        stateful_worker_binary_env, stateful_worker_env,
    };

    #[test]
    fn native_agent_starts_last_without_losing_catalog_workers_or_port_reservations() {
        let catalog = a4n_process_catalog::stateful_worker_catalog().unwrap();
        let mut workers = Vec::new();
        let mut reservations = Vec::new();
        for metadata in &catalog {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            workers.push(super::NativeWorkerSettingsEntry {
                metadata: metadata.clone(),
                address,
                url: format!("http://{address}"),
                url_env: metadata.url_env.clone().unwrap(),
                token: "test-token".into(),
                token_env: stateful_worker_env(&metadata.name, "TOKEN"),
            });
            reservations.push(listener);
        }
        let settings = super::NativeWorkerSettings {
            workers,
            reservations,
        };
        let (ordinary, agent) = settings.split_agent();
        assert_eq!(agent.workers.len(), 1);
        assert_eq!(agent.workers[0].metadata.name, "agent-worker");
        assert_eq!(ordinary.workers.len() + agent.workers.len(), catalog.len());
        for group in [&ordinary, &agent] {
            assert_eq!(group.workers.len(), group.reservations.len());
            for (worker, listener) in group.workers.iter().zip(&group.reservations) {
                assert_eq!(worker.address, listener.local_addr().unwrap());
                assert!(std::net::TcpListener::bind(worker.address).is_err());
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn native_shutdown_stops_consumers_before_their_workers() {
        use std::io::{BufRead, BufReader};
        let directory = tempfile::tempdir().unwrap();
        let order = directory.path().join("stop-order");
        let mut processes = super::NativeWorkerProcesses::default();
        for name in ["record-worker", "agent-worker"] {
            let mut child = std::process::Command::new("sh")
                .args(["-c", "trap 'printf \"%s\\n\" \"$CHILD_LABEL\" >> \"$ORDER_LOG\"; exit 0' TERM; echo ready; while :; do sleep 0.05; done"])
                .env("CHILD_LABEL", name)
                .env("ORDER_LOG", &order)
                .stdout(std::process::Stdio::piped())
                .spawn().unwrap();
            let mut ready = String::new();
            BufReader::new(child.stdout.take().unwrap())
                .read_line(&mut ready)
                .unwrap();
            assert_eq!(ready, "ready\n");
            processes
                .children
                .get_mut()
                .unwrap()
                .push((name.into(), child));
        }
        processes.stop_agent();
        assert_eq!(std::fs::read_to_string(&order).unwrap(), "agent-worker\n");
        let children = processes.children.get_mut().unwrap();
        assert_eq!(children.len(), 1);
        assert!(children[0].1.try_wait().unwrap().is_none());
        processes.stop();
        assert_eq!(
            std::fs::read_to_string(order).unwrap(),
            "agent-worker\nrecord-worker\n"
        );
    }

    #[test]
    fn accepts_only_published_loopback_endpoints() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("http.addr");
        assert!(read_server_address(&path).unwrap().is_none());
        for value in ["127.0.0.1:38191", "[::1]:38192"] {
            std::fs::write(&path, value).unwrap();
            assert_eq!(
                read_server_address(&path).unwrap().unwrap().to_string(),
                value
            );
        }
        for invalid in [
            "",
            "not-an-address",
            "127.0.0.1:0",
            "0.0.0.0:8080",
            "192.0.2.1:8080",
        ] {
            std::fs::write(&path, invalid).unwrap();
            assert!(read_server_address(&path).is_err(), "accepted {invalid}");
        }
        assert_eq!(parse_local_server_address("127.0.0.1:0").unwrap().port(), 0);
    }

    #[test]
    fn resolves_every_supervised_binary_from_one_bundle_directory() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        for name in ["a4n-server", "a4n-control", "a4n-edge", "a4n-migration"] {
            std::fs::write(directory.path().join(name), name).expect("binary fixture should write");
            let resolved = resolve_binary_in_directories(
                name,
                "A4N_TEST_BINARY",
                [directory.path().to_path_buf()],
            )
            .expect("bundle binary should resolve");
            assert_eq!(resolved, directory.path().join(name));
        }

        for worker in a4n_process_catalog::stateful_worker_catalog().unwrap() {
            let bundled_name = format!("{}-x86_64-unknown-linux-gnu", worker.binary);
            std::fs::write(directory.path().join(&bundled_name), &bundled_name)
                .expect("suffixed worker fixture should write");
            let env_name = stateful_worker_binary_env(&worker.name);
            let resolved = resolve_binary_in_directories(
                &worker.binary,
                &env_name,
                [directory.path().to_path_buf()],
            )
            .expect("catalog worker binary should resolve");
            assert_eq!(resolved, directory.path().join(bundled_name));
        }
        assert_eq!(
            stateful_worker_binary_env("workflow-lifecycle-worker"),
            "A4N_WORKFLOW_LIFECYCLE_WORKER_BINARY"
        );
        assert_eq!(
            stateful_worker_env("workflow-lifecycle-worker", "TOKEN"),
            "A4N_WORKFLOW_LIFECYCLE_WORKER_TOKEN"
        );
    }
}
