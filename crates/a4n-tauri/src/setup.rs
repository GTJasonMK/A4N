//! 应用初始化。
//!
//! 负责启动阶段的配置加载、目录准备和事件日志订阅。

mod config;
mod event_logger;

use a4n_error::AppError;
use a4n_runtime::{AppState, RuntimeProfile};
#[cfg(not(feature = "embedded-capabilities"))]
use a4n_service_protocol::MANAGED_SIDECARS_ENV;
use a4n_storage::paths::AppPaths;
use std::sync::Arc;
use tauri::AppHandle;
#[cfg(target_os = "macos")]
use tauri::Manager;

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
    let state = AppState::new_with_profile(config, runtime_profile).await?;
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

pub(crate) fn install_agent_host_dispatcher(runtime: &Arc<AppState>) -> Result<(), AppError> {
    let weak_runtime = Arc::downgrade(runtime);
    runtime.set_agent_host_dispatcher(Arc::new(move |user_id, command, args| {
        let Some(runtime) = weak_runtime.upgrade() else {
            return Box::pin(async { Err("A4N Host 已退出".to_string()) });
        };
        Box::pin(async move {
            a4n_server::dispatch_runtime_command(&runtime, user_id, args, command).await
        })
    }))
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
