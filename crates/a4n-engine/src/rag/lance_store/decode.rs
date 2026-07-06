use std::collections::HashMap;

use a4n_core::error::AppError;
use arrow_array::{Array, RecordBatch, StringArray};

use crate::rag::vector_store::SearchResult;

/// 从 RecordBatch 还原 SearchResult 列表（按 _distance 升序，分数 = 1/(1+distance)）。
pub(super) fn batches_to_results(batches: Vec<RecordBatch>) -> Result<Vec<SearchResult>, AppError> {
    let mut results = Vec::new();
    for batch in batches {
        let n = batch.num_rows();
        let id_col = required_string_column(&batch, "id")?;
        let content_col = required_string_column(&batch, "content")?;
        let metadata_col = required_string_column(&batch, "metadata")?;
        let distance_col = required_f32_column(&batch, "_distance")?;

        for i in 0..n {
            let id = required_row_string(&id_col, i, "id")?;
            if id.is_empty() {
                return Err(AppError::VectorStoreError("查询结果包含空 id".to_string()));
            }
            let content = required_row_string(&content_col, i, "content")?;
            let metadata_raw = required_row_string(&metadata_col, i, "metadata")?;
            let metadata: serde_json::Value = serde_json::from_str(metadata_raw)
                .map_err(|e| AppError::VectorStoreError(format!("metadata 解析失败: {e}")))?;
            let distance = distance_col
                .value_at(i)
                .ok_or_else(|| AppError::VectorStoreError("查询结果缺少 _distance".to_string()))?;
            let score = 1.0 / (1.0 + distance.max(0.0));

            results.push(SearchResult {
                id: id.to_string(),
                content: content.to_string(),
                score,
                vector_score: Some(score),
                bm25_score: None,
                metadata,
            });
        }
    }
    Ok(results)
}

pub(super) fn batches_to_metadata(
    batches: Vec<RecordBatch>,
) -> Result<HashMap<String, serde_json::Value>, AppError> {
    let mut metadata = HashMap::new();
    for batch in batches {
        let n = batch.num_rows();
        let id_col = required_string_column(&batch, "id")?;
        let metadata_col = required_string_column(&batch, "metadata")?;

        for i in 0..n {
            let id = required_row_string(&id_col, i, "id")?;
            if id.is_empty() {
                return Err(AppError::VectorStoreError(
                    "metadata 查询结果包含空 id".to_string(),
                ));
            }
            let metadata_raw = required_row_string(&metadata_col, i, "metadata")?;
            let parsed: serde_json::Value = serde_json::from_str(metadata_raw)
                .map_err(|e| AppError::VectorStoreError(format!("metadata 解析失败: {e}")))?;
            metadata.insert(id.to_string(), parsed);
        }
    }
    Ok(metadata)
}

/// 把列读成 StringArray 的辅助 view。
struct StrView<'a> {
    arr: &'a StringArray,
}

impl<'a> StrView<'a> {
    fn value_at(&self, i: usize) -> Option<&str> {
        if self.arr.is_null(i) {
            None
        } else {
            Some(self.arr.value(i))
        }
    }
}

fn column_as_string<'a>(batch: &'a RecordBatch, name: &str) -> Option<StrView<'a>> {
    batch
        .column_by_name(name)
        .and_then(|c| c.as_any().downcast_ref::<StringArray>())
        .map(|arr| StrView { arr })
}

fn required_string_column<'a>(batch: &'a RecordBatch, name: &str) -> Result<StrView<'a>, AppError> {
    column_as_string(batch, name)
        .ok_or_else(|| AppError::VectorStoreError(format!("查询结果缺少 {name} 列")))
}

fn required_row_string<'a>(
    column: &'a StrView<'a>,
    row: usize,
    name: &str,
) -> Result<&'a str, AppError> {
    column
        .value_at(row)
        .ok_or_else(|| AppError::VectorStoreError(format!("查询结果第 {row} 行缺少 {name}")))
}

struct F32View<'a> {
    arr: &'a arrow_array::Float32Array,
}

impl<'a> F32View<'a> {
    fn value_at(&self, i: usize) -> Option<f32> {
        if self.arr.is_null(i) {
            None
        } else {
            Some(self.arr.value(i))
        }
    }
}

fn column_as_f32<'a>(batch: &'a RecordBatch, name: &str) -> Option<F32View<'a>> {
    batch
        .column_by_name(name)
        .and_then(|c| c.as_any().downcast_ref::<arrow_array::Float32Array>())
        .map(|arr| F32View { arr })
}

fn required_f32_column<'a>(batch: &'a RecordBatch, name: &str) -> Result<F32View<'a>, AppError> {
    column_as_f32(batch, name)
        .ok_or_else(|| AppError::VectorStoreError(format!("查询结果缺少 {name} 列")))
}
