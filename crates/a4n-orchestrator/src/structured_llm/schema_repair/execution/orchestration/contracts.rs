use a4n_core::error::AppError;
use serde_json::Value;

use super::super::contracts::StructuredOutputRepairResult;

pub(super) enum InitialRepairOutcome<T> {
    Complete(StructuredOutputRepairResult<T>),
    Repair(StructuredRepairState),
}

pub(super) enum FieldPatchOutcome<T> {
    Complete(StructuredOutputRepairResult<T>),
    Continue(StructuredRepairState),
}

pub(super) struct StructuredRepairState {
    pub(super) candidate: Option<Value>,
    pub(super) initial_error: AppError,
    pub(super) field_patch_error: Option<String>,
    pub(super) schema_sha256: Option<String>,
}
