use super::super::types::{EmbeddingRow, ImageRow, ImportAllConfigsInput, LlmRow, VideoRow};
use super::*;
use serde_json::{Value, json};

fn full_export_json() -> Value {
    json!({
        "version": "1.0",
        "export_time": "2026-05-27T00:00:00Z",
        "export_type": "all",
        "llm_configs": [],
        "embedding_configs": [],
        "image_configs": [],
        "video_configs": [],
        "prompts_modified": [],
        "themes": [],
        "system_settings": []
    })
}

#[test]
fn all_configs_export_requires_every_section() {
    let mut value = full_export_json();
    value
        .as_object_mut()
        .expect("full export is object")
        .remove("themes");

    let err =
        serde_json::from_value::<AllConfigsExport>(value).expect_err("missing section should fail");
    assert!(err.to_string().contains("themes"));
}

#[test]
fn all_configs_export_requires_video_configs_section() {
    let mut value = full_export_json();
    value
        .as_object_mut()
        .expect("full export is object")
        .remove("video_configs");

    let err =
        serde_json::from_value::<AllConfigsExport>(value).expect_err("video_configs is required");

    assert!(err.to_string().contains("video_configs"));
}

#[test]
fn import_input_requires_explicit_overwrite() {
    let err = serde_json::from_value::<ImportAllConfigsInput>(json!({
        "settings_json": "{}"
    }))
    .expect_err("overwrite should be explicit");

    assert!(err.to_string().contains("overwrite"));
}

#[test]
fn ai_config_rows_require_api_key_field_even_when_value_is_null() {
    let err = serde_json::from_value::<LlmRow>(json!({
        "name": "llm",
        "provider": "deepseek",
        "base_url": "https://api.deepseek.com/v1",
        "model": "deepseek-chat",
        "max_concurrent": 4,
        "timeout_secs": 60,
        "max_context_tokens": 128000,
        "max_output_tokens": 8192,
        "is_default": false
    }))
    .expect_err("all-config LLM rows must explicitly include api_key_encrypted");
    assert!(err.to_string().contains("api_key_encrypted"));

    let llm = serde_json::from_value::<LlmRow>(json!({
        "name": "llm",
        "provider": "deepseek",
        "base_url": "https://api.deepseek.com/v1",
        "api_key_encrypted": null,
        "model": "deepseek-chat",
        "max_concurrent": 4,
        "timeout_secs": 60,
        "max_context_tokens": 128000,
        "max_output_tokens": 8192,
        "is_default": false
    }))
    .expect("explicit null remains a valid imported field value");
    assert_eq!(llm.api_key_encrypted, None);

    let err = serde_json::from_value::<EmbeddingRow>(json!({
        "name": "embedding",
        "backend": "local_hash",
        "base_url": null,
        "model": "local",
        "dimension": 384,
        "batch_size": 16,
        "local_model_path": null,
        "is_default": false
    }))
    .expect_err("all-config Embedding rows must explicitly include api_key_encrypted");
    assert!(err.to_string().contains("api_key_encrypted"));

    let err = serde_json::from_value::<ImageRow>(json!({
        "name": "image",
        "backend": "stable-diffusion",
        "base_url": "http://localhost:7860",
        "model": "sdxl",
        "max_concurrent": 1,
        "default_width": 1024,
        "default_height": 1024,
        "is_default": false
    }))
    .expect_err("all-config Image rows must explicitly include api_key_encrypted");
    assert!(err.to_string().contains("api_key_encrypted"));

    let err = serde_json::from_value::<VideoRow>(json!({
        "name": "video",
        "backend": "openai-compatible-video",
        "base_url": "https://video.example/v1",
        "model": "video-model",
        "max_concurrent": 1,
        "default_width": 1280,
        "default_height": 720,
        "default_duration_secs": 6,
        "min_duration_secs": 1,
        "max_duration_secs": 60,
        "duration_mode": "range",
        "supported_durations_json": null,
        "default_fps": 24,
        "is_default": false
    }))
    .expect_err("all-config Video rows must explicitly include api_key_encrypted");
    assert!(err.to_string().contains("api_key_encrypted"));
}

#[test]
fn export_header_validation_rejects_wrong_type_and_version() {
    let mut export =
        serde_json::from_value::<AllConfigsExport>(full_export_json()).expect("valid export");
    export.export_type = "theme_only".to_string();
    let err = validate_all_configs_export_header(&export).expect_err("wrong type should fail");
    assert!(err.contains("export_type 不匹配"));

    export.export_type = "all".to_string();
    export.version = "0.9".to_string();
    let err = validate_all_configs_export_header(&export).expect_err("wrong version should fail");
    assert!(err.contains("version 不匹配"));
}

#[test]
fn all_configs_payload_rejects_non_positive_llm_timeout() {
    let mut value = full_export_json();
    value["llm_configs"] = json!([{
        "name": "bad llm",
        "provider": "deepseek",
        "base_url": "https://api.deepseek.com/v1",
        "api_key_encrypted": "sk-test",
        "model": "deepseek-chat",
        "max_concurrent": 4,
        "timeout_secs": 0,
        "max_context_tokens": 128000,
        "max_output_tokens": 8192,
        "is_default": true
    }]);
    let export =
        serde_json::from_value::<AllConfigsExport>(value).expect("shape should deserialize");

    let err = validate_all_configs_export(&export).expect_err("bad timeout should fail");

    assert!(err.contains("timeout_secs 必须为正整数"));
}

#[test]
fn all_configs_payload_rejects_llm_max_tokens_above_default_model_cap() {
    let mut value = full_export_json();
    value["llm_configs"] = json!([{
        "name": "main llm",
        "provider": "deepseek",
        "base_url": "https://api.deepseek.com/v1",
        "api_key_encrypted": "sk-test",
        "model": "deepseek-chat",
        "max_concurrent": 4,
        "timeout_secs": 300,
        "max_context_tokens": 128000,
        "max_output_tokens": 8192,
        "is_default": true
    }]);
    let mut max_tokens =
        serde_json::to_value(LlmMaxTokensConfig::default()).expect("default tokens serialize");
    max_tokens["media_prompt"] = json!(8193);
    value["system_settings"] = json!([{
        "key": "llm_max_tokens",
        "value": max_tokens,
        "category": "llm",
        "description": null
    }]);
    let export =
        serde_json::from_value::<AllConfigsExport>(value).expect("shape should deserialize");

    let err = validate_all_configs_export(&export).expect_err("oversized budget should fail");

    assert!(err.contains("llm_max_tokens 无效"));
    assert!(err.contains("media_prompt=8193"));
}

#[test]
fn all_configs_payload_rejects_api_embedding_without_base_url() {
    let mut value = full_export_json();
    value["embedding_configs"] = json!([{
        "name": "api embedding",
        "backend": "api",
        "api_key_encrypted": null,
        "model": "text-embedding-3-small",
        "dimension": 1536,
        "batch_size": 100,
        "is_default": true
    }]);
    let export =
        serde_json::from_value::<AllConfigsExport>(value).expect("shape should deserialize");

    let err = validate_all_configs_export(&export).expect_err("missing base_url should fail");

    assert!(err.contains("必须填写 base_url"));
}

#[test]
fn all_configs_payload_rejects_non_positive_embedding_batch_size() {
    let mut value = full_export_json();
    value["embedding_configs"] = json!([{
        "name": "local embedding",
        "backend": "local_hash",
        "api_key_encrypted": null,
        "model": "local",
        "dimension": 8,
        "batch_size": 0,
        "is_default": false
    }]);
    let export =
        serde_json::from_value::<AllConfigsExport>(value).expect("shape should deserialize");

    let err = validate_all_configs_export(&export).expect_err("bad batch size should fail");

    assert!(err.contains("batch_size 必须大于 0"));
}

#[test]
fn all_configs_payload_rejects_invalid_image_backend() {
    let mut value = full_export_json();
    value["image_configs"] = json!([{
        "name": "bad image",
        "backend": "unknown-image",
        "base_url": "https://example.test/v1",
        "api_key_encrypted": "sk-test",
        "model": "image-model",
        "max_concurrent": 1,
        "default_width": 1024,
        "default_height": 1024,
        "is_default": true
    }]);
    let export =
        serde_json::from_value::<AllConfigsExport>(value).expect("shape should deserialize");

    let err = validate_all_configs_export(&export).expect_err("bad backend should fail");

    assert!(err.contains("未知图片生成后端"));
}

#[test]
fn all_configs_payload_rejects_invalid_video_backend() {
    let mut value = full_export_json();
    value["video_configs"] = json!([{
        "name": "bad video",
        "backend": "unknown-video",
        "base_url": "https://video.example/v1",
        "api_key_encrypted": "sk-test",
        "model": "video-model",
        "max_concurrent": 1,
        "default_width": 1280,
        "default_height": 720,
        "default_duration_secs": 5,
        "min_duration_secs": 1,
        "max_duration_secs": 60,
        "duration_mode": "range",
        "supported_durations_json": null,
        "default_fps": 24,
        "is_default": true
    }]);
    let export =
        serde_json::from_value::<AllConfigsExport>(value).expect("shape should deserialize");

    let err = validate_all_configs_export(&export).expect_err("bad backend should fail");

    assert!(err.contains("未知视频生成后端: unknown-video"));
}

#[test]
fn all_configs_payload_rejects_multiple_default_embedding_configs() {
    let mut value = full_export_json();
    value["embedding_configs"] = json!([
        {
            "name": "local one",
            "backend": "local_hash",
            "api_key_encrypted": null,
            "model": "local",
            "dimension": 8,
            "batch_size": 100,
            "is_default": true
        },
        {
            "name": "local two",
            "backend": "local_hash",
            "api_key_encrypted": null,
            "model": "local",
            "dimension": 8,
            "batch_size": 100,
            "is_default": true
        }
    ]);
    let export =
        serde_json::from_value::<AllConfigsExport>(value).expect("shape should deserialize");

    let err = validate_all_configs_export(&export).expect_err("multiple defaults should fail");

    assert!(err.contains("最多只能有一个默认项"));
}

#[test]
fn all_configs_payload_rejects_multiple_default_video_configs() {
    let mut value = full_export_json();
    value["video_configs"] = json!([
        {
            "name": "video one",
            "backend": "newapi-video",
            "base_url": "https://video.example/v1",
            "api_key_encrypted": "sk-test",
            "model": "video-model",
            "max_concurrent": 1,
            "default_width": 1280,
            "default_height": 720,
            "default_duration_secs": 5,
            "min_duration_secs": 1,
            "max_duration_secs": 60,
            "duration_mode": "range",
            "supported_durations_json": null,
            "default_fps": 24,
            "is_default": true
        },
        {
            "name": "video two",
            "backend": "openai-compatible-video",
            "base_url": "https://video2.example/v1",
            "api_key_encrypted": "sk-test-2",
            "model": "video-model-2",
            "max_concurrent": 1,
            "default_width": 1280,
            "default_height": 720,
            "default_duration_secs": 5,
            "min_duration_secs": 1,
            "max_duration_secs": 60,
            "duration_mode": "range",
            "supported_durations_json": null,
            "default_fps": 24,
            "is_default": true
        }
    ]);
    let export =
        serde_json::from_value::<AllConfigsExport>(value).expect("shape should deserialize");

    let err = validate_all_configs_export(&export).expect_err("multiple defaults should fail");

    assert!(err.contains("视频配置最多只能有一个默认项"));
}

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

#[test]
fn embedding_row_preserves_connection_fields_and_requires_batch_size() {
    let row = EmbeddingRow {
        name: "api embedding".to_string(),
        backend: "api".to_string(),
        base_url: Some("https://api.openai.com/v1".to_string()),
        api_key_encrypted: Some("sk-test".to_string()),
        model: "text-embedding-3-small".to_string(),
        dimension: 1536,
        batch_size: 64,
        local_model_path: Some("/models/cache".to_string()),
        is_default: true,
    };

    let value = serde_json::to_value(&row).expect("row should serialize");

    assert_eq!(value["base_url"], "https://api.openai.com/v1");
    assert_eq!(value["api_key_encrypted"], "sk-test");
    assert_eq!(value["batch_size"], 64);
    assert_eq!(value["local_model_path"], "/models/cache");

    let err = serde_json::from_value::<EmbeddingRow>(json!({
        "name": "missing batch size",
        "backend": "local_hash",
        "api_key_encrypted": null,
        "model": "local",
        "dimension": 8,
        "is_default": false
    }))
    .expect_err("batch_size is required by the current all-config schema");

    assert!(err.to_string().contains("batch_size"));
}

#[test]
fn llm_row_requires_model_token_limits_in_current_all_config_schema() {
    let err = serde_json::from_value::<LlmRow>(json!({
        "name": "missing token limits",
        "provider": "deepseek",
        "base_url": "https://api.deepseek.com/v1",
        "api_key_encrypted": "sk-test",
        "model": "deepseek-chat",
        "max_concurrent": 4,
        "timeout_secs": 60,
        "is_default": false
    }))
    .expect_err("LLM token limits are required by the current all-config schema");

    assert!(err.to_string().contains("max_context_tokens"));
}

#[test]
fn normalize_all_configs_export_trims_import_identities_without_touching_sensitive_content() {
    let mut value = full_export_json();
    value["llm_configs"] = json!([{
        "name": " primary ",
        "provider": " deepseek ",
        "base_url": " https://api.deepseek.com/v1 ",
        "api_key_encrypted": "  encrypted-secret  ",
        "model": " deepseek-chat ",
        "max_concurrent": 4,
        "timeout_secs": 60,
        "max_context_tokens": 128000,
        "max_output_tokens": 8192,
        "is_default": true
    }]);
    value["embedding_configs"] = json!([{
        "name": " local embedding ",
        "backend": " local_hash ",
        "base_url": "   ",
        "api_key_encrypted": null,
        "model": " local ",
        "dimension": 8,
        "batch_size": 100,
        "is_default": false
    }]);
    value["prompts_modified"] = json!([{
        "name": " chapter_prompt ",
        "content": "  keep prompt whitespace  ",
        "title": null,
        "description": null
    }]);
    value["system_settings"] = json!([{
        "key": " llm_max_tokens ",
        "value": {
            "connection_test": 8,
            "short": 1024,
            "standard": 2048,
            "json_standard": 4096,
            "long": 8192,
            "extra_long": 8192,
            "chapter_generation": 8192,
            "chapter_analysis": 4096,
            "outline": 8192,
            "blueprint": 8192,
            "character_discovery": 8192,
            "character_sync": 8192,
            "media_prompt": 4096
        },
        "category": " ai ",
        "description": null
    }]);
    let mut export =
        serde_json::from_value::<AllConfigsExport>(value).expect("shape should deserialize");

    validate_all_configs_export(&export).expect("payload should be valid before normalize");
    normalize_all_configs_export(&mut export).expect("payload should normalize");

    assert_eq!(export.llm_configs[0].name, "primary");
    assert_eq!(export.llm_configs[0].provider, "deepseek");
    assert_eq!(
        export.llm_configs[0].base_url,
        "https://api.deepseek.com/v1"
    );
    assert_eq!(export.llm_configs[0].model, "deepseek-chat");
    assert_eq!(
        export.llm_configs[0].api_key_encrypted.as_deref(),
        Some("  encrypted-secret  ")
    );
    assert_eq!(export.embedding_configs[0].name, "local embedding");
    assert_eq!(export.embedding_configs[0].backend, "local_hash");
    assert_eq!(export.embedding_configs[0].base_url, None);
    assert_eq!(export.prompts_modified[0].name, "chapter_prompt");
    assert_eq!(
        export.prompts_modified[0].content,
        "  keep prompt whitespace  "
    );
    assert_eq!(export.system_settings[0].key, "llm_max_tokens");
    assert_eq!(export.system_settings[0].category, "ai");
}

#[test]
fn normalize_all_configs_export_trims_canonical_embedding_backends() {
    let mut value = full_export_json();
    value["embedding_configs"] = json!([
        {
            "name": "api embedding",
            "backend": " api ",
            "base_url": " https://api.openai.com/v1 ",
            "api_key_encrypted": null,
            "model": "text-embedding-3-small",
            "dimension": 768,
            "batch_size": 100,
            "is_default": false
        },
        {
            "name": "fastembed embedding",
            "backend": " fastembed ",
            "api_key_encrypted": null,
            "model": "BGESmallZHV15",
            "dimension": 512,
            "batch_size": 100,
            "is_default": false
        },
        {
            "name": "local embedding",
            "backend": " local_hash ",
            "api_key_encrypted": null,
            "model": "local",
            "dimension": 8,
            "batch_size": 100,
            "is_default": false
        }
    ]);
    let mut export =
        serde_json::from_value::<AllConfigsExport>(value).expect("shape should deserialize");

    validate_all_configs_export(&export).expect("canonical backends should validate");
    normalize_all_configs_export(&mut export).expect("canonical backends should normalize");

    let backends = export
        .embedding_configs
        .iter()
        .map(|row| row.backend.as_str())
        .collect::<Vec<_>>();
    assert_eq!(backends, vec!["api", "fastembed", "local_hash"]);
}

#[test]
fn normalize_all_configs_export_normalizes_valid_video_config_fields() {
    let mut value = full_export_json();
    value["video_configs"] = json!([{
        "name": " default video ",
        "backend": " newapi-video ",
        "base_url": " https://video.example/v1 ",
        "api_key_encrypted": " secret ",
        "model": " video-model ",
        "max_concurrent": 2,
        "default_width": 1280,
        "default_height": 720,
        "default_duration_secs": 10,
        "min_duration_secs": 5,
        "max_duration_secs": 10,
        "duration_mode": " fixed ",
        "supported_durations_json": [5, 10],
        "default_fps": 24,
        "is_default": true
    }]);
    let mut export =
        serde_json::from_value::<AllConfigsExport>(value).expect("shape should deserialize");

    validate_all_configs_export(&export).expect("payload should be valid before normalize");
    normalize_all_configs_export(&mut export).expect("valid video config should normalize");

    let row = &export.video_configs[0];
    assert_eq!(row.name, "default video");
    assert_eq!(row.backend, "newapi-video");
    assert_eq!(row.base_url.as_deref(), Some("https://video.example/v1"));
    assert_eq!(row.api_key_encrypted.as_deref(), Some("secret"));
    assert_eq!(row.model, "video-model");
    assert_eq!(row.duration_mode, "fixed");
    assert_eq!(row.min_duration_secs, 5);
    assert_eq!(row.max_duration_secs, 10);
    assert_eq!(row.supported_durations_json, Some(json!([5, 10])));
}

#[test]
fn normalize_all_configs_export_returns_explicit_error_when_config_normalization_fails() {
    let mut export = serde_json::from_value::<AllConfigsExport>(json!({
        "version": "1.0",
        "export_time": "2026-05-27T00:00:00Z",
        "export_type": "all",
        "llm_configs": [],
        "embedding_configs": [],
        "image_configs": [{
            "name": "broken image",
            "backend": "unknown-image-backend",
            "base_url": "https://image.example/v1",
            "api_key_encrypted": "secret",
            "model": "image-model",
            "max_concurrent": 1,
            "default_width": 1024,
            "default_height": 1024,
            "is_default": false
        }],
        "video_configs": [],
        "prompts_modified": [],
        "themes": [],
        "system_settings": []
    }))
    .expect("shape should deserialize");

    let err = normalize_all_configs_export(&mut export)
        .expect_err("normalization failures must be explicit");

    assert!(err.contains("图片配置 #1 规范化失败"));
    assert!(err.contains("未知图片生成后端"));
}
