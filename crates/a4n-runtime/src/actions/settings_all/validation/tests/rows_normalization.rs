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
