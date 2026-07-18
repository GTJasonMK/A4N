use a4n_core::error::AppError;
use tantivy::TantivyDocument;
use tantivy::schema::{Field, Value};

pub(super) fn required_text_field<'a>(
    doc: &'a TantivyDocument,
    field: Field,
    name: &str,
) -> Result<&'a str, AppError> {
    doc.get_first(field)
        .and_then(|value| value.as_str())
        .ok_or_else(|| AppError::VectorStoreError(format!("BM25 文档缺少 {name}")))
}

pub(super) fn required_i64_field(
    doc: &TantivyDocument,
    field: Field,
    name: &str,
) -> Result<i64, AppError> {
    doc.get_first(field)
        .and_then(|value| value.as_i64())
        .ok_or_else(|| AppError::VectorStoreError(format!("BM25 文档缺少 {name}")))
}

pub(super) fn optional_text_field(doc: &TantivyDocument, field: Field) -> Option<String> {
    doc.get_first(field)
        .and_then(|value| value.as_str())
        .map(str::to_string)
}

pub(super) fn optional_i64_field(doc: &TantivyDocument, field: Field) -> Option<i64> {
    doc.get_first(field).and_then(|value| value.as_i64())
}
