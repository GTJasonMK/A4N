use super::contracts::PreparedGenerationRagSearch;
use super::execution::{
    execute_prepared_chapter_generation_rag_search_in_store,
    execute_prepared_chapter_generation_rag_search_with_vectors_in_store,
};
use super::formatting::{
    GENERATION_RAG_CONTEXT_CHAR_LIMIT, GENERATION_RAG_MAX_PASSAGE_RESULTS,
    format_generation_rag_context, format_generation_rag_result,
};
use super::fusion::fuse_generation_rag_results;
use super::preparation::build_chapter_generation_rag_search_filter;
use super::query::{GenerationRagQuery, build_generation_rag_queries};
use super::rules::{
    generation_rag_query_fingerprint, generation_rag_search_data_types, previous_chapter_range,
    validate_generation_rag_query_fingerprint,
};
use a4n_db::rag::RagForeshadowingRow;
use a4n_domain::chapter::ChapterOutlineRecord;
use a4n_domain::rag::RagDataType;
use a4n_domain::text_location::extract_scene_location;
use a4n_rag_core::{
    memory_store::MemoryVectorStore,
    metadata::metadata_i64,
    vector_store::{EmbeddingRecord, SearchFilter, SearchResult, VectorStore},
};
use a4n_test_support::{embedding::MatchingEmbeddingProvider, rag::test_embedding_identity};
use serde_json::{Value, json};

fn prepared_generation_search() -> PreparedGenerationRagSearch {
    PreparedGenerationRagSearch {
        project_id: 1,
        chapter_number: 6,
        queries: vec![GenerationRagQuery {
            text: "密室线索".to_string(),
            weight: 1.0,
        }],
        filter: SearchFilter {
            equals: None,
            chapter_range: None,
            data_types: None,
        },
        embedding_identity: test_embedding_identity("chapter-generation-execution"),
    }
}

#[test]
fn generation_rag_query_fingerprint_binds_query_order_and_identity() {
    let prepared = prepared_generation_search();
    let fingerprint = generation_rag_query_fingerprint(
        prepared.project_id,
        prepared.chapter_number,
        &prepared.queries,
        &prepared.embedding_identity,
    )
    .expect("query fingerprint must build");
    assert_eq!(fingerprint.len(), 64);
    validate_generation_rag_query_fingerprint(&fingerprint, &prepared)
        .expect("matching fingerprint must pass");

    let mut changed_query = prepared_generation_search();
    changed_query.queries[0].text.push_str("更新");
    assert!(
        validate_generation_rag_query_fingerprint(&fingerprint, &changed_query)
            .expect_err("changed query must invalidate client vectors")
            .contains("已过期")
    );

    let mut changed_identity = prepared_generation_search();
    changed_identity.embedding_identity.model.push_str("-new");
    assert!(validate_generation_rag_query_fingerprint(&fingerprint, &changed_identity).is_err());
}

async fn store_generation_search_record(store: &MemoryVectorStore) {
    store
        .add_embeddings(
            "1",
            vec![EmbeddingRecord {
                id: "chapter:1:3:p1".to_string(),
                embedding: vec![0.0, 1.0],
                metadata: json!({
                    "project_id": 1,
                    "data_type": RagDataType::Chapter.as_str(),
                    "chapter_number": 3,
                    "title": "第三章",
                }),
                content: "主角在密室发现旧钟线索。".to_string(),
            }],
        )
        .await
        .expect("generation search record should store");
}

fn outline_with_characters() -> ChapterOutlineRecord {
    let now = chrono::Utc::now().naive_utc();
    ChapterOutlineRecord {
        id: 1,
        project_id: 1,
        part_id: None,
        chapter_number: 6,
        title: Some("玄武暗线".to_string()),
        summary: Some("林岚与陈恪前往玄武门调查旧案。".to_string()),
        beats: None,
        key_scenes: None,
        pacing_score: None,
        foreshadowing_setup: None,
        foreshadowing_payoff: None,
        characters_involved: Some(json!(["林岚", "陈恪"])),
        word_count_target: None,
        status: "draft".to_string(),
        created_at: now,
        updated_at: now,
    }
}

fn foreshadowing(description: &str) -> RagForeshadowingRow {
    RagForeshadowingRow {
        id: 1,
        description: description.to_string(),
        setup_chapter: 2,
        payoff_chapter: None,
        status: "pending".to_string(),
        related_characters: None,
        importance: 4,
    }
}

fn rag_result(metadata: Value) -> SearchResult {
    SearchResult {
        id: "chapter-summary-1".to_string(),
        content: "主角发现密室线索。".to_string(),
        score: 0.42,
        metadata,
    }
}

fn search_result(id: &str, source_chapter: i32, score: f32) -> SearchResult {
    search_result_with_type(
        id,
        RagDataType::Chapter.as_str(),
        source_chapter,
        format!("内容 {id}"),
        score,
    )
}

fn search_result_with_type(
    id: &str,
    data_type: &str,
    source_chapter: i32,
    content: impl Into<String>,
    score: f32,
) -> SearchResult {
    SearchResult {
        id: id.to_string(),
        content: content.into(),
        score,
        metadata: json!({
            "data_type": data_type,
            "chapter_number": source_chapter,
            "title": format!("第{source_chapter}章"),
        }),
    }
}

#[test]
fn generation_rag_queries_split_main_character_foreshadowing_and_location() {
    let outline = outline_with_characters();
    let queries = build_generation_rag_queries(
        Some(&outline),
        "## 当前任务\n林岚与陈恪前往玄武门调查旧案。",
        Some("强调悬疑感"),
        &[foreshadowing("旧钟在午夜倒走")],
    );

    assert!(queries[0].text.contains("章节标题: 玄武暗线"));
    assert!(queries[0].text.contains("写作要点: 强调悬疑感"));
    assert!(!queries[0].text.contains("待处理伏笔"));
    assert!(!queries[0].text.contains("旧钟在午夜倒走"));
    assert_eq!(
        queries
            .iter()
            .filter(|query| query.text.starts_with("角色 "))
            .count(),
        2
    );
    assert!(queries.iter().any(|query| query.text.contains("角色 林岚")));
    assert!(
        queries
            .iter()
            .any(|query| query.text == "伏笔: 旧钟在午夜倒走")
    );
    assert!(
        queries
            .iter()
            .any(|query| query.text == "场景 玄武门 中发生的事件和相关设定")
    );
    assert!(queries.len() >= 4);
}

#[test]
fn generation_rag_search_excludes_authoritative_foreshadowing_index_records() {
    let data_types = generation_rag_search_data_types();

    assert!(!data_types.contains(&RagDataType::Foreshadowing.as_str().to_string()));
    assert!(data_types.contains(&RagDataType::Chapter.as_str().to_string()));
    assert!(data_types.contains(&RagDataType::KeyEvent.as_str().to_string()));
}

#[test]
fn previous_chapter_range_requires_a_chapter_after_the_first() {
    assert_eq!(
        previous_chapter_range(2).expect("chapter 2 has a previous range"),
        (1, 1)
    );
    assert!(previous_chapter_range(1).is_err());
    assert!(previous_chapter_range(i32::MIN).is_err());
}

#[test]
fn chapter_generation_filter_adds_previous_range_and_allowed_types() {
    let identity = test_embedding_identity("chapter-generation-filter");
    let filter =
        build_chapter_generation_rag_search_filter(4, identity).expect("chapter generation filter");

    assert_eq!(filter.chapter_range, Some((1, 3)));
    let data_types = filter.data_types.expect("chapter generation data types");
    assert!(data_types.contains(&RagDataType::Chapter.as_str().to_string()));
    assert!(data_types.contains(&RagDataType::KeyEvent.as_str().to_string()));
    assert!(!data_types.contains(&RagDataType::Foreshadowing.as_str().to_string()));
}

#[tokio::test]
async fn generation_rag_provider_execution_returns_domain_context() {
    let store = MemoryVectorStore::new();
    store_generation_search_record(&store).await;
    let prepared = prepared_generation_search();

    let context = execute_prepared_chapter_generation_rag_search_in_store(
        &store,
        &MatchingEmbeddingProvider,
        &prepared,
    )
    .await
    .expect("provider generation RAG search");

    assert!(
        context
            .as_deref()
            .is_some_and(|text| text.contains("密室发现旧钟线索"))
    );
}

#[tokio::test]
async fn generation_rag_vector_execution_validates_identity_and_searches() {
    let store = MemoryVectorStore::new();
    store_generation_search_record(&store).await;
    let prepared = prepared_generation_search();

    let mismatch = execute_prepared_chapter_generation_rag_search_with_vectors_in_store(
        &store,
        &prepared,
        &test_embedding_identity("other"),
        vec![vec![0.0, 1.0]],
    )
    .await
    .expect_err("mismatched identity should reject");
    assert!(mismatch.contains("服务器期望"));

    let context = execute_prepared_chapter_generation_rag_search_with_vectors_in_store(
        &store,
        &prepared,
        &prepared.embedding_identity,
        vec![vec![0.0, 1.0]],
    )
    .await
    .expect("vector generation RAG search");
    assert!(context.is_some());
}

#[test]
fn generation_rag_fusion_deduplicates_and_prefers_recent_context() {
    let fused = fuse_generation_rag_results(
        vec![
            (
                1.0,
                vec![
                    search_result("older", 1, 0.9),
                    search_result("shared", 2, 0.4),
                ],
            ),
            (
                0.85,
                vec![
                    search_result("recent", 5, 0.3),
                    search_result("shared", 4, 0.8),
                ],
            ),
        ],
        6,
    );

    let ids = fused
        .iter()
        .map(|item| item.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids.len(), 3);
    assert!(ids.contains(&"shared"));
    assert!(
        fused
            .iter()
            .position(|item| item.id == "recent")
            .expect("recent result")
            < fused
                .iter()
                .position(|item| item.id == "older")
                .expect("older result")
    );
    let shared = fused
        .iter()
        .find(|item| item.id == "shared")
        .expect("shared result");
    assert_eq!(metadata_i64(&shared.metadata, "chapter_number"), Some(4));
}

#[test]
fn generation_rag_fusion_applies_data_type_weight() {
    let fused = fuse_generation_rag_results(
        vec![(
            1.0,
            vec![
                search_result_with_type(
                    "state",
                    RagDataType::CharacterState.as_str(),
                    3,
                    "角色状态",
                    1.0,
                ),
                search_result_with_type(
                    "event",
                    RagDataType::KeyEvent.as_str(),
                    3,
                    "关键事件",
                    1.0,
                ),
            ],
        )],
        6,
    );

    let ids = fused
        .iter()
        .map(|item| item.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, vec!["event", "state"]);
    assert!(fused[0].score > fused[1].score);
}

#[test]
fn generation_location_extraction_handles_common_scene_cues() {
    assert_eq!(
        extract_scene_location("众人在显德殿中等待消息").as_deref(),
        Some("显德殿")
    );
    assert_eq!(
        extract_scene_location("陈恪前往玄武门确认伏兵").as_deref(),
        Some("玄武门")
    );
    assert!(extract_scene_location("他在沉默之后做出选择").is_none());
}

#[test]
fn format_generation_rag_result_rejects_missing_title() {
    let result = rag_result(json!({
        "data_type": "chapter_summary",
        "chapter_number": 1
    }));

    assert!(format_generation_rag_result(&result, 2).is_none());
}

#[test]
fn format_generation_rag_result_uses_trimmed_title() {
    let result = rag_result(json!({
        "data_type": "chapter_summary",
        "chapter_number": 1,
        "title": "  灰烬低语  "
    }));

    let formatted = format_generation_rag_result(&result, 2).expect("formatted result");

    assert!(formatted.line.contains("第1章《灰烬低语》"));
    assert!(!formatted.line.contains("无标题"));
}

#[test]
fn format_generation_rag_result_includes_character_attribute_event() {
    let result = rag_result(json!({
        "data_type": "character_attribute_event",
        "chapter_number": 3,
        "title": "林舟 第3章属性变更"
    }));

    let formatted = format_generation_rag_result(&result, 5).expect("formatted result");

    assert!(formatted.line.contains("第3章《林舟 第3章属性变更》"));
    assert!(formatted.line.contains("[角色属性变更"));
}

#[test]
fn format_generation_rag_context_layers_continuity_summaries_and_passages() {
    let results = vec![
        search_result_with_type(
            "chapter-first",
            RagDataType::Chapter.as_str(),
            2,
            "正文片段",
            0.9,
        ),
        search_result_with_type(
            "summary",
            RagDataType::ChapterSummary.as_str(),
            3,
            "章节摘要",
            0.8,
        ),
        search_result_with_type(
            "key-event",
            RagDataType::KeyEvent.as_str(),
            1,
            "旧钟会在午夜倒走",
            0.7,
        ),
        search_result_with_type(
            "character-state",
            RagDataType::CharacterState.as_str(),
            4,
            "林岚仍在玄武门附近",
            0.6,
        ),
    ];

    let text = format_generation_rag_context(&results, 6).expect("formatted context");

    let continuity_pos = text.find("### 关键连续性").expect("continuity section");
    let summary_pos = text.find("### 相关摘要").expect("summary section");
    let passage_pos = text.find("### 参考片段").expect("passage section");
    assert!(continuity_pos < summary_pos);
    assert!(summary_pos < passage_pos);
    assert!(text.contains("[关键事件"));
    assert!(text.contains("[角色状态"));
}

#[test]
fn format_generation_rag_context_keeps_continuity_when_passages_arrive_first() {
    let mut results = (0..8)
        .map(|idx| {
            search_result_with_type(
                &format!("passage-{idx}"),
                RagDataType::Chapter.as_str(),
                2,
                format!("普通正文片段 {idx}"),
                1.0 - idx as f32 * 0.01,
            )
        })
        .collect::<Vec<_>>();
    results.push(search_result_with_type(
        "late-key-event",
        RagDataType::KeyEvent.as_str(),
        1,
        "必须保留的迟到伏笔",
        0.1,
    ));

    let text = format_generation_rag_context(&results, 6).expect("formatted context");

    assert_eq!(
        text.matches("普通正文片段").count(),
        GENERATION_RAG_MAX_PASSAGE_RESULTS
    );
    assert!(text.contains("必须保留的迟到伏笔"));
    assert!(text.contains("### 关键连续性"));
}

#[test]
fn format_generation_rag_context_respects_total_char_limit() {
    let long_content = "长内容".repeat(2_000);
    let results = vec![
        search_result_with_type(
            "long-continuity",
            RagDataType::KeyEvent.as_str(),
            1,
            long_content.clone(),
            0.9,
        ),
        search_result_with_type(
            "long-summary",
            RagDataType::ChapterSummary.as_str(),
            2,
            long_content.clone(),
            0.8,
        ),
        search_result_with_type(
            "long-passage",
            RagDataType::Chapter.as_str(),
            3,
            long_content,
            0.7,
        ),
    ];

    let text = format_generation_rag_context(&results, 6).expect("formatted context");

    assert!(text.chars().count() <= GENERATION_RAG_CONTEXT_CHAR_LIMIT + 3);
    assert!(text.contains("### 关键连续性"));
    assert!(text.contains("### 相关摘要"));
    assert!(text.contains("### 参考片段"));
}
