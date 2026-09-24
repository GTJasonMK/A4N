use super::super::*;
use super::state::StartupConfig;

enum RuntimeEvent {
    EdgeExited(ExitStatus),
    EdgeChanged,
    ControlExited(ExitStatus),
    ControlChanged,
    Worker(WorkerLifecycleEvent),
    ReloadMarkerChanged,
}

pub(crate) fn run(
    config: &StartupConfig,
    edge: &mut Child,
    control: &mut Child,
    workers: &mut WorkerProcessRegistry,
    control_fingerprint: &mut BinaryFingerprint,
    edge_fingerprint: &mut BinaryFingerprint,
    reload_marker_fingerprint: &mut Option<BinaryFingerprint>,
) -> anyhow::Result<()> {
    loop {
        match next_event(
            config,
            edge,
            control,
            workers,
            *control_fingerprint,
            *edge_fingerprint,
            reload_marker_fingerprint,
        )? {
            Some(RuntimeEvent::EdgeExited(status)) => {
                eprintln!("A4N Edge 意外退出: {status}；准备重启");
                restart_edge(config, edge, edge_fingerprint, false)?;
            }
            Some(RuntimeEvent::EdgeChanged) => {
                eprintln!("检测到 A4N Edge 构建完成；准备重启");
                restart_edge(config, edge, edge_fingerprint, true)?;
            }
            Some(RuntimeEvent::ControlExited(status)) => {
                eprintln!("A4N Control 意外退出: {status}；准备重启");
                restart_control(config, control, control_fingerprint, false)?;
            }
            Some(RuntimeEvent::ControlChanged) => {
                eprintln!("检测到 A4N Control 构建完成；准备重启");
                restart_control(config, control, control_fingerprint, true)?;
            }
            Some(RuntimeEvent::Worker(event)) => handle_worker_event(workers, event)?,
            Some(RuntimeEvent::ReloadMarkerChanged) => {}
            None => std::thread::sleep(CHILD_MONITOR_INTERVAL),
        }
    }
}

fn handle_worker_event(
    workers: &mut WorkerProcessRegistry,
    event: WorkerLifecycleEvent,
) -> anyhow::Result<()> {
    let (worker_name, stop_first) = match event {
        WorkerLifecycleEvent::Exited {
            worker_name,
            status,
        } => {
            eprintln!("A4N {worker_name} 意外退出: {status}；准备重启");
            (worker_name, false)
        }
        WorkerLifecycleEvent::BinaryChanged { worker_name } => {
            eprintln!("检测到 A4N {worker_name} 构建完成；只替换该 Worker");
            (worker_name, true)
        }
    };
    let worker = workers
        .find_mut(&worker_name)
        .ok_or_else(|| anyhow::anyhow!("未注册需要重启的 Worker: {worker_name}"))?;
    if let Err(error) = restart_managed_worker(worker, stop_first) {
        eprintln!("A4N {worker_name} Worker 重启失败，监督服务继续运行并重试: {error:#}");
    }
    Ok(())
}

fn next_event(
    config: &StartupConfig,
    edge: &mut Child,
    control: &mut Child,
    workers: &mut WorkerProcessRegistry,
    control_fingerprint: BinaryFingerprint,
    edge_fingerprint: BinaryFingerprint,
    reload_marker_fingerprint: &mut Option<BinaryFingerprint>,
) -> anyhow::Result<Option<RuntimeEvent>> {
    if let Some(status) = edge.try_wait()? {
        return Ok(Some(RuntimeEvent::EdgeExited(status)));
    }
    if let Some(status) = control.try_wait()? {
        return Ok(Some(RuntimeEvent::ControlExited(status)));
    }
    for worker in workers.iter_mut() {
        if let Some(status) = worker.child.try_wait()? {
            return Ok(Some(RuntimeEvent::Worker(WorkerLifecycleEvent::Exited {
                worker_name: worker.config.metadata.name.clone(),
                status,
            })));
        }
    }
    if binary_fingerprint(&config.edge_binary)? != edge_fingerprint {
        return Ok(Some(RuntimeEvent::EdgeChanged));
    }
    if binary_fingerprint(&config.control_binary)? != control_fingerprint {
        return Ok(Some(RuntimeEvent::ControlChanged));
    }
    for worker in workers.iter() {
        if binary_fingerprint(&worker.config.binary)? != worker.fingerprint {
            return Ok(Some(RuntimeEvent::Worker(
                WorkerLifecycleEvent::BinaryChanged {
                    worker_name: worker.config.metadata.name.clone(),
                },
            )));
        }
    }
    let latest_marker = optional_binary_fingerprint(config.reload_marker.as_deref())?;
    if latest_marker != *reload_marker_fingerprint {
        *reload_marker_fingerprint = latest_marker;
        return Ok(Some(RuntimeEvent::ReloadMarkerChanged));
    }
    Ok(None)
}

fn restart_control(
    config: &StartupConfig,
    control: &mut Child,
    fingerprint: &mut BinaryFingerprint,
    stop_first: bool,
) -> anyhow::Result<()> {
    if stop_first {
        stop_control(control)?;
    }
    std::thread::sleep(WORKER_RESTART_DELAY);
    let mut replacement =
        spawn_control_from_workers(&config.control_binary, config.control_addr, &config.workers)?;
    if let Err(error) = wait_for_control(&mut replacement, config.control_addr) {
        return match stop_control(&mut replacement) {
            Ok(()) => Err(error),
            Err(cleanup_error) => Err(anyhow::anyhow!(
                "Control readiness 失败: {error:#}; 停止新进程失败: {cleanup_error:#}"
            )),
        };
    }
    let next_fingerprint = binary_fingerprint(&config.control_binary)?;
    *control = replacement;
    *fingerprint = next_fingerprint;
    Ok(())
}

fn restart_edge(
    config: &StartupConfig,
    edge: &mut Child,
    fingerprint: &mut BinaryFingerprint,
    stop_first: bool,
) -> anyhow::Result<()> {
    if stop_first {
        stop_edge(edge)?;
    }
    std::thread::sleep(WORKER_RESTART_DELAY);
    let replacement = spawn_edge(&config.edge_binary, config.control_addr)?;
    let next_fingerprint = binary_fingerprint(&config.edge_binary)?;
    *edge = replacement;
    *fingerprint = next_fingerprint;
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::super::super::*;
    use super::super::state::StartupConfig;
    use super::{RuntimeEvent, handle_worker_event, next_event};

    fn startup_config(
        control_binary: PathBuf,
        edge_binary: PathBuf,
        reload_marker: Option<PathBuf>,
    ) -> StartupConfig {
        StartupConfig {
            control_addr: "127.0.0.1:47674".parse().unwrap(),
            control_fingerprint: binary_fingerprint(&control_binary).unwrap(),
            edge_fingerprint: binary_fingerprint(&edge_binary).unwrap(),
            reload_marker_fingerprint: optional_binary_fingerprint(reload_marker.as_deref())
                .unwrap(),
            control_binary,
            edge_binary,
            reload_marker,
            workers: Vec::new(),
        }
    }

    fn live_child() -> Child {
        Command::new("sleep")
            .arg("30")
            .spawn()
            .expect("test child should start")
    }

    fn worker_process(worker_name: &str, binary: PathBuf, child: Child) -> WorkerProcess {
        let fingerprint = binary_fingerprint(&binary).unwrap();
        WorkerProcess {
            config: WorkerSpec {
                metadata: stateful_worker_catalog_entry_ref(worker_name).unwrap(),
                binary,
                address: "127.0.0.1:47675".parse().unwrap(),
                token: "test-token".into(),
            },
            child,
            fingerprint,
            rollback_binary: None,
        }
    }

    fn stop_children(edge: &mut Child, control: &mut Child, workers: &mut WorkerProcessRegistry) {
        let _ = stop_child(edge);
        let _ = stop_child(control);
        let _ = stop_managed_workers(workers);
    }

    #[test]
    fn worker_exit_event_uses_catalog_name() {
        let mut edge = live_child();
        let mut control = live_child();
        let worker_child = Command::new("sh").args(["-c", "exit 7"]).spawn().unwrap();
        let worker_binary = std::env::current_exe().unwrap();
        let worker = worker_process("video-record-worker", worker_binary.clone(), worker_child);
        let mut workers = WorkerProcessRegistry::new(vec![worker]);
        let sidecar_binary = std::env::current_exe().unwrap();
        let config = startup_config(sidecar_binary.clone(), sidecar_binary, None);
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut reload_marker_fingerprint = config.reload_marker_fingerprint;
        let event = loop {
            if let Some(event) = next_event(
                &config,
                &mut edge,
                &mut control,
                &mut workers,
                config.control_fingerprint,
                config.edge_fingerprint,
                &mut reload_marker_fingerprint,
            )
            .unwrap()
            {
                break event;
            }
            assert!(Instant::now() < deadline, "Worker exit was not observed");
            std::thread::sleep(Duration::from_millis(10));
        };
        assert!(matches!(
            event,
            super::RuntimeEvent::Worker(WorkerLifecycleEvent::Exited {
                worker_name,
                ..
            }) if worker_name == "video-record-worker"
        ));
        stop_children(&mut edge, &mut control, &mut workers);
    }

    #[test]
    fn unavailable_worker_during_update_does_not_fail_supervision() {
        let tempdir = tempfile::tempdir().unwrap();
        let mut edge = live_child();
        let mut control = live_child();
        let mut worker = worker_process(
            "video-record-worker",
            std::env::current_exe().unwrap(),
            live_child(),
        );
        worker.config.binary = tempdir.path().join("missing-worker");
        let worker_name = worker.config.metadata.name.to_string();
        let mut workers = WorkerProcessRegistry::new(vec![worker]);

        handle_worker_event(
            &mut workers,
            WorkerLifecycleEvent::BinaryChanged { worker_name },
        )
        .unwrap();

        assert!(edge.try_wait().unwrap().is_none());
        assert!(control.try_wait().unwrap().is_none());
        assert!(workers[0].child.try_wait().unwrap().is_some());
        stop_children(&mut edge, &mut control, &mut workers);
    }

    #[test]
    fn worker_binary_change_event_uses_only_the_changed_catalog_name() {
        let tempdir = tempfile::tempdir().unwrap();
        let video_binary = tempdir.path().join("video-record-worker");
        let canvas_binary = tempdir.path().join("canvas-record-worker");
        std::fs::write(&video_binary, b"video").unwrap();
        std::fs::write(&canvas_binary, b"canvas").unwrap();
        let mut workers = WorkerProcessRegistry::new(vec![
            worker_process("video-record-worker", video_binary, live_child()),
            worker_process("canvas-record-worker", canvas_binary.clone(), live_child()),
        ]);
        std::fs::write(&canvas_binary, b"updated-canvas-binary").unwrap();

        let mut edge = live_child();
        let mut control = live_child();
        let binary = std::env::current_exe().unwrap();
        let config = startup_config(binary.clone(), binary, None);
        let mut reload_marker_fingerprint = config.reload_marker_fingerprint;
        let event = next_event(
            &config,
            &mut edge,
            &mut control,
            &mut workers,
            config.control_fingerprint,
            config.edge_fingerprint,
            &mut reload_marker_fingerprint,
        )
        .unwrap();

        assert!(matches!(
            event,
            Some(RuntimeEvent::Worker(WorkerLifecycleEvent::BinaryChanged { worker_name }))
                if worker_name == "canvas-record-worker"
        ));
        stop_children(&mut edge, &mut control, &mut workers);
    }

    #[test]
    fn control_and_edge_binary_changes_are_detected() {
        let tempdir = tempfile::tempdir().unwrap();
        let control_binary = tempdir.path().join("a4n-control");
        let edge_binary = tempdir.path().join("a4n-edge");
        std::fs::write(&control_binary, b"control").unwrap();
        std::fs::write(&edge_binary, b"edge").unwrap();
        let config = startup_config(control_binary.clone(), edge_binary.clone(), None);
        let mut control_fingerprint = config.control_fingerprint;
        let mut edge_fingerprint = config.edge_fingerprint;
        let mut reload_marker_fingerprint = config.reload_marker_fingerprint;
        let mut edge = live_child();
        let mut control = live_child();
        let mut workers = WorkerProcessRegistry::new(Vec::new());

        std::fs::write(&control_binary, b"updated-control").unwrap();
        let event = next_event(
            &config,
            &mut edge,
            &mut control,
            &mut workers,
            control_fingerprint,
            edge_fingerprint,
            &mut reload_marker_fingerprint,
        )
        .unwrap();
        assert!(matches!(event, Some(RuntimeEvent::ControlChanged)));

        control_fingerprint = binary_fingerprint(&control_binary).unwrap();
        std::fs::write(&edge_binary, b"updated-edge").unwrap();
        let event = next_event(
            &config,
            &mut edge,
            &mut control,
            &mut workers,
            control_fingerprint,
            edge_fingerprint,
            &mut reload_marker_fingerprint,
        )
        .unwrap();
        assert!(matches!(event, Some(RuntimeEvent::EdgeChanged)));
        edge_fingerprint = binary_fingerprint(&edge_binary).unwrap();
        let event = next_event(
            &config,
            &mut edge,
            &mut control,
            &mut workers,
            control_fingerprint,
            edge_fingerprint,
            &mut reload_marker_fingerprint,
        )
        .unwrap();
        assert!(event.is_none());
        stop_children(&mut edge, &mut control, &mut workers);
    }

    #[test]
    fn reload_marker_changes_are_detected_without_a_worker_change() {
        let tempdir = tempfile::tempdir().unwrap();
        let marker = tempdir.path().join("reload.marker");
        let binary = std::env::current_exe().unwrap();
        let config = startup_config(binary.clone(), binary, Some(marker.clone()));
        let mut edge = live_child();
        let mut control = live_child();
        let mut workers = WorkerProcessRegistry::new(Vec::new());
        let mut reload_marker_fingerprint = config.reload_marker_fingerprint;
        std::fs::write(marker, b"reload").unwrap();

        let event = next_event(
            &config,
            &mut edge,
            &mut control,
            &mut workers,
            config.control_fingerprint,
            config.edge_fingerprint,
            &mut reload_marker_fingerprint,
        )
        .unwrap();

        assert!(matches!(event, Some(RuntimeEvent::ReloadMarkerChanged)));
        assert!(reload_marker_fingerprint.is_some());
        stop_children(&mut edge, &mut control, &mut workers);
    }
}
