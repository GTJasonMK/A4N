use a4n_core::error::AppError;
use serde::Deserialize;
use serde_json::Value;

pub(super) const REVERSE_VARIABLES_ERROR: &str = "variables 必须是 JSON 对象或 null";

#[derive(Debug, Clone, Deserialize)]
pub struct ReverseEngineerInput {
    pub project_id: i64,
    #[serde(default)]
    pub content_sample: String,
    #[serde(default)]
    pub auto_context: bool,
    #[serde(default)]
    pub context_limit: Option<usize>,
    pub variables: Option<Value>,
}

#[derive(Debug, Clone, Copy)]
pub struct ReverseEngineerTaskScope {
    pub project_id: i64,
    pub total: u32,
}

pub type ReverseProgressReporter<'a> =
    &'a (dyn Fn(u32, u32, &str) -> Result<(), AppError> + Send + Sync);
