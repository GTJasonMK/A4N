use a4n_domain::rag::RagDataType;
use a4n_embedding::provider::EmbeddingModelIdentity;
use serde_json::{Value, json};

use super::{contracts::PreparedGenerationRagSearch, query::GenerationRagQuery};

pub(crate) fn generation_rag_query_fingerprint(
    project_id: i64,
    chapter_number: i32,
    queries: &[GenerationRagQuery],
    embedding_identity: &EmbeddingModelIdentity,
) -> Result<String, String> {
    let weighted_queries = queries
        .iter()
        .map(|query| json!({"text": query.text.as_str(), "weight": query.weight}))
        .collect::<Vec<_>>();
    a4n_db::generation::generation_checkpoint_fingerprint_sha256(&json!({
        "version": 1,
        "project_id": project_id,
        "chapter_number": chapter_number,
        "queries": weighted_queries,
        "embedding_identity": embedding_identity,
    }))
    .map_err(|error| format!("构建章节生成 RAG query fingerprint 失败: {error}"))
}

pub(crate) fn validate_generation_rag_query_fingerprint(
    actual: &str,
    prepared: &PreparedGenerationRagSearch,
) -> Result<(), String> {
    let expected = generation_rag_query_fingerprint(
        prepared.project_id,
        prepared.chapter_number,
        &prepared.queries,
        &prepared.embedding_identity,
    )?;
    if actual == expected {
        return Ok(());
    }
    Err("章节生成 RAG query fingerprint 已过期或不匹配，请重新准备查询向量".to_string())
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

pub(super) fn previous_chapter_range(chapter_number: i32) -> Result<(u32, u32), String> {
    let previous_chapter = chapter_number
        .checked_sub(1)
        .ok_or_else(|| format!("章节号无法计算前序 RAG 范围：chapter_number={chapter_number}"))?;
    if previous_chapter < 1 {
        return Err(format!(
            "前序 RAG 检索仅适用于第 2 章及以后，实际章节号为 {chapter_number}"
        ));
    }
    let previous_chapter = u32::try_from(previous_chapter)
        .map_err(|_| format!("前序 RAG 章节号超出 u32 范围：{previous_chapter}"))?;
    Ok((1, previous_chapter))
}

pub(super) fn metadata_str<'a>(metadata: &'a Value, key: &str) -> Option<&'a str> {
    metadata
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
}
