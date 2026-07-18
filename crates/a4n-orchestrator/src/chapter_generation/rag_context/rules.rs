use a4n_core::rag::RagDataType;
use a4n_embedding::provider::EmbeddingModelIdentity;
use serde_json::Value;

pub fn validate_generation_embedding_identity_match(
    actual: &EmbeddingModelIdentity,
    expected: &EmbeddingModelIdentity,
) -> Result<(), String> {
    if actual == expected {
        return Ok(());
    }
    Err(format!(
        "Embedding identity 不匹配：服务器期望 {} / {} / {} 维，客户端提交 {} / {} / {} 维",
        expected.backend,
        expected.model,
        expected.dimension,
        actual.backend,
        actual.model,
        actual.dimension
    ))
}

pub fn validate_generation_query_vectors(
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

pub(super) fn context_text_length(text: Option<&str>) -> usize {
    match text {
        Some(text) => text.chars().count(),
        None => 0,
    }
}

pub fn generation_rag_search_data_types() -> Vec<String> {
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

pub fn previous_chapter_range(chapter_number: i32) -> Result<(u32, u32), String> {
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

pub(super) fn metadata_i64(metadata: &Value, key: &str) -> Option<i64> {
    let value = metadata.get(key)?;
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|value| i64::try_from(value).ok()))
}

pub(super) fn metadata_str<'a>(metadata: &'a Value, key: &str) -> Option<&'a str> {
    metadata
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
}
