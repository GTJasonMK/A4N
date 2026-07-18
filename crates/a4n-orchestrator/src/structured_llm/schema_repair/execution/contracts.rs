use a4n_core::error::AppError;
use a4n_llm::provider::ChatRequest;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use super::super::contract::StructuredRepairMode;

pub type StructuredRepairProgress<'a> = &'a (dyn Fn(&str) -> Result<(), AppError> + Send + Sync);

#[derive(Debug, Clone)]
pub struct StructuredRepairCheckpointContext {
    pub workflow_run_id: i64,
    pub base_call_key: String,
    pub call_order: i32,
}

pub struct StructuredOutputRepairInput<'a, V> {
    pub request: ChatRequest,
    pub raw_response: &'a str,
    pub label: &'a str,
    pub checkpoint: Option<StructuredRepairCheckpointContext>,
    pub cancel_token: Option<&'a CancellationToken>,
    pub progress: Option<StructuredRepairProgress<'a>>,
    pub validate_output: &'a V,
}

pub struct StructuredValueRepairInput<'a, P> {
    pub request: ChatRequest,
    pub raw_response: &'a str,
    pub label: &'a str,
    pub schema: Value,
    pub checkpoint: Option<StructuredRepairCheckpointContext>,
    pub cancel_token: Option<&'a CancellationToken>,
    pub progress: Option<StructuredRepairProgress<'a>>,
    pub parse_output: &'a P,
}

pub struct StructuredOutputRepairResult<T> {
    pub output: T,
    pub final_value: Value,
    pub mode: StructuredRepairMode,
    pub restored: bool,
    pub schema_sha256: Option<String>,
    pub field_patch_error: Option<String>,
}

impl<T> StructuredOutputRepairResult<T> {
    pub(crate) fn artifact_summary(&self) -> serde_json::Map<String, Value> {
        serde_json::Map::from_iter([
            ("schema_version".to_string(), json!(1)),
            ("mode".to_string(), json!(self.mode.as_str())),
            (
                "target_schema_sha256".to_string(),
                json!(self.schema_sha256),
            ),
            (
                "field_patch_error".to_string(),
                json!(self.field_patch_error),
            ),
        ])
    }

    pub(crate) fn artifact_summary_with(
        &self,
        extra_fields: impl IntoIterator<Item = (String, Value)>,
    ) -> serde_json::Map<String, Value> {
        let mut summary = self.artifact_summary();
        summary.extend(extra_fields);
        summary
    }
}
