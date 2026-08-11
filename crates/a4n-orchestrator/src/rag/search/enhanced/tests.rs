use a4n_core::{models::character::foreshadowing_index, rag::RagDataType};
use a4n_rag_core::{
    memory_store::MemoryVectorStore,
    vector_store::{EmbeddingRecord, SearchFilter, SearchResult, VectorStore},
};
use a4n_test_support::{embedding::MatchingEmbeddingProvider, rag::test_embedding_identity};
use serde_json::json;

use super::{
    contracts::{EnhancedRagQuery, PreparedEnhancedRagSearch},
    execution::{
        execute_prepared_enhanced_rag_search_in_store,
        execute_prepared_enhanced_rag_search_with_vectors_in_store,
    },
    expansion::{build_enhanced_rag_queries, filter_relevant_foreshadowing},
    fusion::fuse_enhanced_rag_results,
    rules::{
        MAX_RAG_TOP_K, enhanced_rag_pull_k, normalize_search_data_types, required_nonblank,
        required_top_k, validate_embedding_identity_match, validate_query_vectors,
    },
};

fn prepared_search(queries: Vec<EnhancedRagQuery>) -> PreparedEnhancedRagSearch {
    PreparedEnhancedRagSearch {
        queries,
        filter: SearchFilter {
            equals: None,
            chapter_range: None,
            data_types: None,
        },
        top_k: 5,
        pull_k: 20,
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
) -> foreshadowing_index::Model {
    let now = chrono::Utc::now().naive_utc();
    foreshadowing_index::Model {
        id,
        project_id: 1,
        description: description.to_string(),
        setup_chapter: 1,
        payoff_chapter: None,
        status: "pending".to_string(),
        related_characters,
        importance,
        created_at: now,
        updated_at: now,
    }
}

fn search_result(id: &str, chapter_number: i32, score: f32) -> SearchResult {
    SearchResult {
        id: id.to_string(),
        content: id.to_string(),
        score,
        vector_score: Some(score),
        bm25_score: None,
        metadata: json!({
            "chapter_number": chapter_number,
            "data_type": RagDataType::Chapter.as_str(),
        }),
    }
}

#[test]
fn enhanced_rag_top_k_contract_preserves_limits() {
    assert!(required_top_k(None, "rag_search").is_err());
    assert!(required_top_k(Some(0), "rag_search").is_err());
    assert!(required_top_k(Some(MAX_RAG_TOP_K + 1), "rag_search").is_err());
    assert_eq!(required_top_k(Some(10), "rag_search").unwrap(), 10);
}

#[test]
fn enhanced_rag_query_text_is_trimmed_and_required() {
    assert_eq!(required_nonblank("  星门  ", "query").unwrap(), "星门");
    assert!(required_nonblank("  ", "query").is_err());
}

#[test]
fn enhanced_rag_data_types_are_normalized_and_validated() {
    assert_eq!(
        normalize_search_data_types(Some(vec![
            " chapter ".into(),
            "character".into(),
            "chapter".into(),
            "".into(),
        ]))
        .unwrap(),
        Some(vec!["chapter".to_string(), "character".to_string()])
    );
    assert!(normalize_search_data_types(Some(vec!["unknown".into()])).is_err());
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
    assert!(error.to_string().contains("项目期望"));
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

    assert_eq!(queries[0].kind, "main");
    assert!(queries.iter().any(|query| query.kind == "character"));
    assert!(queries.iter().any(|query| query.kind == "foreshadowing"));
    assert!(queries.iter().any(|query| query.kind == "location"));
    assert!(queries.len() <= 8);
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
    )
    .expect("enhanced fusion");

    assert_eq!(fused[0].id, "current");
}

#[tokio::test]
async fn enhanced_rag_provider_execution_returns_domain_result() {
    let store = MemoryVectorStore::new();
    store_search_record(&store).await;
    let prepared = prepared_search(vec![EnhancedRagQuery {
        kind: "main",
        query: "当前章节上下文".to_string(),
        weight: 1.0,
    }]);

    let result = execute_prepared_enhanced_rag_search_in_store(
        &store,
        &MatchingEmbeddingProvider,
        1,
        &prepared,
    )
    .await
    .expect("provider enhanced search");

    assert_eq!(result.candidate_count, 1);
    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].id, "chapter:1:6:p1");
    assert_eq!(result.queries.len(), 1);
}

#[tokio::test]
async fn enhanced_rag_vector_execution_validates_identity_and_vectors() {
    let store = MemoryVectorStore::new();
    store_search_record(&store).await;
    let prepared = prepared_search(vec![EnhancedRagQuery {
        kind: "main",
        query: "当前章节上下文".to_string(),
        weight: 1.0,
    }]);

    let mismatched = execute_prepared_enhanced_rag_search_with_vectors_in_store(
        &store,
        1,
        &prepared,
        &test_embedding_identity("other"),
        vec![vec![1.0, 0.0]],
    )
    .await
    .expect_err("mismatched identity should reject");
    assert!(mismatched.to_string().contains("项目期望"));

    let result = execute_prepared_enhanced_rag_search_with_vectors_in_store(
        &store,
        1,
        &prepared,
        &prepared.embedding_identity,
        vec![vec![1.0, 0.0]],
    )
    .await
    .expect("vector enhanced search");
    assert_eq!(result.candidate_count, 1);
    assert_eq!(result.results[0].id, "chapter:1:6:p1");
}
