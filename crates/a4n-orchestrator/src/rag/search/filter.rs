use std::collections::HashMap;

use a4n_embedding::provider::EmbeddingModelIdentity;
use a4n_rag_core::vector_store::SearchFilter;

pub(crate) fn identity_scoped_search_filter(
    embedding_identity: EmbeddingModelIdentity,
    data_types: Option<Vec<String>>,
) -> SearchFilter {
    let mut equals = HashMap::new();
    equals.insert(
        "embedding_backend".to_string(),
        serde_json::json!(embedding_identity.backend),
    );
    equals.insert(
        "embedding_model".to_string(),
        serde_json::json!(embedding_identity.model),
    );
    equals.insert(
        "embedding_dimension".to_string(),
        serde_json::json!(embedding_identity.dimension),
    );
    SearchFilter {
        equals: Some(equals),
        chapter_range: None,
        data_types,
    }
}
