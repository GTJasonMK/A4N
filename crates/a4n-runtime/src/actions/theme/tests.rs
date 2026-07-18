use super::contracts::{CreateThemeInput, ImportThemeInput, ThemeImportPayload};
use super::css::render_theme_css_variables;
use super::rules::{
    ensure_same_user, ensure_theme_can_be_deleted, ensure_theme_css_variables_object,
    normalize_theme_mode, normalize_theme_name,
};
use a4n_core::error::AppError;
use serde_json::json;

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

#[test]
fn import_payload_accepts_exported_theme_summary_shape() {
    let payload: ThemeImportPayload = serde_json::from_value(json!({
        "id": 1,
        "user_id": 7,
        "name": "Dark",
        "mode": "dark",
        "css_variables": {"color": "black"},
        "is_system_default": false,
        "is_active": false,
        "created_at": "2026-01-01 00:00:00",
        "updated_at": "2026-01-01 00:00:00"
    }))
    .expect("exported theme summary should be importable");

    assert_eq!(payload.name, "Dark");
    assert_eq!(payload.mode, "dark");
    assert_eq!(payload.css_variables, json!({"color": "black"}));
}

#[test]
fn normalize_theme_name_trims_and_rejects_empty() {
    assert_eq!(
        normalize_theme_name("  Quiet Dark  ".to_string()).unwrap(),
        "Quiet Dark"
    );
    let error = normalize_theme_name("   ".to_string())
        .expect_err("empty theme names should fail at command boundary");
    assert!(error.contains("主题名称不能为空"));
}

#[test]
fn normalize_theme_mode_accepts_only_supported_modes() {
    assert_eq!(normalize_theme_mode(" dark ".to_string()).unwrap(), "dark");
    assert_eq!(normalize_theme_mode("light".to_string()).unwrap(), "light");
    let error = normalize_theme_mode("system".to_string())
        .expect_err("unsupported theme mode should fail explicitly");
    assert!(error.contains("只支持 light、dark"));
}

#[test]
fn theme_css_variables_must_be_object() {
    ensure_theme_css_variables_object(&json!({
        "color-primary": "139 69 19"
    }))
    .expect("object css_variables should pass");
    let error = ensure_theme_css_variables_object(&json!(["not", "object"]))
        .expect_err("array css_variables should fail explicitly");
    assert!(error.contains("css_variables 必须是 JSON 对象"));
}

#[test]
fn delete_theme_rejects_system_default_or_active_theme() {
    let error = ensure_theme_can_be_deleted(true, false)
        .expect_err("system default theme should not be deletable");
    assert!(error.contains("系统默认主题不能删除"));
    let error =
        ensure_theme_can_be_deleted(false, true).expect_err("active theme should not be deletable");
    assert!(error.contains("正在使用的主题不能删除"));
    ensure_theme_can_be_deleted(false, false).expect("inactive non-system theme can delete");
}

#[test]
fn ensure_same_user_rejects_cross_user_theme_access() {
    let error = ensure_same_user(7, 9, "只能访问当前登录用户主题")
        .expect_err("cross-user theme access should be rejected");
    assert!(matches!(error, AppError::Forbidden(_)));
    assert!(error.to_string().contains("只能访问当前登录用户主题"));
}

#[test]
fn create_theme_input_rejects_legacy_nested_user_id() {
    let error = serde_json::from_value::<CreateThemeInput>(json!({
        "user_id": 7,
        "name": "Dark",
        "mode": "dark",
        "css_variables": {"color-primary": "139 69 19"}
    }))
    .expect_err("theme owner must come from authenticated user, not nested input");
    assert!(error.to_string().contains("unknown field `user_id`"));
}

#[test]
fn import_theme_input_rejects_legacy_nested_user_id() {
    let error = serde_json::from_value::<ImportThemeInput>(json!({
        "user_id": 7,
        "theme_json": "{}"
    }))
    .expect_err("theme import owner must come from authenticated user, not nested input");
    assert!(error.to_string().contains("unknown field `user_id`"));
}
