use std::sync::Arc;

use a4n_core::error::AppError;
use arrow_array::{
    FixedSizeListArray, Int32Array, Int64Array, RecordBatch, StringArray, types::Float32Type,
};
use arrow_schema::{DataType, Field, Schema, SchemaRef};

use crate::rag::vector_store::EmbeddingRecord;

/// 构造统一的 schema。
pub(super) fn build_schema(dim: usize) -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new(
            "vector",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                dim as i32,
            ),
            true,
        ),
        Field::new("content", DataType::Utf8, true),
        Field::new("metadata", DataType::Utf8, true),
        Field::new("project_id", DataType::Int64, true),
        Field::new("data_type", DataType::Utf8, true),
        Field::new("chapter_number", DataType::Int32, true),
    ]))
}

/// 把 EmbeddingRecord 列表打包成 RecordBatch。
pub(super) fn records_to_batch(
    schema: SchemaRef,
    dim: usize,
    records: &[EmbeddingRecord],
) -> Result<RecordBatch, AppError> {
    let ids: Vec<&str> = records.iter().map(|r| r.id.as_str()).collect();
    let id_array = StringArray::from(ids);

    for record in records {
        let actual = validate_embedding_vector("batch", &record.id, &record.embedding)?;
        if actual != dim {
            return Err(AppError::VectorStoreError(format!(
                "嵌入向量维度不一致：id={}, expected_dimension={dim}, actual_dimension={actual}",
                record.id
            )));
        }
    }
    let vector_iter = records
        .iter()
        .map(|r| Some(r.embedding.iter().map(|v| Some(*v)).collect::<Vec<_>>()));
    let vector_array =
        FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(vector_iter, dim as i32);

    let contents: Vec<Option<&str>> = records.iter().map(|r| r.content.as_deref()).collect();
    let content_array = StringArray::from(contents);

    let metas: Vec<Option<String>> = records
        .iter()
        .map(|r| {
            if r.metadata.is_null() {
                None
            } else {
                Some(r.metadata.to_string())
            }
        })
        .collect();
    let metadata_array = StringArray::from_iter(metas.iter().map(|m| m.as_deref()));

    let project_ids: Vec<Option<i64>> = records
        .iter()
        .map(|r| r.metadata.get("project_id").and_then(|v| v.as_i64()))
        .collect();
    let project_id_array = Int64Array::from(project_ids);

    let data_types: Vec<Option<String>> = records
        .iter()
        .map(|r| {
            r.metadata
                .get("data_type")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .collect();
    let data_type_array = StringArray::from_iter(data_types.iter().map(|d| d.as_deref()));

    let chapter_numbers: Vec<Option<i32>> = records
        .iter()
        .map(|r| {
            r.metadata
                .get("chapter_number")
                .and_then(|v| v.as_i64())
                .map(|v| v as i32)
        })
        .collect();
    let chapter_array = Int32Array::from(chapter_numbers);

    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(id_array),
            Arc::new(vector_array),
            Arc::new(content_array),
            Arc::new(metadata_array),
            Arc::new(project_id_array),
            Arc::new(data_type_array),
            Arc::new(chapter_array),
        ],
    )
    .map_err(|e| AppError::VectorStoreError(format!("RecordBatch 构造失败: {e}")))
}

pub(super) fn validate_embedding_vector(
    collection: &str,
    id: &str,
    embedding: &[f32],
) -> Result<usize, AppError> {
    if embedding.is_empty() {
        return Err(AppError::VectorStoreError(format!(
            "嵌入向量不能为空：collection={collection}, id={id}"
        )));
    }
    if embedding.iter().any(|value| !value.is_finite()) {
        return Err(AppError::VectorStoreError(format!(
            "嵌入向量包含非有限数：collection={collection}, id={id}"
        )));
    }
    Ok(embedding.len())
}

/// 从 schema 中提取 vector 字段的维度。
pub(super) fn dim_of_vector_field(schema: &Schema) -> Option<usize> {
    schema
        .field_with_name("vector")
        .ok()
        .and_then(|f| match f.data_type() {
            DataType::FixedSizeList(_, d) => Some(*d as usize),
            _ => None,
        })
}
