use super::{CreateThemeInput, ImportThemeInput};
use serde_json::json;

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
