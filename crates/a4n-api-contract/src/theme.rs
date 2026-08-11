use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::serde_update::deserialize_non_null_update_field;

pub use a4n_domain::{ThemeCssVariables, ThemeMode};

/// 创建主题的 Web/Tauri 请求合同。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CreateThemeInput {
    pub name: String,
    pub mode: ThemeMode,
    pub css_variables: ThemeCssVariables,
}

/// 更新主题的 Web/Tauri 请求合同。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UpdateThemeInput {
    #[serde(default, deserialize_with = "deserialize_non_null_update_field")]
    #[schemars(with = "String")]
    pub name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_non_null_update_field")]
    #[schemars(with = "ThemeMode")]
    pub mode: Option<ThemeMode>,
    #[serde(default, deserialize_with = "deserialize_non_null_update_field")]
    #[schemars(with = "ThemeCssVariables")]
    pub css_variables: Option<ThemeCssVariables>,
}

impl UpdateThemeInput {
    pub fn is_empty(&self) -> bool {
        self.name.is_none() && self.mode.is_none() && self.css_variables.is_none()
    }
}

/// 主题查询/变更响应合同。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ThemeSummary {
    pub id: i64,
    #[schemars(extend("x-a4n-required" = true))]
    pub user_id: Option<i64>,
    pub name: String,
    pub mode: ThemeMode,
    pub css_variables: ThemeCssVariables,
    pub is_system_default: bool,
    pub is_active: bool,
}

#[cfg(test)]
mod tests {
    use super::{CreateThemeInput, ThemeMode, ThemeSummary, UpdateThemeInput};
    use serde_json::json;

    #[test]
    fn theme_inputs_keep_strict_wire_contract() {
        let input: CreateThemeInput = serde_json::from_value(json!({
            "name": "Dark",
            "mode": "dark",
            "css_variables": {"--bg": "0 0 0"}
        }))
        .expect("valid create input");
        assert_eq!(input.name, "Dark");
        assert_eq!(input.mode, ThemeMode::Dark);
        assert!(
            serde_json::from_value::<UpdateThemeInput>(json!({
                "unexpected": true
            }))
            .is_err()
        );
        assert!(serde_json::from_value::<UpdateThemeInput>(json!({"name": null})).is_err());
        assert!(serde_json::from_value::<UpdateThemeInput>(json!({"mode": null})).is_err());
        assert!(
            serde_json::from_value::<UpdateThemeInput>(json!({"css_variables": null})).is_err()
        );
        let empty: UpdateThemeInput = serde_json::from_value(json!({})).unwrap();
        assert!(empty.is_empty());
        assert!(
            serde_json::from_value::<CreateThemeInput>(json!({
                "name": "Dark",
                "mode": "dark",
                "css_variables": {"--radius": 8}
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<CreateThemeInput>(json!({
                "name": "Dark",
                "mode": " dark ",
                "css_variables": {}
            }))
            .is_err()
        );
    }

    #[test]
    fn theme_summary_wire_shape_is_stable() {
        let summary = ThemeSummary {
            id: 1,
            user_id: None,
            name: "Dark".to_string(),
            mode: ThemeMode::Dark,
            css_variables: Default::default(),
            is_system_default: true,
            is_active: false,
        };
        let value = serde_json::to_value(summary).expect("serialize summary");
        assert_eq!(value["mode"], "dark");
        assert!(value.get("css_variables").is_some());
    }
}
