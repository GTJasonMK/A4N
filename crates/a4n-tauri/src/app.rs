use crate::{
    command_registry, commands, environment, logging, media_protocol, server_process, window,
};
use a4n_core::{config::app_config::AppConfig, files::paths::AppPaths};
use tauri::Manager;

pub(crate) fn run() {
    let early_log_dir = environment::early_log_dir().expect("日志目录配置无效");
    logging::init(early_log_dir).expect("日志系统初始化失败");

    let app = tauri::Builder::default()
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
            install_signal_exit(app.handle())?;
            match environment::api_mode().map_err(std::io::Error::other)? {
                environment::ApiMode::Native => setup_local_runtime(app)?,
                environment::ApiMode::Http { remote_only } => setup_http_mode(app, remote_only)?,
            }
            #[cfg(debug_assertions)]
            window::open_devtools_in_debug(app);
            Ok(())
        })
        .invoke_handler(command_registry::handler())
        .build(tauri::generate_context!())
        .expect("启动 A4N 应用失败");

    app.run(|app_handle, event| match event {
        tauri::RunEvent::Ready => {
            if let Some(window) = app_handle.get_webview_window("main") {
                if let Err(error) = window.show() {
                    tracing::error!(%error, "显示 A4N 主窗口失败");
                }
                if let Err(error) = window.set_focus() {
                    tracing::error!(%error, "聚焦 A4N 主窗口失败");
                }
            } else {
                tracing::error!("A4N 主窗口未创建");
            }
        }
        tauri::RunEvent::Exit => {
            if let Some(server) = app_handle.try_state::<server_process::LocalServerProcess>() {
                server.stop();
            }
            if let Some(state) = app_handle.try_state::<a4n_runtime::AppState>() {
                if let Err(error) =
                    tauri::async_runtime::block_on(state.shutdown_managed_sidecars())
                {
                    tracing::error!(%error, "停止 managed sidecars 失败");
                }
            }
        }
        _ => {}
    });
}

fn setup_http_mode(
    app: &mut tauri::App,
    remote_only: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::default();
    AppPaths::from_config(&config).ensure_dirs()?;
    if !remote_only {
        let server = tauri::async_runtime::block_on(server_process::LocalServerProcess::start(
            app.handle(),
            &config,
        ))?;
        app.manage(server);
    }
    app.manage(commands::frontend_state::FrontendStatePath(
        AppPaths::from_config(&config).frontend_state_file(),
    ));
    window::create_main_window(app, &config)?;
    app.manage(commands::config::local_embedding::LocalEmbeddingRuntimeState::new(&config));
    tracing::info!(
        remote_only,
        api_base_url = ?std::env::var("VITE_A4N_API_BASE_URL").ok(),
        "A4N Tauri HTTP 模式启动完成，已跳过本地 AppState 初始化并启用本机 Embedding Runtime"
    );
    Ok(())
}

fn setup_local_runtime(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let app_state = tauri::async_runtime::block_on(crate::setup::initialize(app.handle()))?;
    app.manage(commands::frontend_state::FrontendStatePath(
        AppPaths::from_config(app_state.host_app_config()).frontend_state_file(),
    ));
    window::create_main_window(app, app_state.host_app_config())?;
    app.manage(app_state);
    tracing::info!("A4N 原生 Tauri API 兼容模式启动完成");
    Ok(())
}

#[cfg(unix)]
fn install_signal_exit(app: &tauri::AppHandle) -> Result<(), std::io::Error> {
    let mut terminate = tauri::async_runtime::block_on(async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
    })?;
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        terminate.recv().await;
        app.exit(0);
    });
    Ok(())
}

#[cfg(not(unix))]
fn install_signal_exit(_: &tauri::AppHandle) -> Result<(), std::io::Error> {
    Ok(())
}
