use super::validation::validate_image_config_params;
use super::{
    ConfigService, CreateEmbeddingConfigParams, CreateImageConfigParams, CreateLlmConfigInput,
    UpdateEmbeddingConfigParams, UpdateImageConfigParams, UpdateLlmConfigInput,
};
use crate::video::{UpdateVideoConfigParams, VideoConfigParams};
use a4n_core::error::AppError;
use a4n_core::models::ai::{embedding_config, generated_video, llm_config};
use a4n_core::models::project::novel_project;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};
use serde_json::json;
use std::sync::Arc;

async fn setup_db() -> DatabaseConnection {
    a4n_migration::test_support::connect_test_database().await
}

fn test_database_available() -> bool {
    std::env::var("A4N_TEST_DATABASE_URL").is_ok()
}

fn llm_params(name: &'static str) -> CreateLlmConfigInput<'static> {
    CreateLlmConfigInput {
        name,
        provider: "openai",
        base_url: "https://api.openai.com/v1",
        api_key_encrypted: Some("sk-test"),
        model: "gpt-4.1",
        max_concurrent: 2,
        timeout_secs: 60,
        max_context_tokens: 128_000,
        max_output_tokens: 8_192,
        request_options_json: None,
    }
}

fn params() -> CreateImageConfigParams {
    image_params("OpenAI")
}

fn image_params(name: &str) -> CreateImageConfigParams {
    CreateImageConfigParams {
        name: name.into(),
        backend: "openai-compatible-image".into(),
        base_url: "https://api.openai.com/v1".into(),
        api_key_encrypted: Some("sk-test".into()),
        model: "dall-e-3".into(),
        max_concurrent: 1,
        default_width: 1024,
        default_height: 1024,
        provider_options_json: None,
    }
}

fn image_update_params(name: &str) -> UpdateImageConfigParams {
    let params = image_params(name);
    UpdateImageConfigParams {
        name: params.name,
        backend: params.backend,
        base_url: params.base_url,
        api_key_encrypted: params.api_key_encrypted.map(Some),
        model: params.model,
        max_concurrent: params.max_concurrent,
        default_width: params.default_width,
        default_height: params.default_height,
        provider_options_json: params.provider_options_json,
    }
}

fn embedding_params(name: &str) -> CreateEmbeddingConfigParams {
    CreateEmbeddingConfigParams {
        name: name.to_string(),
        backend: "local_hash".to_string(),
        base_url: None,
        api_key_encrypted: None,
        model: "local".to_string(),
        dimension: 8,
        batch_size: 100,
        local_model_path: None,
    }
}

fn embedding_update_params(name: &str) -> UpdateEmbeddingConfigParams {
    let params = embedding_params(name);
    UpdateEmbeddingConfigParams {
        name: params.name,
        backend: params.backend,
        base_url: params.base_url,
        api_key_encrypted: params.api_key_encrypted.map(Some),
        model: params.model,
        dimension: params.dimension,
        batch_size: params.batch_size,
        local_model_path: params.local_model_path,
    }
}

fn video_params(name: &str) -> VideoConfigParams {
    VideoConfigParams {
        name: name.to_string(),
        backend: "openai-compatible-video".to_string(),
        base_url: "https://api.openai.com/v1".to_string(),
        api_key_encrypted: Some("sk-test".to_string()),
        model: "video-model".to_string(),
        max_concurrent: 1,
        default_width: 1280,
        default_height: 720,
        default_duration_secs: 5,
        min_duration_secs: 1,
        max_duration_secs: 10,
        duration_mode: "range".to_string(),
        supported_durations_json: None,
        default_fps: 24,
        provider_options_json: None,
    }
}

fn video_update_params(name: &str) -> UpdateVideoConfigParams {
    let params = video_params(name);
    UpdateVideoConfigParams {
        name: params.name,
        backend: params.backend,
        base_url: params.base_url,
        api_key_encrypted: params.api_key_encrypted.map(Some),
        model: params.model,
        max_concurrent: params.max_concurrent,
        default_width: params.default_width,
        default_height: params.default_height,
        default_duration_secs: params.default_duration_secs,
        min_duration_secs: params.min_duration_secs,
        max_duration_secs: params.max_duration_secs,
        duration_mode: params.duration_mode,
        supported_durations_json: params.supported_durations_json,
        default_fps: params.default_fps,
        provider_options_json: params.provider_options_json,
    }
}

async fn insert_test_project(db: &DatabaseConnection, title: &str) -> novel_project::Model {
    let now = chrono::Utc::now().naive_utc();
    novel_project::ActiveModel {
        user_id: Set(1),
        title: Set(title.to_string()),
        description: Set("配置测试项目".to_string()),
        project_type: Set("novel_long".to_string()),
        project_kind: Set("text".to_string()),
        source_mode: Set("self".to_string()),
        root_project_id: Set(None),
        parent_project_id: Set(None),
        source_snapshot_id: Set(None),
        source_sync_status: Set("current".to_string()),
        derived_project_status: Set(None),
        status: Set("draft".to_string()),
        target_chapters: Set(10),
        current_chapter: Set(0),
        total_chapters: Set(Some(10)),
        chapters_per_part: Set(Some(5)),
        avatar_svg: Set(None),
        avatar_animal: Set(None),
        raw_blueprint: Set(None),
        import_analysis_progress: Set(None),
        export_config: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}

async fn insert_generated_video_for_config(
    db: &DatabaseConnection,
    project_id: i64,
    config_id: i64,
    status: &str,
    created_at: chrono::NaiveDateTime,
) -> generated_video::Model {
    generated_video::ActiveModel {
        project_id: Set(project_id),
        chapter_id: Set(None),
        video_type: Set("chapter_video".to_string()),
        workflow_mode: Set(Some("first_frame_chain".to_string())),
        file_path: Set(None),
        thumbnail_path: Set(None),
        prompt: Set("test prompt".to_string()),
        negative_prompt: Set(None),
        model: Set("video-model".to_string()),
        width: Set(1280),
        height: Set(720),
        duration_secs: Set(5.0),
        fps: Set(24),
        file_size: Set(None),
        source_video_id: Set(None),
        source_image_id: Set(None),
        status: Set(status.to_string()),
        provider_job_id: Set(None),
        error_message: Set(None),
        metadata_json: Set(Some(json!({ "config_id": config_id }))),
        is_favorite: Set(false),
        created_at: Set(created_at),
        updated_at: Set(created_at),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}

#[test]
fn validates_image_config_requires_base_url() {
    let mut params = params();
    params.base_url = " ".into();
    let err = validate_image_config_params(params).unwrap_err();
    assert!(matches!(err, AppError::InvalidArgument(_)));
}

#[test]
fn validates_openai_image_config_allows_template_without_api_key() {
    let mut params = params();
    params.api_key_encrypted = None;
    let params = validate_image_config_params(params)
        .expect("image config templates may omit user-owned API keys");
    assert_eq!(params.api_key_encrypted, None);
}

#[test]
fn validates_image_config_rejects_unknown_backend() {
    let mut params = params();
    params.backend = "unsupported-image-backend".into();
    let err = validate_image_config_params(params).unwrap_err();
    assert!(matches!(err, AppError::InvalidArgument(_)));
}

#[test]
fn validates_openai_chat_image_backend_accepted() {
    let mut params = params();
    params.backend = "openai-chat-image".into();
    params.api_key_encrypted = Some("sk-test".into());
    assert!(validate_image_config_params(params).is_ok());
}

#[test]
fn validates_openai_chat_image_allows_template_without_api_key() {
    let mut params = params();
    params.backend = "openai-chat-image".into();
    params.api_key_encrypted = None;
    let params = validate_image_config_params(params)
        .expect("image config templates may omit user-owned API keys");
    assert_eq!(params.api_key_encrypted, None);
}

#[test]
fn validates_openai_image_edit_backend_accepted() {
    let mut params = params();
    params.backend = "openai-compatible-image-edit".into();
    params.api_key_encrypted = Some("sk-test".into());
    assert!(validate_image_config_params(params).is_ok());
}

#[test]
fn validates_openai_image_edit_allows_template_without_api_key() {
    let mut params = params();
    params.backend = "openai-compatible-image-edit".into();
    params.api_key_encrypted = None;
    let params = validate_image_config_params(params)
        .expect("image config templates may omit user-owned API keys");
    assert_eq!(params.api_key_encrypted, None);
}

#[test]
fn validates_unknown_image_backend_rejected() {
    let mut params = params();
    params.backend = "unsupported-image-backend".into();
    params.api_key_encrypted = Some("sk-test".into());
    let err = validate_image_config_params(params).unwrap_err();
    assert!(matches!(err, AppError::InvalidArgument(_)));
}

#[test]
fn validates_image_provider_options_rejects_non_cliproxy_backend() {
    let mut params = params();
    params.provider_options_json = Some(json!({
        "grok_reference_mode": "first_frame",
    }));

    let err = validate_image_config_params(params)
        .expect_err("non-cliproxy image backend must not silently drop provider options");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(
        err.to_string()
            .contains("provider_options_json 仅支持 cliproxy-image")
    );
}

#[test]
fn validates_image_provider_options_rejects_unknown_cliproxy_keys() {
    let mut params = params();
    params.backend = "cliproxy-image".into();
    params.provider_options_json = Some(json!({
        "grok_reference_mode": "reference",
        "legacy_mode": true,
    }));

    let err = validate_image_config_params(params)
        .expect_err("unknown cliproxy image provider option must fail visibly");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(
        err.to_string()
            .contains("provider_options_json 包含未知字段：legacy_mode")
    );
}

#[test]
fn validates_image_provider_options_normalizes_cliproxy_mode() {
    let mut params = params();
    params.backend = "cliproxy-image".into();
    params.provider_options_json = Some(json!({
        "grok_reference_mode": " first_frame ",
    }));

    let params = validate_image_config_params(params).expect("valid cliproxy image options");

    assert_eq!(
        params.provider_options_json,
        Some(json!({
            "grok_reference_mode": "first_frame",
        }))
    );
}

#[test]
fn default_config_delete_guard_rejects_default_configs() {
    for kind in [" LLM 配置", "图片配置", "视频配置"] {
        let err = super::ensure_default_config_delete_allowed(kind, true)
            .expect_err("default config deletion should fail visibly");

        assert!(matches!(err, AppError::InvalidArgument(_)));
        assert!(
            err.to_string()
                .contains(&format!("不能删除默认{kind}，请先设置其它默认配置"))
        );
    }
}

#[test]
fn default_config_delete_guard_allows_non_default_configs() {
    for kind in [" LLM 配置", "图片配置", "视频配置"] {
        super::ensure_default_config_delete_allowed(kind, false)
            .expect("non-default config deletion should remain allowed");
    }
}

#[tokio::test]
async fn create_llm_rejects_duplicate_trimmed_names() {
    let service = ConfigService::new(Arc::new(setup_db().await));
    service
        .create_llm(llm_params("primary"))
        .await
        .expect("first config should be created");

    let err = service
        .create_llm(CreateLlmConfigInput {
            name: " primary ",
            provider: "deepseek",
            base_url: "https://api.deepseek.com/v1",
            api_key_encrypted: Some("sk-test-2"),
            model: "deepseek-chat",
            max_concurrent: 4,
            timeout_secs: 90,
            ..llm_params("secondary")
        })
        .await
        .expect_err("duplicate business name should be rejected");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(err.to_string().contains("LLM配置名称已存在"));
    assert_eq!(service.list_llm().await.unwrap().len(), 1);
}

#[tokio::test]
async fn create_llm_rejects_invalid_runtime_fields_before_insert() {
    let service = ConfigService::new(Arc::new(setup_db().await));

    let err = service
        .create_llm(CreateLlmConfigInput {
            name: "broken",
            provider: " ",
            base_url: " ",
            api_key_encrypted: Some("sk-test"),
            model: " ",
            max_concurrent: 0,
            timeout_secs: 0,
            max_context_tokens: 0,
            max_output_tokens: 0,
            request_options_json: None,
        })
        .await
        .expect_err("invalid LLM runtime fields should be rejected");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(err.to_string().contains("LLM配置缺少 provider"));
    assert_eq!(service.list_llm().await.unwrap().len(), 0);
}

#[tokio::test]
async fn create_llm_persists_provider_request_options() {
    let service = ConfigService::new(Arc::new(setup_db().await));

    let config = service
        .create_llm(CreateLlmConfigInput {
            request_options_json: Some(json!({ "reasoning_effort": "low" })),
            ..llm_params("primary")
        })
        .await
        .expect("request options should be accepted");

    assert_eq!(
        config.request_options_json,
        Some(json!({ "reasoning_effort": "low" }))
    );
    assert_eq!(config.max_context_tokens, 128_000);
    assert_eq!(config.max_output_tokens, 8_192);
}

#[tokio::test]
async fn create_llm_rejects_output_limit_at_or_above_context_window() {
    let service = ConfigService::new(Arc::new(setup_db().await));

    let err = service
        .create_llm(CreateLlmConfigInput {
            provider: "deepseek",
            base_url: "https://api.deepseek.com/v1",
            model: "deepseek-chat",
            max_context_tokens: 8_192,
            max_output_tokens: 8_192,
            ..llm_params("primary")
        })
        .await
        .expect_err("output limit must be smaller than context window");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(
        err.to_string()
            .contains("max_output_tokens 必须小于 max_context_tokens")
    );
}

#[tokio::test]
async fn create_llm_rejects_reserved_request_option_keys() {
    let service = ConfigService::new(Arc::new(setup_db().await));

    let err = service
        .create_llm(CreateLlmConfigInput {
            request_options_json: Some(json!({ "model": "override" })),
            ..llm_params("primary")
        })
        .await
        .expect_err("reserved request option should be rejected");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(err.to_string().contains("系统保留字段"));
}

#[tokio::test]
async fn update_llm_rejects_renaming_to_existing_name() {
    let service = ConfigService::new(Arc::new(setup_db().await));
    service.create_llm(llm_params("primary")).await.unwrap();
    let secondary = service
        .create_llm(CreateLlmConfigInput {
            provider: "deepseek",
            base_url: "https://api.deepseek.com/v1",
            api_key_encrypted: Some("sk-test-2"),
            model: "deepseek-chat",
            max_concurrent: 4,
            timeout_secs: 90,
            ..llm_params("secondary")
        })
        .await
        .unwrap();

    let err = service
        .update_llm(
            secondary.id,
            UpdateLlmConfigInput {
                name: Some(" primary "),
                ..Default::default()
            },
        )
        .await
        .expect_err("rename to an existing name should be rejected");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    let configs = service.list_llm().await.unwrap();
    assert!(configs.iter().any(|config| config.name == "primary"));
    assert!(configs.iter().any(|config| config.name == "secondary"));
}

#[tokio::test]
async fn update_llm_rejects_invalid_runtime_fields_without_update() {
    let service = ConfigService::new(Arc::new(setup_db().await));
    let config = service.create_llm(llm_params("primary")).await.unwrap();

    let err = service
        .update_llm(
            config.id,
            UpdateLlmConfigInput {
                base_url: Some("   "),
                model: Some("gpt-4.1-mini"),
                max_concurrent: Some(0),
                ..Default::default()
            },
        )
        .await
        .expect_err("invalid update should be rejected before mutating the row");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(err.to_string().contains("LLM配置缺少 base_url"));
    let stored = service.get_llm(config.id).await.unwrap();
    assert_eq!(stored.base_url, "https://api.openai.com/v1");
    assert_eq!(stored.model, "gpt-4.1");
    assert_eq!(stored.max_concurrent, 2);
}

#[tokio::test]
async fn update_llm_rejects_invalid_model_capability_relation() {
    let service = ConfigService::new(Arc::new(setup_db().await));
    let config = service
        .create_llm(CreateLlmConfigInput {
            provider: "deepseek",
            base_url: "https://api.deepseek.com/v1",
            model: "deepseek-chat",
            ..llm_params("primary")
        })
        .await
        .unwrap();

    let err = service
        .update_llm(
            config.id,
            UpdateLlmConfigInput {
                max_context_tokens: Some(8_192),
                ..Default::default()
            },
        )
        .await
        .expect_err("context window cannot be reduced to the output limit");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(
        err.to_string()
            .contains("max_output_tokens 必须小于 max_context_tokens")
    );
    let stored = service.get_llm(config.id).await.unwrap();
    assert_eq!(stored.max_context_tokens, 128_000);
    assert_eq!(stored.max_output_tokens, 8_192);
}

#[tokio::test]
async fn update_llm_persists_provider_changes() {
    let service = ConfigService::new(Arc::new(setup_db().await));
    let config = service
        .create_llm(CreateLlmConfigInput {
            provider: "deepseek",
            base_url: "https://api.deepseek.com/v1",
            model: "deepseek-chat",
            ..llm_params("primary")
        })
        .await
        .unwrap();

    let updated = service
        .update_llm(
            config.id,
            UpdateLlmConfigInput {
                provider: Some(" openai "),
                base_url: Some("https://api.openai.com/v1"),
                model: Some("gpt-4.1-mini"),
                ..Default::default()
            },
        )
        .await
        .expect("provider change should be persisted");

    assert_eq!(updated.provider, "openai");
    assert_eq!(updated.base_url, "https://api.openai.com/v1");
    assert_eq!(updated.model, "gpt-4.1-mini");
    let stored = service.get_llm(config.id).await.unwrap();
    assert_eq!(stored.provider, "openai");
}

#[tokio::test]
async fn update_llm_can_explicitly_clear_api_key() {
    if !test_database_available() {
        eprintln!("skip update_llm_can_explicitly_clear_api_key: A4N_TEST_DATABASE_URL 未设置");
        return;
    }

    let service = ConfigService::new(Arc::new(setup_db().await));
    let config = service.create_llm(llm_params("primary")).await.unwrap();

    let updated = service
        .update_llm(
            config.id,
            UpdateLlmConfigInput {
                api_key_encrypted: Some(None),
                ..Default::default()
            },
        )
        .await
        .expect("explicit null api key should clear the stored key");

    assert_eq!(updated.api_key_encrypted, None);
    assert_eq!(
        service.get_llm(config.id).await.unwrap().api_key_encrypted,
        None
    );
}

#[tokio::test]
async fn set_default_llm_rejects_invalid_existing_config_without_defaulting_it() {
    let db = setup_db().await;
    let service = ConfigService::new(Arc::new(db.clone()));
    let now = chrono::Utc::now().naive_utc();
    let invalid = llm_config::ActiveModel {
        name: Set("broken".to_string()),
        provider: Set(" ".to_string()),
        base_url: Set(" ".to_string()),
        api_key_encrypted: Set(Some("sk-test".to_string())),
        model: Set(" ".to_string()),
        max_concurrent: Set(0),
        timeout_secs: Set(0),
        max_context_tokens: Set(0),
        max_output_tokens: Set(0),
        is_default: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    let err = service
        .set_default_llm(invalid.id)
        .await
        .expect_err("invalid legacy LLM config must not become default");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(err.to_string().contains("LLM配置缺少 provider"));
    let stored = llm_config::Entity::find_by_id(invalid.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert!(!stored.is_default);
}

#[tokio::test]
async fn delete_llm_rejects_missing_config_without_touching_existing_rows() {
    let service = ConfigService::new(Arc::new(setup_db().await));
    service.create_llm(llm_params("primary")).await.unwrap();

    let err = service
        .delete_llm(404)
        .await
        .expect_err("deleting a missing LLM config should fail visibly");

    assert!(matches!(
        err,
        AppError::NotFound(message) if message == "LLM配置不存在: 404"
    ));
    let configs = service.list_llm().await.unwrap();
    assert_eq!(configs.len(), 1);
    assert_eq!(configs[0].name, "primary");
}

#[tokio::test]
async fn create_embedding_rejects_duplicate_trimmed_names() {
    let service = ConfigService::new(Arc::new(setup_db().await));
    service
        .create_embedding(embedding_params("local"))
        .await
        .expect("first config should be created");

    let err = service
        .create_embedding(embedding_params(" local "))
        .await
        .expect_err("duplicate business name should be rejected");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(err.to_string().contains("嵌入配置名称已存在"));
    assert_eq!(service.list_embedding().await.unwrap().len(), 1);
}

#[tokio::test]
async fn create_embedding_rejects_invalid_runtime_fields_before_insert() {
    let service = ConfigService::new(Arc::new(setup_db().await));
    let mut params = embedding_params("broken");
    params.backend = "api".to_string();
    params.base_url = Some("   ".to_string());
    params.model = " ".to_string();
    params.dimension = 0;
    params.batch_size = 0;

    let err = service
        .create_embedding(params)
        .await
        .expect_err("invalid embedding config should be rejected");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(err.to_string().contains("嵌入配置缺少 model"));
    assert_eq!(service.list_embedding().await.unwrap().len(), 0);
}

#[tokio::test]
async fn create_embedding_rejects_fastembed_dimension_mismatch() {
    let service = ConfigService::new(Arc::new(setup_db().await));
    let mut params = embedding_params("fastembed-broken");
    params.backend = "fastembed".to_string();
    params.model = "BGESmallZHV15".to_string();
    params.dimension = 8;

    let err = service
        .create_embedding(params)
        .await
        .expect_err("fastembed dimension mismatch should be rejected");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(err.to_string().contains("实际输出 512 维"));
    assert_eq!(service.list_embedding().await.unwrap().len(), 0);
}

#[tokio::test]
async fn create_embedding_rejects_unknown_backend_without_insert() {
    let service = ConfigService::new(Arc::new(setup_db().await));
    let mut params = embedding_params("unknown-backend");
    params.backend = "unknown-runtime".to_string();

    let err = service
        .create_embedding(params)
        .await
        .expect_err("unknown embedding backend should be rejected");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(err.to_string().contains("不支持的嵌入后端"));
    assert_eq!(service.list_embedding().await.unwrap().len(), 0);
}

#[tokio::test]
async fn update_embedding_rejects_renaming_to_existing_name() {
    let service = ConfigService::new(Arc::new(setup_db().await));
    service
        .create_embedding(embedding_params("primary"))
        .await
        .unwrap();
    let secondary = service
        .create_embedding(embedding_params("secondary"))
        .await
        .unwrap();

    let err = service
        .update_embedding(secondary.id, embedding_update_params(" primary "))
        .await
        .expect_err("rename to an existing embedding config should be rejected");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(err.to_string().contains("嵌入配置名称已存在"));
    let configs = service.list_embedding().await.unwrap();
    assert!(configs.iter().any(|config| config.name == "primary"));
    assert!(configs.iter().any(|config| config.name == "secondary"));
}

#[tokio::test]
async fn update_embedding_rejects_invalid_runtime_fields_without_update() {
    let service = ConfigService::new(Arc::new(setup_db().await));
    let config = service
        .create_embedding(embedding_params("local"))
        .await
        .unwrap();
    let mut update = embedding_update_params("local");
    update.backend = "api".to_string();
    update.base_url = Some("   ".to_string());
    update.model = "updated-model".to_string();

    let err = service
        .update_embedding(config.id, update)
        .await
        .expect_err("invalid embedding update should be rejected before mutation");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(err.to_string().contains("api 嵌入后端必须填写 base_url"));
    let stored = service.get_embedding(config.id).await.unwrap();
    assert_eq!(stored.backend, "local_hash");
    assert_eq!(stored.base_url, None);
    assert_eq!(stored.model, "local");
}

#[tokio::test]
async fn update_embedding_can_explicitly_clear_optional_api_key() {
    if !test_database_available() {
        eprintln!(
            "skip update_embedding_can_explicitly_clear_optional_api_key: A4N_TEST_DATABASE_URL 未设置"
        );
        return;
    }

    let service = ConfigService::new(Arc::new(setup_db().await));
    let mut params = embedding_params("api-embedding");
    params.backend = "api".to_string();
    params.base_url = Some("https://api.openai.com/v1".to_string());
    params.api_key_encrypted = Some("sk-test".to_string());
    params.model = "text-embedding-3-small".to_string();
    params.dimension = 1536;
    let config = service.create_embedding(params).await.unwrap();

    let mut update = embedding_update_params("api-embedding");
    update.backend = "api".to_string();
    update.base_url = Some("https://api.openai.com/v1".to_string());
    update.api_key_encrypted = Some(None);
    update.model = "text-embedding-3-small".to_string();
    update.dimension = 1536;

    let updated = service
        .update_embedding(config.id, update)
        .await
        .expect("api embedding key is optional and explicit null should clear it");

    assert_eq!(updated.api_key_encrypted, None);
    assert_eq!(
        service
            .get_embedding(config.id)
            .await
            .unwrap()
            .api_key_encrypted,
        None
    );
}

#[tokio::test]
async fn set_default_embedding_rejects_invalid_existing_config_without_defaulting_it() {
    let db = setup_db().await;
    let service = ConfigService::new(Arc::new(db.clone()));
    let now = chrono::Utc::now().naive_utc();
    let invalid = embedding_config::ActiveModel {
        name: Set("broken".to_string()),
        backend: Set("api".to_string()),
        base_url: Set(None),
        api_key_encrypted: Set(None),
        model: Set(" ".to_string()),
        dimension: Set(0),
        batch_size: Set(0),
        local_model_path: Set(None),
        is_default: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    let err = service
        .set_default_embedding(invalid.id)
        .await
        .expect_err("invalid legacy config must not become default");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(err.to_string().contains("嵌入配置缺少 model"));
    let stored = embedding_config::Entity::find_by_id(invalid.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert!(!stored.is_default);
}

#[tokio::test]
async fn delete_embedding_rejects_missing_config_without_touching_existing_rows() {
    let service = ConfigService::new(Arc::new(setup_db().await));
    service
        .create_embedding(embedding_params("local"))
        .await
        .unwrap();

    let err = service
        .delete_embedding(404)
        .await
        .expect_err("deleting a missing embedding config should fail visibly");

    assert!(matches!(
        err,
        AppError::NotFound(message) if message == "嵌入配置不存在: 404"
    ));
    let configs = service.list_embedding().await.unwrap();
    assert_eq!(configs.len(), 1);
    assert_eq!(configs[0].name, "local");
}

#[tokio::test]
async fn set_default_embedding_rejects_fastembed_dimension_mismatch() {
    let db = setup_db().await;
    let service = ConfigService::new(Arc::new(db.clone()));
    let now = chrono::Utc::now().naive_utc();
    let invalid = embedding_config::ActiveModel {
        name: Set("fastembed-broken".to_string()),
        backend: Set("fastembed".to_string()),
        base_url: Set(None),
        api_key_encrypted: Set(None),
        model: Set("BGESmallZHV15".to_string()),
        dimension: Set(8),
        batch_size: Set(100),
        local_model_path: Set(None),
        is_default: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    let err = service
        .set_default_embedding(invalid.id)
        .await
        .expect_err("invalid fastembed config must not become default");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(err.to_string().contains("实际输出 512 维"));
    let stored = embedding_config::Entity::find_by_id(invalid.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert!(!stored.is_default);
}

#[tokio::test]
async fn update_image_can_explicitly_clear_api_key() {
    if !test_database_available() {
        eprintln!("skip update_image_can_explicitly_clear_api_key: A4N_TEST_DATABASE_URL 未设置");
        return;
    }

    let service = ConfigService::new(Arc::new(setup_db().await));
    let config = service.create_image(params()).await.unwrap();
    let mut update = image_update_params("OpenAI");
    update.api_key_encrypted = Some(None);

    let updated = service
        .update_image(config.id, update)
        .await
        .expect("image config templates may clear global API keys");

    assert_eq!(updated.api_key_encrypted, None);
    assert_eq!(
        service
            .get_image(config.id)
            .await
            .unwrap()
            .api_key_encrypted,
        None
    );
}

#[tokio::test]
async fn update_image_rejects_renaming_to_existing_name() {
    let service = ConfigService::new(Arc::new(setup_db().await));
    service.create_image(params()).await.unwrap();
    let mut secondary_params = params();
    secondary_params.name = "Other".to_string();
    let secondary = service.create_image(secondary_params).await.unwrap();

    let mut duplicate_params = image_update_params("OpenAI");
    duplicate_params.name = " OpenAI ".to_string();
    let err = service
        .update_image(secondary.id, duplicate_params)
        .await
        .expect_err("rename to an existing name should be rejected");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(err.to_string().contains("图片配置名称已存在"));
    assert_eq!(service.list_image().await.unwrap().len(), 2);
}

#[tokio::test]
async fn delete_image_rejects_missing_config_without_touching_existing_rows() {
    let service = ConfigService::new(Arc::new(setup_db().await));
    service.create_image(params()).await.unwrap();

    let err = service
        .delete_image(404)
        .await
        .expect_err("deleting a missing image config should fail visibly");

    assert!(matches!(err, AppError::NotFound(message) if message == "图片配置不存在"));
    let configs = service.list_image().await.unwrap();
    assert_eq!(configs.len(), 1);
    assert_eq!(configs[0].name, "OpenAI");
}

#[tokio::test]
async fn create_video_rejects_duplicate_trimmed_names() {
    if !test_database_available() {
        eprintln!(
            "skip create_video_rejects_duplicate_trimmed_names: A4N_TEST_DATABASE_URL 未设置"
        );
        return;
    }

    let service = ConfigService::new(Arc::new(setup_db().await));
    service
        .create_video(video_params("primary"))
        .await
        .expect("first video config should be created");

    let err = service
        .create_video(video_params(" primary "))
        .await
        .expect_err("duplicate video config name should be rejected");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(err.to_string().contains("视频配置名称已存在"));
    assert_eq!(service.list_video().await.unwrap().len(), 1);
}

#[tokio::test]
async fn update_video_can_explicitly_clear_api_key() {
    if !test_database_available() {
        eprintln!("skip update_video_can_explicitly_clear_api_key: A4N_TEST_DATABASE_URL 未设置");
        return;
    }

    let service = ConfigService::new(Arc::new(setup_db().await));
    let config = service.create_video(video_params("primary")).await.unwrap();
    let mut update = video_update_params("primary");
    update.api_key_encrypted = Some(None);

    let updated = service
        .update_video(config.id, update)
        .await
        .expect("video config templates may clear global API keys");

    assert_eq!(updated.api_key_encrypted, None);
    assert_eq!(
        service
            .get_video(config.id)
            .await
            .unwrap()
            .api_key_encrypted,
        None
    );
}

#[tokio::test]
async fn update_video_rejects_renaming_to_existing_name() {
    if !test_database_available() {
        eprintln!(
            "skip update_video_rejects_renaming_to_existing_name: A4N_TEST_DATABASE_URL 未设置"
        );
        return;
    }

    let service = ConfigService::new(Arc::new(setup_db().await));
    service.create_video(video_params("primary")).await.unwrap();
    let secondary = service
        .create_video(video_params("secondary"))
        .await
        .unwrap();

    let err = service
        .update_video(secondary.id, video_update_params(" primary "))
        .await
        .expect_err("renaming video config to an existing name should be rejected");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(err.to_string().contains("视频配置名称已存在"));
    let configs = service.list_video().await.unwrap();
    assert!(configs.iter().any(|config| config.name == "primary"));
    assert!(configs.iter().any(|config| config.name == "secondary"));
}

#[tokio::test]
async fn video_config_delete_and_capacity_share_active_task_lookup() {
    if !test_database_available() {
        eprintln!(
            "skip video_config_delete_and_capacity_share_active_task_lookup: A4N_TEST_DATABASE_URL 未设置"
        );
        return;
    }

    let db = setup_db().await;
    let service = ConfigService::new(Arc::new(db.clone()));
    let config = service.create_video(video_params("primary")).await.unwrap();
    let other_config = service
        .create_video(video_params("secondary"))
        .await
        .unwrap();
    let project = insert_test_project(&db, "video config usage").await;
    let earlier = chrono::NaiveDate::from_ymd_opt(2026, 6, 1)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap();
    let later = chrono::NaiveDate::from_ymd_opt(2026, 6, 2)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap();

    let active_later =
        insert_generated_video_for_config(&db, project.id, config.id, "running", later).await;
    insert_generated_video_for_config(&db, project.id, config.id, "completed", earlier).await;
    insert_generated_video_for_config(&db, project.id, other_config.id, "pending", earlier).await;
    let active_earlier =
        insert_generated_video_for_config(&db, project.id, config.id, "pending", earlier).await;

    let active_ids = super::active_generated_video_ids_for_config(&db, config.id)
        .await
        .unwrap();
    assert_eq!(active_ids, vec![active_earlier.id, active_later.id]);

    let err = service
        .delete_video(config.id)
        .await
        .expect_err("active generated videos should block video config deletion");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(err.to_string().contains("仍有 2 个进行中的视频任务"));
    assert!(service.get_video(config.id).await.is_ok());
}
