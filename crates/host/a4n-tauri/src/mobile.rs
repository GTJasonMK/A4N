use tauri::Manager;

pub(crate) fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let state_file = app.path().app_data_dir()?.join("frontend_state.json");
            app.manage(crate::frontend_state::FrontendStatePath(state_file));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            crate::frontend_state::commands::get_all_frontend_state,
            crate::frontend_state::commands::set_frontend_state_many,
            crate::frontend_state::commands::remove_frontend_state_many,
        ])
        .run(tauri::generate_context!())
        .expect("启动 A4N iOS 应用失败");
}
