mod blueprint_records;
mod chapter_cleanup;
mod chapter_records;
mod character_profile_records;
mod continuity_records;
mod outline_records;
mod scene_records;
pub(crate) mod search;
pub use search::{EnhancedRagSearchResult, PreparedEnhancedRagSearch};
mod sync;
mod write;

pub use blueprint_records::{
    blueprint_metadata_rag_record, blueprint_metadata_rag_record_label, blueprint_rag_record,
    blueprint_rag_record_label, blueprint_world_setting_rag_record_label,
    blueprint_world_setting_rag_records, relationship_rag_record, relationship_rag_record_label,
};
pub use chapter_records::{ChapterRagChunkingMode, chapter_rag_record_label};
pub use character_profile_records::{
    blueprint_character_rag_record_label, blueprint_character_rag_records,
    protagonist_rag_record_label, protagonist_rag_records,
};
pub use continuity_records::{
    CharacterProfileRagDeletionSummary, CharacterSyncRagInput, CharacterSyncRagSummary,
    CharacterTimelineRagDeletionSummary, character_attribute_event_rag_record,
    character_state_rag_record, foreshadowing_rag_record,
};
pub use outline_records::{
    outline_rag_record, outline_rag_record_label, part_outline_rag_record,
    part_outline_rag_record_label,
};
pub use scene_records::{
    scene_occurrence_rag_record, scene_occurrence_rag_record_label, scene_profile_rag_record,
    scene_profile_rag_record_label, scene_transition_rag_record, scene_transition_rag_record_label,
};
pub use search::*;
pub(crate) use search::{
    hybrid_rag_search_filtered_in_store, hybrid_rag_search_with_embedding_in_store,
};
pub use sync::*;
pub use write::{RagEmbeddedInput, RagEmbeddingInput};

pub use chapter_records::records::chapter_rag_records_with_provider;
pub use write::provider::{
    embed_and_store_rag_record_in_store, embed_and_store_rag_records_in_store,
};

pub use write::store_embedded_rag_records_in_store;
