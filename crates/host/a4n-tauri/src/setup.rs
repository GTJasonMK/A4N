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

pub(crate) fn load_config(app: &AppHandle) -> Result<AppConfig, AppError> {
    config::load_config(app)
}

pub(crate) fn load_native_config(app: &AppHandle) -> Result<AppConfig, AppError> {
    config::load_config(app)
}

/// 初始化应用
///
/// 按顺序执行：创建目录 → 构建 AppState → 启动事件日志订阅。
pub async fn initialize(config: AppConfig) -> Result<AppState, AppError> {
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

pub(crate) fn configure_bundled_sidecar_resources(
    package: &tauri::utils::PackageInfo,
) -> Result<(), AppError> {
    let resources = tauri::utils::platform::resource_dir(package, &tauri::utils::Env::default())
        .map_err(|error| AppError::ConfigError(format!("无法解析 Tauri 资源目录: {error}")))?;
    let prompts = resources.join("prompts");
    let prompts_env = a4n_service_protocol::PROMPT_SERVICE_PROMPTS_DIR_ENV;
    if a4n_config::optional_non_empty_env_path(prompts_env)?.is_none()
        && a4n_config::optional_non_empty_env_path("A4N_PORTABLE_HOME")?.is_none()
        && prompts.join("_registry.yaml").is_file()
    {
        // SAFETY: this function runs before application threads start.
        unsafe { std::env::set_var(prompts_env, prompts) };
    }
    let configured = a4n_config::optional_non_empty_env_path("A4N_ONNXRUNTIME_ROOT")?;
    let Some(library_dir) = runtime_library_dir(&resources, configured.as_deref())? else {
        return Ok(());
    };
    let loader_variable = if cfg!(target_os = "windows") {
        "PATH"
    } else if cfg!(target_os = "macos") {
        "DYLD_LIBRARY_PATH"
    } else {
        "LD_LIBRARY_PATH"
    };
    let mut paths = vec![library_dir.clone()];
    if let Some(current) = std::env::var_os(loader_variable) {
        paths.extend(std::env::split_paths(&current));
    }
    let value = std::env::join_paths(paths)
        .map_err(|error| AppError::ConfigError(format!("ONNX Runtime 资源路径无效: {error}")))?;
    // SAFETY: app::run calls this before logging, Tauri, or worker threads start.
    unsafe {
        std::env::set_var(loader_variable, value);
        if configured.is_none() {
            std::env::set_var("A4N_ONNXRUNTIME_ROOT", library_dir);
        }
    }
    Ok(())
}

fn runtime_library_dir(
    resources: &std::path::Path,
    configured: Option<&std::path::Path>,
) -> Result<Option<std::path::PathBuf>, AppError> {
    let library = if cfg!(target_os = "linux") {
        "libonnxruntime.so.1".to_string()
    } else {
        format!(
            "{}onnxruntime{}",
            std::env::consts::DLL_PREFIX,
            std::env::consts::DLL_SUFFIX
        )
    };
    let root = configured.unwrap_or(resources);
    for directory in [root.join("lib"), root.to_path_buf()] {
        if directory.join(&library).is_file() {
            return Ok(Some(directory));
        }
    }
    if configured.is_some() {
        return Err(AppError::ConfigError(format!(
            "ONNX Runtime 目录缺少 {library}: {}",
            root.display()
        )));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    #[test]
    fn bundled_runtime_preserves_explicit_roots_and_rejects_missing_libraries() {
        let directory = tempfile::tempdir().unwrap();
        let resources = directory.path().join("resources");
        let custom = directory.path().join("custom");
        assert_eq!(super::runtime_library_dir(&resources, None).unwrap(), None);
        for root in [&resources, &custom.join("lib")] {
            std::fs::create_dir_all(root).unwrap();
            for name in [
                "libonnxruntime.so.1",
                "libonnxruntime.dylib",
                "onnxruntime.dll",
            ] {
                std::fs::write(root.join(name), b"runtime").unwrap();
            }
        }
        assert_eq!(
            super::runtime_library_dir(&resources, None).unwrap(),
            Some(resources.clone())
        );
        assert_eq!(
            super::runtime_library_dir(&resources, Some(&custom)).unwrap(),
            Some(custom.join("lib"))
        );
        assert!(
            super::runtime_library_dir(&resources, Some(&directory.path().join("missing")))
                .is_err()
        );
    }
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
