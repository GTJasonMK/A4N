use a4n_core::{config::app_config::AppConfig, files::paths::AppPaths};
use tauri::Manager;

pub(crate) fn create_main_window(app: &mut tauri::App, config: &AppConfig) -> tauri::Result<()> {
    let webview_data_dir = AppPaths::from_config(config).webview_dir();
    std::fs::create_dir_all(&webview_data_dir)?;
    let main_window_config = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == "main")
        .ok_or(tauri::Error::WindowNotFound)?;
    let builder = tauri::WebviewWindowBuilder::from_config(app.handle(), main_window_config)?
        .data_directory(webview_data_dir);
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    let builder = builder.incognito(true);
    builder.build()?;
    Ok(())
}

pub(crate) fn open_devtools_in_debug(app: &tauri::App) {
    #[cfg(debug_assertions)]
    {
        if let Some(window) = app.get_webview_window("main") {
            window.open_devtools();
        }
    }
}
