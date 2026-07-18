use a4n_core::error::AppError;
use tantivy::{IndexWriter, TantivyDocument, Term};

use super::query::normalized_search_text;
use super::state::{TantivyTextIndex, TantivyTextRecord};

impl TantivyTextIndex {
    pub async fn upsert(
        &self,
        collection: &str,
        records: Vec<TantivyTextRecord>,
    ) -> Result<(), AppError> {
        if records.is_empty() {
            return Ok(());
        }
        let index = self.get_or_create(collection)?;
        let blocking_index = index.clone();
        tokio::task::spawn_blocking(move || -> Result<(), AppError> {
            let mut writer: IndexWriter =
                blocking_index.index.writer(50_000_000).map_err(|error| {
                    AppError::VectorStoreError(format!("打开 writer 失败: {error}"))
                })?;
            for record in &records {
                writer.delete_term(Term::from_field_text(blocking_index.field_id, &record.id));
                let project = record.project_id.ok_or_else(|| {
                    AppError::VectorStoreError(format!("文档 {} 缺少 project_id", record.id))
                })?;
                let mut document = TantivyDocument::new();
                document.add_text(blocking_index.field_id, &record.id);
                document.add_text(blocking_index.field_content, &record.content);
                document.add_text(
                    blocking_index.field_search_text,
                    normalized_search_text(&record.content),
                );
                document.add_i64(blocking_index.field_project, project);
                if let Some(data_type) = record.data_type.as_deref() {
                    document.add_text(blocking_index.field_data_type, data_type);
                }
                if let Some(chapter_number) = record.chapter_number {
                    document.add_i64(blocking_index.field_chapter_number, chapter_number);
                }
                writer.add_document(document).map_err(|error| {
                    AppError::VectorStoreError(format!("写入文档失败: {error}"))
                })?;
            }
            writer
                .commit()
                .map_err(|error| AppError::VectorStoreError(format!("提交失败: {error}")))?;
            Ok(())
        })
        .await
        .map_err(|error| AppError::VectorStoreError(format!("写入 task 失败: {error}")))??;
        index
            .reader
            .reload()
            .map_err(|error| AppError::VectorStoreError(format!("reload 失败: {error}")))?;
        Ok(())
    }
}
