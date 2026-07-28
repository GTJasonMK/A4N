//! A4N Tauri 应用入口。

mod app;
mod command_registry;
mod commands;
mod environment;
mod logging;
mod media_protocol;
mod progress_task;
mod setup;
mod window;

#[cfg(test)]
mod tests;

fn main() {
    app::run();
}
