use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{ThemeDocument, ThemeMode};

/// Metadata for a resource whose bytes have already been staged by Runtime.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PersistThemeResourceInput {
    pub theme_id: i64,
    pub resource_key: String,
    pub file_name: String,
    pub mime_type: String,
    pub file_path: String,
    pub sha256: String,
    pub byte_size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ThemeRecord {
    pub id: i64,
    pub user_id: Option<i64>,
    pub name: String,
    pub mode: String,
    pub theme_document: Value,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ThemePreferenceRecord {
    pub user_id: i64,
    pub appearance: String,
}

pub struct ThemeUpdateChanges {
    pub name: Option<String>,
    pub mode: Option<ThemeMode>,
    pub document: Option<ThemeDocument>,
    pub resources: Vec<PersistThemeResourceInput>,
    pub delete_resource_ids: Vec<i64>,
}
