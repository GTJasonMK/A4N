use std::collections::HashMap;
use std::path::PathBuf;

use a4n_core::error::AppError;
use lancedb::{Connection, connect};
use parking_lot::RwLock;

use super::batch::dim_of_vector_field;

/// LanceDB 向量存储
pub struct LanceVectorStore {
    pub(super) db: Connection,
    /// 每个 collection 的向量维度缓存（首次写入时确定，后续校验）
    dims: RwLock<HashMap<String, usize>>,
}

impl LanceVectorStore {
    /// 打开或创建 LanceDB 数据库
    pub async fn open(root: PathBuf) -> Result<Self, AppError> {
        std::fs::create_dir_all(&root)
            .map_err(|err| AppError::VectorStoreError(format!("创建 lance 根目录失败: {err}")))?;
        let uri = root.to_string_lossy().to_string();
        let db = connect(&uri)
            .execute()
            .await
            .map_err(|err| AppError::VectorStoreError(format!("lancedb connect 失败: {err}")))?;

        let mut store = Self {
            db,
            dims: RwLock::new(HashMap::new()),
        };
        store.refresh_dims_cache().await?;
        Ok(store)
    }

    async fn refresh_dims_cache(&mut self) -> Result<(), AppError> {
        let names = self
            .db
            .table_names()
            .execute()
            .await
            .map_err(|err| AppError::VectorStoreError(format!("table_names 失败: {err}")))?;
        let mut refreshed_dims = HashMap::new();
        for name in names {
            let table = self.db.open_table(&name).execute().await.map_err(|err| {
                AppError::VectorStoreError(format!("open_table {name} 失败: {err}"))
            })?;
            let schema = table
                .schema()
                .await
                .map_err(|err| AppError::VectorStoreError(format!("schema {name} 失败: {err}")))?;
            let dimension = dim_of_vector_field(schema.as_ref(), &name)?;
            refreshed_dims.insert(name, dimension);
        }
        *self.dims.write() = refreshed_dims;
        Ok(())
    }

    pub(super) async fn table_exists(&self, table_name: &str) -> Result<bool, AppError> {
        let names = self
            .db
            .table_names()
            .execute()
            .await
            .map_err(|err| AppError::VectorStoreError(format!("table_names 失败: {err}")))?;
        Ok(names.iter().any(|name| name == table_name))
    }

    pub(super) fn sanitize(name: &str) -> String {
        name.chars()
            .map(|character| {
                if character.is_alphanumeric() || matches!(character, '_' | '-') {
                    character
                } else {
                    '_'
                }
            })
            .collect()
    }

    pub(super) fn cached_dim(&self, collection: &str) -> Option<usize> {
        self.dims.read().get(collection).copied()
    }

    pub(super) fn remember_dim(&self, collection: &str, dimension: usize) {
        self.dims.write().insert(collection.to_string(), dimension);
    }

    pub(super) fn forget_dim(&self, collection: &str) {
        self.dims.write().remove(collection);
    }
}
