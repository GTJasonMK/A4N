use a4n_config::app_config::{AppConfig, portable_home_dir};
use reqwest::StatusCode;
use std::net::SocketAddr;
#[cfg(target_os = "linux")]
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;
use tauri::{AppHandle, Manager};

const SERVER_BINARY_ENV: &str = "A4N_SERVER_BINARY";
const CONTROL_BINARY_ENV: &str = "A4N_CONTROL_BINARY";
const EDGE_BINARY_ENV: &str = "A4N_EDGE_BINARY";
const AGENT_WORKER_BINARY_ENV: &str = "A4N_AGENT_WORKER_BINARY";
const NOVEL_PLANNING_WORKER_BINARY_ENV: &str = "A4N_NOVEL_PLANNING_WORKER_BINARY";
const ARTIFACT_MEDIA_WORKER_BINARY_ENV: &str = "A4N_ARTIFACT_MEDIA_WORKER_BINARY";
const OUTLINE_WORKER_BINARY_ENV: &str = "A4N_OUTLINE_WORKER_BINARY";
const BLUEPRINT_WORKER_BINARY_ENV: &str = "A4N_BLUEPRINT_WORKER_BINARY";
const ASSET_GRAPH_WORKER_BINARY_ENV: &str = "A4N_ASSET_GRAPH_WORKER_BINARY";
const FORESHADOWING_WORKER_BINARY_ENV: &str = "A4N_FORESHADOWING_WORKER_BINARY";
const OBJECT_PROFILE_WORKER_BINARY_ENV: &str = "A4N_OBJECT_PROFILE_WORKER_BINARY";
const SCENE_PROFILE_WORKER_BINARY_ENV: &str = "A4N_SCENE_PROFILE_WORKER_BINARY";
const PROFILE_MATERIAL_WORKER_BINARY_ENV: &str = "A4N_PROFILE_MATERIAL_WORKER_BINARY";
const PROJECT_STATUS_WORKER_BINARY_ENV: &str = "A4N_PROJECT_STATUS_WORKER_BINARY";
const THEME_READ_WORKER_BINARY_ENV: &str = "A4N_THEME_READ_WORKER_BINARY";
const PROJECT_READ_WORKER_BINARY_ENV: &str = "A4N_PROJECT_READ_WORKER_BINARY";
const PORTRAIT_WORKER_BINARY_ENV: &str = "A4N_PORTRAIT_WORKER_BINARY";
const SOURCE_SNAPSHOT_WORKER_BINARY_ENV: &str = "A4N_SOURCE_SNAPSHOT_WORKER_BINARY";
const MIGRATION_BINARY_ENV: &str = "A4N_MIGRATION_BINARY";

pub(crate) struct LocalServerProcess {
    child: Mutex<Option<Child>>,
    address_directory: tempfile::TempDir,
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
        let blueprint_worker =
            resolve_binary(app, "a4n-blueprint-worker", BLUEPRINT_WORKER_BINARY_ENV)?;
        let asset_graph_worker =
            resolve_binary(app, "a4n-asset-graph-worker", ASSET_GRAPH_WORKER_BINARY_ENV)?;
        let foreshadowing_worker = resolve_binary(
            app,
            "a4n-foreshadowing-worker",
            FORESHADOWING_WORKER_BINARY_ENV,
        )?;
        let object_profile_worker = resolve_binary(
            app,
            "a4n-object-profile-worker",
            OBJECT_PROFILE_WORKER_BINARY_ENV,
        )?;
        let scene_profile_worker = resolve_binary(
            app,
            "a4n-scene-profile-worker",
            SCENE_PROFILE_WORKER_BINARY_ENV,
        )?;
        let profile_material_worker = resolve_binary(
            app,
            "a4n-profile-material-worker",
            PROFILE_MATERIAL_WORKER_BINARY_ENV,
        )?;
        let project_status_worker = resolve_binary(
            app,
            "a4n-project-status-worker",
            PROJECT_STATUS_WORKER_BINARY_ENV,
        )?;
        let theme_read_worker =
            resolve_binary(app, "a4n-theme-read-worker", THEME_READ_WORKER_BINARY_ENV)?;
        let project_read_worker = resolve_binary(
            app,
            "a4n-project-read-worker",
            PROJECT_READ_WORKER_BINARY_ENV,
        )?;
        let portrait_worker =
            resolve_binary(app, "a4n-portrait-worker", PORTRAIT_WORKER_BINARY_ENV)?;
        let source_snapshot_worker = resolve_binary(
            app,
            "a4n-source-snapshot-worker",
            SOURCE_SNAPSHOT_WORKER_BINARY_ENV,
        )?;
        let migration = resolve_binary(app, "a4n-migration", MIGRATION_BINARY_ENV)?;
        run_migration(&migration)?;

        let mut command = Command::new(&server);
        command
            .env(
                "A4N_WEB_LISTEN_FILE",
                address_directory.path().join("http.addr"),
            )
            .env(CONTROL_BINARY_ENV, &control)
            .env(EDGE_BINARY_ENV, &edge)
            .env(AGENT_WORKER_BINARY_ENV, &agent_worker)
            .env(NOVEL_PLANNING_WORKER_BINARY_ENV, &novel_planning_worker)
            .env(ARTIFACT_MEDIA_WORKER_BINARY_ENV, &artifact_media_worker)
            .env(OUTLINE_WORKER_BINARY_ENV, &outline_worker)
            .env(BLUEPRINT_WORKER_BINARY_ENV, &blueprint_worker)
            .env(ASSET_GRAPH_WORKER_BINARY_ENV, &asset_graph_worker)
            .env(FORESHADOWING_WORKER_BINARY_ENV, &foreshadowing_worker)
            .env(OBJECT_PROFILE_WORKER_BINARY_ENV, &object_profile_worker)
            .env(SCENE_PROFILE_WORKER_BINARY_ENV, &scene_profile_worker)
            .env(PROFILE_MATERIAL_WORKER_BINARY_ENV, &profile_material_worker)
            .env(PROJECT_STATUS_WORKER_BINARY_ENV, &project_status_worker)
            .env(THEME_READ_WORKER_BINARY_ENV, &theme_read_worker)
            .env(PROJECT_READ_WORKER_BINARY_ENV, &project_read_worker)
            .env(PORTRAIT_WORKER_BINARY_ENV, &portrait_worker)
            .env(SOURCE_SNAPSHOT_WORKER_BINARY_ENV, &source_snapshot_worker)
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
        if let Some(address) = configured_address {
            command.env("A4N_WEB_ADDR", address.to_string());
        }
        configure_parent_death_signal(&mut command);

        if let Some(sidecar_dir) = bundled_sidecar_dir(app, &server) {
            command.env("A4N_SIDECAR_BIN_DIR", sidecar_dir);
        }

        let child = command
            .spawn()
            .map_err(|error| format!("启动 A4N Server 失败: {}: {error}", server.display()))?;
        let process = Self {
            child: Mutex::new(Some(child)),
            address_directory,
        };
        let api_base_url = wait_until_healthy(&process).await?;

        tracing::info!(url = %api_base_url, "A4N HTTP-only Server 启动完成");
        Ok((process, api_base_url))
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

#[cfg(test)]
mod tests {
    use super::{parse_local_server_address, read_server_address, resolve_binary_in_directories};

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
        for name in [
            "a4n-server",
            "a4n-control",
            "a4n-edge",
            "a4n-agent-worker",
            "a4n-novel-planning-worker",
            "a4n-artifact-media-worker",
            "a4n-outline-worker",
            "a4n-blueprint-worker",
            "a4n-asset-graph-worker",
            "a4n-foreshadowing-worker",
            "a4n-object-profile-worker",
            "a4n-scene-profile-worker",
            "a4n-profile-material-worker",
            "a4n-project-status-worker",
            "a4n-theme-read-worker",
            "a4n-project-read-worker",
            "a4n-portrait-worker",
            "a4n-source-snapshot-worker",
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
