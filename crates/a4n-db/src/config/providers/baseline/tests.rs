use super::{BaselineImageConfigSeed, BaselineLlmConfigSeed, BaselineVideoConfigSeed};
use serde_json::json;

#[test]
fn baseline_seed_contracts_preserve_provider_specific_fields() {
    let llm = BaselineLlmConfigSeed {
        name: "公共 LLM".to_string(),
        provider: "openai-compatible".to_string(),
        base_url: "https://llm.example/v1".to_string(),
        api_key_encrypted: Some("encrypted-llm-key".to_string()),
        model: "example-chat".to_string(),
        max_concurrent: 3,
        timeout_secs: 90,
        max_context_tokens: 32_000,
        max_output_tokens: 4_000,
        request_options_json: Some(json!({"reasoning_effort": "medium"})),
    };
    assert_eq!(llm.clone(), llm);
    assert_eq!(
        llm.request_options_json,
        Some(json!({"reasoning_effort": "medium"}))
    );

    let image = BaselineImageConfigSeed {
        name: "公共图片".to_string(),
        backend: "openai-image".to_string(),
        base_url: "https://image.example/v1".to_string(),
        api_key_encrypted: Some("encrypted-image-key".to_string()),
        model: "example-image".to_string(),
        max_concurrent: 2,
        default_width: 1_024,
        default_height: 768,
    };
    assert_eq!(image.clone(), image);
    assert_eq!((image.default_width, image.default_height), (1_024, 768));

    let video = BaselineVideoConfigSeed {
        name: "公共视频".to_string(),
        backend: "openai-compatible-video".to_string(),
        base_url: "https://video.example/v1".to_string(),
        api_key_encrypted: Some("encrypted-video-key".to_string()),
        model: "example-video".to_string(),
        max_concurrent: 1,
        default_width: 1_280,
        default_height: 720,
        default_duration_secs: 5,
        min_duration_secs: 1,
        max_duration_secs: 60,
        duration_mode: "discrete".to_string(),
        supported_durations_json: Some(json!([5, 10, 15])),
        default_fps: 24,
    };
    assert_eq!(video.clone(), video);
    assert_eq!(video.supported_durations_json, Some(json!([5, 10, 15])));
    assert_eq!(
        (
            video.min_duration_secs,
            video.default_duration_secs,
            video.max_duration_secs,
        ),
        (1, 5, 60)
    );
}
