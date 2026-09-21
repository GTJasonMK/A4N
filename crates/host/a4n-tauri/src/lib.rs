#[cfg(not(mobile))]
mod app;
#[cfg(not(mobile))]
mod command_registry;
#[cfg(not(mobile))]
mod commands;
#[cfg(not(mobile))]
mod environment;
#[cfg(mobile)]
#[path = "commands/frontend_state.rs"]
mod frontend_state;
#[cfg(not(mobile))]
mod logging;
#[cfg(not(mobile))]
mod media_protocol;
#[cfg(mobile)]
mod mobile;
#[cfg(not(mobile))]
mod server_process;
#[cfg(not(mobile))]
mod setup;
#[cfg(not(mobile))]
mod window;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(not(mobile))]
    app::run();

    #[cfg(mobile)]
    mobile::run();
}
