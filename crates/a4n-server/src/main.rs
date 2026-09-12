use std::{
    env,
    net::SocketAddr,
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::Duration,
};

#[cfg(target_os = "linux")]
use std::os::unix::process::CommandExt;

const CONTROL_PROCESS_ENV: &str = "A4N_CONTROL_PROCESS";
const CONTROL_BINARY_ENV: &str = "A4N_CONTROL_BINARY";

fn main() -> anyhow::Result<()> {
    if env::var(CONTROL_PROCESS_ENV).as_deref() == Ok("1") {
        return a4n_control::run();
    }

    a4n_control::init_tracing()?;
    let control_addr = control_addr()?;
    let mut control = spawn_control(control_addr)?;
    let edge_result = a4n_edge::run_with_control_addr(Some(control_addr));
    stop_control(&mut control)?;
    edge_result
}

fn control_addr() -> anyhow::Result<SocketAddr> {
    if let Some(value) = env::var_os("A4N_CONTROL_ADDR") {
        let value = value
            .into_string()
            .map_err(|_| anyhow::anyhow!("A4N_CONTROL_ADDR 必须是有效 UTF-8 地址"))?;
        let address = value
            .parse()
            .map_err(|error| anyhow::anyhow!("A4N_CONTROL_ADDR 无效: {error}"))?;
        ensure_nonzero_port("A4N_CONTROL_ADDR", address)
    } else {
        let web_addr = env::var("A4N_WEB_ADDR")
            .unwrap_or_else(|_| "127.0.0.1:8080".to_string())
            .parse::<SocketAddr>()
            .map_err(|error| anyhow::anyhow!("A4N_WEB_ADDR 无效: {error}"))?;
        let control_port = web_addr
            .port()
            .checked_add(1)
            .filter(|port| *port != 0)
            .unwrap_or(18080);
        Ok(SocketAddr::from(([127, 0, 0, 1], control_port)))
    }
}

fn ensure_nonzero_port(name: &str, address: SocketAddr) -> anyhow::Result<SocketAddr> {
    if address.port() == 0 {
        anyhow::bail!("{name} 不允许使用端口 0；Edge 必须知道 Control 的固定地址");
    }
    Ok(address)
}

fn spawn_control(address: SocketAddr) -> anyhow::Result<Child> {
    let binary = control_binary()?;
    let mut command = Command::new(&binary);
    command
        .env(CONTROL_PROCESS_ENV, "1")
        .env("A4N_CONTROL_ADDR", address.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    configure_parent_death_signal(&mut command);
    command
        .spawn()
        .map_err(|error| anyhow::anyhow!("启动 A4N Control 失败: {}: {error}", binary.display()))
}

fn control_binary() -> anyhow::Result<PathBuf> {
    if let Some(path) = env::var_os(CONTROL_BINARY_ENV).map(PathBuf::from) {
        if path.is_file() {
            return Ok(path);
        }
        anyhow::bail!("{CONTROL_BINARY_ENV} 指向的文件不存在: {}", path.display());
    }

    Ok(env::current_exe()?)
}

fn stop_control(child: &mut Child) -> anyhow::Result<()> {
    if child.try_wait()?.is_some() {
        return Ok(());
    }

    #[cfg(unix)]
    {
        let _ = Command::new("kill")
            .args(["-TERM", &child.id().to_string()])
            .status();
        for _ in 0..50 {
            if child.try_wait()?.is_some() {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    child.kill()?;
    child.wait()?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn configure_parent_death_signal(command: &mut Command) {
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
fn configure_parent_death_signal(_: &mut Command) {}
