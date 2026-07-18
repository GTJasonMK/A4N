mod contracts;
mod diagnostics;
mod filter;

pub use contracts::{LogReadResult, ReadLogsInput, SetLogLevelInput};
pub use diagnostics::{
    ensure_log_diagnostics_admin, log_dir_path, log_dir_path_for_admin, read_recent_logs,
    read_recent_logs_for_admin,
};
pub use filter::build_log_filter;

#[cfg(test)]
mod tests;
