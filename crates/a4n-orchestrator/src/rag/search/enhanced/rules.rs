use a4n_core::{error::AppError, rag::RagDataType};
use a4n_embedding::provider::EmbeddingModelIdentity;

pub const MAX_RAG_TOP_K: usize = 50;
const ENHANCED_RAG_PULL_MULTIPLIER: usize = 3;
const ENHANCED_RAG_MIN_PULL_K: usize = 20;

pub fn validate_embedding_identity_match(
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

pub fn validate_query_vectors(
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

pub fn required_top_k(value: Option<usize>, command: &str) -> Result<usize, AppError> {
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

pub fn required_nonblank(value: &str, field: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::InvalidArgument(format!("{field} 不能为空")));
    }
    Ok(value.to_string())
}

pub fn normalize_search_data_types(
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

pub fn enhanced_rag_pull_k(top_k: usize) -> usize {
    (top_k * ENHANCED_RAG_PULL_MULTIPLIER).max(ENHANCED_RAG_MIN_PULL_K)
}
