//! 应用初始化。
//!
//! 负责启动阶段的配置加载、目录准备和事件日志订阅。

mod config;
mod event_logger;

use a4n_core::error::AppError;
use a4n_core::files::paths::AppPaths;
use a4n_runtime::{AppState, state::RuntimeProfile};
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
    let app_paths = AppPaths::from_config(&config);
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

    if std::env::var_os("A4N_PROMPT_SERVICE_PROMPTS_DIR").is_none() {
        let prompts_dir = resource_dir.join("prompts");
        if !prompts_dir.join("_registry.yaml").is_file() {
            return Err(AppError::ConfigError(format!(
                "Tauri bundle 缺少提示词注册表：{}",
                prompts_dir.join("_registry.yaml").display()
            )));
        }
        // SAFETY: Tauri setup runs before AppState starts managed sidecar processes.
        unsafe {
            std::env::set_var("A4N_PROMPT_SERVICE_PROMPTS_DIR", prompts_dir);
        }
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
