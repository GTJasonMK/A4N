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
        serde_json::to_value(a4n_core::config::app_config::LlmMaxTokensConfig::default())
            .expect("default tokens serialize");
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
