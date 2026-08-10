use super::{embedding::EmbeddingModelIdentity, generation_tasks::GenerationStartRequestMeta};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Start a chapter body generation through the canonical generation task path.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StartChapterGenerationInput {
    pub meta: GenerationStartRequestMeta,
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub chapter_number: i32,
    #[schemars(length(max = 32768))]
    pub writing_notes: Option<String>,
    #[schemars(range(min = 1))]
    pub min_word_count: Option<i32>,
    #[schemars(range(min = 1))]
    pub max_word_count: Option<i32>,
    pub client_rag_embedding_identity: Option<EmbeddingModelIdentity>,
    #[schemars(length(equal = 64))]
    pub client_rag_query_fingerprint: Option<String>,
    #[schemars(length(max = 7))]
    pub client_rag_query_vectors: Option<Vec<Vec<f32>>>,
}
