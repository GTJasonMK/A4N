use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use a4n_db::rag::PendingForeshadowingSearchRow;
use a4n_domain::{ProviderError, rag::RagDataType};
use a4n_embedding_provider_contract::{EmbeddingBackend, EmbeddingProvider};
use a4n_rag_core::{
    EnhancedRagQuery, PreparedEnhancedRagSearch,
    memory_store::MemoryVectorStore,
    search::{
        MAX_RAG_TOP_K, enhanced_rag_pull_k, normalize_search_data_types, required_nonblank,
        required_top_k,
    },
    validation::{validate_embedding_identity_match, validate_query_vectors},
    vector_store::{EmbeddingRecord, SearchFilter, SearchResult, VectorStore},
};
use a4n_test_support::{embedding::MatchingEmbeddingProvider, rag::test_embedding_identity};
use serde_json::json;

struct CountingEmbeddingProvider {
    calls: Arc<AtomicUsize>,
}

#[async_trait::async_trait]
impl EmbeddingProvider for CountingEmbeddingProvider {
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, ProviderError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        Ok(texts.into_iter().map(|_| vec![1.0, 0.0]).collect())
    }

    fn dimension(&self) -> usize {
        2
    }

    fn model_name(&self) -> &str {
        "batch-counting-test"
    }

    fn backend(&self) -> EmbeddingBackend {
        EmbeddingBackend::LocalHash
    }
}

use super::{
    execution::{
        execute_prepared_enhanced_rag_search_in_store,
        execute_prepared_enhanced_rag_search_with_vectors_in_store,
    },
    expansion::{build_enhanced_rag_queries, filter_relevant_foreshadowing},
    fusion::fuse_enhanced_rag_results,
};

fn prepared_search(queries: Vec<EnhancedRagQuery>) -> PreparedEnhancedRagSearch {
    PreparedEnhancedRagSearch {
        project_id: 1,
        queries,
        filter: SearchFilter {
            equals: None,
            chapter_range: None,
            data_types: None,
        },
        top_k: 5,
        chapter_number: Some(6),
        embedding_identity: test_embedding_identity("enhanced-execution"),
    }
}

async fn store_search_record(store: &MemoryVectorStore) {
    store
        .add_embeddings(
            "1",
            vec![EmbeddingRecord {
                id: "chapter:1:6:p1".to_string(),
                embedding: vec![1.0, 0.0],
                metadata: json!({
                    "data_type": RagDataType::Chapter.as_str(),
                    "chapter_number": 6,
                }),
                content: "当前章节上下文".to_string(),
            }],
        )
        .await
        .expect("search record should store");
}

fn foreshadowing(
    id: i64,
    description: &str,
    importance: i32,
    related_characters: Option<serde_json::Value>,
) -> PendingForeshadowingSearchRow {
    PendingForeshadowingSearchRow {
        id,
        description: description.to_string(),
        related_characters,
        importance,
    }
}

fn search_result(id: &str, chapter_number: i32, score: f32) -> SearchResult {
    SearchResult {
        id: id.to_string(),
        content: id.to_string(),
        score,
        metadata: json!({
            "chapter_number": chapter_number,
            "data_type": RagDataType::Chapter.as_str(),
        }),
    }
}

#[test]
fn enhanced_rag_top_k_contract_preserves_limits() {
    assert!(required_top_k(0).is_err());
    assert!(required_top_k(MAX_RAG_TOP_K + 1).is_err());
    assert_eq!(required_top_k(10).unwrap(), 10);
}

#[test]
fn enhanced_rag_query_text_is_trimmed_and_required() {
    assert_eq!(required_nonblank("  星门  ").unwrap(), "星门");
    assert!(required_nonblank("  ").is_err());
}

#[test]
fn enhanced_rag_data_types_require_unique_canonical_names() {
    assert_eq!(
        normalize_search_data_types(Some(vec!["chapter".into(), "character".into()])).unwrap(),
        Some(vec!["chapter".to_string(), "character".to_string()])
    );
    for invalid in [
        vec![],
        vec![" chapter ".into()],
        vec!["CHAPTER".into()],
        vec!["chapter".into(), "chapter".into()],
        vec!["unknown".into()],
    ] {
        assert!(normalize_search_data_types(Some(invalid)).is_err());
    }
}

#[test]
fn enhanced_rag_vectors_require_exact_finite_shape() {
    assert!(validate_query_vectors(&[vec![0.1, 0.2]], 1, 2).is_ok());
    assert!(validate_query_vectors(&[], 1, 2).is_err());
    assert!(validate_query_vectors(&[vec![0.1]], 1, 2).is_err());
    assert!(validate_query_vectors(&[vec![f32::INFINITY, 0.2]], 1, 2).is_err());
}

#[test]
fn enhanced_rag_identity_mismatch_keeps_original_error_contract() {
    let expected = test_embedding_identity("expected");
    let actual = test_embedding_identity("actual");

    assert!(validate_embedding_identity_match(&expected, &expected).is_ok());
    let error = validate_embedding_identity_match(&actual, &expected)
        .expect_err("mismatched identity should reject");
    assert!(error.to_string().contains("服务器期望"));
    assert!(error.to_string().contains("客户端提交"));
}

#[test]
fn enhanced_rag_pull_count_keeps_minimum_and_multiplier() {
    assert_eq!(enhanced_rag_pull_k(1), 20);
    assert_eq!(enhanced_rag_pull_k(10), 30);
}

#[test]
fn enhanced_rag_queries_include_character_foreshadowing_and_location() {
    let queries = build_enhanced_rag_queries(
        "林岚前往玄武门确认旧钟线索",
        &["林岚".to_string()],
        &[foreshadowing(1, "旧钟在午夜倒走", 5, None)],
    );

    assert_eq!(queries[0].label, "主查询");
    assert!(queries.iter().any(|query| query.label == "角色"));
    assert!(queries.iter().any(|query| query.label == "伏笔"));
    assert!(queries.iter().any(|query| query.label == "场景"));
}

#[test]
fn enhanced_rag_foreshadowing_filter_keeps_importance_overlap_and_character_match() {
    let selected = filter_relevant_foreshadowing(
        vec![
            foreshadowing(1, "无关低权重", 1, None),
            foreshadowing(2, "重要伏笔", 4, None),
            foreshadowing(3, "玄武门", 1, None),
            foreshadowing(4, "角色相关", 1, Some(json!(["林岚"]))),
        ],
        "前往玄武门",
        &["林岚".to_string()],
    );

    assert_eq!(
        selected.iter().map(|item| item.id).collect::<Vec<_>>(),
        vec![2, 3, 4]
    );
}

#[test]
fn enhanced_rag_fusion_prefers_current_chapter_context() {
    let fused = fuse_enhanced_rag_results(
        vec![(
            1.0,
            vec![
                search_result("older", 1, 0.9),
                search_result("current", 6, 0.4),
            ],
        )],
        Some(6),
    );

    assert_eq!(fused[0].id, "current");
}

#[tokio::test]
async fn enhanced_rag_provider_execution_returns_domain_result() {
    let store = MemoryVectorStore::new();
    store_search_record(&store).await;
    let prepared = prepared_search(vec![EnhancedRagQuery {
        label: "主查询",
        query: "当前章节上下文".to_string(),
        weight: 1.0,
    }]);

    let result =
        execute_prepared_enhanced_rag_search_in_store(&store, &MatchingEmbeddingProvider, prepared)
            .await
            .expect("provider enhanced search");

    assert_eq!(result.candidate_count, 1);
    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].id, "chapter:1:6:p1");
    assert_eq!(result.queries.len(), 1);
}

#[tokio::test]
async fn enhanced_rag_batches_query_embeddings_into_one_provider_call() {
    let store = MemoryVectorStore::new();
    store_search_record(&store).await;
    let calls = Arc::new(AtomicUsize::new(0));
    let provider = CountingEmbeddingProvider {
        calls: calls.clone(),
    };
    let mut prepared = prepared_search(vec![
        EnhancedRagQuery {
            label: "主查询",
            query: "主查询".to_string(),
            weight: 1.0,
        },
        EnhancedRagQuery {
            label: "角色",
            query: "角色查询".to_string(),
            weight: 0.85,
        },
        EnhancedRagQuery {
            label: "场景",
            query: "场景查询".to_string(),
            weight: 0.75,
        },
    ]);
    prepared.embedding_identity = test_embedding_identity("batch-counting-test");

    execute_prepared_enhanced_rag_search_in_store(&store, &provider, prepared)
        .await
        .expect("batched provider enhanced search");

    assert_eq!(calls.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn enhanced_rag_vector_execution_validates_identity_and_vectors() {
    let store = MemoryVectorStore::new();
    store_search_record(&store).await;
    let prepared = prepared_search(vec![EnhancedRagQuery {
        label: "主查询",
        query: "当前章节上下文".to_string(),
        weight: 1.0,
    }]);

    let mismatched = execute_prepared_enhanced_rag_search_with_vectors_in_store(
        &store,
        prepared,
        &test_embedding_identity("other"),
        vec![vec![1.0, 0.0]],
    )
    .await
    .err()
    .expect("mismatched identity should reject");
    assert!(mismatched.to_string().contains("服务器期望"));

    let prepared = prepared_search(vec![EnhancedRagQuery {
        label: "主查询",
        query: "当前章节上下文".to_string(),
        weight: 1.0,
    }]);
    let embedding_identity = prepared.embedding_identity.clone();
    let result = execute_prepared_enhanced_rag_search_with_vectors_in_store(
        &store,
        prepared,
        &embedding_identity,
        vec![vec![1.0, 0.0]],
    )
    .await
    .expect("vector enhanced search");
    assert_eq!(result.candidate_count, 1);
    assert_eq!(result.results[0].id, "chapter:1:6:p1");
}
