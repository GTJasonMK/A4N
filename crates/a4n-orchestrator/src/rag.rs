mod blueprint_records;
mod chapter_cleanup;
mod chapter_records;
mod character_profile_records;
mod continuity_records;
mod keys;
mod outline_records;
mod scene_records;
pub(crate) mod search;
mod sync;
mod write;

pub use blueprint_records::{
    blueprint_metadata_rag_record, blueprint_rag_record, blueprint_world_setting_rag_records,
    relationship_rag_record,
};
pub use chapter_records::ChapterRagChunkingMode;
pub(crate) use chapter_records::chapter_rag_records_with_provider;
pub use character_profile_records::{blueprint_character_rag_records, protagonist_rag_records};
pub use continuity_records::{
    CharacterProfileRagDeletionSummary, CharacterSyncRagInput, CharacterSyncRagSummary,
    CharacterTimelineRagDeletionSummary, character_attribute_event_rag_record,
    character_state_rag_record, foreshadowing_rag_record, foreshadowing_rag_record_from_row,
};
pub(crate) use keys::{
    chapter_rag_chunk_prefix, character_attribute_event_rag_chunk_id, character_state_rag_chunk_id,
    foreshadowing_rag_chunk_id, protagonist_rag_chunk_id, protagonist_rag_chunk_prefix,
    rag_chunk_id, rag_project_prefix,
};
pub use outline_records::{
    outline_rag_record, outline_rag_record_from_row, part_outline_rag_record,
};
pub use scene_records::{
    scene_occurrence_rag_record, scene_profile_rag_record, scene_transition_rag_record,
};
pub use search::{
    EnhancedRagQuery, EnhancedRagSearchResult, PreparedEnhancedRagSearch,
    normalize_search_data_types, required_nonblank, required_top_k,
};
pub(crate) use search::{
    hybrid_rag_search_filtered_in_store, hybrid_rag_search_with_validated_embedding_in_store,
    identity_scoped_search_filter,
};
pub use sync::{
    DeleteRagRecordsInput, PreparedRagRecordSync, RagRecord, RagRecordSyncOutcome, RagSyncContext,
    RagSyncError, classify_rag_record, ensure_embedding_identity_matches,
    ensure_project_embedding_identity_matches,
};
pub use write::RagEmbeddedInput;
pub(crate) use write::RagEmbeddingInput;

pub(crate) use write::{embed_and_store_rag_record_in_store, embed_and_store_rag_records_in_store};

pub(crate) use write::store_embedded_rag_records_in_store;
