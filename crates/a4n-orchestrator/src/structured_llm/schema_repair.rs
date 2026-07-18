mod contract;
mod diagnostic;
mod execution;
mod patch;
mod request;

pub use contract::StructuredRepairMode;
pub(crate) use diagnostic::json_shape_schema_from_example;
pub use execution::{
    StructuredOutputRepairInput, StructuredOutputRepairResult, StructuredRepairCheckpointContext,
    StructuredValueRepairInput,
};
pub(super) const MAX_SCHEMA_DIAGNOSTICS: usize = 16;
pub(super) const MAX_SCHEMA_PATCH_BYTES: usize = 32 * 1024;
pub(super) const MAX_SCHEMA_PATCH_TOKENS: usize = 2048;
pub(super) const MAX_LOCAL_CONTEXT_BYTES: usize = 8 * 1024;
