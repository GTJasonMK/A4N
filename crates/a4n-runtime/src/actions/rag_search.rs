use crate::{
    AppState,
    actions::{
        access,
        rag_fusion::{ensure_rag_scores_finite, fuse_weighted_rag_results},
    },
};
use a4n_core::error::AppError;
use a4n_core::models::character::foreshadowing_index;
use a4n_core::models::project::blueprint_character;
use a4n_core::rag::RagDataType;
use a4n_engine::embedding::provider::EmbeddingModelIdentity;
use a4n_engine::rag::vector_store::{SearchFilter, SearchResult};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const ENHANCED_RAG_PULL_MULTIPLIER: usize = 3;
const ENHANCED_RAG_MIN_PULL_K: usize = 20;
const ENHANCED_RAG_MAX_QUERIES: usize = 8;
const MAX_RAG_TOP_K: usize = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagSearchResult {
    pub id: String,
    pub content: String,
    pub score: f32,
    pub vector_score: Option<f32>,
    pub bm25_score: Option<f32>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EnhancedSearchInput {
    pub query: String,
    pub top_k: Option<usize>,
    pub chapter_number: Option<i32>,
    pub data_types: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagEnhancedQuerySummary {
    pub kind: String,
    pub query: String,
    pub weight: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagEnhancedSearchOutput {
    pub results: Vec<RagSearchResult>,
    pub queries: Vec<RagEnhancedQuerySummary>,
    pub candidate_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagPreparedEnhancedSearchOutput {
    pub queries: Vec<RagEnhancedQuerySummary>,
    pub query_texts: Vec<String>,
    pub embedding_identity: EmbeddingModelIdentity,
    pub top_k: usize,
    pub pull_k: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RagEnhancedSearchWithVectorsInput {
    pub search: EnhancedSearchInput,
    pub embedding_identity: EmbeddingModelIdentity,
    pub query_vectors: Vec<Vec<f32>>,
}

impl From<SearchResult> for RagSearchResult {
    fn from(result: SearchResult) -> Self {
        Self {
            id: result.id,
            content: result.content,
            score: result.score,
            vector_score: result.vector_score,
            bm25_score: result.bm25_score,
            metadata: result.metadata,
        }
    }
}

#[derive(Debug, Clone)]
struct EnhancedRagQuery {
    kind: &'static str,
    query: String,
    weight: f32,
}

impl EnhancedRagQuery {
    fn summary(&self) -> RagEnhancedQuerySummary {
        RagEnhancedQuerySummary {
            kind: self.kind.to_string(),
            query: self.query.clone(),
            weight: self.weight,
        }
    }
}

pub async fn rag_enhanced_search(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: EnhancedSearchInput,
) -> Result<RagEnhancedSearchOutput, AppError> {
    access::verify_project_editor(state, user_id, project_id).await?;
    let query = required_nonblank(&input.query, "rag_enhanced_search query")?;
    let top_k = required_top_k(input.top_k, "rag_enhanced_search")?;
    let data_types = normalize_search_data_types(input.data_types)?;
    enhanced_rag_search_inner(
        state,
        project_id,
        &query,
        top_k,
        input.chapter_number,
        data_types,
    )
    .await
}

pub async fn rag_prepare_enhanced_search(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: EnhancedSearchInput,
) -> Result<RagPreparedEnhancedSearchOutput, AppError> {
    access::verify_project_editor(state, user_id, project_id).await?;
    let prepared = prepare_enhanced_rag_search(state, project_id, input).await?;
    Ok(RagPreparedEnhancedSearchOutput {
        queries: prepared
            .queries
            .iter()
            .map(EnhancedRagQuery::summary)
            .collect(),
        query_texts: prepared
            .queries
            .iter()
            .map(|query| query.query.clone())
            .collect(),
        embedding_identity: prepared.embedding_identity,
        top_k: prepared.top_k,
        pull_k: prepared.pull_k,
    })
}

pub async fn rag_enhanced_search_with_vectors(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    input: RagEnhancedSearchWithVectorsInput,
) -> Result<RagEnhancedSearchOutput, AppError> {
    access::verify_project_editor(state, user_id, project_id).await?;
    let prepared = prepare_enhanced_rag_search(state, project_id, input.search).await?;
    validate_embedding_identity_match(&input.embedding_identity, &prepared.embedding_identity)?;
    validate_query_vectors(
        &input.query_vectors,
        prepared.queries.len(),
        input.embedding_identity.dimension,
    )?;
    if prepared.queries.is_empty() {
        return Ok(RagEnhancedSearchOutput {
            results: Vec::new(),
            queries: Vec::new(),
            candidate_count: 0,
        });
    }

    let mut batches = Vec::new();
    for (query, vector) in prepared.queries.iter().zip(input.query_vectors) {
        let results = state
            .rag
            .hybrid_search_with_embedding(
                project_id,
                &query.query,
                vector,
                &prepared.embedding_identity,
                prepared.pull_k,
                Some(prepared.filter.clone()),
            )
            .await?;
        batches.push((query.weight, results));
    }
    let candidate_count = batches.iter().map(|(_, results)| results.len()).sum();
    let mut results = fuse_enhanced_rag_results(batches, prepared.chapter_number)?
        .into_iter()
        .take(prepared.top_k)
        .map(RagSearchResult::from)
        .collect::<Vec<_>>();
    sort_rag_results_by_score_desc(&mut results)?;

    Ok(RagEnhancedSearchOutput {
        results,
        queries: prepared
            .queries
            .iter()
            .map(EnhancedRagQuery::summary)
            .collect(),
        candidate_count,
    })
}

async fn enhanced_rag_search_inner(
    state: &AppState,
    project_id: i64,
    base_query: &str,
    top_k: usize,
    chapter_number: Option<i32>,
    data_types: Option<Vec<String>>,
) -> Result<RagEnhancedSearchOutput, AppError> {
    let prepared = prepare_enhanced_rag_search_parts(
        state,
        project_id,
        base_query,
        top_k,
        chapter_number,
        data_types,
    )
    .await?;
    let queries = prepared.queries;
    if queries.is_empty() {
        return Ok(RagEnhancedSearchOutput {
            results: Vec::new(),
            queries: Vec::new(),
            candidate_count: 0,
        });
    }

    let mut batches = Vec::new();
    for query in &queries {
        let results = state
            .rag
            .hybrid_search_filtered(
                project_id,
                &query.query,
                state.embedding.as_ref(),
                prepared.pull_k,
                Some(prepared.filter.clone()),
            )
            .await?;
        batches.push((query.weight, results));
    }
    let candidate_count = batches.iter().map(|(_, results)| results.len()).sum();
    let mut results = fuse_enhanced_rag_results(batches, prepared.chapter_number)?
        .into_iter()
        .take(prepared.top_k)
        .map(RagSearchResult::from)
        .collect::<Vec<_>>();
    sort_rag_results_by_score_desc(&mut results)?;

    Ok(RagEnhancedSearchOutput {
        results,
        queries: queries.iter().map(EnhancedRagQuery::summary).collect(),
        candidate_count,
    })
}

struct PreparedEnhancedRagSearch {
    queries: Vec<EnhancedRagQuery>,
    filter: SearchFilter,
    top_k: usize,
    pull_k: usize,
    chapter_number: Option<i32>,
    embedding_identity: EmbeddingModelIdentity,
}

async fn prepare_enhanced_rag_search(
    state: &AppState,
    project_id: i64,
    input: EnhancedSearchInput,
) -> Result<PreparedEnhancedRagSearch, AppError> {
    let query = required_nonblank(&input.query, "rag_enhanced_search query")?;
    let top_k = required_top_k(input.top_k, "rag_enhanced_search")?;
    let data_types = normalize_search_data_types(input.data_types)?;
    prepare_enhanced_rag_search_parts(
        state,
        project_id,
        &query,
        top_k,
        input.chapter_number,
        data_types,
    )
    .await
}

async fn prepare_enhanced_rag_search_parts(
    state: &AppState,
    project_id: i64,
    base_query: &str,
    top_k: usize,
    chapter_number: Option<i32>,
    data_types: Option<Vec<String>>,
) -> Result<PreparedEnhancedRagSearch, AppError> {
    let character_names = matched_blueprint_character_names(state, project_id, base_query).await?;
    let foreshadowing = filter_relevant_foreshadowing(
        candidate_foreshadowing(state, project_id, chapter_number).await?,
        base_query,
        &character_names,
    );
    let queries = build_enhanced_rag_queries(base_query, &character_names, &foreshadowing);
    let embedding_identity = current_embedding_identity(state).await?;
    let stored_metadata = crate::rag_ingest::project_rag_metadata(state, project_id)
        .await
        .map_err(AppError::InvalidArgument)?;
    crate::rag_ingest::ensure_project_embedding_identity_matches(
        &stored_metadata,
        &embedding_identity,
    )
    .map_err(AppError::InvalidArgument)?;
    let filter = identity_scoped_search_filter(project_id, embedding_identity.clone(), data_types);
    Ok(PreparedEnhancedRagSearch {
        queries,
        filter,
        top_k,
        pull_k: (top_k * ENHANCED_RAG_PULL_MULTIPLIER).max(ENHANCED_RAG_MIN_PULL_K),
        chapter_number,
        embedding_identity,
    })
}

pub(crate) async fn current_embedding_identity(
    state: &AppState,
) -> Result<EmbeddingModelIdentity, AppError> {
    if let Some(model) = crate::runtime_config::current_embedding_model() {
        return crate::dynamic_embedding::model_identity_from_config(&model);
    }
    state.embedding.model_identity().await
}

pub(crate) fn identity_scoped_search_filter(
    project_id: i64,
    embedding_identity: EmbeddingModelIdentity,
    data_types: Option<Vec<String>>,
) -> SearchFilter {
    let mut equals = HashMap::new();
    equals.insert("project_id".to_string(), serde_json::json!(project_id));
    equals.insert("root_project_id".to_string(), serde_json::json!(project_id));
    equals.insert("source_snapshot_id".to_string(), serde_json::Value::Null);
    equals.insert("derived_project_id".to_string(), serde_json::Value::Null);
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

fn validate_embedding_identity_match(
    actual: &EmbeddingModelIdentity,
    expected: &EmbeddingModelIdentity,
) -> Result<(), AppError> {
    if actual == expected {
        return Ok(());
    }
    Err(AppError::InvalidArgument(format!(
        "Embedding identity 不匹配，项目期望 {} / {} / {} 维，客户端提交 {} / {} / {} 维；请切回项目模型或重建 RAG",
        expected.backend,
        expected.model,
        expected.dimension,
        actual.backend,
        actual.model,
        actual.dimension
    )))
}

fn validate_query_vectors(
    vectors: &[Vec<f32>],
    expected_count: usize,
    expected_dimension: usize,
) -> Result<(), AppError> {
    if vectors.len() != expected_count {
        return Err(AppError::InvalidArgument(format!(
            "RAG query vector 数量不匹配：expected={expected_count}, actual={}",
            vectors.len()
        )));
    }
    for (index, vector) in vectors.iter().enumerate() {
        if vector.len() != expected_dimension {
            return Err(AppError::InvalidArgument(format!(
                "RAG query vector 维度不匹配：index={index}, expected={expected_dimension}, actual={}",
                vector.len()
            )));
        }
        if vector.iter().any(|value| !value.is_finite()) {
            return Err(AppError::InvalidArgument(format!(
                "RAG query vector 包含非有限数：index={index}"
            )));
        }
    }
    Ok(())
}

async fn matched_blueprint_character_names(
    state: &AppState,
    project_id: i64,
    query: &str,
) -> Result<Vec<String>, AppError> {
    let characters = blueprint_character::Entity::find()
        .filter(blueprint_character::Column::ProjectId.eq(project_id))
        .order_by_asc(blueprint_character::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(AppError::Database)?;
    let mut names = characters
        .into_iter()
        .map(|character| character.name.trim().to_string())
        .filter(|name| !name.is_empty() && query.contains(name))
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    names.truncate(4);
    Ok(names)
}

async fn candidate_foreshadowing(
    state: &AppState,
    project_id: i64,
    chapter_number: Option<i32>,
) -> Result<Vec<foreshadowing_index::Model>, AppError> {
    let mut query = foreshadowing_index::Entity::find()
        .filter(foreshadowing_index::Column::ProjectId.eq(project_id))
        .filter(foreshadowing_index::Column::Status.ne("resolved"))
        .filter(foreshadowing_index::Column::Status.ne("abandoned"))
        .order_by_desc(foreshadowing_index::Column::Importance)
        .order_by_desc(foreshadowing_index::Column::SetupChapter)
        .limit(12);
    if let Some(chapter_number) = chapter_number.filter(|value| *value > 1) {
        query = query.filter(foreshadowing_index::Column::SetupChapter.lt(chapter_number));
    }
    query
        .all(state.db.as_ref())
        .await
        .map_err(AppError::Database)
}

fn build_enhanced_rag_queries(
    base_query: &str,
    character_names: &[String],
    foreshadowing: &[foreshadowing_index::Model],
) -> Vec<EnhancedRagQuery> {
    let mut queries = Vec::new();
    push_enhanced_query(&mut queries, "main", base_query, 1.0);

    for name in character_names.iter().take(3) {
        push_enhanced_query(
            &mut queries,
            "character",
            format!("角色 {name} 的行动、状态、关系和立场变化"),
            0.85,
        );
    }
    for item in foreshadowing.iter().take(3) {
        push_enhanced_query(
            &mut queries,
            "foreshadowing",
            format!("伏笔: {}", item.description.trim()),
            0.95,
        );
    }
    if let Some(location) = extract_query_location(base_query) {
        push_enhanced_query(
            &mut queries,
            "location",
            format!("场景 {location} 中发生的事件和相关设定"),
            0.75,
        );
    }

    queries.truncate(ENHANCED_RAG_MAX_QUERIES);
    queries
}

fn push_enhanced_query(
    queries: &mut Vec<EnhancedRagQuery>,
    kind: &'static str,
    query: impl Into<String>,
    weight: f32,
) {
    let query = query.into().trim().to_string();
    if query.is_empty() || queries.iter().any(|item| item.query == query) {
        return;
    }
    queries.push(EnhancedRagQuery {
        kind,
        query,
        weight,
    });
}

fn filter_relevant_foreshadowing(
    items: Vec<foreshadowing_index::Model>,
    query: &str,
    character_names: &[String],
) -> Vec<foreshadowing_index::Model> {
    let mut selected = items
        .into_iter()
        .filter(|item| {
            item.importance >= 4
                || text_has_direct_overlap(query, &item.description)
                || item
                    .related_characters
                    .as_ref()
                    .is_some_and(|value| json_contains_any_character_name(value, character_names))
        })
        .collect::<Vec<_>>();
    selected.truncate(3);
    selected
}

fn text_has_direct_overlap(query: &str, text: &str) -> bool {
    let query = query.trim();
    let text = text.trim();
    query.chars().count() >= 2
        && text.chars().count() >= 2
        && (query.contains(text) || text.contains(query))
}

fn json_contains_any_character_name(value: &serde_json::Value, names: &[String]) -> bool {
    match value {
        serde_json::Value::String(text) => names.iter().any(|name| text.contains(name)),
        serde_json::Value::Array(items) => items
            .iter()
            .any(|item| json_contains_any_character_name(item, names)),
        serde_json::Value::Object(map) => map
            .values()
            .any(|item| json_contains_any_character_name(item, names)),
        _ => false,
    }
}

fn fuse_enhanced_rag_results(
    batches: Vec<(f32, Vec<SearchResult>)>,
    chapter_number: Option<i32>,
) -> Result<Vec<SearchResult>, AppError> {
    ensure_rag_scores_finite(&batches)?;
    Ok(fuse_weighted_rag_results(batches, |result| {
        enhanced_temporal_multiplier(result, chapter_number)
    }))
}

fn enhanced_temporal_multiplier(result: &SearchResult, chapter_number: Option<i32>) -> f32 {
    let Some(chapter_number) = chapter_number.filter(|value| *value > 0) else {
        return 1.0;
    };
    let Some(source_chapter) =
        metadata_i64(&result.metadata, "chapter_number").map(|source| source as i32)
    else {
        return 1.0;
    };
    let distance = (chapter_number - source_chapter).abs();
    if distance == 0 {
        return 1.3;
    }
    if distance <= 5 {
        1.0 + 0.25 * (1.0 - ((distance - 1) as f32 / 5.0))
    } else {
        1.0
    }
}

fn metadata_i64(metadata: &serde_json::Value, key: &str) -> Option<i64> {
    metadata.get(key).and_then(|value| {
        value
            .as_i64()
            .or_else(|| value.as_u64().and_then(|v| i64::try_from(v).ok()))
    })
}

fn extract_query_location(text: &str) -> Option<String> {
    for cue in ["来到", "抵达", "前往", "回到"] {
        if let Some(location) = extract_location_after_cue(text, cue, false) {
            return Some(location);
        }
    }
    extract_location_after_cue(text, "在", true)
}

fn extract_location_after_cue(text: &str, cue: &str, require_suffix: bool) -> Option<String> {
    let chars = text.chars().collect::<Vec<_>>();
    let cue_chars = cue.chars().collect::<Vec<_>>();
    if cue_chars.is_empty() || chars.len() <= cue_chars.len() {
        return None;
    }
    for start in 0..=chars.len().saturating_sub(cue_chars.len()) {
        if chars[start..start + cue_chars.len()] != cue_chars {
            continue;
        }
        let mut location = String::new();
        let mut has_suffix = false;
        for ch in chars.iter().skip(start + cue_chars.len()) {
            if location.chars().count() >= 10 || is_location_stop_char(*ch) {
                break;
            }
            if location.chars().count() >= 2 && is_location_action_start(*ch) {
                break;
            }
            if matches!(*ch, '中' | '里' | '内' | '上' | '下' | '旁' | '边') {
                has_suffix = true;
                break;
            }
            location.push(*ch);
        }
        let location = location.trim_matches(|ch: char| {
            ch.is_whitespace() || matches!(ch, '《' | '》' | '"' | '\'' | '“' | '”' | '「' | '」')
        });
        if (2..=10).contains(&location.chars().count()) && (!require_suffix || has_suffix) {
            return Some(location.to_string());
        }
    }
    None
}

fn is_location_stop_char(ch: char) -> bool {
    ch.is_whitespace()
        || matches!(
            ch,
            '，' | '。'
                | '、'
                | '；'
                | '：'
                | '！'
                | '？'
                | ','
                | '.'
                | ';'
                | ':'
                | '!'
                | '?'
                | '\n'
                | '\r'
        )
}

fn is_location_action_start(ch: char) -> bool {
    matches!(
        ch,
        '调' | '确' | '寻' | '会' | '发' | '阻' | '救' | '追' | '见' | '问' | '记' | '查'
    )
}

fn required_top_k(value: Option<usize>, command: &str) -> Result<usize, AppError> {
    let top_k = value.ok_or_else(|| AppError::InvalidArgument(format!("{command} 缺少 top_k")))?;
    if top_k == 0 {
        return Err(AppError::InvalidArgument(format!(
            "{command} top_k 必须大于 0"
        )));
    }
    if top_k > MAX_RAG_TOP_K {
        return Err(AppError::InvalidArgument(format!(
            "{command} top_k 不能超过 {MAX_RAG_TOP_K}"
        )));
    }
    Ok(top_k)
}

fn required_nonblank(value: &str, field: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::InvalidArgument(format!("{field} 不能为空")));
    }
    Ok(value.to_string())
}

fn sort_rag_results_by_score_desc(results: &mut [RagSearchResult]) -> Result<(), AppError> {
    for result in results.iter() {
        if !result.score.is_finite() {
            return Err(AppError::InvalidArgument(format!(
                "RAG 检索结果 score 必须是有限数: id={}",
                result.id
            )));
        }
    }
    results.sort_by(|a, b| b.score.total_cmp(&a.score));
    Ok(())
}

fn normalize_search_data_types(
    value: Option<Vec<String>>,
) -> Result<Option<Vec<String>>, AppError> {
    let Some(values) = value else {
        return Ok(None);
    };
    let mut normalized = Vec::new();
    for value in values {
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        let data_type = RagDataType::parse(value).ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "不支持的 RAG 数据类型: {value}，支持的类型: {}",
                RagDataType::ingest_hint()
            ))
        })?;
        let text = data_type.as_str().to_string();
        if !normalized.contains(&text) {
            normalized.push(text);
        }
    }
    Ok((!normalized.is_empty()).then_some(normalized))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn result_with_score(id: &str, score: f32) -> RagSearchResult {
        RagSearchResult {
            id: id.to_string(),
            content: String::new(),
            score,
            vector_score: None,
            bm25_score: None,
            metadata: json!({}),
        }
    }

    #[test]
    fn required_top_k_rejects_missing() {
        let err = required_top_k(None, "rag_search").expect_err("missing top_k should reject");

        assert!(err.to_string().contains("缺少 top_k"));
    }

    #[test]
    fn required_top_k_rejects_zero() {
        let err = required_top_k(Some(0), "rag_search").expect_err("zero top_k should reject");

        assert!(err.to_string().contains("必须大于 0"));
    }

    #[test]
    fn required_top_k_rejects_over_limit() {
        let err = required_top_k(Some(MAX_RAG_TOP_K + 1), "rag_search")
            .expect_err("oversized top_k should reject");

        assert!(err.to_string().contains("不能超过"));
    }

    #[test]
    fn required_top_k_accepts_valid_value() {
        assert_eq!(required_top_k(Some(10), "rag_search").unwrap(), 10);
    }

    #[test]
    fn required_nonblank_trims_text() {
        assert_eq!(required_nonblank("  星门  ", "query").unwrap(), "星门");
    }

    #[test]
    fn required_nonblank_rejects_blank() {
        let err = required_nonblank("  ", "query").expect_err("blank value should reject");

        assert!(err.to_string().contains("不能为空"));
    }

    #[test]
    fn sort_rag_results_rejects_nonfinite_score() {
        let mut results = vec![result_with_score("bad", f32::NAN)];
        let err =
            sort_rag_results_by_score_desc(&mut results).expect_err("NaN score should be explicit");

        assert!(err.to_string().contains("score 必须是有限数"));
    }

    #[test]
    fn sort_rag_results_orders_by_score_desc() {
        let mut results = vec![
            result_with_score("low", 0.1),
            result_with_score("high", 0.9),
            result_with_score("mid", 0.5),
        ];

        sort_rag_results_by_score_desc(&mut results).expect("finite scores sort");

        assert_eq!(
            results
                .iter()
                .map(|result| result.id.as_str())
                .collect::<Vec<_>>(),
            vec!["high", "mid", "low"]
        );
    }

    #[test]
    fn normalize_search_data_types_deduplicates_known_types() {
        assert_eq!(
            normalize_search_data_types(Some(vec![
                " chapter ".to_string(),
                "character".to_string(),
                "chapter".to_string(),
                "".to_string(),
            ]))
            .unwrap(),
            Some(vec!["chapter".to_string(), "character".to_string()])
        );
    }

    #[test]
    fn normalize_search_data_types_rejects_unknown_type() {
        let err = normalize_search_data_types(Some(vec!["unknown".to_string()]))
            .expect_err("unknown data type should fail");

        assert!(err.to_string().contains("不支持的 RAG 数据类型"));
    }
}
