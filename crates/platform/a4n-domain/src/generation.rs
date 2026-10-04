mod canonical;
mod lifecycle;
mod records;

pub use canonical::{canonical_json_bytes, canonical_json_sha256, canonical_json_text};
pub use lifecycle::{
    GenerationLifecycleTask, LifecycleGateClaimState, PendingLifecycleOperation,
    ProjectLifecycleGateClaim, ProjectLifecycleOperation, UserLifecycleGateClaim,
};
pub use records::{
    GenerationRunTraceTarget, GenerationSpecRecord, GenerationTaskCheckpointRecord,
    GenerationTaskCheckpointSummaryRecord, GenerationTaskListCursor, PromptArtifactRecord,
    RecordCompletedGenerationRunInput,
};

pub const SOURCE_CONTEXT_FREEFORM: &str = "freeform";

pub fn freeform_source_version_hash(params_json: &serde_json::Value) -> Option<String> {
    params_json
        .get("source_material")
        .filter(|value| value.is_object())
        .map(|value| crate::source_text_hash(&value.to_string()))
}
