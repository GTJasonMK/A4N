use a4n_config::app_config::AppConfig;
use a4n_runtime::AppState;
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, SystemTime},
};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_updater::{Update, UpdaterExt};

const BACKUP_RETENTION: usize = 2;
const MIN_FREE_SPACE_BYTES: u64 = 128 * 1024 * 1024;
const UPDATE_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppUpdateInfo {
    pub version: String,
    pub notes: Option<String>,
    pub pub_date: Option<String>,
    pub target: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppUpdatePreflight {
    pub backup_path: Option<String>,
    pub migrations: Vec<a4n_migration::migrations::canvas_project_split::DataMigrationPreflight>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
pub enum AppUpdateProgressEvent {
    Preparing {
        migration_count: usize,
    },
    Started {
        chunk_length: usize,
        content_length: Option<u64>,
    },
    Progress {
        chunk_length: usize,
    },
    Finished,
}

#[derive(Default)]
pub struct UpdateInstallState {
    busy: AtomicBool,
    prepared: Mutex<Option<PreparedUpdate>>,
    downloaded: Mutex<Option<DownloadedUpdate>>,
}

#[derive(Clone)]
struct PreparedUpdate {
    version: String,
    data_migrations: Vec<String>,
    pending_migrations: Vec<String>,
    backup_path: Option<PathBuf>,
}

struct DownloadedUpdate {
    version: String,
    data_migrations: Vec<String>,
    bytes: Vec<u8>,
    update: Update,
}

struct OperationGuard<'a>(&'a AtomicBool);

impl Drop for OperationGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

impl UpdateInstallState {
    fn begin(&self) -> Result<OperationGuard<'_>, String> {
        self.busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| "应用更新已在进行中".to_string())?;
        Ok(OperationGuard(&self.busy))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UpdateRecoveryState {
    pub state: String,
    pub backup_path: Option<String>,
    pub target_version: String,
    pub previous_version: String,
    pub pending_migrations: Vec<String>,
    pub failure_reason: Option<String>,
}

#[tauri::command]
pub async fn check_app_update(app: AppHandle) -> Result<Option<AppUpdateInfo>, String> {
    ensure_updater_installation()?;
    let update = build_updater(&app)?
        .check()
        .await
        .map_err(|error| format!("检查更新失败：{error}"))?;
    update
        .map(|update| -> Result<AppUpdateInfo, String> {
            data_migrations(&update.raw_json)?;
            Ok(AppUpdateInfo {
                version: update.version,
                notes: update.body,
                pub_date: update.date.map(|date| date.to_string()),
                target: update.target,
            })
        })
        .transpose()
}

#[tauri::command]
pub async fn prepare_app_update_install(
    app: AppHandle,
    install_state: State<'_, UpdateInstallState>,
    state: State<'_, AppState>,
    expected_version: String,
) -> Result<AppUpdatePreflight, String> {
    ensure_updater_installation()?;
    let _operation = install_state.begin()?;
    let config = state.host_app_config();
    ensure_no_pending_update_recovery(&config)?;
    *install_state
        .prepared
        .lock()
        .map_err(|_| "读取更新预检状态失败".to_string())? = None;
    *install_state
        .downloaded
        .lock()
        .map_err(|_| "读取已下载更新状态失败".to_string())? = None;
    let update = checked_update(&app, &expected_version).await?;
    let active =
        a4n_runtime::host_api::generation_tasks::active_generation_task_count(state.inner())
            .await
            .map_err(|error| format!("检查活动生成任务失败：{error}"))?;
    if active > 0 {
        return Err(format!(
            "有 {} 个生成任务仍在运行，请等待完成后再更新",
            active
        ));
    }

    let all_data_migrations = data_migrations(&update.raw_json)?;
    let migration_preflight = a4n_migration::migrator::preflight_update_migrations(
        &config.database_url,
        &config.data_dir,
        &all_data_migrations,
    )
    .await
    .map_err(|error| format!("数据库更新预检失败：{error}"))?;
    if !migration_preflight.pending_schema_migrations.is_empty() {
        return Err("当前数据库仍有未完成的应用迁移，请重启 A4N 后再更新".to_string());
    }
    let pending_migrations = migration_preflight.pending_data_migrations;
    let migrations = migration_preflight.migrations;
    let backup_path = if pending_migrations.is_empty() {
        None
    } else {
        Some(create_verified_database_backup(
            &state,
            &pending_migrations,
        )?)
    };
    *install_state
        .prepared
        .lock()
        .map_err(|_| "读取更新预检状态失败".to_string())? = Some(PreparedUpdate {
        version: update.version,
        data_migrations: all_data_migrations,
        pending_migrations,
        backup_path: backup_path.clone(),
    });

    Ok(AppUpdatePreflight {
        backup_path: backup_path.map(|path| path.display().to_string()),
        migrations,
    })
}

#[tauri::command]
pub async fn download_app_update(
    app: AppHandle,
    install_state: State<'_, UpdateInstallState>,
    progress: tauri::ipc::Channel<AppUpdateProgressEvent>,
    expected_version: String,
) -> Result<(), String> {
    ensure_updater_installation()?;
    let _operation = install_state.begin()?;
    let prepared = install_state
        .prepared
        .lock()
        .map_err(|_| "读取更新预检状态失败".to_string())?
        .as_ref()
        .cloned()
        .ok_or_else(|| "请先完成更新预检".to_string())?;
    if prepared.version != expected_version {
        return Err("更新版本与预检版本不一致，请重新检查更新".to_string());
    }

    let update = checked_update(&app, &expected_version).await?;
    let migrations = data_migrations(&update.raw_json)?;
    if migrations != prepared.data_migrations {
        return Err("更新清单中的数据迁移已变化，请重新检查更新".to_string());
    }
    progress
        .send(AppUpdateProgressEvent::Preparing {
            migration_count: prepared.pending_migrations.len(),
        })
        .map_err(|error| format!("发送更新进度失败：{error}"))?;

    let started = AtomicBool::new(false);
    let bytes = update
        .download(
            |chunk_length, content_length| {
                let event = if started.swap(true, Ordering::AcqRel) {
                    AppUpdateProgressEvent::Progress { chunk_length }
                } else {
                    AppUpdateProgressEvent::Started {
                        chunk_length,
                        content_length,
                    }
                };
                let _ = progress.send(event);
            },
            || {
                let _ = progress.send(AppUpdateProgressEvent::Finished);
            },
        )
        .await
        .map_err(|error| format!("下载或校验更新失败：{error}"))?;

    *install_state
        .downloaded
        .lock()
        .map_err(|_| "暂存已验证更新包失败".to_string())? = Some(DownloadedUpdate {
        version: update.version.clone(),
        data_migrations: migrations,
        bytes,
        update,
    });
    Ok(())
}

#[tauri::command]
pub async fn install_app_update(
    app: AppHandle,
    install_state: State<'_, UpdateInstallState>,
    state: State<'_, AppState>,
    expected_version: String,
) -> Result<(), String> {
    ensure_updater_installation()?;
    let _operation = install_state.begin()?;
    let prepared = install_state
        .prepared
        .lock()
        .map_err(|_| "读取更新预检状态失败".to_string())?
        .as_ref()
        .cloned()
        .ok_or_else(|| "请先完成更新预检".to_string())?;
    if prepared.version != expected_version {
        return Err("更新版本与预检版本不一致，请重新检查更新".to_string());
    }

    let active =
        a4n_runtime::host_api::generation_tasks::active_generation_task_count(state.inner())
            .await
            .map_err(|error| format!("安装前检查活动生成任务失败：{error}"))?;
    if active > 0 {
        return Err(format!(
            "有 {} 个生成任务仍在运行，更新包已下载，请等待任务完成后重试",
            active
        ));
    }

    let config = state.host_app_config();
    ensure_no_pending_update_recovery(&config)?;

    let downloaded = install_state
        .downloaded
        .lock()
        .map_err(|_| "读取已下载更新状态失败".to_string())?
        .take()
        .ok_or_else(|| "请先下载并验证更新包".to_string())?;
    if downloaded.version != expected_version
        || downloaded.data_migrations != prepared.data_migrations
    {
        return Err("已下载更新包与预检版本不一致，请重新下载".to_string());
    }

    let recovery = UpdateRecoveryState {
        state: "installing".to_string(),
        backup_path: prepared
            .backup_path
            .as_ref()
            .map(|path| path.display().to_string()),
        target_version: expected_version,
        previous_version: app.package_info().version.to_string(),
        pending_migrations: prepared.pending_migrations,
        failure_reason: None,
    };
    write_update_recovery_state(config, &recovery)?;

    if let Err(failure) = shutdown_for_update(&app, &state).await {
        mark_update_failed(config, &recovery, &failure.message);
        if failure.partial_shutdown {
            tauri::process::restart(&app.env());
        }
        return Err(failure.message);
    }
    if let Err(error) = downloaded.update.install(&downloaded.bytes) {
        mark_update_failed(config, &recovery, &error.to_string());
        tracing::error!(%error, "启动应用更新安装器失败，正在恢复旧应用");
    }
    tauri::process::restart(&app.env());
}

#[tauri::command]
pub fn get_update_recovery_state(
    state: State<'_, AppState>,
) -> Result<Option<UpdateRecoveryState>, String> {
    let config = state.host_app_config();
    let recovery = read_update_recovery_state(config)?;
    Ok(recovery.filter(|value| value.state == "failed"))
}

#[tauri::command]
pub fn clear_update_recovery_state(state: State<'_, AppState>) -> Result<(), String> {
    let config = state.host_app_config();
    let recovery = read_update_recovery_state(config)?;
    if recovery
        .as_ref()
        .is_none_or(|value| value.state != "failed")
    {
        return Err("没有可关闭的更新失败记录".to_string());
    }
    std::fs::remove_file(update_recovery_path(config))
        .map_err(|error| format!("清理更新失败记录失败：{error}"))
}

pub(crate) fn pending_update_data_migrations(
    app: &AppHandle,
    config: &AppConfig,
) -> Result<Option<Vec<String>>, String> {
    let Some(mut recovery) = read_update_recovery_state(config)? else {
        return Ok(None);
    };
    if recovery.state != "installing" {
        return Ok(None);
    }
    let current_version = app.package_info().version.to_string();
    if current_version == recovery.target_version {
        return Ok(Some(recovery.pending_migrations));
    }

    recovery.state = "failed".to_string();
    recovery.failure_reason = Some(format!(
        "更新后仍运行版本 {current_version}，目标版本为 {}；数据库备份已保留",
        recovery.target_version
    ));
    write_update_recovery_state(config, &recovery)?;
    Ok(None)
}

pub(crate) fn complete_update_startup(app: &AppHandle, config: &AppConfig) -> Result<(), String> {
    let Some(recovery) = read_update_recovery_state(config)? else {
        return Ok(());
    };
    if recovery.target_version != app.package_info().version.to_string() {
        return Err("更新恢复状态与当前版本不一致".to_string());
    }
    std::fs::remove_file(update_recovery_path(config))
        .map_err(|error| format!("清理已完成的更新状态失败：{error}"))
}

pub(crate) fn record_update_migration_failure(
    config: &AppConfig,
    reason: &str,
) -> Result<(), String> {
    let Some(mut recovery) = read_update_recovery_state(config)? else {
        return Ok(());
    };
    if recovery.state != "installing" {
        return Ok(());
    }
    recovery.failure_reason = Some(reason.to_string());
    write_update_recovery_state(config, &recovery)
}

async fn checked_update(app: &AppHandle, version: &str) -> Result<Update, String> {
    let update = build_updater(app)?
        .check()
        .await
        .map_err(|error| format!("重新验证更新清单失败：{error}"))?
        .ok_or_else(|| "更新已不存在，请重新检查版本".to_string())?;
    if update.version != version {
        return Err("更新版本已变化，请重新检查版本".to_string());
    }
    data_migrations(&update.raw_json)?;
    Ok(update)
}

fn build_updater(app: &AppHandle) -> Result<tauri_plugin_updater::Updater, String> {
    app.updater_builder()
        .timeout(UPDATE_TIMEOUT)
        .build()
        .map_err(|error| format!("创建更新检查失败：{error}"))
}

fn ensure_updater_installation() -> Result<(), String> {
    if cfg!(debug_assertions) || tauri::utils::platform::bundle_type().is_none() {
        return Err("应用内更新仅支持正式桌面安装包".to_string());
    }
    Ok(())
}

fn data_migrations(raw_json: &serde_json::Value) -> Result<Vec<String>, String> {
    let values = raw_json
        .get("data_migrations")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "更新清单缺少 data_migrations，拒绝安装".to_string())?;
    let mut names = Vec::with_capacity(values.len());
    for value in values {
        let name = value
            .as_str()
            .filter(|name| !name.trim().is_empty())
            .ok_or_else(|| "更新清单包含无效的数据迁移名称".to_string())?;
        if names.iter().any(|existing| existing == name) {
            return Err(format!("更新清单重复声明数据迁移：{name}"));
        }
        if !a4n_migration::migrator::migration_requires_data_backup(name) {
            return Err(format!("更新清单包含未注册的数据迁移：{name}"));
        }
        names.push(name.to_string());
    }
    Ok(names)
}

async fn shutdown_for_update(app: &AppHandle, state: &AppState) -> Result<(), ShutdownFailure> {
    let active = a4n_runtime::host_api::generation_tasks::active_generation_task_count(state)
        .await
        .map_err(|error| {
            ShutdownFailure::before_shutdown(format!("再次检查活动生成任务失败：{error}"))
        })?;
    if active > 0 {
        return Err(ShutdownFailure::before_shutdown(
            "检测到刚刚启动的生成任务，取消本次更新".to_string(),
        ));
    }

    let mut partial_shutdown = false;
    if let Some(server) = app.try_state::<crate::server_process::LocalServerProcess>() {
        server.stop();
        partial_shutdown = true;
    }
    if let Some(workers) = app.try_state::<crate::server_process::NativeWorkerProcesses>() {
        workers.stop_agent();
        partial_shutdown = true;
    }
    if let Err(error) = state.shutdown_managed_sidecars().await {
        return Err(ShutdownFailure {
            message: format!("停止本地服务失败，更新已取消：{error}"),
            partial_shutdown,
        });
    }
    if let Some(workers) = app.try_state::<crate::server_process::NativeWorkerProcesses>() {
        workers.stop();
    }
    Ok(())
}

struct ShutdownFailure {
    message: String,
    partial_shutdown: bool,
}

impl ShutdownFailure {
    fn before_shutdown(message: String) -> Self {
        Self {
            message,
            partial_shutdown: false,
        }
    }
}

fn mark_update_failed(config: &AppConfig, recovery: &UpdateRecoveryState, reason: &str) {
    let mut failed = recovery.clone();
    failed.state = "failed".to_string();
    failed.failure_reason = Some(reason.to_string());
    if let Err(error) = write_update_recovery_state(config, &failed) {
        tracing::error!(%error, "写入应用更新失败状态失败");
    }
}

fn update_recovery_path(config: &AppConfig) -> PathBuf {
    config.data_dir.join("update-recovery").join("state.json")
}

fn read_update_recovery_state(config: &AppConfig) -> Result<Option<UpdateRecoveryState>, String> {
    let path = update_recovery_path(config);
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("读取应用更新恢复状态失败：{error}")),
    };
    let recovery = serde_json::from_slice::<UpdateRecoveryState>(&bytes)
        .map_err(|error| format!("应用更新恢复状态无效：{error}"))?;
    if !matches!(recovery.state.as_str(), "installing" | "failed")
        || recovery
            .pending_migrations
            .iter()
            .any(|name| !a4n_migration::migrator::migration_requires_data_backup(name))
    {
        return Err("应用更新恢复状态包含无效的迁移或状态".to_string());
    }
    Ok(Some(recovery))
}

fn ensure_no_pending_update_recovery(config: &AppConfig) -> Result<(), String> {
    if let Some(recovery) = read_update_recovery_state(config)? {
        return Err(format!(
            "存在尚未处理的应用更新恢复记录（状态：{}），请先处理后再更新",
            recovery.state
        ));
    }
    Ok(())
}

fn write_update_recovery_state(
    config: &AppConfig,
    recovery: &UpdateRecoveryState,
) -> Result<(), String> {
    let path = update_recovery_path(config);
    let directory = path
        .parent()
        .ok_or_else(|| "应用更新恢复目录无效".to_string())?;
    std::fs::create_dir_all(directory)
        .map_err(|error| format!("创建应用更新恢复目录失败：{error}"))?;
    let mut temp = tempfile::NamedTempFile::new_in(directory)
        .map_err(|error| format!("创建应用更新恢复状态临时文件失败：{error}"))?;
    serde_json::to_writer_pretty(&mut temp, recovery)
        .map_err(|error| format!("序列化应用更新恢复状态失败：{error}"))?;
    temp.as_file()
        .sync_all()
        .map_err(|error| format!("同步应用更新恢复状态失败：{error}"))?;
    temp.persist(&path)
        .map_err(|error| format!("提交应用更新恢复状态失败：{}", error.error))?;
    Ok(())
}

fn create_verified_database_backup(
    state: &AppState,
    pending_migrations: &[String],
) -> Result<PathBuf, String> {
    let config = state.host_app_config();
    let backups_dir = config.data_dir.join("update-backups");
    std::fs::create_dir_all(&backups_dir)
        .map_err(|error| format!("创建数据库备份目录失败：{error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&backups_dir, std::fs::Permissions::from_mode(0o700))
            .map_err(|error| format!("限制数据库备份目录权限失败：{error}"))?;
    }
    let available = fs2::available_space(&backups_dir)
        .map_err(|error| format!("检查备份磁盘空间失败：{error}"))?;
    if available < MIN_FREE_SPACE_BYTES {
        return Err("可用空间少于 128 MiB，拒绝启动更新".to_string());
    }

    let database_url = config.database_url.trim();
    if database_url.is_empty() {
        return Err("数据库连接尚未配置，不能创建更新备份".to_string());
    }
    let safe_database_url = sanitized_database_url(database_url)?;
    let password_file = write_pgpass(database_url, &backups_dir)?;
    let pg_dump = postgres_tool("pg_dump")?;
    let pg_restore = postgres_tool("pg_restore")?;
    let timestamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
    let backup_path = backups_dir.join(format!(
        "a4n-before-update-{timestamp}-{}.dump",
        uuid::Uuid::new_v4()
    ));
    let partial = tempfile::Builder::new()
        .prefix("a4n-before-update-")
        .suffix(".dump.partial")
        .tempfile_in(&backups_dir)
        .map_err(|error| format!("创建数据库备份临时文件失败：{error}"))?
        .into_temp_path();
    let output = std::process::Command::new(pg_dump)
        .args(["--format=custom", "--no-password", "--file"])
        .arg(&partial)
        .arg("--dbname")
        .arg(&safe_database_url)
        .env("PGPASSFILE", password_file.path())
        .output()
        .map_err(|error| format!("启动 pg_dump 失败，请安装 PostgreSQL 客户端工具：{error}"))?;
    if !output.status.success() {
        return Err(format!(
            "数据库备份失败，拒绝启动更新：{}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let verify = std::process::Command::new(pg_restore)
        .args(["--list", "--no-password"])
        .arg(&partial)
        .output()
        .map_err(|error| format!("启动 pg_restore 备份校验失败：{error}"))?;
    if !verify.status.success() {
        return Err(format!(
            "数据库备份校验失败：{}",
            String::from_utf8_lossy(&verify.stderr).trim()
        ));
    }

    let checksum = sha256_file(&partial)?;
    let size_bytes = std::fs::metadata(&partial)
        .map_err(|error| format!("读取数据库备份大小失败：{error}"))?
        .len();
    partial
        .persist(&backup_path)
        .map_err(|error| format!("提交已验证数据库备份失败：{}", error.error))?;
    let metadata = serde_json::json!({
        "pending_migrations": pending_migrations,
        "created_at": chrono::Utc::now().to_rfc3339(),
        "sha256": checksum,
        "size_bytes": size_bytes,
    });
    write_json_atomically(&backup_path.with_extension("dump.json"), &metadata)?;
    retain_recent_backups(&backups_dir, BACKUP_RETENTION)?;
    Ok(backup_path)
}

fn sanitized_database_url(database_url: &str) -> Result<String, String> {
    let mut url =
        url::Url::parse(database_url).map_err(|error| format!("数据库 URL 无效：{error}"))?;
    url.set_password(None)
        .map_err(|_| "数据库 URL 凭据无效".to_string())?;
    Ok(url.to_string())
}

fn write_pgpass(database_url: &str, directory: &Path) -> Result<tempfile::NamedTempFile, String> {
    let url = url::Url::parse(database_url).map_err(|error| format!("数据库 URL 无效：{error}"))?;
    let host = url
        .host_str()
        .ok_or_else(|| "数据库 URL 缺少主机名".to_string())?;
    let port = url.port_or_known_default().unwrap_or(5432);
    let database = decode_url_component(url.path().trim_start_matches('/'));
    let username = decode_url_component(url.username());
    let password = decode_url_component(url.password().unwrap_or_default());
    let escape = |value: &str| {
        value
            .replace('\\', "\\\\")
            .replace(':', "\\:")
            .replace('*', "\\*")
    };
    let mut file = tempfile::Builder::new()
        .prefix(".pgpass-")
        .tempfile_in(directory)
        .map_err(|error| format!("创建临时 PostgreSQL 凭据文件失败：{error}"))?;
    writeln!(
        file,
        "{}:{}:{}:{}:{}",
        escape(host),
        port,
        escape(&database),
        escape(&username),
        escape(&password)
    )
    .map_err(|error| format!("写入临时 PostgreSQL 凭据文件失败：{error}"))?;
    file.as_file()
        .sync_all()
        .map_err(|error| format!("同步临时 PostgreSQL 凭据文件失败：{error}"))?;
    Ok(file)
}

fn decode_url_component(value: &str) -> String {
    percent_encoding::percent_decode_str(value)
        .decode_utf8_lossy()
        .into_owned()
}

fn postgres_tool(name: &str) -> Result<PathBuf, String> {
    let suffix = if cfg!(windows) { ".exe" } else { "" };
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .map(|directory| directory.join(format!("{name}{suffix}")))
        .find(|path| path.is_file())
        .ok_or_else(|| format!("找不到 {name}，请安装兼容版本的 PostgreSQL 客户端工具"))
}

fn sha256_file(path: &Path) -> Result<String, String> {
    use sha2::Digest;

    let mut file =
        std::fs::File::open(path).map_err(|error| format!("打开数据库备份失败：{error}"))?;
    let mut digest = sha2::Sha256::new();
    let mut buffer = [0; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("读取数据库备份失败：{error}"))?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn write_json_atomically(path: &Path, value: &serde_json::Value) -> Result<(), String> {
    let directory = path
        .parent()
        .ok_or_else(|| "JSON 文件目录无效".to_string())?;
    let mut temp = tempfile::NamedTempFile::new_in(directory)
        .map_err(|error| format!("创建备份元数据临时文件失败：{error}"))?;
    serde_json::to_writer_pretty(&mut temp, value)
        .map_err(|error| format!("序列化备份元数据失败：{error}"))?;
    temp.as_file()
        .sync_all()
        .map_err(|error| format!("同步备份元数据失败：{error}"))?;
    temp.persist(path)
        .map_err(|error| format!("提交备份元数据失败：{}", error.error))?;
    Ok(())
}

fn retain_recent_backups(directory: &Path, retain: usize) -> Result<(), String> {
    let mut backups = Vec::new();
    for entry in
        std::fs::read_dir(directory).map_err(|error| format!("读取数据库备份目录失败：{error}"))?
    {
        let entry = entry.map_err(|error| format!("读取数据库备份目录项失败：{error}"))?;
        let path = entry.path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if name.starts_with("a4n-before-update-")
            && path
                .extension()
                .is_some_and(|extension| extension == "dump")
        {
            let modified = entry
                .metadata()
                .and_then(|metadata| metadata.modified())
                .map_err(|error| format!("读取数据库备份时间失败：{error}"))?;
            backups.push((modified, path));
        }
    }
    backups.sort_by_key(|(modified, _)| std::cmp::Reverse(*modified));
    for (_, path) in backups.into_iter().skip(retain) {
        std::fs::remove_file(&path)
            .map_err(|error| format!("清理旧更新备份失败（{}）：{error}", path.display()))?;
        match std::fs::remove_file(path.with_extension("dump.json")) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(format!("清理旧更新备份清单失败：{error}")),
        }
    }
    remove_stale_partial_backups(directory)
}

fn remove_stale_partial_backups(directory: &Path) -> Result<(), String> {
    let cutoff = SystemTime::now() - Duration::from_secs(24 * 60 * 60);
    for entry in
        std::fs::read_dir(directory).map_err(|error| format!("读取数据库备份目录失败：{error}"))?
    {
        let entry = entry.map_err(|error| format!("读取数据库备份目录项失败：{error}"))?;
        let path = entry.path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if name.starts_with("a4n-before-update-") && name.ends_with(".dump.partial") {
            let modified = entry
                .metadata()
                .and_then(|metadata| metadata.modified())
                .map_err(|error| format!("读取临时备份时间失败：{error}"))?;
            if modified < cutoff {
                std::fs::remove_file(path)
                    .map_err(|error| format!("清理中断的备份文件失败：{error}"))?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        UpdateRecoveryState, ensure_no_pending_update_recovery, read_update_recovery_state,
        update_recovery_path, write_update_recovery_state,
    };
    use a4n_config::app_config::AppConfig;

    #[test]
    fn pending_recovery_blocks_update_without_replacing_the_record() {
        let directory = tempfile::tempdir().unwrap();
        let config = AppConfig {
            data_dir: directory.path().join("data"),
            ..AppConfig::default()
        };
        let recovery = UpdateRecoveryState {
            state: "failed".to_string(),
            backup_path: Some("/work/backup.dump".to_string()),
            target_version: "2.0.0".to_string(),
            previous_version: "1.9.0".to_string(),
            pending_migrations: Vec::new(),
            failure_reason: Some("installer failed".to_string()),
        };
        write_update_recovery_state(&config, &recovery).unwrap();

        let error = ensure_no_pending_update_recovery(&config).unwrap_err();

        assert!(error.contains("状态：failed"));
        assert_eq!(
            read_update_recovery_state(&config)
                .unwrap()
                .unwrap()
                .target_version,
            "2.0.0"
        );
    }

    #[test]
    fn unreadable_recovery_record_blocks_update_and_remains_intact() {
        let directory = tempfile::tempdir().unwrap();
        let config = AppConfig {
            data_dir: directory.path().join("data"),
            ..AppConfig::default()
        };
        let path = update_recovery_path(&config);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"invalid recovery JSON").unwrap();

        let error = ensure_no_pending_update_recovery(&config).unwrap_err();

        assert!(error.contains("更新恢复状态无效"));
        assert_eq!(std::fs::read(path).unwrap(), b"invalid recovery JSON");
    }
}
