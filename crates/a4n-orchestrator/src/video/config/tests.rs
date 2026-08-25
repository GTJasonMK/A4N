use super::contracts::*;
use super::*;
use serde_json::json;

fn valid_params() -> VideoConfigParams {
    VideoConfigParams {
        name: " 默认视频 ".to_string(),
        backend: NEWAPI_VIDEO_BACKEND.to_string(),
        base_url: " https://video.example/v1 ".to_string(),
        api_key_encrypted: Some(" secret ".to_string()),
        model: " video-model ".to_string(),
        max_concurrent: 2,
        default_width: 1280,
        default_height: 720,
        default_duration_secs: 5,
        min_duration_secs: 1,
        max_duration_secs: 60,
        duration_mode: VIDEO_DURATION_MODE_RANGE.to_string(),
        supported_durations_json: None,
        default_fps: 24,
    }
}

#[test]
fn validate_video_config_params_trims_required_fields() {
    let params = validate_video_config_params(valid_params()).expect("valid config");

    assert_eq!(params.name, "默认视频");
    assert_eq!(params.base_url, "https://video.example/v1");
    assert_eq!(params.api_key_encrypted.as_deref(), Some("secret"));
    assert_eq!(params.model, "video-model");
}

#[test]
fn validate_video_config_params_accepts_gemini_veo_backend() {
    let mut params = valid_params();
    params.backend = GEMINI_VEO_VIDEO_BACKEND.to_string();

    let params = validate_video_config_params(params).expect("gemini veo config");

    assert_eq!(params.backend, GEMINI_VEO_VIDEO_BACKEND);
}

#[test]
fn validate_video_config_params_accepts_seedance2_backend() {
    let mut params = valid_params();
    params.backend = SEEDANCE2_VIDEO_BACKEND.to_string();
    params.min_duration_secs = 4;
    params.max_duration_secs = 15;

    let params = validate_video_config_params(params).expect("seedance2 config");

    assert_eq!(params.backend, SEEDANCE2_VIDEO_BACKEND);
}

#[test]
fn validate_video_config_params_accepts_cliproxy_openai_backend() {
    let mut params = valid_params();
    params.backend = CLIPROXY_OPENAI_VIDEO_BACKEND.to_string();
    params.max_duration_secs = 15;

    let params = validate_video_config_params(params).expect("cliproxy video config");

    assert_eq!(params.backend, CLIPROXY_OPENAI_VIDEO_BACKEND);
}

#[test]
fn validate_video_config_params_rejects_cliproxy_duration_over_15_secs() {
    let mut params = valid_params();
    params.backend = CLIPROXY_OPENAI_VIDEO_BACKEND.to_string();
    params.duration_mode = VIDEO_DURATION_MODE_FIXED.to_string();
    params.default_duration_secs = 15;
    params.min_duration_secs = 5;
    params.max_duration_secs = 20;
    params.supported_durations_json = Some(json!([5, 15, 20]));

    let err = validate_video_config_params(params).expect_err("cliproxy duration cap should fail");

    assert!(err.contains("1-15"));
}

#[test]
fn validate_video_config_params_rejects_seedance_duration_outside_provider_range() {
    let mut params = valid_params();
    params.backend = SEEDANCE2_VIDEO_BACKEND.to_string();
    params.default_duration_secs = 3;
    params.min_duration_secs = 1;
    params.max_duration_secs = 15;

    let err = validate_video_config_params(params).expect_err("seedance duration cap should fail");

    assert!(err.contains("4-15"));
}

#[test]
fn validate_video_config_params_rejects_unknown_backend() {
    let mut params = valid_params();
    params.backend = "local-video".to_string();

    let err = validate_video_config_params(params).expect_err("backend should be known");

    assert!(err.contains("未知视频生成后端: local-video"));
}

#[test]
fn validate_video_config_params_allows_template_without_api_key() {
    let mut params = valid_params();
    params.api_key_encrypted = None;

    let params = validate_video_config_params(params)
        .expect("video config templates may omit user-owned API keys");

    assert_eq!(params.api_key_encrypted, None);
}

#[test]
fn validate_video_config_params_rejects_out_of_range_values() {
    let mut params = valid_params();
    params.max_concurrent = 0;

    let err = validate_video_config_params(params).expect_err("max_concurrent should be in range");

    assert!(err.contains("视频配置 max_concurrent 必须在 1-8 之间"));
}

#[test]
fn validate_video_config_params_rejects_missing_duration_mode() {
    let mut params = valid_params();
    params.duration_mode = "  ".to_string();
    params.supported_durations_json = Some(json!([5, 10]));

    let err = validate_video_config_params(params).expect_err("duration mode is explicit");

    assert!(err.contains("duration_mode 不能为空"));
}

#[test]
fn validate_video_config_params_rejects_duration_mode_aliases() {
    let mut params = valid_params();
    params.duration_mode = "discrete".to_string();
    params.supported_durations_json = Some(json!([5, 10]));

    let err = validate_video_config_params(params).expect_err("duration mode aliases are dead");

    assert!(err.contains("duration_mode 无效: discrete"));
}

#[test]
fn validate_video_config_params_preserves_supported_durations_order() {
    let mut params = valid_params();
    params.duration_mode = VIDEO_DURATION_MODE_FIXED.to_string();
    params.default_duration_secs = 5;
    params.min_duration_secs = 5;
    params.max_duration_secs = 15;
    params.supported_durations_json = Some(json!([5, 10, 15]));

    let params = validate_video_config_params(params).expect("valid fixed durations");

    assert_eq!(params.duration_mode, VIDEO_DURATION_MODE_FIXED);
    assert_eq!(params.supported_durations_json, Some(json!([5, 10, 15])));
    assert_eq!(params.min_duration_secs, 5);
    assert_eq!(params.max_duration_secs, 15);
}

#[test]
fn validate_video_config_params_rejects_duplicate_supported_durations() {
    let mut params = valid_params();
    params.duration_mode = VIDEO_DURATION_MODE_FIXED.to_string();
    params.default_duration_secs = 10;
    params.min_duration_secs = 5;
    params.max_duration_secs = 10;
    params.supported_durations_json = Some(json!([5, 10, 10]));

    let err = validate_video_config_params(params).expect_err("duplicate durations should fail");

    assert!(err.contains("supported_durations_json 包含重复时长：10"));
}

#[test]
fn validate_video_config_params_rejects_unsorted_supported_durations() {
    let mut params = valid_params();
    params.duration_mode = VIDEO_DURATION_MODE_FIXED.to_string();
    params.default_duration_secs = 10;
    params.min_duration_secs = 5;
    params.max_duration_secs = 10;
    params.supported_durations_json = Some(json!([10, 5]));

    let err = validate_video_config_params(params).expect_err("unsorted durations should fail");

    assert!(err.contains("supported_durations_json 必须按从小到大填写"));
}

#[test]
fn validate_video_config_params_rejects_default_outside_supported_durations() {
    let mut params = valid_params();
    params.duration_mode = VIDEO_DURATION_MODE_FIXED.to_string();
    params.default_duration_secs = 6;
    params.supported_durations_json = Some(json!([5, 10]));

    let err = validate_video_config_params(params).expect_err("default must be supported");

    assert!(err.contains("必须包含在 supported_durations_json 中"));
}

#[test]
fn validate_video_config_params_rejects_supported_durations_in_range_mode() {
    let mut params = valid_params();
    params.supported_durations_json = Some(json!([5, 10]));

    let err = validate_video_config_params(params).expect_err("range mode should reject list");

    assert!(err.contains("range 模式不能填写 supported_durations_json"));
}

#[test]
fn validate_video_config_params_rejects_fixed_min_max_mismatch() {
    let mut params = valid_params();
    params.duration_mode = VIDEO_DURATION_MODE_FIXED.to_string();
    params.default_duration_secs = 10;
    params.min_duration_secs = 1;
    params.max_duration_secs = 60;
    params.supported_durations_json = Some(json!([5, 10]));

    let err = validate_video_config_params(params).expect_err("fixed min/max should be explicit");

    assert!(
        err.contains("fixed 模式 min_duration_secs 必须等于 supported_durations_json 第一项 5")
    );
}

#[test]
fn validate_video_config_params_requires_fixed_duration_list() {
    let mut params = valid_params();
    params.duration_mode = VIDEO_DURATION_MODE_FIXED.to_string();

    let err = validate_video_config_params(params).expect_err("fixed mode needs list");

    assert!(err.contains("固定时间档位模式必须填写"));
}
