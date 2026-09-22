use a4n_config::app_config::AppConfig;
use a4n_storage::paths::AppPaths;

pub(crate) fn create_main_window(
    app: &mut tauri::App,
    config: &AppConfig,
    api_base_url: Option<&str>,
) -> tauri::Result<()> {
    let webview_data_dir = AppPaths::new(
        config.data_dir.clone(),
        config.cache_dir.clone(),
        config.temp_dir.clone(),
        config.log_dir.clone(),
    )
    .webview_dir();
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
    let builder = if let Some(api_base_url) = api_base_url {
        builder.initialization_script(format!(
            "Object.defineProperty(window, '__A4N_API_BASE_URL__', {{ value: {} }});",
            serde_json::to_string(api_base_url)?
        ))
    } else {
        builder
    };
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    let builder = builder.incognito(true);
    let window = builder.visible(true).focused(true).build()?;
    window.show()?;
    window.set_focus()?;
    Ok(())
}
