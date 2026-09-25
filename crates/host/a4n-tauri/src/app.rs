use crate::{
    command_registry, commands, environment, logging, media_protocol, server_process, setup, window,
};
use a4n_config::app_config::AppConfig;
use a4n_storage::paths::AppPaths;
use tauri::Manager;

pub(crate) fn run() {
    let native_worker_settings = match environment::api_mode() {
        Ok(environment::ApiMode::Native) => match server_process::NativeWorkerSettings::prepare() {
            Ok(settings) => Some(settings),
            Err(error) => {
                eprintln!("准备 Native worker 配置失败: {error}");
                return;
            }
        },
        _ => None,
    };
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
        .setup(move |app| {
            install_signal_exit(app.handle())?;
            match environment::api_mode().map_err(std::io::Error::other)? {
                environment::ApiMode::Native => setup_local_runtime(
                    app,
                    native_worker_settings
                        .ok_or_else(|| std::io::Error::other("Native worker 配置未在启动前准备"))?,
                )?,
                environment::ApiMode::Http { remote_only } => setup_http_mode(app, remote_only)?,
            }
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
            if let Some(state) = app_handle.try_state::<a4n_runtime::AppState>()
                && let Err(error) =
                    tauri::async_runtime::block_on(state.shutdown_managed_sidecars())
            {
                tracing::error!(%error, "停止 managed sidecars 失败");
            }
            if let Some(workers) = app_handle.try_state::<server_process::NativeWorkerProcesses>() {
                workers.stop();
            }
        }
        _ => {}
    });
}

fn setup_http_mode(
    app: &mut tauri::App,
    remote_only: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = if remote_only {
        AppConfig::default()
    } else {
        setup::load_config(app.handle())
            .map_err(|error| std::io::Error::other(error.to_string()))?
    };
    if !remote_only {
        configure_local_server_runtime();
    }
    app_paths(&config).ensure_dirs()?;
    let api_base_url = if !remote_only {
        let (server, api_base_url) = tauri::async_runtime::block_on(
            server_process::LocalServerProcess::start(app.handle(), &config),
        )?;
        app.manage(server);
        Some(api_base_url)
    } else {
        None
    };
    app.manage(commands::frontend_state::FrontendStatePath(
        app_paths(&config).frontend_state_file(),
    ));
    window::create_main_window(app, &config, api_base_url.as_deref())?;
    app.manage(commands::config::local_embedding::LocalEmbeddingRuntimeState::new(&config));
    tracing::info!(
        remote_only,
        api_base_url = ?api_base_url.or_else(|| std::env::var("VITE_A4N_API_BASE_URL").ok()),
        "A4N Tauri HTTP 模式启动完成，已跳过本地 AppState 初始化并启用本机 Embedding Runtime"
    );
    Ok(())
}

fn configure_local_server_runtime() {
    if std::env::var_os("A4N_RUNTIME_PROFILE").is_none() {
        // SAFETY: Tauri setup runs before the local Server child is spawned.
        unsafe { std::env::set_var("A4N_RUNTIME_PROFILE", "sidecar") };
    }
    if std::env::var_os("A4N_MANAGED_SIDECARS").is_none() {
        // SAFETY: Tauri setup runs before the local Server child is spawned.
        unsafe { std::env::set_var("A4N_MANAGED_SIDECARS", "1") };
    }
}

fn setup_local_runtime(
    app: &mut tauri::App,
    worker_settings: server_process::NativeWorkerSettings,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = setup::load_native_config(app.handle())?;
    let workers = tauri::async_runtime::block_on(worker_settings.start(app.handle(), &config))?;
    app.manage(workers);
    let app_state = tauri::async_runtime::block_on(crate::setup::initialize(config))?;
    let app_state = std::sync::Arc::new(app_state);
    a4n_control::install_agent_host_dispatcher(&app_state)?;
    tauri::async_runtime::block_on(app_state.spawn_agent_worker())?;
    app.manage(commands::frontend_state::FrontendStatePath(
        app_paths(app_state.host_app_config()).frontend_state_file(),
    ));
    window::create_main_window(app, app_state.host_app_config(), None)?;
    // Keep an Arc alive so the dispatcher can upgrade its weak reference, and
    // retain the existing AppState-managed command contract for all commands.
    app.manage(app_state.clone());
    app.manage((*app_state).clone());
    tracing::info!("A4N 原生 Tauri API 兼容模式启动完成");
    Ok(())
}

fn app_paths(config: &AppConfig) -> AppPaths {
    AppPaths::new(
        config.data_dir.clone(),
        config.cache_dir.clone(),
        config.temp_dir.clone(),
        config.log_dir.clone(),
    )
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
