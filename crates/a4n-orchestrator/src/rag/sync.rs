mod context;
mod contracts;
mod identity;
mod operations;

pub use context::RagSyncContext;
pub use contracts::{
    DeleteRagRecordsInput, PreparedRagRecordSync, RagRecord, RagRecordSyncOutcome, RagSyncError,
};
pub use identity::{
    classify_rag_record, ensure_embedding_identity_matches,
    ensure_project_embedding_identity_matches,
};

#[cfg(test)]
mod tests;
