use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq)]
pub struct ThemeRecord {
    pub id: i64,
    pub user_id: Option<i64>,
    pub name: String,
    pub mode: String,
    pub theme_document: serde_json::Value,
    pub is_system_default: bool,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ThemeResourceRecord {
    pub id: i64,
    pub theme_id: i64,
    pub resource_key: String,
    pub file_name: String,
    pub mime_type: String,
    pub file_path: String,
    pub sha256: String,
    pub byte_size: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemePreferenceRecord {
    pub appearance: String,
}
