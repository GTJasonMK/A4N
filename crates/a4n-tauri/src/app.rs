use crate::{command_registry, commands, environment, logging, media_protocol, setup, window};
use a4n_core::{config::app_config::AppConfig, files::paths::AppPaths};
use tauri::Manager;

pub(crate) fn run() {
    let early_log_dir = environment::early_log_dir().expect("日志目录配置无效");
    logging::init(early_log_dir).expect("日志系统初始化失败");

    tauri::Builder::default()
        .manage(commands::generation_tasks::GenerationSubscriptionRegistry::default())
        .register_uri_scheme_protocol("a4n-media", media_protocol::handle_request)
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::Destroyed) {
                // 与 process 证据共用唯一清理入口，禁止旁路 cancel_window。
                commands::generation_tasks::handle_window_destroyed(
                    window
                        .app_handle()
                        .state::<commands::generation_tasks::GenerationSubscriptionRegistry>()
                        .inner(),
                    window.label(),
                );
            }
        })
        .setup(|app| {
            if environment::remote_api_only().map_err(std::io::Error::other)? {
                setup_remote_api_mode(app)?;
            } else {
                setup_local_runtime(app)?;
            }
            window::open_devtools_in_debug(app);
            Ok(())
        })
        .invoke_handler(command_registry::handler())
        .run(tauri::generate_context!())
        .expect("启动 A4N 应用失败");
}

fn setup_remote_api_mode(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::default();
    AppPaths::from_config(&config).ensure_dirs()?;
    window::create_main_window(app, &config)?;
    app.manage(commands::config::LocalEmbeddingRuntimeState::new(&config));
    tracing::info!(
        api_base_url = ?std::env::var("VITE_A4N_API_BASE_URL").ok(),
        "A4N Tauri 远程 API 模式启动完成，已跳过本地 AppState 初始化并启用本机 Embedding Runtime"
    );
    Ok(())
}

fn setup_local_runtime(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let app_state = tauri::async_runtime::block_on(setup::initialize(app.handle()))?;
    window::create_main_window(app, app_state.host_app_config())?;
    app.manage(app_state);
    tracing::info!("A4N 应用启动完成");
    Ok(())
}
