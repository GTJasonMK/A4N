use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, SystemTime};

use anyhow::{Context, anyhow};
use tracing_appender::non_blocking::{NonBlocking, NonBlockingBuilder, WorkerGuard};
use tracing_appender::rolling::RollingFileAppender;
use tracing_subscriber::{EnvFilter, fmt::writer::MakeWriterExt};

use crate::environment::{optional_non_empty_path, u64_or_default};

const LOG_RETENTION_DAYS_ENV: &str = "A4N_SIDECAR_LOG_RETENTION_DAYS";
const LOG_MAX_BYTES_ENV: &str = "A4N_SIDECAR_LOG_MAX_BYTES";
const DEFAULT_LOG_RETENTION_DAYS: u64 = 14;
const DEFAULT_LOG_MAX_BYTES: u64 = 256 * 1024 * 1024;
const LOG_PRUNE_INTERVAL: Duration = Duration::from_secs(15 * 60);
static FILE_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

pub(super) fn init(log_env: &str, filter_text: &str) -> anyhow::Result<()> {
    let filter = EnvFilter::try_new(filter_text)
        .with_context(|| format!("{log_env} 不是有效日志过滤表达式"))?;
    let file_prefix = log_file_prefix(log_env)?;
    let Some(log_root) = optional_non_empty_path("A4N_LOG_DIR")? else {
        return tracing_subscriber::fmt()
            .with_env_filter(filter)
            .try_init()
            .map_err(|error| anyhow!("初始化 sidecar 日志系统失败: {error}"));
    };

    let service_log_dir = log_root.join("services");
    fs::create_dir_all(&service_log_dir)
        .with_context(|| format!("创建 sidecar 日志目录失败: {}", service_log_dir.display()))?;
    let retention_days = u64_or_default(LOG_RETENTION_DAYS_ENV, DEFAULT_LOG_RETENTION_DAYS)?;
    let max_bytes = u64_or_default(LOG_MAX_BYTES_ENV, DEFAULT_LOG_MAX_BYTES)?;
    prune_logs(&service_log_dir, retention_days, max_bytes)?;
    let file = tracing_appender::rolling::daily(&service_log_dir, file_prefix);
    let (file, guard) = live_file_writer(file);
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stdout.and(file))
        .try_init()
        .map_err(|error| anyhow!("初始化 sidecar 日志系统失败: {error}"))?;
    FILE_GUARD
        .set(guard)
        .map_err(|_| anyhow!("sidecar 日志文件 guard 已初始化"))?;
    spawn_log_pruner(service_log_dir, retention_days, max_bytes);
    Ok(())
}

fn live_file_writer(file: RollingFileAppender) -> (NonBlocking, WorkerGuard) {
    NonBlockingBuilder::default().lossy(false).finish(file)
}

fn spawn_log_pruner(dir: PathBuf, retention_days: u64, max_bytes: u64) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(LOG_PRUNE_INTERVAL).await;
            if let Err(error) = prune_logs(&dir, retention_days, max_bytes) {
                tracing::warn!(directory = %dir.display(), %error, "sidecar 日志保留清理失败");
            }
        }
    });
}

fn log_file_prefix(log_env: &str) -> anyhow::Result<String> {
    let name = log_env
        .strip_prefix("A4N_")
        .and_then(|name| name.strip_suffix("_LOG"))
        .ok_or_else(|| anyhow!("sidecar 日志环境键必须符合 A4N_<SERVICE>_LOG: {log_env}"))?;
    Ok(format!(
        "{}.log",
        name.to_ascii_lowercase().replace('_', "-")
    ))
}

fn prune_logs(dir: &Path, retention_days: u64, max_bytes: u64) -> anyhow::Result<()> {
    let now = SystemTime::now();
    let retention = Duration::from_secs(
        retention_days
            .checked_mul(24 * 60 * 60)
            .ok_or_else(|| anyhow!("sidecar 日志保留天数过大: {retention_days}"))?,
    );
    let mut files = Vec::new();
    for entry in fs::read_dir(dir)
        .with_context(|| format!("读取 sidecar 日志目录失败: {}", dir.display()))?
    {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if !metadata.is_file() {
            continue;
        }
        let modified = metadata.modified()?;
        if now
            .duration_since(modified)
            .is_ok_and(|age| age > retention)
        {
            fs::remove_file(entry.path())?;
            continue;
        }
        files.push((entry.path(), modified, metadata.len()));
    }

    files.sort_by(|left, right| left.1.cmp(&right.1).then_with(|| left.0.cmp(&right.0)));
    let newest_path = files.last().map(|(path, _, _)| path.clone());
    let mut total = files.iter().map(|(_, _, size)| *size).sum::<u64>();
    // Keep the newest file open by tracing-appender; the cap applies to removable history.
    // ponytail: one active daily file may exceed max_bytes; rotate it before enforcing a hard cap.
    for (path, _, size) in files {
        if newest_path.as_ref() == Some(&path) || total <= max_bytes {
            break;
        }
        fs::remove_file(path)?;
        total -= size;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{live_file_writer, log_file_prefix, prune_logs};
    use std::io::Write;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    #[test]
    fn log_file_name_is_derived_from_existing_filter_env() {
        assert_eq!(
            log_file_prefix("A4N_LLM_GATEWAY_LOG").unwrap(),
            "llm-gateway.log"
        );
        assert!(log_file_prefix("RUST_LOG").is_err());
    }

    #[test]
    fn file_log_is_visible_before_process_exit() {
        let dir = std::env::temp_dir().join(format!(
            "a4n-sidecar-live-log-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("live.log");
        let file = tracing_appender::rolling::never(&dir, "live.log");
        let (mut writer, guard) = live_file_writer(file);

        writer.write_all(b"visible\n").unwrap();
        let visible = (0..100).any(|_| {
            if std::fs::metadata(&path).is_ok_and(|metadata| metadata.len() > 0) {
                true
            } else {
                std::thread::sleep(Duration::from_millis(10));
                false
            }
        });

        drop(writer);
        drop(guard);
        let _ = std::fs::remove_dir_all(&dir);
        assert!(visible, "file log should flush while the process is alive");
    }

    #[test]
    fn log_pruning_keeps_newest_file_and_removes_old_history_over_limit() {
        let dir = std::env::temp_dir().join(format!(
            "a4n-sidecar-log-prune-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let old = dir.join("a-old.log");
        let newest = dir.join("z-new.log");
        std::fs::write(&old, b"old").unwrap();
        std::thread::sleep(Duration::from_millis(20));
        std::fs::write(&newest, b"new").unwrap();

        prune_logs(&dir, u64::MAX / (24 * 60 * 60), 0).unwrap();

        assert!(!old.exists());
        assert!(newest.exists());
        let _ = std::fs::remove_dir_all(dir);
    }
}
