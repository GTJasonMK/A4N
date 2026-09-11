use super::{LogReadResult, ReadLogsInput};
use crate::AppState;
use crate::actions::admin_records::{AdminPermission, require_admin_permission};
use a4n_error::AppError;
use std::path::Path;

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
    Ok(path_to_string(log_dir))
}

pub(super) fn read_recent_logs(
    log_dir: &Path,
    input: ReadLogsInput,
) -> Result<LogReadResult, String> {
    let lines = input.lines.min(10_000);
    let days_ago = input.days_ago;
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

pub(super) fn log_date_for_days_ago(
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
