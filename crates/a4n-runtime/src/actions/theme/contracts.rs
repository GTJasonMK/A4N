use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeSummary {
    pub id: i64,
    pub user_id: Option<i64>,
    pub name: String,
    pub mode: String,
    pub css_variables: Value,
    pub is_system_default: bool,
    pub is_active: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateThemeInput {
    pub name: String,
    pub mode: String,
    pub css_variables: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateThemeInput {
    pub name: Option<String>,
    pub mode: Option<String>,
    pub css_variables: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportThemeInput {
    pub theme_json: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct ThemeImportPayload {
    pub(super) name: String,
    pub(super) mode: String,
    pub(super) css_variables: Value,
}
