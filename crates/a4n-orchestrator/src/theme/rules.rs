//! 主题 Host 输入的纯校验规则（无 DB）。

use serde_json::Value;

const THEME_MODES: &[&str] = &["light", "dark"];

/// 主题名称 trim 后非空。
pub fn normalize_theme_name(name: String) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("主题名称不能为空".to_string());
    }
    Ok(name.to_string())
}

/// 主题 mode 仅允许 light/dark。
pub fn normalize_theme_mode(mode: String) -> Result<String, String> {
    let mode = mode.trim();
    if !THEME_MODES.contains(&mode) {
        return Err(format!(
            "主题 mode 无效：{mode}，只支持 {}",
            THEME_MODES.join("、")
        ));
    }
    Ok(mode.to_string())
}

/// css_variables 必须是 JSON 对象。
pub fn ensure_theme_css_variables_object(css_variables: &Value) -> Result<(), String> {
    if !css_variables.is_object() {
        return Err("主题 css_variables 必须是 JSON 对象".to_string());
    }
    Ok(())
}

/// 将 css_variables JSON 对象渲染为 `:root { --k: v; }` 文本。
pub fn render_theme_css_variables(css_variables: &Value) -> String {
    let mut css = String::from(":root {\n");
    if let Some(object) = css_variables.as_object() {
        for (key, value) in object {
            let value = match value {
                Value::String(text) => text.clone(),
                other => other.to_string(),
            };
            css.push_str(&format!("  {}: {};\n", css_variable_name(key), value));
        }
    }
    css.push('}');
    css
}

fn css_variable_name(key: &str) -> String {
    if key.starts_with("--") {
        key.to_string()
    } else {
        format!("--{key}")
    }
}

/// 系统默认主题与当前激活主题不可删除。
pub fn ensure_theme_can_be_deleted(is_system_default: bool, is_active: bool) -> Result<(), String> {
    if is_system_default {
        return Err("系统默认主题不能删除".to_string());
    }
    if is_active {
        return Err("正在使用的主题不能删除，请先激活其他主题".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalize_theme_name_trims_and_rejects_empty() {
        assert_eq!(
            normalize_theme_name("  Quiet Dark  ".to_string()).unwrap(),
            "Quiet Dark"
        );
        let error = normalize_theme_name("   ".to_string()).expect_err("empty");
        assert!(error.contains("主题名称不能为空"));
    }

    #[test]
    fn normalize_theme_mode_accepts_only_supported_modes() {
        assert_eq!(normalize_theme_mode(" dark ".to_string()).unwrap(), "dark");
        assert_eq!(normalize_theme_mode("light".to_string()).unwrap(), "light");
        let error = normalize_theme_mode("system".to_string()).expect_err("unsupported");
        assert!(error.contains("只支持 light、dark"));
    }

    #[test]
    fn theme_css_variables_must_be_object() {
        ensure_theme_css_variables_object(&json!({"color-primary": "139 69 19"}))
            .expect("object ok");
        let error =
            ensure_theme_css_variables_object(&json!(["not", "object"])).expect_err("array");
        assert!(error.contains("css_variables 必须是 JSON 对象"));
    }

    #[test]
    fn delete_theme_rejects_system_default_or_active() {
        let error = ensure_theme_can_be_deleted(true, false).expect_err("system");
        assert!(error.contains("系统默认主题不能删除"));
        let error = ensure_theme_can_be_deleted(false, true).expect_err("active");
        assert!(error.contains("正在使用的主题不能删除"));
        ensure_theme_can_be_deleted(false, false).expect("ok");
    }

    #[test]
    fn render_theme_css_variables_preserves_existing_custom_property_prefix() {
        let css = render_theme_css_variables(&json!({
            "--color-primary": "139 69 19",
            "color-secondary": "255 255 255",
            "radius": 8,
        }));
        assert!(css.contains("  --color-primary: 139 69 19;"));
        assert!(css.contains("  --color-secondary: 255 255 255;"));
        assert!(css.contains("  --radius: 8;"));
    }
}
