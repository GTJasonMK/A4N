use a4n_core::error::AppError;
use tantivy::collector::DocSetCollector;
use tantivy::query::AllQuery;
use tantivy::{IndexWriter, TantivyDocument, Term};

use super::document::required_text_field;
use super::state::TantivyTextIndex;

impl TantivyTextIndex {
    pub async fn delete_by_ids(
        &self,
        collection: &str,
        ids: Vec<String>,
    ) -> Result<usize, AppError> {
        if ids.is_empty() {
            return Ok(0);
        }
        let index = match self.collections.read().get(collection).cloned() {
            Some(index) => index,
            None => return Ok(0),
        };
        let count = ids.len();
        let blocking_index = index.clone();
        tokio::task::spawn_blocking(move || -> Result<(), AppError> {
            let mut writer: IndexWriter =
                blocking_index.index.writer(50_000_000).map_err(|error| {
                    AppError::VectorStoreError(format!("打开 writer 失败: {error}"))
                })?;
            for id in ids {
                writer.delete_term(Term::from_field_text(blocking_index.field_id, &id));
            }
            writer
                .commit()
                .map_err(|error| AppError::VectorStoreError(format!("提交删除失败: {error}")))?;
            Ok(())
        })
        .await
        .map_err(|error| AppError::VectorStoreError(format!("删除 task 失败: {error}")))??;
        index
            .reader
            .reload()
            .map_err(|error| AppError::VectorStoreError(format!("reload 失败: {error}")))?;
        Ok(count)
    }

    pub async fn delete_by_prefixes(
        &self,
        collection: &str,
        prefixes: Vec<String>,
    ) -> Result<usize, AppError> {
        let prefixes = prefixes
            .into_iter()
            .map(|prefix| prefix.trim().to_string())
            .filter(|prefix| !prefix.is_empty())
            .collect::<Vec<_>>();
        if prefixes.is_empty() {
            return Ok(0);
        }
        let index = match self.collections.read().get(collection).cloned() {
            Some(index) => index,
            None => return Ok(0),
        };
        let blocking_index = index.clone();
        let deleted = tokio::task::spawn_blocking(move || -> Result<usize, AppError> {
            let searcher = blocking_index.reader.searcher();
            let addresses = searcher
                .search(&AllQuery, &DocSetCollector)
                .map_err(|error| {
                    AppError::VectorStoreError(format!("扫描待删除文档失败: {error}"))
                })?;
            let mut ids = Vec::new();
            for address in addresses {
                let document: TantivyDocument = searcher.doc(address).map_err(|error| {
                    AppError::VectorStoreError(format!("读取待删除文档失败: {error}"))
                })?;
                let id = required_text_field(&document, blocking_index.field_id, "id")?;
                if id.is_empty() {
                    return Err(AppError::VectorStoreError(
                        "待删除文档包含空 id".to_string(),
                    ));
                }
                if prefixes.iter().any(|prefix| id.starts_with(prefix)) {
                    ids.push(id.to_string());
                }
            }
            if ids.is_empty() {
                return Ok(0);
            }
            let count = ids.len();
            let mut writer: IndexWriter =
                blocking_index.index.writer(50_000_000).map_err(|error| {
                    AppError::VectorStoreError(format!("打开 writer 失败: {error}"))
                })?;
            for id in ids {
                writer.delete_term(Term::from_field_text(blocking_index.field_id, &id));
            }
            writer.commit().map_err(|error| {
                AppError::VectorStoreError(format!("提交前缀删除失败: {error}"))
            })?;
            Ok(count)
        })
        .await
        .map_err(|error| AppError::VectorStoreError(format!("前缀删除 task 失败: {error}")))??;
        if deleted > 0 {
            index
                .reader
                .reload()
                .map_err(|error| AppError::VectorStoreError(format!("reload 失败: {error}")))?;
        }
        Ok(deleted)
    }
}
