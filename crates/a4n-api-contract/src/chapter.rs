use super::generation_tasks::GenerationStartRequestMeta;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Start a chapter body generation through the canonical generation task path.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
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
    #[schemars(length(max = 262144))]
    pub client_rag_context: Option<String>,
}
