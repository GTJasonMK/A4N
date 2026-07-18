use a4n_orchestrator::rag::{PreparedRagRecordSync, RagRecord};

#[derive(Debug, Clone)]
pub(in crate::rag_ingest) struct LabeledRagRecord {
    pub(super) record: RagRecord,
    pub(super) label: String,
    pub(super) error_context: String,
}

#[derive(Debug, Clone)]
pub(super) struct PreparedLabeledRagRecord {
    pub(super) prepared: PreparedRagRecordSync,
    pub(super) label: String,
    pub(super) error_context: String,
}
