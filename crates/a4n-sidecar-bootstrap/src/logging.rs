use std::env::VarError;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use anyhow::{Context, anyhow};
use tracing_subscriber::{EnvFilter, fmt::writer::MakeWriterExt};

const LOG_RETENTION_DAYS_ENV: &str = "A4N_SIDECAR_LOG_RETENTION_DAYS";
const LOG_MAX_BYTES_ENV: &str = "A4N_SIDECAR_LOG_MAX_BYTES";
const DEFAULT_LOG_RETENTION_DAYS: u64 = 14;
const DEFAULT_LOG_MAX_BYTES: u64 = 256 * 1024 * 1024;
const LOG_PRUNE_INTERVAL: Duration = Duration::from_secs(15 * 60);

pub(super) fn init(log_env: &str, filter_text: &str) -> anyhow::Result<()> {
    let filter = EnvFilter::try_new(filter_text)
        .with_context(|| format!("{log_env} 不是有效日志过滤表达式"))?;
    let Some(log_root) = optional_path_env("A4N_LOG_DIR")? else {
        return tracing_subscriber::fmt()
            .with_env_filter(filter)
            .try_init()
            .map_err(|error| anyhow!("初始化 sidecar 日志系统失败: {error}"));
    };

    let service_log_dir = log_root.join("services");
    fs::create_dir_all(&service_log_dir)
        .with_context(|| format!("创建 sidecar 日志目录失败: {}", service_log_dir.display()))?;
    let retention_days = env_u64(LOG_RETENTION_DAYS_ENV, DEFAULT_LOG_RETENTION_DAYS)?;
    let max_bytes = env_u64(LOG_MAX_BYTES_ENV, DEFAULT_LOG_MAX_BYTES)?;
    prune_logs(&service_log_dir, retention_days, max_bytes)?;
    let file = tracing_appender::rolling::daily(&service_log_dir, log_file_prefix(log_env));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stdout.and(file))
        .try_init()
        .map_err(|error| anyhow!("初始化 sidecar 日志系统失败: {error}"))?;
    spawn_log_pruner(service_log_dir, retention_days, max_bytes);
    Ok(())
}

fn spawn_log_pruner(dir: PathBuf, retention_days: u64, max_bytes: u64) {
    let Ok(runtime) = tokio::runtime::Handle::try_current() else {
        return;
    };
    runtime.spawn(async move {
        loop {
            tokio::time::sleep(LOG_PRUNE_INTERVAL).await;
            if let Err(error) = prune_logs(&dir, retention_days, max_bytes) {
                tracing::warn!(directory = %dir.display(), %error, "sidecar 日志保留清理失败");
            }
        }
    });
}

fn optional_path_env(key: &str) -> anyhow::Result<Option<PathBuf>> {
    match std::env::var(key) {
        Ok(value) => {
            let value = value.trim();
            if value.is_empty() {
                Err(anyhow!("{key} 不能为空"))
            } else {
                Ok(Some(PathBuf::from(value)))
            }
        }
        Err(VarError::NotPresent) => Ok(None),
        Err(error) => Err(anyhow!("读取 {key} 失败: {error}")),
    }
}

fn env_u64(key: &str, default: u64) -> anyhow::Result<u64> {
    match std::env::var(key) {
        Ok(value) => value
            .trim()
            .parse::<u64>()
            .with_context(|| format!("{key} 必须是非负整数")),
        Err(VarError::NotPresent) => Ok(default),
        Err(error) => Err(anyhow!("读取 {key} 失败: {error}")),
    }
}

fn log_file_prefix(log_env: &str) -> String {
    let name = log_env
        .strip_prefix("A4N_")
        .unwrap_or(log_env)
        .strip_suffix("_LOG")
        .unwrap_or_else(|| log_env.strip_prefix("A4N_").unwrap_or(log_env));
    format!("{}.log", name.to_ascii_lowercase().replace('_', "-"))
}

fn prune_logs(dir: &Path, retention_days: u64, max_bytes: u64) -> anyhow::Result<()> {
    let now = SystemTime::now();
    let retention = Duration::from_secs(retention_days.saturating_mul(24 * 60 * 60));
    let mut files = Vec::new();
    for entry in fs::read_dir(dir)
        .with_context(|| format!("读取 sidecar 日志目录失败: {}", dir.display()))?
    {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if !metadata.is_file() {
            continue;
        }
        let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
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
        total = total.saturating_sub(size);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{log_file_prefix, prune_logs};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    #[test]
    fn log_file_name_is_derived_from_existing_filter_env() {
        assert_eq!(log_file_prefix("A4N_LLM_GATEWAY_LOG"), "llm-gateway.log");
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

        prune_logs(&dir, u64::MAX, 0).unwrap();

        assert!(!old.exists());
        assert!(newest.exists());
        let _ = std::fs::remove_dir_all(dir);
    }
}
