use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct StoragePathsSummary {
    pub application_home: String,
    pub data_dir: String,
    pub config_file: String,
    pub database_url: String,
    pub projects_dir: String,
    pub exports_dir: String,
    pub backups_dir: String,
    pub cache_dir: String,
    pub embedding_models_dir: String,
    pub temp_dir: String,
    pub log_dir: String,
    pub webview_dir: String,
    pub frontend_state_file: String,
    pub prompts_git_dir: String,
    /// Qdrant 连接地址（独立 RAG 索引，非业务库）
    pub qdrant_url: String,
    /// Qdrant 物理 collection 名
    pub qdrant_collection: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct OrphanProjectDataDirSummary {
    pub project_id: i64,
    pub path: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct OrphanProjectDataDirsSummary {
    pub items: Vec<OrphanProjectDataDirSummary>,
    pub total_count: usize,
    pub total_size_bytes: u64,
}
