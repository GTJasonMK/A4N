use std::collections::HashMap;

use a4n_domain::ProviderError as AppError;
use a4n_domain::rag::RagDataType;
use a4n_embedding::provider::EmbeddingModelIdentity;

use crate::vector_store::SearchFilter;

pub const MAX_RAG_TOP_K: usize = 50;
const ENHANCED_RAG_PULL_MULTIPLIER: usize = 3;
const ENHANCED_RAG_MIN_PULL_K: usize = 20;

pub fn required_top_k(top_k: usize) -> Result<usize, AppError> {
    if top_k == 0 {
        return Err(AppError::InvalidArgument(
            "rag_enhanced_search top_k 必须大于 0".to_string(),
        ));
    }
    if top_k > MAX_RAG_TOP_K {
        return Err(AppError::InvalidArgument(format!(
            "rag_enhanced_search top_k 不能超过 {MAX_RAG_TOP_K}"
        )));
    }
    Ok(top_k)
}

pub fn required_nonblank(value: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::InvalidArgument(
            "rag_enhanced_search query 不能为空".to_string(),
        ));
    }
    Ok(value.to_string())
}

pub fn normalize_search_data_types(
    value: Option<Vec<String>>,
) -> Result<Option<Vec<String>>, AppError> {
    let Some(values) = value else {
        return Ok(None);
    };
    if values.is_empty() {
        return Err(AppError::InvalidArgument(
            "RAG data_types 不能为空数组".to_string(),
        ));
    }
    let mut normalized = Vec::new();
    for value in values {
        if value.is_empty() || value != value.trim() {
            return Err(AppError::InvalidArgument(format!(
                "RAG 数据类型必须使用无空白的 canonical 名称，实际为: {value:?}"
            )));
        }
        let data_type = RagDataType::parse(&value).ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "不支持的 RAG 数据类型: {value}，支持的类型: {}",
                RagDataType::ingest_hint()
            ))
        })?;
        let text = data_type.as_str().to_string();
        if text != value {
            return Err(AppError::InvalidArgument(format!(
                "RAG 数据类型必须使用 canonical 名称 {text}，实际为 {value}"
            )));
        }
        if normalized.contains(&text) {
            return Err(AppError::InvalidArgument(format!(
                "RAG data_types 包含重复类型: {text}"
            )));
        }
        normalized.push(text);
    }
    Ok(Some(normalized))
}

pub fn identity_scoped_search_filter(
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

pub fn enhanced_rag_pull_k(top_k: usize) -> usize {
    (top_k * ENHANCED_RAG_PULL_MULTIPLIER).max(ENHANCED_RAG_MIN_PULL_K)
}
