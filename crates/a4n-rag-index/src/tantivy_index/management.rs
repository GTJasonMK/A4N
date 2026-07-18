use std::collections::HashSet;

use a4n_core::error::AppError;
use tantivy::TantivyDocument;
use tantivy::collector::DocSetCollector;
use tantivy::query::AllQuery;

use super::document::required_text_field;
use super::state::TantivyTextIndex;

impl TantivyTextIndex {
    pub async fn drop_collection(&self, collection: &str) -> Result<(), AppError> {
        self.collections.write().remove(collection);
        let directory = self.collection_dir(collection);
        if directory.exists() {
            tokio::fs::remove_dir_all(&directory)
                .await
                .map_err(|error| {
                    AppError::VectorStoreError(format!("删除 collection 目录失败: {error}"))
                })?;
        }
        Ok(())
    }

    pub fn exists(&self, collection: &str) -> bool {
        self.collections.read().contains_key(collection) || self.collection_dir(collection).exists()
    }

    pub fn count(&self, collection: &str) -> Result<usize, AppError> {
        let Some(index) = self.collections.read().get(collection).cloned() else {
            return Ok(0);
        };
        let count = index.reader.searcher().num_docs();
        usize::try_from(count).map_err(|_| {
            AppError::VectorStoreError(format!("Tantivy 文档数量超出 usize 范围: {count}"))
        })
    }

    pub async fn ids(&self, collection: &str) -> Result<HashSet<String>, AppError> {
        let Some(index) = self.collections.read().get(collection).cloned() else {
            return Ok(HashSet::new());
        };
        tokio::task::spawn_blocking(move || -> Result<HashSet<String>, AppError> {
            let searcher = index.reader.searcher();
            let addresses = searcher
                .search(&AllQuery, &DocSetCollector)
                .map_err(|error| {
                    AppError::VectorStoreError(format!("扫描文档 id 失败: {error}"))
                })?;
            let mut ids = HashSet::with_capacity(addresses.len());
            for address in addresses {
                let document: TantivyDocument = searcher.doc(address).map_err(|error| {
                    AppError::VectorStoreError(format!("读取文档 id 失败: {error}"))
                })?;
                let id = required_text_field(&document, index.field_id, "id")?;
                if id.is_empty() {
                    return Err(AppError::VectorStoreError(
                        "文档 id 列表包含空 id".to_string(),
                    ));
                }
                ids.insert(id.to_string());
            }
            Ok(ids)
        })
        .await
        .map_err(|error| AppError::VectorStoreError(format!("扫描 id task 失败: {error}")))?
    }

    pub fn list_collections(&self) -> Vec<String> {
        self.collections.read().keys().cloned().collect()
    }
}
