mod formatting;
mod fusion;
mod query;

pub(super) use self::formatting::format_generation_rag_context;
pub(super) use self::fusion::fuse_generation_rag_results;
pub(super) use self::query::{GenerationRagQueries, build_generation_rag_queries};

use crate::AppState;
use a4n_core::models::{chapter::chapter_outline, character::foreshadowing_index};
use a4n_core::rag::RagDataType;
use a4n_engine::embedding::provider::EmbeddingModelIdentity;
use a4n_engine::rag::vector_store::{SearchFilter, SearchResult};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use serde::Serialize;
use serde_json::Value;

const GENERATION_RAG_PULL_K: usize = 24;

pub(crate) async fn build_generation_rag_context(
    state: &AppState,
    project_id: i64,
    chapter_number: i32,
    outline: Option<&chapter_outline::Model>,
    outline_text: &str,
    writing_instruction: Option<&str>,
) -> Result<GenerationRagContext, String> {
    let pending_foreshadowing = foreshadowing_index::Entity::find()
        .filter(foreshadowing_index::Column::ProjectId.eq(project_id))
        .filter(foreshadowing_index::Column::SetupChapter.lt(chapter_number))
        .filter(foreshadowing_index::Column::Status.ne("resolved"))
        .filter(foreshadowing_index::Column::Status.ne("abandoned"))
        .order_by_desc(foreshadowing_index::Column::Importance)
        .order_by_desc(foreshadowing_index::Column::SetupChapter)
        .limit(6)
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    let queries = build_generation_rag_queries(
        outline,
        outline_text,
        writing_instruction,
        &pending_foreshadowing,
    );
    if queries.is_empty() {
        return Ok(GenerationRagContext {
            text: None,
            statistics: GenerationRagStatistics::default(),
        });
    }

    let embedding_identity = crate::actions::rag_search::current_embedding_identity(state)
        .await
        .map_err(|err| err.to_string())?;
    let stored_metadata = crate::rag_ingest::project_rag_metadata(state, project_id).await?;
    crate::rag_ingest::ensure_project_embedding_identity_matches(
        &stored_metadata,
        &embedding_identity,
    )?;
    let mut filter = crate::actions::rag_search::identity_scoped_search_filter(
        project_id,
        embedding_identity,
        Some(generation_rag_search_data_types()),
    );
    filter.chapter_range = Some((1, (chapter_number - 1) as u32));
    let results =
        search_generation_rag_queries(state, project_id, chapter_number, &queries, filter).await?;

    let formatted = format_generation_rag_context(&results, project_id, chapter_number);
    let statistics = GenerationRagStatistics {
        chunk_count: formatted.chunk_count,
        summary_count: formatted.summary_count,
        continuity_count: formatted.continuity_count,
        context_length: context_text_length(formatted.text.as_deref()),
        query_main: Some(queries.main_query.clone()),
        query_characters: queries.character_queries.clone(),
        query_foreshadowing: queries.foreshadowing_queries.clone(),
        query_location: queries.location_query.clone(),
    };

    Ok(GenerationRagContext {
        text: formatted.text,
        statistics,
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct PreparedGenerationRagEmbeddingRequest {
    pub query_texts: Vec<String>,
    pub embedding_identity: EmbeddingModelIdentity,
}

pub(crate) async fn prepare_generation_rag_embedding_request(
    state: &AppState,
    project_id: i64,
    chapter_number: i32,
    outline: Option<&chapter_outline::Model>,
    outline_text: &str,
    writing_instruction: Option<&str>,
) -> Result<PreparedGenerationRagEmbeddingRequest, String> {
    let prepared = prepare_generation_rag_search(
        state,
        project_id,
        chapter_number,
        outline,
        outline_text,
        writing_instruction,
    )
    .await?;
    Ok(PreparedGenerationRagEmbeddingRequest {
        query_texts: prepared
            .queries
            .weighted_queries()
            .into_iter()
            .map(|query| query.text.to_string())
            .collect(),
        embedding_identity: prepared.embedding_identity,
    })
}

pub(crate) async fn build_generation_rag_context_with_embeddings(
    state: &AppState,
    project_id: i64,
    chapter_number: i32,
    outline: Option<&chapter_outline::Model>,
    outline_text: &str,
    writing_instruction: Option<&str>,
    embedding_identity: EmbeddingModelIdentity,
    query_vectors: Vec<Vec<f32>>,
) -> Result<GenerationRagContext, String> {
    let prepared = prepare_generation_rag_search(
        state,
        project_id,
        chapter_number,
        outline,
        outline_text,
        writing_instruction,
    )
    .await?;
    if embedding_identity != prepared.embedding_identity {
        return Err(format!(
            "Embedding identity 不匹配：服务器期望 {} / {} / {} 维，客户端提交 {} / {} / {} 维",
            prepared.embedding_identity.backend,
            prepared.embedding_identity.model,
            prepared.embedding_identity.dimension,
            embedding_identity.backend,
            embedding_identity.model,
            embedding_identity.dimension
        ));
    }
    let weighted_queries = prepared.queries.weighted_queries();
    validate_generation_query_vectors(
        &query_vectors,
        weighted_queries.len(),
        prepared.embedding_identity.dimension,
    )?;
    if weighted_queries.is_empty() {
        return Ok(GenerationRagContext {
            text: None,
            statistics: GenerationRagStatistics::default(),
        });
    }

    let mut batches = Vec::new();
    for (query, vector) in weighted_queries.into_iter().zip(query_vectors) {
        let results = state
            .rag
            .hybrid_search_with_embedding(
                project_id,
                query.text,
                vector,
                &prepared.embedding_identity,
                GENERATION_RAG_PULL_K,
                Some(prepared.filter.clone()),
            )
            .await
            .map_err(|e| e.to_string())?;
        batches.push((query.weight, results));
    }
    let results = fuse_generation_rag_results(batches, chapter_number);
    let formatted = format_generation_rag_context(&results, project_id, chapter_number);
    let statistics = GenerationRagStatistics {
        chunk_count: formatted.chunk_count,
        summary_count: formatted.summary_count,
        continuity_count: formatted.continuity_count,
        context_length: context_text_length(formatted.text.as_deref()),
        query_main: Some(prepared.queries.main_query.clone()),
        query_characters: prepared.queries.character_queries.clone(),
        query_foreshadowing: prepared.queries.foreshadowing_queries.clone(),
        query_location: prepared.queries.location_query.clone(),
    };
    Ok(GenerationRagContext {
        text: formatted.text,
        statistics,
    })
}

struct PreparedGenerationRagSearch {
    queries: GenerationRagQueries,
    filter: SearchFilter,
    embedding_identity: EmbeddingModelIdentity,
}

async fn prepare_generation_rag_search(
    state: &AppState,
    project_id: i64,
    chapter_number: i32,
    outline: Option<&chapter_outline::Model>,
    outline_text: &str,
    writing_instruction: Option<&str>,
) -> Result<PreparedGenerationRagSearch, String> {
    let pending_foreshadowing = foreshadowing_index::Entity::find()
        .filter(foreshadowing_index::Column::ProjectId.eq(project_id))
        .filter(foreshadowing_index::Column::SetupChapter.lt(chapter_number))
        .filter(foreshadowing_index::Column::Status.ne("resolved"))
        .filter(foreshadowing_index::Column::Status.ne("abandoned"))
        .order_by_desc(foreshadowing_index::Column::Importance)
        .order_by_desc(foreshadowing_index::Column::SetupChapter)
        .limit(6)
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    let queries = build_generation_rag_queries(
        outline,
        outline_text,
        writing_instruction,
        &pending_foreshadowing,
    );
    let embedding_identity = crate::actions::rag_search::current_embedding_identity(state)
        .await
        .map_err(|err| err.to_string())?;
    let stored_metadata = crate::rag_ingest::project_rag_metadata(state, project_id).await?;
    crate::rag_ingest::ensure_project_embedding_identity_matches(
        &stored_metadata,
        &embedding_identity,
    )?;
    let mut filter = crate::actions::rag_search::identity_scoped_search_filter(
        project_id,
        embedding_identity.clone(),
        Some(generation_rag_search_data_types()),
    );
    filter.chapter_range = Some((1, (chapter_number - 1) as u32));
    Ok(PreparedGenerationRagSearch {
        queries,
        filter,
        embedding_identity,
    })
}

async fn search_generation_rag_queries(
    state: &AppState,
    project_id: i64,
    chapter_number: i32,
    queries: &GenerationRagQueries,
    filter: SearchFilter,
) -> Result<Vec<SearchResult>, String> {
    let mut batches = Vec::new();
    for query in queries.weighted_queries() {
        let results = state
            .rag
            .hybrid_search_filtered(
                project_id,
                query.text,
                state.embedding.as_ref(),
                GENERATION_RAG_PULL_K,
                Some(filter.clone()),
            )
            .await
            .map_err(|e| e.to_string())?;
        batches.push((query.weight, results));
    }
    Ok(fuse_generation_rag_results(batches, chapter_number))
}

fn validate_generation_query_vectors(
    vectors: &[Vec<f32>],
    expected_count: usize,
    expected_dimension: usize,
) -> Result<(), String> {
    if vectors.len() != expected_count {
        return Err(format!(
            "章节生成 RAG query vector 数量不匹配：expected={expected_count}, actual={}",
            vectors.len()
        ));
    }
    for (index, vector) in vectors.iter().enumerate() {
        if vector.len() != expected_dimension {
            return Err(format!(
                "章节生成 RAG query vector 维度不匹配：index={index}, expected={expected_dimension}, actual={}",
                vector.len()
            ));
        }
        if vector.iter().any(|value| !value.is_finite()) {
            return Err(format!(
                "章节生成 RAG query vector 包含非有限数：index={index}"
            ));
        }
    }
    Ok(())
}

fn context_text_length(text: Option<&str>) -> usize {
    match text {
        Some(text) => text.chars().count(),
        None => 0,
    }
}

pub(super) fn generation_rag_search_data_types() -> Vec<String> {
    [
        RagDataType::ChapterSummary,
        RagDataType::Chapter,
        RagDataType::ChapterMetadata,
        RagDataType::KeyEvent,
        RagDataType::CharacterState,
        RagDataType::CharacterAttributeEvent,
    ]
    .iter()
    .map(|data_type| data_type.as_str().to_string())
    .collect()
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct GenerationRagContext {
    pub text: Option<String>,
    pub statistics: GenerationRagStatistics,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct GenerationRagStatistics {
    pub chunk_count: usize,
    pub summary_count: usize,
    pub continuity_count: usize,
    pub context_length: usize,
    pub query_main: Option<String>,
    pub query_characters: Vec<String>,
    pub query_foreshadowing: Vec<String>,
    pub query_location: Option<String>,
}

pub(super) fn metadata_i64(metadata: &Value, key: &str) -> Option<i64> {
    let value = metadata.get(key)?;
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|value| i64::try_from(value).ok()))
}

fn metadata_str<'a>(metadata: &'a Value, key: &str) -> Option<&'a str> {
    metadata
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
}

#[cfg(test)]
mod tests {
    use super::formatting::{
        GENERATION_RAG_CONTEXT_CHAR_LIMIT, GENERATION_RAG_MAX_PASSAGE_RESULTS,
        format_generation_rag_context, format_generation_rag_result,
    };
    use super::fusion::fuse_generation_rag_results;
    use super::query::{build_generation_rag_queries, extract_generation_location};
    use super::*;
    use a4n_engine::rag::vector_store::SearchResult;
    use serde_json::json;

    fn outline_with_characters() -> chapter_outline::Model {
        let now = chrono::Utc::now().naive_utc();
        chapter_outline::Model {
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

    fn foreshadowing(description: &str) -> foreshadowing_index::Model {
        let now = chrono::Utc::now().naive_utc();
        foreshadowing_index::Model {
            id: 1,
            project_id: 1,
            description: description.to_string(),
            setup_chapter: 2,
            payoff_chapter: None,
            status: "pending".to_string(),
            related_characters: None,
            importance: 4,
            created_at: now,
            updated_at: now,
        }
    }

    fn rag_result(metadata: Value) -> SearchResult {
        SearchResult {
            id: "chapter-summary-1".to_string(),
            content: "主角发现密室线索。".to_string(),
            score: 0.42,
            vector_score: None,
            bm25_score: None,
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
            vector_score: Some(score),
            bm25_score: None,
            metadata: json!({
                "project_id": 1,
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

        assert!(queries.main_query.contains("章节标题: 玄武暗线"));
        assert!(queries.main_query.contains("写作要点: 强调悬疑感"));
        assert!(!queries.main_query.contains("待处理伏笔"));
        assert!(!queries.main_query.contains("旧钟在午夜倒走"));
        assert_eq!(queries.character_queries.len(), 2);
        assert!(queries.character_queries[0].contains("角色 林岚"));
        assert_eq!(queries.foreshadowing_queries, vec!["伏笔: 旧钟在午夜倒走"]);
        assert_eq!(
            queries.location_query.as_deref(),
            Some("场景 玄武门 中发生的事件和相关设定")
        );
        assert!(queries.weighted_queries().len() >= 4);
    }

    #[test]
    fn generation_rag_search_excludes_authoritative_foreshadowing_index_records() {
        let data_types = generation_rag_search_data_types();

        assert!(!data_types.contains(&RagDataType::Foreshadowing.as_str().to_string()));
        assert!(data_types.contains(&RagDataType::Chapter.as_str().to_string()));
        assert!(data_types.contains(&RagDataType::KeyEvent.as_str().to_string()));
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
            extract_generation_location("众人在显德殿中等待消息").as_deref(),
            Some("显德殿")
        );
        assert_eq!(
            extract_generation_location("陈恪前往玄武门确认伏兵").as_deref(),
            Some("玄武门")
        );
        assert!(extract_generation_location("他在沉默之后做出选择").is_none());
    }

    #[test]
    fn format_generation_rag_result_rejects_missing_title() {
        let result = rag_result(json!({
            "project_id": 1,
            "data_type": "chapter_summary",
            "chapter_number": 1
        }));

        assert!(format_generation_rag_result(&result, 1, 2).is_none());
    }

    #[test]
    fn format_generation_rag_result_uses_trimmed_title() {
        let result = rag_result(json!({
            "project_id": 1,
            "data_type": "chapter_summary",
            "chapter_number": 1,
            "title": "  灰烬低语  "
        }));

        let formatted = format_generation_rag_result(&result, 1, 2).expect("formatted result");

        assert!(formatted.is_summary);
        assert!(formatted.line.contains("第1章《灰烬低语》"));
        assert!(!formatted.line.contains("无标题"));
    }

    #[test]
    fn format_generation_rag_result_includes_character_attribute_event() {
        let result = rag_result(json!({
            "project_id": 1,
            "data_type": "character_attribute_event",
            "chapter_number": 3,
            "title": "林舟 第3章属性变更"
        }));

        let formatted = format_generation_rag_result(&result, 1, 5).expect("formatted result");

        assert!(!formatted.is_summary);
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
                "foreshadowing",
                RagDataType::Foreshadowing.as_str(),
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

        let formatted = format_generation_rag_context(&results, 1, 6);
        let text = formatted.text.expect("formatted context");

        assert_eq!(formatted.continuity_count, 2);
        assert_eq!(formatted.summary_count, 1);
        assert_eq!(formatted.chunk_count, 1);
        let continuity_pos = text.find("### 关键连续性").expect("continuity section");
        let summary_pos = text.find("### 相关摘要").expect("summary section");
        let passage_pos = text.find("### 参考片段").expect("passage section");
        assert!(continuity_pos < summary_pos);
        assert!(summary_pos < passage_pos);
        assert!(text.contains("[伏笔"));
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
            "late-foreshadowing",
            RagDataType::Foreshadowing.as_str(),
            1,
            "必须保留的迟到伏笔",
            0.1,
        ));

        let formatted = format_generation_rag_context(&results, 1, 6);
        let text = formatted.text.expect("formatted context");

        assert_eq!(formatted.continuity_count, 1);
        assert_eq!(formatted.chunk_count, GENERATION_RAG_MAX_PASSAGE_RESULTS);
        assert!(text.contains("必须保留的迟到伏笔"));
        assert!(text.contains("### 关键连续性"));
    }

    #[test]
    fn format_generation_rag_context_respects_total_char_limit() {
        let long_content = "长内容".repeat(2_000);
        let results = vec![
            search_result_with_type(
                "long-continuity",
                RagDataType::Foreshadowing.as_str(),
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

        let formatted = format_generation_rag_context(&results, 1, 6);
        let text = formatted.text.expect("formatted context");

        assert!(text.chars().count() <= GENERATION_RAG_CONTEXT_CHAR_LIMIT + 3);
        assert!(text.contains("### 关键连续性"));
        assert!(text.contains("### 相关摘要"));
        assert!(text.contains("### 参考片段"));
    }
}
