mod diagnostics;
mod filter;

pub use a4n_api_contract::logging::{LogReadResult, ReadLogsInput, SetLogLevelInput};
pub use diagnostics::{
    ensure_log_diagnostics_admin, log_dir_path_for_admin, read_recent_logs_for_admin,
};
#[cfg(test)]
pub(crate) use filter::build_log_filter;
pub use filter::{LogFilterReloader, build_log_filter_from_env};

#[cfg(test)]
mod tests;
