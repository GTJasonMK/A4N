use super::super::tantivy_index::TantivyTextRecord;
use super::super::vector_store::EmbeddingRecord;

pub(super) fn extract_text_records(records: &[EmbeddingRecord]) -> Vec<TantivyTextRecord> {
    records
        .iter()
        .filter_map(|record| {
            let content = record.content.as_deref()?;
            if content.is_empty() {
                return None;
            }
            let project_id = record.metadata.get("project_id").and_then(|v| v.as_i64());
            let data_type = record
                .metadata
                .get("data_type")
                .and_then(|v| v.as_str())
                .map(str::to_string);
            let chapter_number = record
                .metadata
                .get("chapter_number")
                .and_then(|v| v.as_i64());
            Some(TantivyTextRecord {
                id: record.id.clone(),
                content: content.to_string(),
                project_id,
                data_type,
                chapter_number,
            })
        })
        .collect()
}
