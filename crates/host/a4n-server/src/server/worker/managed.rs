use super::super::*;
use anyhow::Context;

fn launch(config: &ManagedWorkerConfig) -> anyhow::Result<(Child, BinaryFingerprint)> {
    let mut child = spawn_managed_worker(config)?;
    if let Err(error) = wait_for_managed_worker(&mut child, config) {
        if let Err(stop_error) = stop_child(&mut child) {
            return Err(anyhow::anyhow!(
                "{} readiness 失败: {error:#}; 停止失败: {stop_error:#}",
                config.metadata.name
            ));
        }
        return Err(error);
    }
    let fingerprint = match binary_fingerprint(&config.binary) {
        Ok(fingerprint) => fingerprint,
        Err(error) => {
            return match stop_child(&mut child) {
                Ok(()) => Err(error),
                Err(stop_error) => Err(anyhow::anyhow!(
                    "读取 {} 指纹失败: {error:#}; 停止进程失败: {stop_error:#}",
                    config.metadata.name
                )),
            };
        }
    };
    Ok((child, fingerprint))
}

fn startup_failure_context(
    worker_name: &str,
    cleanup_results: impl IntoIterator<Item = (String, anyhow::Result<()>)>,
) -> String {
    let cleanup_errors = cleanup_results
        .into_iter()
        .filter_map(|(name, result)| result.err().map(|error| format!("{name}: {error:#}")))
        .collect::<Vec<_>>();
    if cleanup_errors.is_empty() {
        format!("启动 {worker_name} 失败")
    } else {
        format!(
            "启动 {worker_name} 失败；回滚已启动 Worker 也失败: {}",
            cleanup_errors.join("; ")
        )
    }
}

pub(crate) fn start_managed_workers(
    configs: &[ManagedWorkerConfig],
) -> anyhow::Result<WorkerProcessRegistry> {
    let mut workers = Vec::with_capacity(configs.len());
    let mut rollback_binaries = std::collections::HashMap::new();
    for config in configs {
        if !rollback_binaries.contains_key(&config.binary) {
            match backup_binary(&config.binary) {
                Ok(Some(rollback_binary)) => {
                    rollback_binaries.insert(config.binary.clone(), rollback_binary);
                }
                Ok(None) => {}
                Err(error) => {
                    let cleanup_results = stop_worker_processes(&mut workers);
                    cleanup_rollback_binaries(&mut rollback_binaries);
                    return Err(error.context(startup_failure_context(
                        &config.metadata.name,
                        cleanup_results,
                    )));
                }
            }
        }
        match launch(config) {
            Ok((child, fingerprint)) => workers.push(ManagedWorker {
                config: config.clone(),
                child,
                fingerprint,
            }),
            Err(error) => {
                let cleanup_results = stop_worker_processes(&mut workers);
                cleanup_rollback_binaries(&mut rollback_binaries);
                return Err(error.context(startup_failure_context(
                    &config.metadata.name,
                    cleanup_results,
                )));
            }
        }
    }
    Ok(WorkerProcessRegistry::with_rollback_binaries(
        workers,
        rollback_binaries,
    ))
}

pub(crate) fn restart_managed_worker(
    worker: &mut ManagedWorker,
    stop_first: bool,
) -> anyhow::Result<()> {
    if stop_first && let Err(error) = stop_child(&mut worker.child) {
        return Err(error);
    }
    std::thread::sleep(WORKER_RESTART_DELAY);
    match launch(&worker.config) {
        Ok((child, fingerprint)) => {
            worker.child = child;
            worker.fingerprint = fingerprint;
            Ok(())
        }
        Err(error) => Err(error),
    }
}

pub(crate) fn restart_managed_binary_group(
    workers: &mut WorkerProcessRegistry,
    binary: &Path,
) -> anyhow::Result<()> {
    restart_managed_binary_group_with(workers, binary, launch, backup_binary)
}

fn restart_managed_binary_group_with(
    workers: &mut WorkerProcessRegistry,
    binary: &Path,
    mut launch_worker: impl FnMut(&ManagedWorkerConfig) -> anyhow::Result<(Child, BinaryFingerprint)>,
    mut backup: impl FnMut(&Path) -> anyhow::Result<Option<tempfile::TempPath>>,
) -> anyhow::Result<()> {
    let indices = workers.indices_for_binary(binary);
    if indices.is_empty() {
        anyhow::bail!("没有 Worker 使用共享二进制 {}", binary.display());
    }
    let next_rollback_binary = backup(binary)?
        .ok_or_else(|| anyhow::anyhow!("共享 Worker 二进制不存在: {}", binary.display()))?;
    let rollback_binary = workers
        .rollback_binaries
        .remove(binary)
        .ok_or_else(|| anyhow::anyhow!("缺少共享 Worker 回滚快照: {}", binary.display()))?;

    let stop_errors = indices
        .iter()
        .filter_map(|index| {
            stop_child(&mut workers[*index].child)
                .err()
                .map(|error| format!("{}: {error:#}", workers[*index].config.metadata.name))
        })
        .collect::<Vec<_>>();
    if !stop_errors.is_empty() {
        workers
            .rollback_binaries
            .insert(binary.to_path_buf(), rollback_binary);
        remove_backup(Some(next_rollback_binary));
        anyhow::bail!("停止共享二进制 Worker 组失败: {}", stop_errors.join("; "));
    }

    std::thread::sleep(WORKER_RESTART_DELAY);
    let mut restarted_indices = Vec::with_capacity(indices.len());
    let mut update_error = None;
    for index in &indices {
        match launch_worker(&workers[*index].config) {
            Ok((child, fingerprint)) => {
                workers[*index].child = child;
                workers[*index].fingerprint = fingerprint;
                restarted_indices.push(*index);
            }
            Err(error) => {
                update_error = Some((workers[*index].config.metadata.name.clone(), error));
                break;
            }
        }
    }
    if let Some((failed_worker, update_error)) = update_error {
        let cleanup_errors = restarted_indices
            .iter()
            .filter_map(|index| {
                stop_child(&mut workers[*index].child)
                    .err()
                    .map(|error| format!("{}: {error:#}", workers[*index].config.metadata.name))
            })
            .collect::<Vec<_>>();
        if !cleanup_errors.is_empty() {
            remove_backup(Some(next_rollback_binary));
            workers
                .rollback_binaries
                .insert(binary.to_path_buf(), rollback_binary);
            return Err(anyhow::anyhow!(
                "共享 Worker 组更新失败（{failed_worker}）: {update_error:#}; 停止新进程失败: {}; 保留当前宿主和旧版快照，监督循环将重试整组更新",
                cleanup_errors.join("; ")
            ));
        }
        remove_backup(Some(next_rollback_binary));
        if let Err(error) = std::fs::copy(&rollback_binary, binary) {
            workers
                .rollback_binaries
                .insert(binary.to_path_buf(), rollback_binary);
            return Err(anyhow::anyhow!(
                "共享 Worker 组更新失败（{failed_worker}）: {update_error:#}; 恢复旧二进制失败: {error}"
            ));
        }
        let mut rollback_errors = Vec::new();
        let rollback_fingerprint = match binary_fingerprint(binary) {
            Ok(fingerprint) => fingerprint,
            Err(error) => {
                workers
                    .rollback_binaries
                    .insert(binary.to_path_buf(), rollback_binary);
                return Err(anyhow::anyhow!(
                    "共享 Worker 组更新失败（{failed_worker}）: {update_error:#}; 已恢复二进制但读取其指纹失败: {error:#}"
                ));
            }
        };
        for index in &indices {
            match launch_worker(&workers[*index].config) {
                Ok((child, fingerprint)) => {
                    workers[*index].child = child;
                    workers[*index].fingerprint = fingerprint;
                }
                Err(error) => {
                    workers[*index].fingerprint = rollback_fingerprint;
                    rollback_errors.push(format!(
                        "{}: {error:#}",
                        workers[*index].config.metadata.name
                    ));
                }
            }
        }
        workers
            .rollback_binaries
            .insert(binary.to_path_buf(), rollback_binary);
        let mut details = vec![format!("更新 {failed_worker} 失败: {update_error:#}")];
        if !rollback_errors.is_empty() {
            details.push(format!(
                "恢复 Worker 组失败: {}",
                rollback_errors.join("; ")
            ));
        } else {
            details.push("已恢复旧版本 Worker 组".to_string());
        }
        return Err(anyhow::anyhow!(details.join("; ")));
    }
    workers
        .rollback_binaries
        .insert(binary.to_path_buf(), next_rollback_binary);
    remove_backup(Some(rollback_binary));
    Ok(())
}

fn backup_binary(binary: &Path) -> anyhow::Result<Option<tempfile::TempPath>> {
    let mut config = a4n_config::app_config::AppConfig::default();
    a4n_config::app_config::apply_storage_env_overrides(
        &mut config,
        &a4n_config::app_config::portable_home_dir(),
    )?;
    backup_binary_in(binary, &config.temp_dir)
}

fn backup_binary_in(binary: &Path, directory: &Path) -> anyhow::Result<Option<tempfile::TempPath>> {
    if !binary.is_file() {
        return Ok(None);
    }
    std::fs::create_dir_all(directory)
        .with_context(|| format!("创建 Worker 回滚备份目录 {} 失败", directory.display()))?;
    // Use the application's storage root: OS temporary directories may be quota-limited tmpfs.
    let backup = tempfile::Builder::new()
        .prefix("a4n-worker-rollback-")
        .tempfile_in(directory)
        .with_context(|| format!("创建 Worker 回滚备份文件失败: {}", directory.display()))?
        .into_temp_path();
    std::fs::copy(binary, &backup).with_context(|| {
        format!(
            "备份 Worker 二进制 {} 到 {} 失败",
            binary.display(),
            backup.display()
        )
    })?;
    Ok(Some(backup))
}

fn remove_backup(path: Option<tempfile::TempPath>) {
    if let Some(path) = path {
        let name = path.display().to_string();
        if let Err(error) = path.close() {
            eprintln!("清理 Worker 回滚备份 {name} 失败: {error}");
        }
    }
}

fn stop_worker_processes(workers: &mut [ManagedWorker]) -> Vec<(String, anyhow::Result<()>)> {
    workers
        .iter_mut()
        .rev()
        .map(|worker| {
            let result = stop_child(&mut worker.child);
            (worker.config.metadata.name.clone(), result)
        })
        .collect()
}

fn cleanup_rollback_binaries(
    rollback_binaries: &mut std::collections::HashMap<PathBuf, tempfile::TempPath>,
) {
    for (_, backup) in std::mem::take(rollback_binaries) {
        remove_backup(Some(backup));
    }
}

pub(crate) fn stop_managed_workers(
    workers: &mut WorkerProcessRegistry,
) -> Vec<(String, anyhow::Result<()>)> {
    let results = workers
        .iter_mut()
        .rev()
        .map(|worker| {
            let result = stop_child(&mut worker.child);
            (worker.config.metadata.name.clone(), result)
        })
        .collect();
    cleanup_rollback_binaries(&mut workers.rollback_binaries);
    results
}

#[cfg(test)]
mod tests {
    use super::{
        BinaryFingerprint, ManagedWorkerConfig, WorkerProcess, WorkerProcessRegistry, WorkerSpec,
        backup_binary_in, binary_fingerprint, remove_backup, restart_managed_binary_group_with,
        startup_failure_context, stop_managed_workers,
    };
    use std::collections::HashMap;

    #[cfg(unix)]
    fn worker_process(name: &str, binary: &std::path::Path) -> WorkerProcess {
        use std::process::Command;

        WorkerProcess {
            config: WorkerSpec {
                metadata: a4n_process_catalog::stateful_worker_catalog_entry_ref(name).unwrap(),
                binary: binary.to_path_buf(),
                address: "127.0.0.1:47675".parse().unwrap(),
                control_addr: "127.0.0.1:47674".parse().unwrap(),
                token: "test-token".into(),
            },
            child: Command::new("sleep").arg("30").spawn().unwrap(),
            fingerprint: binary_fingerprint(binary).unwrap(),
        }
    }

    #[cfg(unix)]
    fn launch_fixture(
        config: &ManagedWorkerConfig,
    ) -> anyhow::Result<(std::process::Child, BinaryFingerprint)> {
        use std::process::Command;

        let child = Command::new("sleep").arg("30").spawn()?;
        Ok((child, binary_fingerprint(&config.binary)?))
    }

    #[test]
    fn startup_failure_context_reports_cleanup_errors() {
        assert_eq!(
            startup_failure_context(
                "canvas-record-worker",
                std::iter::empty::<(String, anyhow::Result<()>)>()
            ),
            "启动 canvas-record-worker 失败"
        );
        assert_eq!(
            startup_failure_context(
                "canvas-record-worker",
                [(
                    "video-record-worker".to_string(),
                    Err(anyhow::anyhow!("permission denied"))
                )]
            ),
            "启动 canvas-record-worker 失败；回滚已启动 Worker 也失败: video-record-worker: permission denied"
        );
    }

    #[test]
    fn backup_binary_preserves_its_snapshot_when_the_source_path_changes() {
        let tempdir = tempfile::tempdir().unwrap();
        let binary = tempdir.path().join("worker");
        let backup_dir = tempdir.path().join("application-tmp");
        std::fs::write(&binary, b"running-version").unwrap();

        let backup = backup_binary_in(&binary, &backup_dir).unwrap().unwrap();
        assert_eq!(backup.parent(), Some(backup_dir.as_path()));
        std::fs::write(&binary, b"new-version").unwrap();

        assert_eq!(std::fs::read(&backup).unwrap(), b"running-version");
        assert_eq!(std::fs::read(&binary).unwrap(), b"new-version");
        let backup_path = backup.to_path_buf();
        remove_backup(Some(backup));
        assert!(!backup_path.exists());
    }

    #[cfg(unix)]
    #[test]
    fn shared_binary_update_rolls_back_every_role_after_partial_startup_failure() {
        let tempdir = tempfile::tempdir().unwrap();
        let binary = tempdir.path().join("a4n-component-host");
        let backup_dir = tempdir.path().join("application-tmp");
        std::fs::write(&binary, b"running-version").unwrap();
        let rollback = backup_binary_in(&binary, &backup_dir).unwrap().unwrap();
        let mut rollback_binaries = HashMap::new();
        rollback_binaries.insert(binary.clone(), rollback);
        let mut workers = WorkerProcessRegistry::with_rollback_binaries(
            vec![
                worker_process("video-record-worker", &binary),
                worker_process("canvas-record-worker", &binary),
            ],
            rollback_binaries,
        );
        std::fs::write(&binary, b"new-version").unwrap();

        let result = restart_managed_binary_group_with(
            &mut workers,
            &binary,
            |config| {
                if config.metadata.name == "canvas-record-worker"
                    && std::fs::read(&config.binary)? == b"new-version"
                {
                    anyhow::bail!("fixture readiness failure");
                }
                launch_fixture(config)
            },
            |path| backup_binary_in(path, &backup_dir),
        );

        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("已恢复旧版本 Worker 组")
        );
        assert_eq!(std::fs::read(&binary).unwrap(), b"running-version");
        assert_eq!(workers.rollback_binaries.len(), 1);
        let snapshot = workers.rollback_binaries.get(&binary).unwrap();
        assert_eq!(std::fs::read(snapshot).unwrap(), b"running-version");
        assert_eq!(std::fs::read_dir(&backup_dir).unwrap().count(), 1);
        for worker in workers.iter_mut() {
            assert!(worker.child.try_wait().unwrap().is_none());
            assert_eq!(worker.fingerprint, binary_fingerprint(&binary).unwrap());
        }
        let _ = stop_managed_workers(&mut workers);
    }

    #[cfg(unix)]
    #[test]
    fn shared_binary_update_rotates_one_snapshot_after_all_roles_start() {
        let tempdir = tempfile::tempdir().unwrap();
        let binary = tempdir.path().join("a4n-component-host");
        let backup_dir = tempdir.path().join("application-tmp");
        std::fs::write(&binary, b"running-version").unwrap();
        let rollback = backup_binary_in(&binary, &backup_dir).unwrap().unwrap();
        let mut rollback_binaries = HashMap::new();
        rollback_binaries.insert(binary.clone(), rollback);
        let mut workers = WorkerProcessRegistry::with_rollback_binaries(
            vec![
                worker_process("video-record-worker", &binary),
                worker_process("canvas-record-worker", &binary),
            ],
            rollback_binaries,
        );
        std::fs::write(&binary, b"new-version").unwrap();

        restart_managed_binary_group_with(&mut workers, &binary, launch_fixture, |path| {
            backup_binary_in(path, &backup_dir)
        })
        .unwrap();

        assert_eq!(workers.rollback_binaries.len(), 1);
        let snapshot = workers.rollback_binaries.get(&binary).unwrap();
        assert_eq!(std::fs::read(snapshot).unwrap(), b"new-version");
        assert_eq!(std::fs::read_dir(&backup_dir).unwrap().count(), 1);
        for worker in workers.iter_mut() {
            assert!(worker.child.try_wait().unwrap().is_none());
            assert_eq!(worker.fingerprint, binary_fingerprint(&binary).unwrap());
        }
        let _ = stop_managed_workers(&mut workers);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn failed_backup_removes_the_incomplete_file() {
        let tempdir = tempfile::tempdir().unwrap();
        let backup_dir = tempdir.path().join("application-tmp");
        // This regular proc file fails reads at offset zero, including when tests run as root.
        let binary = std::path::Path::new("/proc/self/mem");

        let error = backup_binary_in(binary, &backup_dir).unwrap_err();
        assert!(format!("{error:#}").contains("备份 Worker 二进制"));
        assert!(std::fs::read_dir(&backup_dir).unwrap().next().is_none());
    }
}
