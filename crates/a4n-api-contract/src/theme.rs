use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 创建主题的 Web/Tauri 请求合同。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CreateThemeInput {
    pub name: String,
    pub mode: String,
    pub css_variables: Value,
}

/// 更新主题的 Web/Tauri 请求合同。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UpdateThemeInput {
    pub name: Option<String>,
    pub mode: Option<String>,
    pub css_variables: Option<Value>,
}

/// 导入主题的 Web/Tauri 请求合同。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ImportThemeInput {
    pub theme_json: String,
}

/// 主题查询/变更响应合同。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ThemeSummary {
    pub id: i64,
    #[schemars(extend("x-a4n-required" = true))]
    pub user_id: Option<i64>,
    pub name: String,
    pub mode: String,
    pub css_variables: Value,
    pub is_system_default: bool,
    pub is_active: bool,
    #[schemars(extend("x-a4n-required" = true))]
    pub created_at: Option<String>,
    #[schemars(extend("x-a4n-required" = true))]
    pub updated_at: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{CreateThemeInput, ThemeSummary, UpdateThemeInput};
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
        assert!(
            serde_json::from_value::<UpdateThemeInput>(json!({
                "unexpected": true
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
            mode: "dark".to_string(),
            css_variables: json!({}),
            is_system_default: true,
            is_active: false,
            created_at: None,
            updated_at: None,
        };
        let value = serde_json::to_value(summary).expect("serialize summary");
        assert_eq!(value["mode"], "dark");
        assert!(value.get("css_variables").is_some());
    }
}
