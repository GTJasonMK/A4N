use a4n_db::config::{
    SettingsAllConfigImportInput, SettingsAllConfigRows, SettingsAllEmbeddingConfigRow,
    SettingsAllImageConfigRow, SettingsAllLlmConfigRow, SettingsAllSystemSettingRow,
    SettingsAllThemeRow, SettingsAllVideoConfigRow, import_settings_all_configs,
    settings_all_config_rows, validate_settings_all_config_import_defaults,
};
use a4n_domain::{ImageSize, ThemeDocument, ThemeMode};
use serde_json::json;

fn settings_rows() -> SettingsAllConfigRows {
    SettingsAllConfigRows {
        llm_configs: vec![SettingsAllLlmConfigRow {
            name: "settings-all-llm".to_string(),
            provider: "openai".to_string(),
            base_url: "https://llm.example.test/v1".to_string(),
            api_key_encrypted: None,
            model: "test-llm".to_string(),
            max_concurrent: 1,
            timeout_secs: 30,
            max_context_tokens: 8192,
            max_output_tokens: 2048,
            extra_body_json: None,
            is_default: true,
        }],
        embedding_configs: vec![SettingsAllEmbeddingConfigRow {
            name: "settings-all-embedding".to_string(),
            backend: "fastembed".to_string(),
            base_url: None,
            api_key_encrypted: None,
            model: "BAAI/bge-small-zh-v1.5".to_string(),
            dimension: 512,
            batch_size: 16,
            local_model_path: None,
            is_default: true,
        }],
        image_configs: vec![SettingsAllImageConfigRow {
            name: "settings-all-image".to_string(),
            backend: "openai-compatible-image".to_string(),
            base_url: "https://image.example.test/v1".to_string(),
            api_key_encrypted: None,
            model: "test-image".to_string(),
            max_concurrent: 1,
            supported_sizes: vec![ImageSize::new(1024, 1024)],
            is_default: true,
        }],
        video_configs: vec![SettingsAllVideoConfigRow {
            name: "settings-all-video".to_string(),
            backend: "openai-compatible-video".to_string(),
            base_url: "https://video.example.test/v1".to_string(),
            api_key_encrypted: None,
            model: "test-video".to_string(),
            max_concurrent: 1,
            default_width: 1280,
            default_height: 720,
            default_duration_secs: 5,
            min_duration_secs: 1,
            max_duration_secs: 10,
            duration_mode: "range".to_string(),
            supported_durations_json: None,
            default_fps: 24,
            is_default: true,
        }],
        themes: vec![SettingsAllThemeRow {
            user_id: None,
            name: "settings-all-theme".to_string(),
            mode: ThemeMode::Light,
            document: ThemeDocument {
                tokens: std::collections::BTreeMap::from([(
                    "--color-primary".to_string(),
                    "18 52 86".to_string(),
                )]),
                ..Default::default()
            },
            is_system_default: true,
            is_active: true,
        }],
        system_settings: vec![SettingsAllSystemSettingRow {
            key: "settings_all_roundtrip".to_string(),
            value: json!({"enabled": true}),
            category: "test".to_string(),
            description: None,
        }],
    }
}

#[tokio::test]
async fn settings_all_rows_round_trip_and_skip_existing_rows() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let input = SettingsAllConfigImportInput {
        overwrite: false,
        rows: settings_rows(),
    };

    validate_settings_all_config_import_defaults(&db, &input)
        .await
        .expect("settings-all defaults must be valid");
    let imported = import_settings_all_configs(&db, input.clone())
        .await
        .expect("settings-all rows must import");
    for count in [
        imported.llm,
        imported.embedding,
        imported.image,
        imported.video,
        imported.theme,
        imported.system_settings,
    ] {
        assert_eq!(count.imported, 1);
        assert_eq!(count.skipped, 0);
    }

    let exported = settings_all_config_rows(&db)
        .await
        .expect("settings-all rows must export");
    assert!(
        exported
            .llm_configs
            .iter()
            .any(|row| row.name == "settings-all-llm")
    );
    assert!(
        exported
            .embedding_configs
            .iter()
            .any(|row| row.name == "settings-all-embedding" && row.base_url.is_none())
    );
    assert!(exported.image_configs.iter().any(|row| {
        row.name == "settings-all-image" && row.base_url == "https://image.example.test/v1"
    }));
    assert!(exported.video_configs.iter().any(|row| {
        row.name == "settings-all-video" && row.base_url == "https://video.example.test/v1"
    }));
    assert!(
        exported
            .themes
            .iter()
            .any(|row| row.name == "settings-all-theme" && row.mode == ThemeMode::Light)
    );
    assert!(
        exported
            .system_settings
            .iter()
            .any(|row| row.key == "settings_all_roundtrip")
    );

    let skipped = import_settings_all_configs(&db, input)
        .await
        .expect("existing settings-all rows must be skipped");
    for count in [
        skipped.llm,
        skipped.embedding,
        skipped.image,
        skipped.video,
        skipped.theme,
        skipped.system_settings,
    ] {
        assert_eq!(count.imported, 0);
        assert_eq!(count.skipped, 1);
    }
}

#[tokio::test]
async fn settings_all_rejects_multiple_incoming_defaults() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let mut rows = settings_rows();
    let mut duplicate = rows.llm_configs[0].clone();
    duplicate.name = "settings-all-second-default-llm".to_string();
    rows.llm_configs.push(duplicate);

    let error = validate_settings_all_config_import_defaults(
        &db,
        &SettingsAllConfigImportInput {
            overwrite: false,
            rows,
        },
    )
    .await
    .expect_err("multiple incoming defaults must be rejected");
    assert!(error.to_string().contains("只能声明一个默认 LLM 配置"));
}
