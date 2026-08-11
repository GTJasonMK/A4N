use crate::{commands, logging};

pub(crate) fn handler() -> impl Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        commands::dispatch::invoke_host_command,
        commands::auth::register_user,
        commands::auth::login_user,
        commands::auth::refresh_token,
        commands::auth::oauth2_authorize_url,
        commands::auth::oauth2_callback,
        commands::config::local_list_embedding_models,
        commands::config::local_get_embedding_model_download_info,
        commands::config::local_download_embedding_model,
        commands::config::local_cancel_embedding_model_download,
        commands::config::local_delete_embedding_model,
        commands::config::local_test_embedding_config,
        commands::config::local_embed_texts,
        commands::frontend_state::get_all_frontend_state,
        commands::frontend_state::set_frontend_state_many,
        commands::frontend_state::remove_frontend_state_many,
        commands::generation_tasks::subscribe_generation_task,
        commands::generation_tasks::unsubscribe_generation_task,
        commands::generation_tasks::get_generation_task_summary,
        commands::generation_tasks::get_generation_task_snapshot,
        commands::generation_tasks::list_active_generation_tasks,
        commands::generation_tasks::list_generation_task_history,
        logging::get_log_dir,
        logging::read_recent_logs,
        logging::set_log_level,
        logging::log_from_frontend,
    ]
}
