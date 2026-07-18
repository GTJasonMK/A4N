use a4n_embedding::provider::EmbeddingModelIdentity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneRagSyncTraceSource {
    WorkflowRun(i64),
    Independent,
    CurrentOperation,
}

#[derive(Debug, Clone, Copy)]
pub struct RagIngestTaskScope {
    pub project_id: i64,
    pub expected_records: usize,
    pub total: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct PreparedClientRagIngestRecord {
    pub chunk_id: String,
    pub text: String,
    pub content_hash: String,
    pub outcome: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClientRagIngestPrepareOutput {
    pub project_id: i64,
    pub operation_trace_id: String,
    pub embedding_identity: EmbeddingModelIdentity,
    pub records: Vec<PreparedClientRagIngestRecord>,
    pub expected_count: usize,
    pub skipped_count: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UploadedRagEmbedding {
    pub chunk_id: String,
    pub embedding: Vec<f32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ClientRagIngestUploadInput {
    pub operation_trace_id: String,
    pub embedding_identity: EmbeddingModelIdentity,
    pub embeddings: Vec<UploadedRagEmbedding>,
}
