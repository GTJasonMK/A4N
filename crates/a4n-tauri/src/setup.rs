//! 应用初始化。
//!
//! 负责启动阶段的配置加载、目录准备和事件日志订阅。

mod config;
mod event_logger;

use a4n_core::error::AppError;
use a4n_core::files::paths::AppPaths;
use a4n_runtime::{AppState, state::RuntimeProfile};
use tauri::AppHandle;

/// 初始化应用
///
/// 按顺序执行：加载配置 → 创建目录 → 构建 AppState → 启动事件日志订阅。
pub async fn initialize(app: &AppHandle) -> Result<AppState, AppError> {
    let config = config::load_config(app)?;
    let app_paths = AppPaths::from_config(&config);
    app_paths.ensure_dirs().map_err(AppError::Io)?;

    let runtime_profile = runtime_profile_for_tauri_host()?;
    let state = AppState::new_with_profile(config, runtime_profile).await?;
    event_logger::spawn_event_logger(state.event_bus.clone()).await;

    Ok(state)
}

fn runtime_profile_for_tauri_host() -> Result<RuntimeProfile, AppError> {
    #[cfg(not(feature = "embedded-capabilities"))]
    {
        if std::env::var_os("A4N_RUNTIME_PROFILE").is_none() {
            default_managed_sidecars_for_minimal_host();
            tracing::info!(
                runtime_profile = RuntimeProfile::Sidecar.as_str(),
                "Tauri host built without embedded capabilities; defaulting to bundled sidecars"
            );
            return Ok(RuntimeProfile::Sidecar);
        }
    }

    RuntimeProfile::from_env()
}

#[cfg(not(feature = "embedded-capabilities"))]
fn default_managed_sidecars_for_minimal_host() {
    if std::env::var_os("A4N_MANAGED_SIDECARS").is_some() {
        return;
    }

    // SAFETY: this runs during Tauri setup before AppState creates runtime workers
    // or starts sidecar manager processes, and only sets a fixed ASCII key/value.
    unsafe {
        std::env::set_var("A4N_MANAGED_SIDECARS", "1");
    }
}
