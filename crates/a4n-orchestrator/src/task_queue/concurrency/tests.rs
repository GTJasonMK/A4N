use a4n_core::error::AppError;
use a4n_core::models::ai::{image_gen_config, llm_config, video_gen_config};
use chrono::Utc;
use sea_orm::{EntityTrait, Set};

use super::super::PersistentTaskQueue;
use super::super::test_support::setup_db;

#[tokio::test]
async fn queue_init_rejects_invalid_default_llm_concurrency() {
    let db = setup_db().await;
    let now = Utc::now().naive_utc();
    llm_config::Entity::insert(llm_config::ActiveModel {
        name: Set("invalid-llm".to_string()),
        provider: Set("openai".to_string()),
        base_url: Set("https://api.openai.com/v1".to_string()),
        api_key_encrypted: Set(Some("sk-test".to_string())),
        model: Set("gpt-4.1".to_string()),
        max_concurrent: Set(0),
        timeout_secs: Set(60),
        max_context_tokens: Set(128_000),
        max_output_tokens: Set(4_096),
        extra_body_json: Set(None),
        is_default: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    })
    .exec(db.as_ref())
    .await
    .unwrap();

    let err = match PersistentTaskQueue::new(db).await {
        Ok(_) => panic!("invalid default LLM concurrency should fail visibly"),
        Err(err) => err,
    };
    assert!(matches!(err, AppError::ConfigError(_)));
    assert!(err.to_string().contains("默认 LLM 配置 max_concurrent"));
}

#[tokio::test]
async fn queue_init_rejects_invalid_default_image_concurrency() {
    let db = setup_db().await;
    let now = Utc::now().naive_utc();
    image_gen_config::Entity::insert(image_gen_config::ActiveModel {
        name: Set("invalid-image".to_string()),
        backend: Set("openai-compatible-image".to_string()),
        base_url: Set(Some("https://api.openai.com/v1".to_string())),
        api_key_encrypted: Set(Some("sk-test".to_string())),
        model: Set("dall-e-3".to_string()),
        max_concurrent: Set(0),
        default_width: Set(1024),
        default_height: Set(1024),
        is_default: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    })
    .exec(db.as_ref())
    .await
    .unwrap();

    let err = match PersistentTaskQueue::new(db).await {
        Ok(_) => panic!("invalid default image concurrency should fail visibly"),
        Err(err) => err,
    };
    assert!(matches!(err, AppError::ConfigError(_)));
    assert!(err.to_string().contains("默认图片配置 max_concurrent"));
}

#[tokio::test]
async fn queue_init_rejects_invalid_default_video_concurrency() {
    let db = setup_db().await;
    let now = Utc::now().naive_utc();
    video_gen_config::Entity::insert(video_gen_config::ActiveModel {
        name: Set("invalid-video".to_string()),
        backend: Set("openai-compatible-video".to_string()),
        base_url: Set(Some("https://api.example.com/v1".to_string())),
        api_key_encrypted: Set(Some("sk-test".to_string())),
        model: Set("video-model".to_string()),
        max_concurrent: Set(0),
        default_width: Set(1280),
        default_height: Set(720),
        default_duration_secs: Set(5),
        min_duration_secs: Set(1),
        max_duration_secs: Set(10),
        duration_mode: Set("range".to_string()),
        supported_durations_json: Set(None),
        default_fps: Set(24),
        is_default: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    })
    .exec(db.as_ref())
    .await
    .unwrap();

    let err = match PersistentTaskQueue::new(db).await {
        Ok(_) => panic!("invalid default video concurrency should fail visibly"),
        Err(err) => err,
    };
    assert!(matches!(err, AppError::ConfigError(_)));
    assert!(err.to_string().contains("默认视频配置 max_concurrent"));
}
