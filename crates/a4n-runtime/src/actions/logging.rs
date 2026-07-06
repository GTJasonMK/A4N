use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing_subscriber::EnvFilter;

use crate::AppState;
use crate::actions::admin_records::{AdminPermission, require_admin_permission};
use a4n_core::error::AppError;

#[derive(Debug, Deserialize)]
pub struct SetLogLevelInput {
    /// 新的过滤表达式，例如 "debug" / "a4n=trace,info" / "warn"
    pub filter: String,
}

pub fn build_log_filter(filter: &str) -> Result<EnvFilter, String> {
    let filter = filter.trim();
    if filter.is_empty() {
        return Err("日志过滤表达式不能为空".to_string());
    }
    EnvFilter::try_new(filter).map_err(|err| format!("非法过滤表达式: {err}"))
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadLogsInput {
    /// 读取最后 N 行（默认 500）
    pub lines: Option<usize>,
    /// 读取相对于今天的偏移天数（0=今天，1=昨天）
    pub days_ago: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LogReadResult {
    pub file_path: String,
    pub total_lines: usize,
    pub returned_lines: usize,
    pub lines: Vec<String>,
}

pub fn log_dir_path(log_dir: &Path) -> String {
    path_to_string(log_dir)
}

pub async fn ensure_log_diagnostics_admin(
    state: &AppState,
    caller_user_id: i64,
) -> Result<(), AppError> {
    require_admin_permission(state, caller_user_id, AdminPermission::ManageSettings).await
}

pub async fn log_dir_path_for_admin(
    state: &AppState,
    caller_user_id: i64,
    log_dir: &Path,
) -> Result<String, AppError> {
    ensure_log_diagnostics_admin(state, caller_user_id).await?;
    Ok(log_dir_path(log_dir))
}

pub fn read_recent_logs(log_dir: &Path, input: ReadLogsInput) -> Result<LogReadResult, String> {
    let lines = input.lines.unwrap_or(500).min(10_000);
    let days_ago = input.days_ago.unwrap_or(0);
    let date = log_date_for_days_ago(chrono::Local::now().date_naive(), days_ago)?;
    let date_str = date.format("%Y-%m-%d").to_string();
    let file_path = log_dir.join(format!("a4n.log.{date_str}"));

    if !file_path.exists() {
        return Ok(LogReadResult {
            file_path: path_to_string(&file_path),
            total_lines: 0,
            returned_lines: 0,
            lines: vec![format!("（{date_str} 没有日志文件）")],
        });
    }

    let content = std::fs::read_to_string(&file_path).map_err(|e| format!("读取日志失败: {e}"))?;
    let all_lines = content.lines().collect::<Vec<_>>();
    let total = all_lines.len();
    let start = total.saturating_sub(lines);
    let tail = all_lines[start..]
        .iter()
        .map(|line| (*line).to_string())
        .collect::<Vec<_>>();
    let returned = tail.len();
    Ok(LogReadResult {
        file_path: path_to_string(&file_path),
        total_lines: total,
        returned_lines: returned,
        lines: tail,
    })
}

pub async fn read_recent_logs_for_admin(
    state: &AppState,
    caller_user_id: i64,
    log_dir: &Path,
    input: ReadLogsInput,
) -> Result<LogReadResult, AppError> {
    ensure_log_diagnostics_admin(state, caller_user_id).await?;
    read_recent_logs(log_dir, input).map_err(AppError::InvalidArgument)
}

fn log_date_for_days_ago(
    today: chrono::NaiveDate,
    days_ago: u32,
) -> Result<chrono::NaiveDate, String> {
    today
        .checked_sub_signed(chrono::Duration::days(days_ago as i64))
        .ok_or_else(|| format!("日志日期超出支持范围：days_ago={days_ago}"))
}

fn path_to_string(path: impl AsRef<Path>) -> String {
    path.as_ref().to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(prefix: &str) -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .expect("system clock must be after UNIX_EPOCH");
        std::env::temp_dir().join(format!("{prefix}_{}_{}", std::process::id(), stamp))
    }

    #[test]
    fn build_log_filter_rejects_blank_filter() {
        let err = build_log_filter("  ").expect_err("blank filter should fail");
        assert!(err.contains("不能为空"));
    }

    #[test]
    fn build_log_filter_rejects_invalid_filter() {
        let err = build_log_filter("a4n==debug").expect_err("invalid filter should fail");
        assert!(err.contains("非法过滤表达式"));
    }

    #[test]
    fn build_log_filter_accepts_valid_directives() {
        build_log_filter("info,a4n=debug").expect("valid filter should parse");
    }

    #[test]
    fn read_recent_logs_returns_tail_lines() {
        let dir = unique_temp_dir("a4n_logging_tail");
        std::fs::create_dir_all(&dir).expect("create log dir");
        let date_str = chrono::Local::now()
            .date_naive()
            .format("%Y-%m-%d")
            .to_string();
        std::fs::write(dir.join(format!("a4n.log.{date_str}")), "a\nb\nc\nd\n")
            .expect("write log file");

        let result = read_recent_logs(
            &dir,
            ReadLogsInput {
                lines: Some(2),
                days_ago: Some(0),
            },
        )
        .expect("read recent logs");

        assert_eq!(result.total_lines, 4);
        assert_eq!(result.returned_lines, 2);
        assert_eq!(result.lines, vec!["c".to_string(), "d".to_string()]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn read_recent_logs_reports_missing_day() {
        let dir = unique_temp_dir("a4n_logging_missing");
        std::fs::create_dir_all(&dir).expect("create log dir");

        let result = read_recent_logs(
            &dir,
            ReadLogsInput {
                lines: None,
                days_ago: Some(0),
            },
        )
        .expect("read missing log");

        assert_eq!(result.total_lines, 0);
        assert_eq!(result.returned_lines, 0);
        assert_eq!(result.lines.len(), 1);
        assert!(result.lines[0].contains("没有日志文件"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn log_date_for_days_ago_rejects_out_of_range_dates() {
        let today = chrono::NaiveDate::MIN;
        let err =
            log_date_for_days_ago(today, 1).expect_err("underflowing date should be rejected");
        assert!(err.contains("日志日期超出支持范围"));
    }

    #[test]
    fn log_date_for_days_ago_rejects_large_offset_instead_of_panicking() {
        let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 6).expect("valid date");
        let err =
            log_date_for_days_ago(today, u32::MAX).expect_err("large offset should be rejected");

        assert!(err.contains("日志日期超出支持范围"));
        assert!(err.contains("4294967295"));
    }
}
