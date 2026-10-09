#[cfg(target_os = "linux")]
use std::os::unix::process::CommandExt;

pub fn init_worker_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

pub async fn wait_for_ctrl_c() {
    let _ = tokio::signal::ctrl_c().await;
}

#[cfg(target_os = "linux")]
/// Deliver SIGTERM to a child process when its parent exits.
pub fn configure_parent_death_signal(command: &mut std::process::Command) {
    unsafe {
        command.pre_exec(|| {
            unsafe extern "C" {
                fn prctl(option: i32, ...) -> i32;
            }
            const PR_SET_PDEATHSIG: i32 = 1;
            const SIGTERM: i32 = 15;
            if prctl(PR_SET_PDEATHSIG, SIGTERM) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

#[cfg(not(target_os = "linux"))]
pub fn configure_parent_death_signal(_: &mut std::process::Command) {}
