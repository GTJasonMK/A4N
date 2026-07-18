#[test]
fn all_configs_payload_rejects_duplicate_import_identity_keys() {
    let mut value = full_export_json();
    value["llm_configs"] = json!([
        {
            "name": "primary",
            "provider": "deepseek",
            "base_url": "https://api.deepseek.com/v1",
            "api_key_encrypted": "sk-test",
            "model": "deepseek-chat",
            "max_concurrent": 4,
            "timeout_secs": 60,
            "max_context_tokens": 128000,
            "max_output_tokens": 8192,
            "is_default": true
        },
        {
            "name": " primary ",
            "provider": "openai",
            "base_url": "https://api.openai.com/v1",
            "api_key_encrypted": "sk-test-2",
            "model": "gpt-4.1",
            "max_concurrent": 2,
            "timeout_secs": 90,
            "max_context_tokens": 128000,
            "max_output_tokens": 8192,
            "is_default": false
        }
    ]);
    let export =
        serde_json::from_value::<AllConfigsExport>(value).expect("shape should deserialize");

    let err = validate_all_configs_export(&export).expect_err("duplicate name should fail");

    assert!(err.contains("LLM 配置 name 重复"));
    assert!(err.contains("primary"));
}

#[test]
fn all_configs_payload_rejects_blank_prompt_content_before_import() {
    let mut value = full_export_json();
    value["prompts_modified"] = json!([{
        "name": "chapter_prompt",
        "content": " \n\t ",
        "title": null,
        "description": null
    }]);
    let export =
        serde_json::from_value::<AllConfigsExport>(value).expect("shape should deserialize");

    let err = validate_all_configs_export(&export).expect_err("blank prompt should fail");

    assert!(err.contains("提示词 #1 缺少 content"));
}

#[test]
fn theme_identity_allows_same_name_for_different_user_scopes() {
    let mut value = full_export_json();
    value["themes"] = json!([
        {
            "user_id": null,
            "name": "夜间",
            "mode": "dark",
            "css_variables": {},
            "is_system_default": true,
            "is_active": true
        },
        {
            "user_id": 7,
            "name": "夜间",
            "mode": "dark",
            "css_variables": {},
            "is_system_default": false,
            "is_active": true
        }
    ]);
    let export =
        serde_json::from_value::<AllConfigsExport>(value).expect("shape should deserialize");

    validate_all_configs_export(&export).expect("different user scopes should be allowed");

    let mut duplicate = full_export_json();
    duplicate["themes"] = json!([
        {
            "user_id": 7,
            "name": "夜间",
            "mode": "dark",
            "css_variables": {},
            "is_system_default": false,
            "is_active": true
        },
        {
            "user_id": 7,
            "name": " 夜间 ",
            "mode": "light",
            "css_variables": {},
            "is_system_default": false,
            "is_active": false
        }
    ]);
    let export =
        serde_json::from_value::<AllConfigsExport>(duplicate).expect("shape should deserialize");

    let err = validate_all_configs_export(&export).expect_err("same scope should fail");

    assert!(err.contains("主题 user_id/name 重复"));
    assert!(err.contains("7:夜间"));
}

#[test]
fn theme_payload_rejects_multiple_system_defaults() {
    let mut value = full_export_json();
    value["themes"] = json!([
        {
            "user_id": null,
            "name": "默认亮色",
            "mode": "light",
            "css_variables": {},
            "is_system_default": true,
            "is_active": false
        },
        {
            "user_id": 7,
            "name": "默认暗色",
            "mode": "dark",
            "css_variables": {},
            "is_system_default": true,
            "is_active": false
        }
    ]);
    let export =
        serde_json::from_value::<AllConfigsExport>(value).expect("shape should deserialize");

    let err = validate_all_configs_export(&export).expect_err("multiple defaults should fail");

    assert!(err.contains("系统默认主题最多只能有一个默认项"));
}

#[test]
fn theme_payload_rejects_multiple_active_themes_in_same_scope() {
    let mut value = full_export_json();
    value["themes"] = json!([
        {
            "user_id": 7,
            "name": "用户亮色",
            "mode": "light",
            "css_variables": {},
            "is_system_default": false,
            "is_active": true
        },
        {
            "user_id": 7,
            "name": "用户暗色",
            "mode": "dark",
            "css_variables": {},
            "is_system_default": false,
            "is_active": true
        }
    ]);
    let export =
        serde_json::from_value::<AllConfigsExport>(value).expect("shape should deserialize");

    let err =
        validate_all_configs_export(&export).expect_err("same-scope active themes should fail");

    assert!(err.contains("主题 active user_id 重复：7"));
}

#[test]
fn theme_payload_allows_active_global_and_user_theme() {
    let mut value = full_export_json();
    value["themes"] = json!([
        {
            "user_id": null,
            "name": "全局主题",
            "mode": "light",
            "css_variables": {},
            "is_system_default": true,
            "is_active": true
        },
        {
            "user_id": 7,
            "name": "用户主题",
            "mode": "dark",
            "css_variables": {},
            "is_system_default": false,
            "is_active": true
        }
    ]);
    let export =
        serde_json::from_value::<AllConfigsExport>(value).expect("shape should deserialize");

    validate_all_configs_export(&export).expect("different active scopes should be allowed");
}
