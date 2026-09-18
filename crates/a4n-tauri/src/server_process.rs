use a4n_config::app_config::{AppConfig, portable_home_dir};
use reqwest::StatusCode;
#[cfg(target_os = "linux")]
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use tauri::{AppHandle, Manager};

const LOCAL_SERVER_URL: &str = "http://127.0.0.1:8080";
const SERVER_BINARY_ENV: &str = "A4N_SERVER_BINARY";
const CONTROL_BINARY_ENV: &str = "A4N_CONTROL_BINARY";
const EDGE_BINARY_ENV: &str = "A4N_EDGE_BINARY";
const AGENT_WORKER_BINARY_ENV: &str = "A4N_AGENT_WORKER_BINARY";
const NOVEL_PLANNING_WORKER_BINARY_ENV: &str = "A4N_NOVEL_PLANNING_WORKER_BINARY";
const ARTIFACT_MEDIA_WORKER_BINARY_ENV: &str = "A4N_ARTIFACT_MEDIA_WORKER_BINARY";
const OUTLINE_WORKER_BINARY_ENV: &str = "A4N_OUTLINE_WORKER_BINARY";
const MIGRATION_BINARY_ENV: &str = "A4N_MIGRATION_BINARY";

#[derive(Debug, PartialEq, Eq)]
enum ServerProbe {
    Healthy,
    Occupied,
    Unreachable,
}

pub(crate) struct LocalServerProcess {
    child: Mutex<Option<Child>>,
}

impl LocalServerProcess {
    pub(crate) async fn start(
        app: &AppHandle,
        config: &AppConfig,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        match server_probe().await {
            ServerProbe::Healthy => {
                return Err(format!(
                    "{LOCAL_SERVER_URL} 已有运行中的 A4N Server；local-server 模式不会复用未知版本的进程，请先停止它，或使用 remote 模式显式连接"
                )
                .into());
            }
            ServerProbe::Occupied => {
                return Err(format!(
                    "{LOCAL_SERVER_URL} 已被其他 HTTP 服务占用；请停止占用 8080 端口的服务，或使用 remote 模式显式连接"
                )
                .into());
            }
            ServerProbe::Unreachable => {}
        }

        let server = resolve_binary(app, "a4n-server", SERVER_BINARY_ENV)?;
        let control = resolve_binary(app, "a4n-control", CONTROL_BINARY_ENV)?;
        let edge = resolve_binary(app, "a4n-edge", EDGE_BINARY_ENV)?;
        let agent_worker = resolve_binary(app, "a4n-agent-worker", AGENT_WORKER_BINARY_ENV)?;
        let novel_planning_worker = resolve_binary(
            app,
            "a4n-novel-planning-worker",
            NOVEL_PLANNING_WORKER_BINARY_ENV,
        )?;
        let artifact_media_worker = resolve_binary(
            app,
            "a4n-artifact-media-worker",
            ARTIFACT_MEDIA_WORKER_BINARY_ENV,
        )?;
        let outline_worker = resolve_binary(app, "a4n-outline-worker", OUTLINE_WORKER_BINARY_ENV)?;
        let migration = resolve_binary(app, "a4n-migration", MIGRATION_BINARY_ENV)?;
        run_migration(&migration)?;

        let mut command = Command::new(&server);
        command
            .env("A4N_WEB_ADDR", "127.0.0.1:8080")
            .env(CONTROL_BINARY_ENV, &control)
            .env(EDGE_BINARY_ENV, &edge)
            .env(AGENT_WORKER_BINARY_ENV, &agent_worker)
            .env(NOVEL_PLANNING_WORKER_BINARY_ENV, &novel_planning_worker)
            .env(ARTIFACT_MEDIA_WORKER_BINARY_ENV, &artifact_media_worker)
            .env(OUTLINE_WORKER_BINARY_ENV, &outline_worker)
            .env("A4N_RUNTIME_PROFILE", "sidecar")
            .env("A4N_MANAGED_SIDECARS", "1")
            .env("A4N_PORTABLE_HOME", server_portable_home())
            .env("A4N_DATA_DIR", &config.data_dir)
            .env("A4N_CACHE_DIR", &config.cache_dir)
            .env("A4N_TEMP_DIR", &config.temp_dir)
            .env("A4N_LOG_DIR", &config.log_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        configure_parent_death_signal(&mut command);

        if let Some(sidecar_dir) = bundled_sidecar_dir(app, &server) {
            command.env("A4N_SIDECAR_BIN_DIR", sidecar_dir);
        }
        configure_onnxruntime(&mut command, app)?;

        let child = command
            .spawn()
            .map_err(|error| format!("启动 A4N Server 失败: {}: {error}", server.display()))?;
        let process = Self {
            child: Mutex::new(Some(child)),
        };
        if let Err(error) = wait_until_healthy(&process).await {
            process.stop();
            return Err(error);
        }

        tracing::info!(url = LOCAL_SERVER_URL, "A4N HTTP-only Server 启动完成");
        Ok(process)
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
    }
}

async fn server_probe() -> ServerProbe {
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
        return ServerProbe::Unreachable;
    };
    let Ok(response) = client
        .get(format!("{LOCAL_SERVER_URL}/api/health"))
        .send()
        .await
    else {
        return ServerProbe::Unreachable;
    };
    let status = response.status();
    let body = response.text().await.ok();
    classify_server_response(status, body.as_deref())
}

fn classify_server_response(status: StatusCode, body: Option<&str>) -> ServerProbe {
    if status == StatusCode::OK && body.is_some_and(|body| body.trim() == "ok") {
        ServerProbe::Healthy
    } else {
        ServerProbe::Occupied
    }
}

async fn wait_until_healthy(
    process: &LocalServerProcess,
) -> Result<(), Box<dyn std::error::Error>> {
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
            return Err(format!(
                "A4N Server 在健康检查通过前退出（{status}）；请检查 Server 日志，并确认 {LOCAL_SERVER_URL} 未被占用"
            )
            .into());
        }
        match server_probe().await {
            ServerProbe::Healthy => return Ok(()),
            ServerProbe::Occupied => {
                return Err(format!(
                    "{LOCAL_SERVER_URL} 在启动过程中被其他 HTTP 服务占用；请停止占用 8080 端口的服务"
                )
                .into());
            }
            ServerProbe::Unreachable => {}
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
    Err("A4N Server 启动超时；请检查 Server 日志".into())
}

fn run_migration(binary: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(binary)
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

fn bundled_sidecar_dir(app: &AppHandle, server: &Path) -> Option<PathBuf> {
    let server_parent = server.parent()?;
    let service_dir = server_parent.join("services");
    if service_dir.is_dir() {
        return Some(service_dir);
    }
    let resource_dir = app.path().resource_dir().ok()?;
    resource_dir.is_dir().then_some(resource_dir)
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

fn configure_onnxruntime(
    command: &mut Command,
    app: &AppHandle,
) -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::var_os("A4N_ONNXRUNTIME_ROOT")
        .map(PathBuf::from)
        .or_else(|| {
            app.path()
                .resource_dir()
                .ok()
                .filter(|path| path.join(onnxruntime_library_name()).is_file())
                .and_then(|path| path.parent().map(Path::to_path_buf))
        })
        .or_else(|| {
            std::env::var_os("HOME").map(PathBuf::from).map(|home| {
                home.join(".local/share/a4n/onnxruntime")
                    .join(onnxruntime_version())
            })
        })
        .ok_or("无法确定 ONNX Runtime 目录；请设置 A4N_ONNXRUNTIME_ROOT")?;
    let mut library_dir = root.join("lib");
    if !library_dir.join(onnxruntime_library_name()).is_file()
        && root.join(onnxruntime_library_name()).is_file()
    {
        library_dir = root.clone();
    }
    if !library_dir.join(onnxruntime_library_name()).is_file() {
        return Err(format!(
            "缺少 ONNX Runtime 动态库: {}",
            library_dir.join(onnxruntime_library_name()).display()
        )
        .into());
    }

    let loader_path = prepend_env_path(runtime_loader_path_var(), &library_dir)?;
    command
        .env("A4N_ONNXRUNTIME_ROOT", &root)
        .env("ORT_LIB_PATH", &library_dir)
        .env("ORT_PREFER_DYNAMIC_LINK", "1")
        .env(runtime_loader_path_var(), loader_path);
    Ok(())
}

fn prepend_env_path(name: &str, directory: &Path) -> Result<std::ffi::OsString, std::io::Error> {
    let mut paths = vec![directory.to_path_buf()];
    if let Some(current) = std::env::var_os(name) {
        paths.extend(std::env::split_paths(&current));
    }
    std::env::join_paths(paths)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidInput, error))
}

fn onnxruntime_version() -> &'static str {
    include_str!("../../../tools/onnxruntime/VERSION").trim()
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

#[cfg(test)]
mod tests {
    use super::{ServerProbe, classify_server_response, resolve_binary_in_directories};
    use reqwest::StatusCode;

    #[test]
    fn classifies_any_http_response_as_port_occupied() {
        assert_eq!(
            classify_server_response(StatusCode::OK, Some("ok\n")),
            ServerProbe::Healthy
        );
        assert_eq!(
            classify_server_response(StatusCode::NOT_FOUND, Some("not found")),
            ServerProbe::Occupied
        );
        assert_eq!(
            classify_server_response(StatusCode::OK, None),
            ServerProbe::Occupied
        );
    }

    #[test]
    fn resolves_every_supervised_binary_from_one_bundle_directory() {
        let directory = tempfile::tempdir().expect("temporary directory should be created");
        for name in [
            "a4n-server",
            "a4n-control",
            "a4n-edge",
            "a4n-agent-worker",
            "a4n-novel-planning-worker",
            "a4n-artifact-media-worker",
            "a4n-outline-worker",
            "a4n-migration",
        ] {
            std::fs::write(directory.path().join(name), name).expect("binary fixture should write");
            let resolved = resolve_binary_in_directories(
                name,
                "A4N_TEST_BINARY",
                [directory.path().to_path_buf()],
            )
            .expect("bundle binary should resolve");
            assert_eq!(resolved, directory.path().join(name));
        }
    }
}
