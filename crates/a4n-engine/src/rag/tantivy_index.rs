//! Tantivy 全文索引
//!
//! 为每个 collection 维护一个独立的 Tantivy 索引目录（`storage_dir/{collection}/`），
//! 提供 BM25 全文搜索能力。所有写入操作都在 `tokio::task::spawn_blocking` 中执行，
//! 避免阻塞异步运行时；读取使用单一长生命周期 Reader，写入后 reload 才可见。

use std::collections::HashMap;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use a4n_core::error::AppError;
use a4n_core::rag::RAG_TEXT_INDEX_VERSION;
use parking_lot::RwLock;
use tantivy::collector::{DocSetCollector, TopDocs};
use tantivy::query::AllQuery;
use tantivy::{IndexWriter, TantivyDocument, Term};

use super::vector_store::{SearchFilter, SearchResult};

mod collection;
mod document;
mod query;

#[cfg(test)]
mod tests;

use collection::CollectionIndex;
use document::{optional_i64_field, optional_text_field, required_i64_field, required_text_field};
use query::{
    build_search_query, collection_excluded_by_filter, ngram_query_terms, normalized_search_text,
};

pub(super) const CONTENT_NGRAM_TOKENIZER: &str = "a4n_content_ngram_2_3";
pub(super) const FIELD_CHAPTER_NUMBER: &str = "chapter_number";
pub(super) const FIELD_PROJECT_ID: &str = "project_id";
pub(super) const QUERY_NGRAM_LIMIT: usize = 384;

#[derive(Debug, Clone)]
pub struct TantivyTextRecord {
    pub id: String,
    pub content: String,
    pub project_id: Option<i64>,
    pub data_type: Option<String>,
    pub chapter_number: Option<i64>,
}

impl TantivyTextRecord {
    pub fn new(id: impl Into<String>, content: impl Into<String>, project_id: Option<i64>) -> Self {
        Self {
            id: id.into(),
            content: content.into(),
            project_id,
            data_type: None,
            chapter_number: None,
        }
    }
}

/// Tantivy 全文索引：按 collection 分库
pub struct TantivyTextIndex {
    root_dir: PathBuf,
    collections: RwLock<HashMap<String, Arc<CollectionIndex>>>,
}

impl TantivyTextIndex {
    /// 创建新实例，根目录下每个 collection 一个子目录
    pub fn new(root_dir: PathBuf) -> Result<Self, AppError> {
        let root_dir = root_dir.join(RAG_TEXT_INDEX_VERSION);
        std::fs::create_dir_all(&root_dir)
            .map_err(|e| AppError::VectorStoreError(format!("创建 tantivy 根目录失败: {e}")))?;

        // 扫描已有 collection 目录并预加载
        let mut map: HashMap<String, Arc<CollectionIndex>> = HashMap::new();
        let entries = std::fs::read_dir(&root_dir)
            .map_err(|e| AppError::VectorStoreError(format!("读取 tantivy 根目录失败: {e}")))?;
        for entry in entries {
            let entry = entry
                .map_err(|e| AppError::VectorStoreError(format!("读取索引目录项失败: {e}")))?;
            let file_type = entry
                .file_type()
                .map_err(|e| AppError::VectorStoreError(format!("读取索引目录类型失败: {e}")))?;
            if file_type.is_dir() {
                let name = entry
                    .file_name()
                    .to_str()
                    .map(str::to_string)
                    .ok_or_else(|| {
                        AppError::VectorStoreError(
                            "tantivy collection 目录名不是 UTF-8".to_string(),
                        )
                    })?;
                let ci = CollectionIndex::open_or_create(entry.path())?;
                map.insert(name, Arc::new(ci));
            }
        }

        Ok(Self {
            root_dir,
            collections: RwLock::new(map),
        })
    }

    fn collection_dir(&self, collection: &str) -> PathBuf {
        self.root_dir.join(sanitize(collection))
    }

    /// 获取或创建一个 collection 的索引上下文
    fn get_or_create(&self, collection: &str) -> Result<Arc<CollectionIndex>, AppError> {
        if let Some(ci) = self.collections.read().get(collection) {
            return Ok(ci.clone());
        }
        let mut guard = self.collections.write();
        if let Some(ci) = guard.get(collection) {
            return Ok(ci.clone());
        }
        let ci = Arc::new(CollectionIndex::open_or_create(
            self.collection_dir(collection),
        )?);
        guard.insert(collection.to_string(), ci.clone());
        Ok(ci)
    }

    /// upsert 一批文档（先按 id 删除再插入），并 commit
    pub async fn upsert(
        &self,
        collection: &str,
        records: Vec<TantivyTextRecord>,
    ) -> Result<(), AppError> {
        if records.is_empty() {
            return Ok(());
        }
        let ci = self.get_or_create(collection)?;
        let ci_for_block = ci.clone();
        tokio::task::spawn_blocking(move || -> Result<(), AppError> {
            let mut writer: IndexWriter = ci_for_block
                .index
                .writer(50_000_000)
                .map_err(|e| AppError::VectorStoreError(format!("打开 writer 失败: {e}")))?;

            for record in &records {
                let term = Term::from_field_text(ci_for_block.field_id, &record.id);
                writer.delete_term(term);
                let project = record.project_id.ok_or_else(|| {
                    AppError::VectorStoreError(format!("文档 {} 缺少 project_id", record.id))
                })?;
                let search_text = normalized_search_text(&record.content);
                let mut doc = TantivyDocument::new();
                doc.add_text(ci_for_block.field_id, &record.id);
                doc.add_text(ci_for_block.field_content, &record.content);
                doc.add_text(ci_for_block.field_search_text, &search_text);
                doc.add_i64(ci_for_block.field_project, project);
                if let Some(data_type) = record.data_type.as_deref() {
                    doc.add_text(ci_for_block.field_data_type, data_type);
                }
                if let Some(chapter_number) = record.chapter_number {
                    doc.add_i64(ci_for_block.field_chapter_number, chapter_number);
                }
                writer
                    .add_document(doc)
                    .map_err(|e| AppError::VectorStoreError(format!("写入文档失败: {e}")))?;
            }
            writer
                .commit()
                .map_err(|e| AppError::VectorStoreError(format!("提交失败: {e}")))?;
            Ok(())
        })
        .await
        .map_err(|e| AppError::VectorStoreError(format!("写入 task 失败: {e}")))??;

        // 重新加载 reader，让读路径看到新写入
        ci.reader
            .reload()
            .map_err(|e| AppError::VectorStoreError(format!("reload 失败: {e}")))?;
        Ok(())
    }

    /// 按 BM25 查询
    pub async fn search(
        &self,
        collection: &str,
        query_text: &str,
        top_k: usize,
    ) -> Result<Vec<SearchResult>, AppError> {
        self.search_filtered(collection, query_text, top_k, None)
            .await
    }

    /// 按 BM25 查询，并在全文索引层尽早应用可表达的结构化过滤条件。
    pub async fn search_filtered(
        &self,
        collection: &str,
        query_text: &str,
        top_k: usize,
        filters: Option<&SearchFilter>,
    ) -> Result<Vec<SearchResult>, AppError> {
        let ci = match self.collections.read().get(collection).cloned() {
            Some(ci) => ci,
            None => return Ok(vec![]),
        };
        if collection_excluded_by_filter(collection, filters) {
            return Ok(Vec::new());
        }

        let terms = ngram_query_terms(ci.field_search_text, query_text);
        if terms.is_empty() {
            return Ok(Vec::new());
        }
        let Some(query) = build_search_query(&ci, terms, filters) else {
            return Ok(Vec::new());
        };
        tokio::task::spawn_blocking(move || -> Result<Vec<SearchResult>, AppError> {
            let searcher = ci.reader.searcher();
            let hits = searcher
                .search(query.as_ref(), &TopDocs::with_limit(top_k))
                .map_err(|e| AppError::VectorStoreError(format!("BM25 检索失败: {e}")))?;

            let mut results = Vec::with_capacity(hits.len());
            for (score, addr) in hits {
                let doc: TantivyDocument = searcher
                    .doc(addr)
                    .map_err(|e| AppError::VectorStoreError(format!("读取文档失败: {e}")))?;
                let id = required_text_field(&doc, ci.field_id, "id")?.to_string();
                if id.is_empty() {
                    return Err(AppError::VectorStoreError("BM25 结果包含空 id".to_string()));
                }
                let content = required_text_field(&doc, ci.field_content, "content")?.to_string();
                let project_id = required_i64_field(&doc, ci.field_project, "project_id")?;
                let data_type = optional_text_field(&doc, ci.field_data_type);
                let chapter_number = optional_i64_field(&doc, ci.field_chapter_number);
                let mut metadata = serde_json::json!({"project_id": project_id});
                if let serde_json::Value::Object(object) = &mut metadata {
                    if let Some(data_type) = data_type {
                        object.insert("data_type".to_string(), serde_json::json!(data_type));
                    }
                    if let Some(chapter_number) = chapter_number {
                        object.insert(
                            "chapter_number".to_string(),
                            serde_json::json!(chapter_number),
                        );
                    }
                }
                results.push(SearchResult {
                    id,
                    content,
                    score,
                    vector_score: None,
                    bm25_score: Some(score),
                    metadata,
                });
            }
            results.sort_by(|a, b| b.score.total_cmp(&a.score));
            results.truncate(top_k);
            Ok(results)
        })
        .await
        .map_err(|e| AppError::VectorStoreError(format!("检索 task 失败: {e}")))?
    }

    /// 按 id 列表删除
    pub async fn delete_by_ids(
        &self,
        collection: &str,
        ids: Vec<String>,
    ) -> Result<usize, AppError> {
        if ids.is_empty() {
            return Ok(0);
        }
        let ci = match self.collections.read().get(collection).cloned() {
            Some(ci) => ci,
            None => return Ok(0),
        };

        let count = ids.len();
        let ci_for_block = ci.clone();
        tokio::task::spawn_blocking(move || -> Result<(), AppError> {
            let mut writer: IndexWriter = ci_for_block
                .index
                .writer(50_000_000)
                .map_err(|e| AppError::VectorStoreError(format!("打开 writer 失败: {e}")))?;
            for id in ids {
                let term = Term::from_field_text(ci_for_block.field_id, &id);
                writer.delete_term(term);
            }
            writer
                .commit()
                .map_err(|e| AppError::VectorStoreError(format!("提交删除失败: {e}")))?;
            Ok(())
        })
        .await
        .map_err(|e| AppError::VectorStoreError(format!("删除 task 失败: {e}")))??;

        ci.reader
            .reload()
            .map_err(|e| AppError::VectorStoreError(format!("reload 失败: {e}")))?;
        Ok(count)
    }

    /// 按 id 前缀删除。
    ///
    /// Tantivy 索引只保存检索所需的精简字段，无法可靠按完整 metadata 过滤。
    /// A4N 的 RAG 记录使用稳定的 project-scoped id 前缀，因此精细清理走这里。
    pub async fn delete_by_prefixes(
        &self,
        collection: &str,
        prefixes: Vec<String>,
    ) -> Result<usize, AppError> {
        let prefixes: Vec<String> = prefixes
            .into_iter()
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty())
            .collect();
        if prefixes.is_empty() {
            return Ok(0);
        }

        let ci = match self.collections.read().get(collection).cloned() {
            Some(ci) => ci,
            None => return Ok(0),
        };

        let ci_for_block = ci.clone();
        let deleted = tokio::task::spawn_blocking(move || -> Result<usize, AppError> {
            let searcher = ci_for_block.reader.searcher();
            let addresses = searcher
                .search(&AllQuery, &DocSetCollector)
                .map_err(|e| AppError::VectorStoreError(format!("扫描待删除文档失败: {e}")))?;

            let mut ids = Vec::new();
            for addr in addresses {
                let doc: TantivyDocument = searcher
                    .doc(addr)
                    .map_err(|e| AppError::VectorStoreError(format!("读取待删除文档失败: {e}")))?;
                let id = required_text_field(&doc, ci_for_block.field_id, "id")?;
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
            let mut writer: IndexWriter = ci_for_block
                .index
                .writer(50_000_000)
                .map_err(|e| AppError::VectorStoreError(format!("打开 writer 失败: {e}")))?;
            for id in ids {
                let term = Term::from_field_text(ci_for_block.field_id, &id);
                writer.delete_term(term);
            }
            writer
                .commit()
                .map_err(|e| AppError::VectorStoreError(format!("提交前缀删除失败: {e}")))?;
            Ok(count)
        })
        .await
        .map_err(|e| AppError::VectorStoreError(format!("前缀删除 task 失败: {e}")))??;

        if deleted > 0 {
            ci.reader
                .reload()
                .map_err(|e| AppError::VectorStoreError(format!("reload 失败: {e}")))?;
        }
        Ok(deleted)
    }

    /// 删除整个 collection（关闭 + 删目录）
    pub async fn drop_collection(&self, collection: &str) -> Result<(), AppError> {
        self.collections.write().remove(collection);
        let dir = self.collection_dir(collection);
        if dir.exists() {
            tokio::fs::remove_dir_all(&dir).await.map_err(|e| {
                AppError::VectorStoreError(format!("删除 collection 目录失败: {e}"))
            })?;
        }
        Ok(())
    }

    /// collection 是否存在
    pub fn exists(&self, collection: &str) -> bool {
        self.collections.read().contains_key(collection) || self.collection_dir(collection).exists()
    }

    /// 当前文档数（近似值，从 searcher 拿）
    pub fn count(&self, collection: &str) -> Result<usize, AppError> {
        let Some(ci) = self.collections.read().get(collection).cloned() else {
            return Ok(0);
        };
        let searcher = ci.reader.searcher();
        Ok(searcher.num_docs() as usize)
    }

    /// 列出 collection 中所有文档 id。
    pub async fn ids(&self, collection: &str) -> Result<HashSet<String>, AppError> {
        let Some(ci) = self.collections.read().get(collection).cloned() else {
            return Ok(HashSet::new());
        };

        tokio::task::spawn_blocking(move || -> Result<HashSet<String>, AppError> {
            let searcher = ci.reader.searcher();
            let addresses = searcher
                .search(&AllQuery, &DocSetCollector)
                .map_err(|e| AppError::VectorStoreError(format!("扫描文档 id 失败: {e}")))?;

            let mut ids = HashSet::with_capacity(addresses.len());
            for addr in addresses {
                let doc: TantivyDocument = searcher
                    .doc(addr)
                    .map_err(|e| AppError::VectorStoreError(format!("读取文档 id 失败: {e}")))?;
                let id = required_text_field(&doc, ci.field_id, "id")?;
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
        .map_err(|e| AppError::VectorStoreError(format!("扫描 id task 失败: {e}")))?
    }

    /// 列出所有 collection
    pub fn list_collections(&self) -> Vec<String> {
        self.collections.read().keys().cloned().collect()
    }
}

/// 防止 collection 名包含路径分隔符等危险字符
fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
