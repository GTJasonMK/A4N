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
