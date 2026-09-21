//! Pure RAG values shared by application ports and storage implementations.
//!
//! Search, ingest, and vector-store implementations stay outside this crate
//! so application contracts do not inherit the RAG engine.

pub mod chapter {
    #[derive(Clone, Copy, PartialEq)]
    pub enum ChapterRagChunkingMode {
        Semantic,
        ClientDesktop,
    }
}

pub mod search {
    use a4n_embedding_value_contract::EmbeddingModelIdentity;

    use crate::vector_store::{SearchFilter, SearchResult};

    pub struct EnhancedRagQuery {
        pub label: &'static str,
        pub query: String,
        pub weight: f32,
    }

    pub struct PreparedEnhancedRagSearch {
        pub project_id: i64,
        pub queries: Vec<EnhancedRagQuery>,
        pub filter: SearchFilter,
        pub top_k: usize,
        pub chapter_number: Option<i32>,
        pub embedding_identity: EmbeddingModelIdentity,
    }

    pub struct EnhancedRagSearchResult {
        pub results: Vec<SearchResult>,
        pub queries: Vec<EnhancedRagQuery>,
        pub candidate_count: usize,
    }
}

pub mod sync {
    use serde_json::Value;

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum RagRecordSyncOutcome {
        Added,
        Updated,
        Skipped,
    }

    pub struct RagRecord {
        pub chunk_id: String,
        pub text: String,
        pub metadata: Value,
    }

    pub struct PreparedRagRecordSync {
        pub(super) record: RagRecord,
        pub(super) outcome: RagRecordSyncOutcome,
    }

    impl PreparedRagRecordSync {
        pub fn new(record: RagRecord, outcome: RagRecordSyncOutcome) -> Self {
            Self { record, outcome }
        }

        pub fn outcome(&self) -> RagRecordSyncOutcome {
            self.outcome
        }

        pub fn chunk_id(&self) -> &str {
            &self.record.chunk_id
        }

        pub fn into_parts(self) -> (RagRecord, RagRecordSyncOutcome) {
            (self.record, self.outcome)
        }
    }

    #[derive(Debug, Clone)]
    pub struct RagEmbeddedInput {
        pub chunk_id: String,
        pub text: String,
        pub metadata: Value,
        pub embedding: Vec<f32>,
    }
}

pub mod vector_store {
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;

    /// 检索过滤条件。
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct SearchFilter {
        /// 等值过滤。
        pub equals: Option<HashMap<String, serde_json::Value>>,
        /// 章节范围过滤。
        pub chapter_range: Option<(u32, u32)>,
        /// 数据类型过滤。
        pub data_types: Option<Vec<String>>,
    }

    /// 检索结果。
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub struct SearchResult {
        pub id: String,
        pub content: String,
        /// 当前检索实现给出的排序分数。
        pub score: f32,
        pub metadata: serde_json::Value,
    }
}

pub use chapter::ChapterRagChunkingMode;
pub use search::{EnhancedRagQuery, EnhancedRagSearchResult, PreparedEnhancedRagSearch};
pub use sync::{PreparedRagRecordSync, RagEmbeddedInput, RagRecord, RagRecordSyncOutcome};
pub use vector_store::{SearchFilter, SearchResult};
