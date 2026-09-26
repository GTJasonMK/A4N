use serde::{Deserialize, Serialize};

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

pub struct ThemeUpdateChanges {
    pub name: Option<String>,
    pub mode: Option<ThemeMode>,
    pub document: Option<ThemeDocument>,
    pub resources: Vec<PersistThemeResourceInput>,
    pub delete_resource_ids: Vec<i64>,
}
