use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use a4n_core::error::AppError;
use a4n_core::rag::RAG_TEXT_INDEX_VERSION;
use parking_lot::RwLock;

use super::collection::CollectionIndex;

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

pub struct TantivyTextIndex {
    pub(super) root_dir: PathBuf,
    pub(super) collections: RwLock<HashMap<String, Arc<CollectionIndex>>>,
}

impl TantivyTextIndex {
    pub fn new(root_dir: PathBuf) -> Result<Self, AppError> {
        let root_dir = root_dir.join(RAG_TEXT_INDEX_VERSION);
        std::fs::create_dir_all(&root_dir).map_err(|error| {
            AppError::VectorStoreError(format!("创建 tantivy 根目录失败: {error}"))
        })?;
        let mut collections = HashMap::new();
        let entries = std::fs::read_dir(&root_dir).map_err(|error| {
            AppError::VectorStoreError(format!("读取 tantivy 根目录失败: {error}"))
        })?;
        for entry in entries {
            let entry = entry.map_err(|error| {
                AppError::VectorStoreError(format!("读取索引目录项失败: {error}"))
            })?;
            let file_type = entry.file_type().map_err(|error| {
                AppError::VectorStoreError(format!("读取索引目录类型失败: {error}"))
            })?;
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
                collections.insert(
                    name,
                    Arc::new(CollectionIndex::open_or_create(entry.path())?),
                );
            }
        }
        Ok(Self {
            root_dir,
            collections: RwLock::new(collections),
        })
    }

    pub(super) fn collection_dir(&self, collection: &str) -> PathBuf {
        self.root_dir.join(sanitize(collection))
    }

    pub(super) fn get_or_create(&self, collection: &str) -> Result<Arc<CollectionIndex>, AppError> {
        if let Some(index) = self.collections.read().get(collection) {
            return Ok(index.clone());
        }
        let mut collections = self.collections.write();
        if let Some(index) = collections.get(collection) {
            return Ok(index.clone());
        }
        let index = Arc::new(CollectionIndex::open_or_create(
            self.collection_dir(collection),
        )?);
        collections.insert(collection.to_string(), index.clone());
        Ok(index)
    }
}

fn sanitize(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect()
}
