use std::{
    env,
    net::{SocketAddr, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant, SystemTime},
};

#[cfg(target_os = "linux")]
use std::os::unix::process::CommandExt;

#[cfg(feature = "test-embedded-processes")]
const CONTROL_PROCESS_ENV: &str = "A4N_CONTROL_PROCESS";
const CONTROL_BINARY_ENV: &str = "A4N_CONTROL_BINARY";
const AGENT_WORKER_BINARY_ENV: &str = "A4N_AGENT_WORKER_BINARY";
const AGENT_WORKER_TOKEN_ENV: &str = "A4N_AGENT_WORKER_TOKEN";
const CONTROL_READY_TIMEOUT: Duration = Duration::from_secs(30);
const CONTROL_READY_RETRY_INTERVAL: Duration = Duration::from_millis(50);
const CONTROL_READY_CONNECT_TIMEOUT: Duration = Duration::from_millis(250);
const CHILD_MONITOR_INTERVAL: Duration = Duration::from_millis(100);
const WORKER_RESTART_DELAY: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BinaryFingerprint {
    modified: SystemTime,
    length: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChildWatchEvent {
    ControlExited(ExitStatus),
    WorkerExited(ExitStatus),
    ControlBinaryChanged,
    WorkerBinaryChanged,
}

fn main() -> anyhow::Result<()> {
    #[cfg(feature = "test-embedded-processes")]
    {
        if env::var(CONTROL_PROCESS_ENV).as_deref() == Ok("1") {
            return a4n_control::run();
        }
        if env::var("A4N_AGENT_WORKER_PROCESS").as_deref() == Ok("1") {
            return a4n_agent_worker::run();
        }
    }

    let control_addr = control_addr()?;
    let worker_token = worker_token()?;
    let control_binary = control_binary()?;
    let mut control_fingerprint = binary_fingerprint(&control_binary)?;
    let worker_binary = agent_worker_binary()?;
    let mut worker_fingerprint = binary_fingerprint(&worker_binary)?;
    let mut control = spawn_control(&control_binary, control_addr, &worker_token)?;
    if let Err(error) = wait_for_control(&mut control, control_addr) {
        let cleanup_result = stop_control(&mut control);
        return match cleanup_result {
            Ok(()) => Err(error),
            Err(cleanup_error) => Err(anyhow::anyhow!(
                "A4N Control 未就绪: {error:#}；清理 Control 失败: {cleanup_error:#}"
            )),
        };
    }
    let mut agent_worker = match spawn_agent_worker(&worker_binary, control_addr, &worker_token) {
        Ok(child) => child,
        Err(error) => {
            let _ = stop_control(&mut control);
            return Err(error);
        }
    };
    let edge_stop = Arc::new(AtomicBool::new(false));
    let edge_stop_for_thread = edge_stop.clone();
    let edge_thread = std::thread::spawn(move || {
        a4n_edge::run_with_control_addr_until(Some(control_addr), edge_stop_for_thread)
    });
    let watch_result = supervise_agent_worker(
        &edge_thread,
        &mut control,
        &mut agent_worker,
        control_addr,
        &worker_token,
        &control_binary,
        &mut control_fingerprint,
        &worker_binary,
        &mut worker_fingerprint,
    );
    edge_stop.store(true, Ordering::Release);
    let edge_result = match edge_thread.join() {
        Ok(result) => result,
        Err(_) => Err(anyhow::anyhow!("A4N Edge 线程异常退出")),
    };
    let edge_result = merge_watch_result(watch_result, edge_result);
    let worker_result = stop_agent_worker(&mut agent_worker);
    let control_result = stop_control(&mut control);
    finish_shutdown(edge_result, worker_result, control_result)
}

fn supervise_agent_worker(
    edge_thread: &std::thread::JoinHandle<anyhow::Result<()>>,
    control: &mut Child,
    agent_worker: &mut Child,
    control_addr: SocketAddr,
    worker_token: &str,
    control_binary: &Path,
    control_fingerprint: &mut BinaryFingerprint,
    worker_binary: &Path,
    worker_fingerprint: &mut BinaryFingerprint,
) -> anyhow::Result<Option<String>> {
    loop {
        let Some(event) = wait_for_edge_or_child(
            edge_thread,
            control,
            agent_worker,
            control_binary,
            *control_fingerprint,
            worker_binary,
            *worker_fingerprint,
        )?
        else {
            return Ok(None);
        };

        match event {
            ChildWatchEvent::ControlExited(status) => {
                eprintln!("A4N Control 意外退出: {status}；准备重启");
                std::thread::sleep(WORKER_RESTART_DELAY);
                *control = spawn_control(control_binary, control_addr, worker_token)?;
                wait_for_control(control, control_addr)?;
                *control_fingerprint = binary_fingerprint(control_binary)?;
            }
            ChildWatchEvent::ControlBinaryChanged => {
                eprintln!(
                    "检测到 A4N Control 二进制更新: {}；执行热替换",
                    control_binary.display()
                );
                stop_control(control)?;
                *control = spawn_control(control_binary, control_addr, worker_token)?;
                wait_for_control(control, control_addr)?;
                *control_fingerprint = binary_fingerprint(control_binary)?;
            }
            ChildWatchEvent::WorkerExited(status) => {
                eprintln!("A4N Agent worker 意外退出: {status}；准备重启");
                std::thread::sleep(WORKER_RESTART_DELAY);
                *agent_worker = spawn_agent_worker(worker_binary, control_addr, worker_token)?;
                *worker_fingerprint = binary_fingerprint(worker_binary)?;
            }
            ChildWatchEvent::WorkerBinaryChanged => {
                eprintln!(
                    "检测到 A4N Agent worker 二进制更新: {}；执行热替换",
                    worker_binary.display()
                );
                stop_agent_worker(agent_worker)?;
                *agent_worker = spawn_agent_worker(worker_binary, control_addr, worker_token)?;
                *worker_fingerprint = binary_fingerprint(worker_binary)?;
            }
        }
    }
}

fn wait_for_edge_or_child(
    edge_thread: &std::thread::JoinHandle<anyhow::Result<()>>,
    control: &mut Child,
    agent_worker: &mut Child,
    control_binary: &Path,
    control_fingerprint: BinaryFingerprint,
    worker_binary: &Path,
    worker_fingerprint: BinaryFingerprint,
) -> anyhow::Result<Option<ChildWatchEvent>> {
    loop {
        if edge_thread.is_finished() {
            return Ok(None);
        }
        if let Some(status) = control.try_wait()? {
            return Ok(Some(ChildWatchEvent::ControlExited(status)));
        }
        if let Some(status) = agent_worker.try_wait()? {
            return Ok(Some(ChildWatchEvent::WorkerExited(status)));
        }
        if binary_fingerprint(control_binary)? != control_fingerprint {
            return Ok(Some(ChildWatchEvent::ControlBinaryChanged));
        }
        if binary_fingerprint(worker_binary)? != worker_fingerprint {
            return Ok(Some(ChildWatchEvent::WorkerBinaryChanged));
        }
        std::thread::sleep(CHILD_MONITOR_INTERVAL);
    }
}

fn binary_fingerprint(path: &Path) -> anyhow::Result<BinaryFingerprint> {
    let metadata = std::fs::metadata(path)
        .map_err(|error| anyhow::anyhow!("读取 worker 二进制 {} 失败: {error}", path.display()))?;
    let modified = metadata.modified().map_err(|error| {
        anyhow::anyhow!(
            "读取 worker 二进制 {} 修改时间失败: {error}",
            path.display()
        )
    })?;
    Ok(BinaryFingerprint {
        modified,
        length: metadata.len(),
    })
}

fn merge_watch_result(
    watch_result: anyhow::Result<Option<String>>,
    edge_result: anyhow::Result<()>,
) -> anyhow::Result<()> {
    let watch_result = match watch_result {
        Ok(Some(reason)) => Err(anyhow::anyhow!(reason)),
        Ok(None) => Ok(()),
        Err(error) => Err(error),
    };
    match (watch_result, edge_result) {
        (Ok(()), edge_result) => edge_result,
        (Err(watch_error), Ok(())) => Err(watch_error),
        (Err(watch_error), Err(edge_error)) => {
            Err(anyhow::anyhow!("{watch_error:#}；Edge: {edge_error:#}"))
        }
    }
}

fn wait_for_control(child: &mut Child, address: SocketAddr) -> anyhow::Result<()> {
    let started = Instant::now();
    let mut last_error = None;
    while started.elapsed() < CONTROL_READY_TIMEOUT {
        match TcpStream::connect_timeout(&address, CONTROL_READY_CONNECT_TIMEOUT) {
            Ok(_) => return Ok(()),
            Err(error) => last_error = Some(error),
        }
        if let Some(status) = child.try_wait()? {
            anyhow::bail!(
                "A4N Control 在 listener 就绪前退出: {status}; 最后一次连接错误: {}",
                last_error
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "无".to_string())
            );
        }
        std::thread::sleep(CONTROL_READY_RETRY_INTERVAL);
    }

    anyhow::bail!(
        "A4N Control 在 {} 秒内未监听 {address}; 最后一次连接错误: {}",
        CONTROL_READY_TIMEOUT.as_secs(),
        last_error
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "无".to_string())
    )
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

fn spawn_control(binary: &Path, address: SocketAddr, worker_token: &str) -> anyhow::Result<Child> {
    let mut command = Command::new(&binary);
    command
        .env("A4N_CONTROL_ADDR", address.to_string())
        .env(AGENT_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(CONTROL_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command
        .spawn()
        .map_err(|error| anyhow::anyhow!("启动 A4N Control 失败: {}: {error}", binary.display()))
}

fn spawn_agent_worker(
    binary: &Path,
    address: SocketAddr,
    worker_token: &str,
) -> anyhow::Result<Child> {
    let mut command = Command::new(&binary);
    command
        .env("A4N_CONTROL_ADDR", address.to_string())
        .env(AGENT_WORKER_TOKEN_ENV, worker_token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env("A4N_AGENT_WORKER_PROCESS", "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!("启动 A4N Agent worker 失败: {}: {error}", binary.display())
    })
}

fn control_binary() -> anyhow::Result<PathBuf> {
    child_binary(CONTROL_BINARY_ENV, "a4n-control")
}

fn agent_worker_binary() -> anyhow::Result<PathBuf> {
    child_binary(AGENT_WORKER_BINARY_ENV, "a4n-agent-worker")
}

fn child_binary(env_name: &str, name: &str) -> anyhow::Result<PathBuf> {
    if let Some(path) = env::var_os(env_name).map(PathBuf::from) {
        if path.is_file() {
            return Ok(path);
        }
        anyhow::bail!("{env_name} 指向的文件不存在: {}", path.display());
    }

    if let Some(path) = sibling_binary(name) {
        return Ok(path);
    }

    #[cfg(feature = "test-embedded-processes")]
    if env::var("A4N_TEST_EMBEDDED_PROCESS_FALLBACK").as_deref() == Ok("1") {
        return Ok(env::current_exe()?);
    }

    anyhow::bail!("未找到 {name}；请设置 {env_name} 或将其放在 a4n-server 同目录")
}

fn sibling_binary(name: &str) -> Option<PathBuf> {
    let executable = env::current_exe().ok()?;
    let parent = executable.parent()?;
    let path = parent.join(executable_name(name));
    path.is_file().then_some(path)
}

#[cfg(windows)]
fn executable_name(name: &str) -> PathBuf {
    format!("{name}.exe").into()
}

#[cfg(not(windows))]
fn executable_name(name: &str) -> PathBuf {
    name.into()
}

fn worker_token() -> anyhow::Result<String> {
    if let Some(value) = env::var_os(AGENT_WORKER_TOKEN_ENV) {
        let value = value
            .into_string()
            .map_err(|_| anyhow::anyhow!("{AGENT_WORKER_TOKEN_ENV} 必须是有效 UTF-8"))?;
        let value = value.trim().to_string();
        if value.is_empty() {
            anyhow::bail!("{AGENT_WORKER_TOKEN_ENV} 不能为空");
        }
        return Ok(value);
    }
    Ok(uuid::Uuid::new_v4().to_string())
}

fn stop_agent_worker(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

fn stop_control(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

fn stop_child(child: &mut Child) -> anyhow::Result<()> {
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

fn finish_shutdown(
    edge_result: anyhow::Result<()>,
    worker_result: anyhow::Result<()>,
    control_result: anyhow::Result<()>,
) -> anyhow::Result<()> {
    let mut failures = Vec::new();
    for (name, result) in [
        ("Edge", edge_result),
        ("Agent worker", worker_result),
        ("Control", control_result),
    ] {
        if let Err(error) = result {
            failures.push(format!("{name}: {error:#}"));
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        anyhow::bail!(failures.join("；"));
    }
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

#[cfg(all(test, unix))]
mod tests {
    use super::wait_for_control;
    use std::{
        net::TcpListener,
        process::{Child, Command},
    };

    fn live_child() -> Child {
        Command::new("sh")
            .args(["-c", "sleep 5"])
            .spawn()
            .expect("test child should start")
    }

    fn exited_child() -> Child {
        Command::new("sh")
            .args(["-c", "exit 7"])
            .spawn()
            .expect("test child should start")
    }

    #[test]
    fn waits_for_control_listener() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("test listener should bind");
        let address = listener
            .local_addr()
            .expect("test listener should expose address");
        let mut child = live_child();

        wait_for_control(&mut child, address).expect("listener should be considered ready");

        child.kill().expect("test child should stop");
        child.wait().expect("test child should be reaped");
    }

    #[test]
    fn reports_control_exit_before_listener() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("test listener should bind");
        let address = listener
            .local_addr()
            .expect("test listener should expose address");
        drop(listener);
        let mut child = exited_child();

        let error = wait_for_control(&mut child, address).expect_err("exited Control must fail");
        assert!(error.to_string().contains("listener 就绪前退出"));
    }

    #[test]
    fn reports_child_exit_while_edge_is_running() {
        let (release, wait) = std::sync::mpsc::channel();
        let edge_thread = std::thread::spawn(move || {
            let _ = wait.recv();
            Ok(())
        });
        let mut control = exited_child();
        let mut worker = live_child();
        let tempdir = tempfile::tempdir().expect("temporary directory should be created");
        let control_binary = tempdir.path().join("a4n-control");
        std::fs::write(&control_binary, b"control").expect("control fixture should be written");
        let control_fingerprint =
            super::binary_fingerprint(&control_binary).expect("control fingerprint should work");
        let worker_binary = tempdir.path().join("a4n-agent-worker");
        std::fs::write(&worker_binary, b"worker").expect("worker fixture should be written");
        let worker_fingerprint =
            super::binary_fingerprint(&worker_binary).expect("worker fingerprint should work");

        let result = super::wait_for_edge_or_child(
            &edge_thread,
            &mut control,
            &mut worker,
            &control_binary,
            control_fingerprint,
            &worker_binary,
            worker_fingerprint,
        )
        .expect("child polling should succeed");

        assert!(matches!(
            result,
            Some(super::ChildWatchEvent::ControlExited(_))
        ));
        let _ = release.send(());
        let _ = edge_thread.join();
        worker.kill().expect("test worker should stop");
        worker.wait().expect("test worker should be reaped");
    }

    #[test]
    fn detects_replaced_worker_binary() {
        let tempdir = tempfile::tempdir().expect("temporary directory should be created");
        let worker_binary = tempdir.path().join("a4n-agent-worker");
        std::fs::write(&worker_binary, b"old-worker")
            .expect("old worker fixture should be written");
        let old = super::binary_fingerprint(&worker_binary).expect("old fingerprint should work");
        std::fs::write(&worker_binary, b"new-worker-with-a-different-size")
            .expect("new worker fixture should be written");
        let new = super::binary_fingerprint(&worker_binary).expect("new fingerprint should work");

        assert_ne!(old, new);
    }
}
