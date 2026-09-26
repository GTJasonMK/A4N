use super::*;

#[cfg(target_os = "linux")]
use std::os::unix::process::CommandExt;

pub(crate) fn spawn_control_from_workers(
    binary: &Path,
    address: SocketAddr,
    workers: &[WorkerSpec],
) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    command
        .env("A4N_CONTROL_ADDR", address.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    for worker in workers {
        let url_env = worker.metadata.url_env.as_deref().ok_or_else(|| {
            anyhow::anyhow!("进程目录 Worker {} 缺少 URL 环境变量", worker.metadata.name)
        })?;
        command
            .env(url_env, format!("http://{}", worker.address))
            .env(managed_worker_env(worker.metadata, "TOKEN"), &worker.token);
    }
    #[cfg(feature = "test-embedded-processes")]
    command.env(CONTROL_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command
        .spawn()
        .map_err(|error| anyhow::anyhow!("启动 A4N Control 失败: {}: {error}", binary.display()))
}

pub(crate) fn spawn_managed_worker(config: &ManagedWorkerConfig) -> anyhow::Result<Child> {
    let mut command = Command::new(&config.binary);
    command
        .env(
            config.metadata.addr_env.as_str(),
            config.address.to_string(),
        )
        .env("A4N_CONTROL_ADDR", config.control_addr.to_string())
        .env(managed_worker_env(config.metadata, "TOKEN"), &config.token)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(managed_worker_env(config.metadata, "PROCESS"), "1");
    configure_parent_death_signal(&mut command);
    command.spawn().map_err(|error| {
        anyhow::anyhow!(
            "启动 {} 失败: {}: {error}",
            config.metadata.name,
            config.binary.display()
        )
    })
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn managed_worker_receives_resolved_control_address() {
        let temp_dir = tempfile::tempdir().unwrap();
        let binary = temp_dir.path().join("agent-worker");
        let output = temp_dir.path().join("control-address");
        std::fs::write(
            &binary,
            "#!/bin/sh\nprintf '%s' \"$A4N_CONTROL_ADDR\" > \"$A4N_AGENT_WORKER_TOKEN\"\n",
        )
        .unwrap();
        let mut permissions = std::fs::metadata(&binary).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&binary, permissions).unwrap();
        let control_addr = control_addr_from_web("127.0.0.1:0".parse().unwrap()).unwrap();
        assert_eq!(control_addr.to_string(), "127.0.0.1:18080");
        let config = ManagedWorkerConfig {
            metadata: stateful_worker_catalog_entry_ref("agent-worker").unwrap(),
            binary,
            address: "127.0.0.1:47650".parse().unwrap(),
            control_addr,
            token: output.display().to_string(),
        };

        let mut child = spawn_managed_worker(&config).unwrap();
        assert!(child.wait().unwrap().success());
        assert_eq!(
            std::fs::read_to_string(output).unwrap(),
            control_addr.to_string()
        );
    }
}

pub(crate) fn spawn_edge(binary: &Path, control_addr: SocketAddr) -> anyhow::Result<Child> {
    let mut command = Command::new(binary);
    command
        .env("A4N_CONTROL_ADDR", control_addr.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    #[cfg(feature = "test-embedded-processes")]
    command.env(EDGE_PROCESS_ENV, "1");
    configure_parent_death_signal(&mut command);
    command
        .spawn()
        .map_err(|error| anyhow::anyhow!("启动 A4N Edge 失败: {}: {error}", binary.display()))
}

pub(crate) fn stop_control(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

pub(crate) fn stop_edge(child: &mut Child) -> anyhow::Result<()> {
    stop_child(child)
}

pub(crate) fn stop_child(child: &mut Child) -> anyhow::Result<()> {
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
pub(crate) fn configure_parent_death_signal(command: &mut Command) {
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
pub(crate) fn configure_parent_death_signal(_: &mut Command) {}
