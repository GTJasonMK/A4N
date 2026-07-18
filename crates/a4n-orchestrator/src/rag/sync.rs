mod context;
mod contracts;
mod identity;
mod keys;
mod metadata;
mod operations;

pub use context::RagSyncContext;
pub use contracts::{
    DeleteRagRecordsInput, PreparedRagRecordSync, RagRecord, RagRecordSyncOutcome, RagSyncError,
};
pub use identity::{
    classify_rag_record, ensure_embedding_identity_matches,
    ensure_project_embedding_identity_matches,
};
pub use keys::{
    chapter_rag_chunk_prefix, character_attribute_event_rag_chunk_id, character_state_rag_chunk_id,
    foreshadowing_rag_chunk_id, protagonist_rag_chunk_id, protagonist_rag_chunk_prefix,
};
pub use metadata::root_text_family_metadata;

#[cfg(test)]
mod tests;
