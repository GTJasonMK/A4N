//! 应用初始化。
//!
//! 负责启动阶段的配置加载、目录准备和事件日志订阅。

mod config;
mod event_logger;

use a4n_core::error::AppError;
use a4n_core::files::paths::AppPaths;
use a4n_runtime::AppState;
use tauri::AppHandle;

/// 初始化应用
///
/// 按顺序执行：加载配置 → 创建目录 → 构建 AppState → 启动事件日志订阅。
pub async fn initialize(app: &AppHandle) -> Result<AppState, AppError> {
    let config = config::load_config(app)?;
    let app_paths = AppPaths::from_config(&config);
    app_paths.ensure_dirs().map_err(AppError::Io)?;

    let state = AppState::new(config).await?;
    event_logger::spawn_event_logger(state.event_bus.clone()).await;

    Ok(state)
}
