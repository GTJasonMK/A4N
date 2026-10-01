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
    for config in configs {
        let rollback_binary = match backup_binary(&config.binary) {
            Ok(rollback_binary) => rollback_binary,
            Err(error) => {
                return Err(error.context(startup_failure_context(
                    &config.metadata.name,
                    stop_managed_workers(&mut workers),
                )));
            }
        };
        match launch(config) {
            Ok((child, fingerprint)) => workers.push(ManagedWorker {
                config: config.clone(),
                child,
                fingerprint,
                rollback_binary,
            }),
            Err(error) => {
                remove_backup(rollback_binary);
                return Err(error.context(startup_failure_context(
                    &config.metadata.name,
                    stop_managed_workers(&mut workers),
                )));
            }
        }
    }
    Ok(WorkerProcessRegistry::new(workers))
}

pub(crate) fn restart_managed_worker(
    worker: &mut ManagedWorker,
    stop_first: bool,
) -> anyhow::Result<()> {
    let next_rollback_binary = backup_binary(&worker.config.binary)?;
    let rollback_binary = worker.rollback_binary.take();
    if stop_first && let Err(error) = stop_child(&mut worker.child) {
        worker.rollback_binary = rollback_binary;
        remove_backup(next_rollback_binary);
        return Err(error);
    }
    std::thread::sleep(WORKER_RESTART_DELAY);
    match launch(&worker.config) {
        Ok((child, fingerprint)) => {
            worker.child = child;
            worker.fingerprint = fingerprint;
            if next_rollback_binary.is_some() {
                remove_backup(rollback_binary);
                worker.rollback_binary = next_rollback_binary;
            } else {
                worker.rollback_binary = rollback_binary;
            }
            Ok(())
        }
        Err(error) => {
            remove_backup(next_rollback_binary);
            let Some(rollback_binary) = rollback_binary else {
                return Err(error);
            };
            rollback_worker(worker, rollback_binary, error)
        }
    }
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

fn rollback_worker(
    worker: &mut ManagedWorker,
    backup: tempfile::TempPath,
    update_error: anyhow::Error,
) -> anyhow::Result<()> {
    if let Err(error) = std::fs::copy(&backup, &worker.config.binary) {
        worker.rollback_binary = Some(backup);
        return Err(anyhow::anyhow!(
            "{} 更新失败: {update_error:#}; 恢复旧 Worker 二进制失败: {error}",
            worker.config.metadata.name
        ));
    }
    let (child, fingerprint) = match launch(&worker.config) {
        Ok(result) => result,
        Err(error) => {
            worker.rollback_binary = Some(backup);
            return Err(anyhow::anyhow!(
                "{} 更新失败: {update_error:#}; 恢复旧 Worker 失败: {error:#}",
                worker.config.metadata.name
            ));
        }
    };
    worker.fingerprint = fingerprint;
    worker.child = child;
    worker.rollback_binary = Some(backup);
    Err(anyhow::anyhow!(
        "{} 更新失败: {update_error:#}; 已恢复旧 Worker 并继续监督",
        worker.config.metadata.name
    ))
}

fn remove_backup(path: Option<tempfile::TempPath>) {
    if let Some(path) = path {
        let name = path.display().to_string();
        if let Err(error) = path.close() {
            eprintln!("清理 Worker 回滚备份 {name} 失败: {error}");
        }
    }
}

pub(crate) fn stop_managed_workers(
    workers: &mut [ManagedWorker],
) -> Vec<(String, anyhow::Result<()>)> {
    workers
        .iter_mut()
        .rev()
        .map(|worker| {
            let rollback_binary = worker.rollback_binary.take();
            let result = stop_child(&mut worker.child);
            remove_backup(rollback_binary);
            (worker.config.metadata.name.clone(), result)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{backup_binary_in, remove_backup, startup_failure_context};

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
