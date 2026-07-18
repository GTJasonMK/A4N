use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_dir(prefix: &str) -> std::path::PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be after UNIX_EPOCH")
        .as_nanos();
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
    let err = super::diagnostics::log_date_for_days_ago(today, 1)
        .expect_err("underflowing date should be rejected");
    assert!(err.contains("日志日期超出支持范围"));
}

#[test]
fn log_date_for_days_ago_rejects_large_offset_instead_of_panicking() {
    let today = chrono::NaiveDate::from_ymd_opt(2026, 6, 6).expect("valid date");
    let err = super::diagnostics::log_date_for_days_ago(today, u32::MAX)
        .expect_err("large offset should be rejected");
    assert!(err.contains("日志日期超出支持范围"));
    assert!(err.contains("4294967295"));
}
