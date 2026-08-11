use a4n_core::error::AppError;

pub use a4n_api_contract::import_reverse::{
    ConfirmReverseEngineeringCandidateInput, ReverseEngineerInput,
};

pub(super) const REVERSE_VARIABLES_ERROR: &str = "variables 必须是 JSON 对象或 null";

#[derive(Debug, Clone, Copy)]
pub struct ReverseEngineerTaskScope {
    pub project_id: i64,
    pub total: u32,
}

pub type ReverseProgressReporter<'a> =
    &'a (dyn Fn(u32, u32, &str) -> Result<(), AppError> + Send + Sync + 'a);
