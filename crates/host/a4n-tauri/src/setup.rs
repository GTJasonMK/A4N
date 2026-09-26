//! 应用初始化。
//!
//! 负责启动阶段的配置加载、目录准备和事件日志订阅。

mod config;
mod event_logger;

use a4n_config::app_config::AppConfig;
use a4n_error::AppError;
use a4n_runtime::{AppState, RuntimeProfile, RuntimeRole};
#[cfg(not(feature = "embedded-capabilities"))]
use a4n_service_protocol::MANAGED_SIDECARS_ENV;
use a4n_storage::paths::AppPaths;
use tauri::AppHandle;
#[cfg(target_os = "macos")]
use tauri::Manager;

pub(crate) fn load_config(app: &AppHandle) -> Result<AppConfig, AppError> {
    config::load_config(app)
}

/// 初始化应用
///
/// 按顺序执行：加载配置 → 创建目录 → 构建 AppState → 启动事件日志订阅。
pub async fn initialize(app: &AppHandle) -> Result<AppState, AppError> {
    #[cfg(target_os = "macos")]
    configure_bundled_sidecar_resources(app)?;

    let config = config::load_config(app)?;
    let app_paths = AppPaths::new(
        config.data_dir.clone(),
        config.cache_dir.clone(),
        config.temp_dir.clone(),
        config.log_dir.clone(),
    );
    app_paths.ensure_dirs().map_err(AppError::Io)?;

    let runtime_profile = runtime_profile_for_tauri_host()?;
    let state =
        a4n_host_assembly::initialize_runtime(config, runtime_profile, RuntimeRole::DesktopHost)
            .await?;
    event_logger::spawn_event_logger(&state);

    Ok(state)
}

#[cfg(target_os = "macos")]
fn configure_bundled_sidecar_resources(app: &AppHandle) -> Result<(), AppError> {
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|error| AppError::ConfigError(format!("无法解析 Tauri 资源目录: {error}")))?;
    if !resource_dir.join("libonnxruntime.dylib").is_file() {
        return Ok(());
    }

    let mut paths = vec![resource_dir];
    if let Some(current) = std::env::var_os("DYLD_LIBRARY_PATH") {
        paths.extend(std::env::split_paths(&current));
    }
    let value = std::env::join_paths(paths)
        .map_err(|error| AppError::ConfigError(format!("ONNX Runtime 资源路径无效: {error}")))?;
    // SAFETY: Tauri setup runs before AppState starts managed sidecar processes.
    unsafe {
        std::env::set_var("DYLD_LIBRARY_PATH", value);
    }
    Ok(())
}

fn runtime_profile_for_tauri_host() -> Result<RuntimeProfile, AppError> {
    let configured = RuntimeProfile::configured_from_env()?;
    #[cfg(not(feature = "embedded-capabilities"))]
    {
        if configured.is_none() {
            default_managed_sidecars_for_minimal_host();
            tracing::info!(
                runtime_profile = RuntimeProfile::Sidecar.as_str(),
                "Tauri host built without embedded capabilities; defaulting to bundled sidecars"
            );
        }
    }
    Ok(configured.unwrap_or_else(RuntimeProfile::default_for_build))
}

#[cfg(not(feature = "embedded-capabilities"))]
fn default_managed_sidecars_for_minimal_host() {
    if std::env::var_os(MANAGED_SIDECARS_ENV).is_some() {
        return;
    }

    // SAFETY: this runs during Tauri setup before AppState creates runtime workers
    // or starts sidecar manager processes, and only sets a fixed ASCII key/value.
    unsafe {
        std::env::set_var(MANAGED_SIDECARS_ENV, "1");
    }
}
