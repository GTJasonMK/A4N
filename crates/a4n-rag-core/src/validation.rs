use a4n_domain::ProviderError as AppError;
use a4n_embedding::provider::EmbeddingModelIdentity;

pub fn required_chunk_id(chunk_id: &str) -> Result<String, AppError> {
    let chunk_id = chunk_id.trim();
    if chunk_id.is_empty() {
        return Err(AppError::InvalidArgument(
            "RAG chunk_id 不能为空".to_string(),
        ));
    }
    Ok(chunk_id.to_string())
}

pub fn required_content(text: &str) -> Result<String, AppError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(AppError::InvalidArgument(
            "RAG content 不能为空".to_string(),
        ));
    }
    Ok(text.to_string())
}

pub fn ensure_embedding_write_count(expected: usize, actual: usize) -> Result<(), AppError> {
    if actual == expected {
        Ok(())
    } else {
        Err(AppError::VectorStore(format!(
            "向量存储写入数量不匹配：expected={expected}, actual={actual}"
        )))
    }
}

pub fn validate_embedding_batch<T: AsRef<[f32]>>(
    embeddings: &[T],
    expected_count: usize,
    expected_dimension: usize,
) -> Result<(), AppError> {
    if embeddings.len() != expected_count {
        return Err(AppError::Embedding(format!(
            "嵌入返回数量不匹配：expected={expected_count}, actual={}",
            embeddings.len()
        )));
    }
    if embeddings
        .iter()
        .any(|embedding| embedding.as_ref().is_empty())
    {
        return Err(AppError::Embedding("嵌入返回空向量".to_string()));
    }
    for (index, embedding) in embeddings
        .iter()
        .map(|embedding| embedding.as_ref())
        .enumerate()
    {
        if embedding.len() != expected_dimension {
            return Err(AppError::Embedding(format!(
                "嵌入返回维度不匹配：index={index}, expected={expected_dimension}, actual={}",
                embedding.len()
            )));
        }
        if embedding.iter().any(|value| !value.is_finite()) {
            return Err(AppError::Embedding(format!(
                "嵌入返回包含非有限数：index={index}"
            )));
        }
    }
    Ok(())
}

pub fn validate_embedding_identity_match(
    actual: &EmbeddingModelIdentity,
    expected: &EmbeddingModelIdentity,
) -> Result<(), AppError> {
    if actual == expected {
        return Ok(());
    }
    Err(AppError::InvalidArgument(format!(
        "Embedding identity 不匹配，服务器期望 {} / {} / {} 维，客户端提交 {} / {} / {} 维；请切回项目模型或重建 RAG",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedding_batch_validation_preserves_count_empty_dimension_and_finite_contracts() {
        let count_error = validate_embedding_batch(&[vec![1.0, 0.0]], 2, 2)
            .expect_err("mismatched count should reject");
        assert_eq!(
            count_error.to_string(),
            "嵌入计算失败：嵌入返回数量不匹配：expected=2, actual=1"
        );

        let empty_error =
            validate_embedding_batch(&[Vec::new()], 1, 2).expect_err("empty vector should reject");
        assert_eq!(empty_error.to_string(), "嵌入计算失败：嵌入返回空向量");

        let dimension_error = validate_embedding_batch(&[vec![1.0]], 1, 2)
            .expect_err("wrong dimension should reject");
        assert_eq!(
            dimension_error.to_string(),
            "嵌入计算失败：嵌入返回维度不匹配：index=0, expected=2, actual=1"
        );

        let finite_error = validate_embedding_batch(&[vec![f32::NAN, 0.0]], 1, 2)
            .expect_err("non-finite vector should reject");
        assert_eq!(
            finite_error.to_string(),
            "嵌入计算失败：嵌入返回包含非有限数：index=0"
        );

        validate_embedding_batch(&[vec![1.0, 0.0]], 1, 2)
            .expect("matching embedding batch should pass");
    }

    #[test]
    fn input_and_write_count_contracts_preserve_original_errors() {
        assert_eq!(
            required_chunk_id("  chunk:1  ").expect("chunk id"),
            "chunk:1"
        );
        assert_eq!(required_content("  正文  ").expect("content"), "正文");
        assert_eq!(
            required_chunk_id(" ")
                .expect_err("blank chunk id should reject")
                .to_string(),
            "参数无效：RAG chunk_id 不能为空"
        );
        assert_eq!(
            required_content(" ")
                .expect_err("blank content should reject")
                .to_string(),
            "参数无效：RAG content 不能为空"
        );
        assert_eq!(
            ensure_embedding_write_count(2, 1)
                .expect_err("short write should reject")
                .to_string(),
            "向量检索失败：向量存储写入数量不匹配：expected=2, actual=1"
        );
    }
}
