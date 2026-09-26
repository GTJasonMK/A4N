pub use a4n_domain::{
    ThemeAppearance, ThemeBackgroundSettings, ThemeDocument, ThemeMode, ThemeStreamDisplayMode,
    ThemeStreamDisplaySettings,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::serde_update::deserialize_non_null_update_field;

pub use a4n_domain::{
    THEME_DOCUMENT_VERSION, THEME_GLOBAL_ROUTE_FAMILY, THEME_ROUTE_FAMILIES, THEME_TOKEN_NAMES,
    validate_theme_document,
};

/// 创建主题的 Web/Tauri 请求合同。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CreateThemeInput {
    pub name: String,
    pub mode: ThemeMode,
    pub document: ThemeDocument,
}

/// 更新主题的 Web/Tauri 请求合同。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UpdateThemeInput {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_non_null_update_field"
    )]
    #[schemars(with = "String")]
    pub name: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_non_null_update_field"
    )]
    #[schemars(with = "ThemeMode")]
    pub mode: Option<ThemeMode>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_non_null_update_field"
    )]
    #[schemars(with = "ThemeDocument")]
    pub document: Option<ThemeDocument>,
}

impl UpdateThemeInput {
    pub fn is_empty(&self) -> bool {
        self.name.is_none() && self.mode.is_none() && self.document.is_none()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ThemeResourceSummary {
    pub id: i64,
    pub theme_id: i64,
    pub resource_key: String,
    pub file_name: String,
    pub mime_type: String,
    /// Relative path inside the protected application data directory.
    pub file_path: String,
    pub byte_size: i64,
}

/// Internal application projection used by resource persistence and package
/// assembly. It deliberately keeps the checksum while avoiding ORM models.
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ThemeResourceUploadInput {
    pub theme_id: i64,
    pub resource_key: String,
    pub file_name: String,
    pub mime_type: String,
    pub data_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DeleteThemeResourceInput {
    pub resource_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ThemePreferenceSummary {
    pub user_id: i64,
    pub appearance: ThemeAppearance,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UpdateThemePreferenceInput {
    pub appearance: ThemeAppearance,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ThemePackageUploadInput {
    pub file_name: String,
    pub data_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ThemePackageExport {
    pub file_name: String,
    pub mime_type: String,
    pub data_base64: String,
}

/// 主题查询/变更响应合同。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ThemeSummary {
    pub id: i64,
    #[schemars(extend("x-a4n-required" = true))]
    pub user_id: Option<i64>,
    pub name: String,
    pub mode: ThemeMode,
    pub document: ThemeDocument,
    pub resources: Vec<ThemeResourceSummary>,
    pub is_system_default: bool,
    pub is_active: bool,
}

/// Theme/settings read boundary.
///
/// Theme ownership, validation and persistence stay in the implementing
/// application. Runtime owns only file I/O and package byte handling.
#[async_trait::async_trait]
pub trait ThemeReadApplicationPort: Send + Sync {
    async fn list_themes(&self, user_id: i64) -> Result<Vec<ThemeSummary>, a4n_error::AppError>;

    async fn export_theme(
        &self,
        user_id: i64,
        theme_id: i64,
    ) -> Result<ThemeSummary, a4n_error::AppError>;

    async fn list_resources(
        &self,
        user_id: i64,
        theme_id: i64,
    ) -> Result<Vec<ThemeResourceRecord>, a4n_error::AppError>;

    async fn get_preference(
        &self,
        user_id: i64,
    ) -> Result<ThemePreferenceSummary, a4n_error::AppError>;
}

/// Theme/settings mutation boundary.
#[async_trait::async_trait]
pub trait ThemeMutationApplicationPort: Send + Sync {
    async fn create_theme(
        &self,
        user_id: i64,
        input: CreateThemeInput,
    ) -> Result<ThemeSummary, a4n_error::AppError>;

    async fn update_theme(
        &self,
        user_id: i64,
        theme_id: i64,
        input: UpdateThemeInput,
    ) -> Result<ThemeSummary, a4n_error::AppError>;

    async fn delete_theme(&self, user_id: i64, theme_id: i64) -> Result<(), a4n_error::AppError>;

    async fn activate_theme(
        &self,
        user_id: i64,
        theme_id: i64,
    ) -> Result<ThemeSummary, a4n_error::AppError>;

    async fn import_theme(
        &self,
        user_id: i64,
        input: CreateThemeInput,
    ) -> Result<ThemeSummary, a4n_error::AppError>;

    async fn upsert_resource(
        &self,
        user_id: i64,
        input: PersistThemeResourceInput,
    ) -> Result<ThemeResourceRecord, a4n_error::AppError>;

    async fn delete_resource(
        &self,
        user_id: i64,
        resource_id: i64,
    ) -> Result<ThemeResourceRecord, a4n_error::AppError>;

    async fn update_preference(
        &self,
        user_id: i64,
        input: UpdateThemePreferenceInput,
    ) -> Result<ThemePreferenceSummary, a4n_error::AppError>;
}

/// Persisted resource metadata supplied after Runtime has written the file.
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

#[cfg(test)]
mod tests {
    use super::{
        CreateThemeInput, ThemeAppearance, ThemeDocument, ThemeMode, ThemeSummary, UpdateThemeInput,
    };
    use serde_json::json;

    #[test]
    fn theme_inputs_require_the_versioned_document() {
        let input: CreateThemeInput = serde_json::from_value(json!({
            "name": "Dark",
            "mode": "dark",
            "document": {"version": 1, "tokens": {}, "backgrounds": {}}
        }))
        .expect("document create input");
        assert_eq!(input.document, ThemeDocument::default());

        assert!(
            serde_json::from_value::<CreateThemeInput>(json!({
                "name": "Dark",
                "mode": "dark",
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<CreateThemeInput>(json!({
                "name": "Dark",
                "mode": "dark",
                "document": {"tokens": {}, "backgrounds": {}}
            }))
            .is_err()
        );
    }

    #[test]
    fn update_input_rejects_null_and_empty_update_is_explicit() {
        for payload in [json!({"name": "renamed theme"}), json!({"mode": "dark"})] {
            let input: UpdateThemeInput = serde_json::from_value(payload.clone()).unwrap();
            let forwarded = serde_json::to_value(&input).unwrap();
            assert_eq!(forwarded, payload);
            assert_eq!(
                serde_json::from_value::<UpdateThemeInput>(forwarded).unwrap(),
                input
            );
        }
        assert!(serde_json::from_value::<UpdateThemeInput>(json!({"document": null})).is_err());
        let empty: UpdateThemeInput = serde_json::from_value(json!({})).unwrap();
        assert!(empty.is_empty());
    }

    #[test]
    fn preference_and_summary_wire_shapes_are_stable() {
        assert_eq!(
            serde_json::to_value(ThemeAppearance::System).unwrap(),
            "system"
        );
        let summary = ThemeSummary {
            id: 1,
            user_id: None,
            name: "Dark".to_string(),
            mode: ThemeMode::Dark,
            document: ThemeDocument::default(),
            resources: Vec::new(),
            is_system_default: true,
            is_active: false,
        };
        let value = serde_json::to_value(summary).expect("serialize summary");
        assert_eq!(value["mode"], "dark");
        assert_eq!(value["document"]["version"], 1);
        assert!(value.get("resources").is_some());
    }
}
