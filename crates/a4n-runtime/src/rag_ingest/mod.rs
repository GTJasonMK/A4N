mod client_ingest;
mod contracts;
mod expected_records;
mod orchestration;
mod sync;
#[cfg(test)]
mod tests;

pub use self::client_ingest::{rag_prepare_ingest_all, rag_upload_ingest_embeddings};
pub use self::contracts::{RagIngestTaskScope, SceneRagSyncTraceSource};
pub use self::orchestration::{rag_ingest_all, rag_ingest_task_scope, sync_scene_rag_records};
pub(crate) use self::orchestration::{
    sync_blueprint_material_rag_records, sync_outline_rag_records, sync_protagonist_rag_records,
};
pub use self::sync::{RagIngestProgress, RagProgressReporter};
pub use a4n_api_contract::rag::{
    ClientRagIngestPrepareOutput, ClientRagIngestUploadInput, PreparedClientRagIngestRecord,
    RagIngestOptions, RagIngestResult, UploadedRagEmbedding,
};

pub(crate) fn chapter_rag_chunking_mode() -> a4n_orchestrator::rag::ChapterRagChunkingMode {
    if crate::runtime_config::uses_client_desktop_embedding() {
        a4n_orchestrator::rag::ChapterRagChunkingMode::ClientDesktop
    } else {
        a4n_orchestrator::rag::ChapterRagChunkingMode::Semantic
    }
}
