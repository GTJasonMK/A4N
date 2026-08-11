mod chapters;
mod characters;
mod client_ingest;
mod contracts;
mod expected_records;
mod orchestration;
mod scenes;
mod story_records;
mod sync;
#[cfg(test)]
mod tests;

pub use self::client_ingest::{rag_prepare_ingest_all, rag_upload_ingest_embeddings};
pub use self::contracts::{
    ClientRagIngestPrepareOutput, ClientRagIngestUploadInput, PreparedClientRagIngestRecord,
    RagIngestTaskScope, SceneRagSyncTraceSource, UploadedRagEmbedding,
};
pub use self::orchestration::{
    expected_rag_record_total, rag_ingest_all, rag_ingest_task_scope, sync_scene_rag_records,
};
pub use self::sync::{ExpectedSyncStats as IngestStats, RagIngestProgress, RagProgressReporter};
pub(crate) use self::{
    characters::{
        ingest_character_records_inner, ingest_protagonists_inner, ingest_relationships_inner,
    },
    story_records::{
        ingest_blueprint_metadata_inner, ingest_blueprints_inner, ingest_outlines_inner,
        ingest_part_outlines_inner, ingest_world_setting_inner,
    },
};

pub(crate) fn chapter_rag_chunking_mode() -> a4n_orchestrator::rag::ChapterRagChunkingMode {
    if crate::runtime_config::uses_client_desktop_embedding() {
        a4n_orchestrator::rag::ChapterRagChunkingMode::ClientDesktop
    } else {
        a4n_orchestrator::rag::ChapterRagChunkingMode::Semantic
    }
}
