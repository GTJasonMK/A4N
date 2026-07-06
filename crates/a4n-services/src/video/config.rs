use super::VideoDurationCapability;
use crate::config::normalize_grok_reference_mode_options_json;
use a4n_core::models::ai::video_gen_config;
use a4n_engine::video::{VideoProvider, pick_video_provider};
use serde_json::{Value, json};
use std::collections::HashSet;

const OPENAI_COMPATIBLE_VIDEO_BACKEND: &str = "openai-compatible-video";
const NEWAPI_VIDEO_BACKEND: &str = "newapi-video";
const GEMINI_VEO_VIDEO_BACKEND: &str = "gemini-veo-video";
const SEEDANCE2_VIDEO_BACKEND: &str = "seedance2-video";
const OPENAI_CHAT_VIDEO_BACKEND: &str = "openai-chat-video";
const CLIPROXY_OPENAI_VIDEO_BACKEND: &str = "cliproxy-openai-video";
const VIDEO_DURATION_MODE_RANGE: &str = "range";
const VIDEO_DURATION_MODE_FIXED: &str = "fixed";

#[derive(Debug, Clone)]
pub struct VideoConfigParams {
    pub name: String,
    pub backend: String,
    pub base_url: String,
    pub api_key_encrypted: Option<String>,
    pub model: String,
    pub max_concurrent: i32,
    pub default_width: i32,
    pub default_height: i32,
    pub default_duration_secs: i32,
    pub min_duration_secs: i32,
    pub max_duration_secs: i32,
    pub duration_mode: String,
    pub supported_durations_json: Option<Value>,
    pub default_fps: i32,
    pub provider_options_json: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct UpdateVideoConfigParams {
    pub name: String,
    pub backend: String,
    pub base_url: String,
    pub api_key_encrypted: Option<Option<String>>,
    pub model: String,
    pub max_concurrent: i32,
    pub default_width: i32,
    pub default_height: i32,
    pub default_duration_secs: i32,
    pub min_duration_secs: i32,
    pub max_duration_secs: i32,
    pub duration_mode: String,
    pub supported_durations_json: Option<Value>,
    pub default_fps: i32,
    pub provider_options_json: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VideoDurationCapabilityValues {
    duration_mode: String,
    min_duration_secs: i32,
    max_duration_secs: i32,
    supported_durations_secs: Vec<i32>,
}

pub fn validate_video_config_model(
    cfg: &video_gen_config::Model,
) -> Result<VideoConfigParams, String> {
    validate_video_config_params(VideoConfigParams {
        name: cfg.name.clone(),
        backend: cfg.backend.clone(),
        base_url: cfg
            .base_url
            .clone()
            .ok_or_else(|| "视频生成配置缺少 base_url".to_string())?,
        api_key_encrypted: cfg.api_key_encrypted.clone(),
        model: cfg.model.clone(),
        max_concurrent: cfg.max_concurrent,
        default_width: cfg.default_width,
        default_height: cfg.default_height,
        default_duration_secs: cfg.default_duration_secs,
        min_duration_secs: cfg.min_duration_secs,
        max_duration_secs: cfg.max_duration_secs,
        duration_mode: cfg.duration_mode.clone(),
        supported_durations_json: cfg.supported_durations_json.clone(),
        default_fps: cfg.default_fps,
        provider_options_json: cfg.provider_options_json.clone(),
    })
}

pub fn validate_video_config_params(
    mut params: VideoConfigParams,
) -> Result<VideoConfigParams, String> {
    params.name = required_video_config_value(params.name, "name")?;
    params.backend = required_video_config_value(params.backend, "backend")?;
    params.base_url = required_video_config_value(params.base_url, "base_url")?;
    params.model = required_video_config_value(params.model, "model")?;

    match params.backend.as_str() {
        OPENAI_COMPATIBLE_VIDEO_BACKEND
        | NEWAPI_VIDEO_BACKEND
        | GEMINI_VEO_VIDEO_BACKEND
        | SEEDANCE2_VIDEO_BACKEND
        | OPENAI_CHAT_VIDEO_BACKEND
        | CLIPROXY_OPENAI_VIDEO_BACKEND => {
            params.api_key_encrypted = params
                .api_key_encrypted
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
        }
        other => {
            return Err(format!("未知视频生成后端: {other}"));
        }
    }

    validate_video_max_concurrent(params.max_concurrent)?;
    validate_i32_positive(params.default_width, "default_width")?;
    validate_i32_positive(params.default_height, "default_height")?;
    validate_i32_range(params.default_duration_secs, "default_duration_secs", 1, 60)?;
    validate_video_duration_capability(&mut params)?;
    validate_backend_duration_constraints(&params)?;
    validate_i32_range(params.default_fps, "default_fps", 1, 120)?;
    params.provider_options_json =
        normalize_video_provider_options_json(&params.backend, params.provider_options_json)?;

    Ok(params)
}

fn normalize_video_provider_options_json(
    backend: &str,
    value: Option<Value>,
) -> Result<Option<Value>, String> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    if backend != CLIPROXY_OPENAI_VIDEO_BACKEND {
        return Err(format!(
            "视频 provider_options_json 仅支持 {CLIPROXY_OPENAI_VIDEO_BACKEND} 后端，当前后端为 {backend}"
        ));
    }
    if let Value::Object(map) = &value
        && let Some(key) = map.keys().find(|key| key.as_str() != "grok_reference_mode")
    {
        return Err(format!("视频 provider_options_json 包含未知字段：{key}"));
    }
    normalize_grok_reference_mode_options_json(true, "视频", Some(value))
        .map_err(|err| err.to_string())
}

fn validate_backend_duration_constraints(params: &VideoConfigParams) -> Result<(), String> {
    if params.backend != CLIPROXY_OPENAI_VIDEO_BACKEND {
        return Ok(());
    }
    if params.min_duration_secs < 1 || params.max_duration_secs > 15 {
        return Err("CLIProxy / Grok 视频配置支持时长范围必须在 1-15 秒之间".to_string());
    }
    if params.default_duration_secs > 15 {
        return Err("CLIProxy / Grok 视频默认时长不能超过 15 秒".to_string());
    }
    if params.duration_mode == VIDEO_DURATION_MODE_FIXED {
        let supported = params
            .supported_durations_json
            .as_ref()
            .and_then(|value| value.as_array())
            .ok_or_else(|| "CLIProxy / Grok 视频 fixed 模式必须填写固定时长档位".to_string())?;
        if supported
            .iter()
            .any(|value| value.as_i64().unwrap_or(0) > 15)
        {
            return Err("CLIProxy / Grok 视频固定时长档位不能超过 15 秒".to_string());
        }
    }
    Ok(())
}

pub fn duration_capability_for_config(
    cfg: &video_gen_config::Model,
) -> Result<VideoDurationCapability, String> {
    validate_video_duration_capability_values(
        &cfg.duration_mode,
        cfg.default_duration_secs,
        cfg.min_duration_secs,
        cfg.max_duration_secs,
        cfg.supported_durations_json.as_ref(),
    )
    .map(|values| VideoDurationCapability {
        model: Some(cfg.model.clone()),
        duration_mode: values.duration_mode,
        default_duration_secs: cfg.default_duration_secs,
        min_duration_secs: values.min_duration_secs,
        max_duration_secs: values.max_duration_secs,
        supported_durations_secs: values.supported_durations_secs,
    })
}

pub fn validate_video_duration_for_config_secs(
    cfg: &video_gen_config::Model,
    duration_secs: i32,
) -> Result<(), String> {
    duration_capability_for_config(cfg)?
        .validate_duration(duration_secs)
        .map_err(|err| err.to_string())
}

pub fn pick_provider_for_config(
    cfg: &video_gen_config::Model,
) -> Result<Box<dyn VideoProvider>, String> {
    let params = validate_video_config_model(cfg)?;
    let key = params
        .api_key_encrypted
        .as_deref()
        .ok_or_else(|| "视频生成配置缺少 api_key_encrypted".to_string())?;
    pick_video_provider(
        &params.backend,
        &params.base_url,
        key,
        params.provider_options_json.as_ref(),
    )
    .map_err(|err| err.to_string())
}

fn required_video_config_value(value: String, field: &str) -> Result<String, String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(format!("视频生成配置缺少 {field}"));
    }
    Ok(value)
}

fn validate_i32_positive(value: i32, field: &str) -> Result<(), String> {
    if value <= 0 {
        return Err(format!("视频配置 {field} 必须大于 0"));
    }
    Ok(())
}

pub fn validate_video_max_concurrent(value: i32) -> Result<(), String> {
    validate_i32_range(value, "max_concurrent", 1, 8)
}

fn validate_i32_range(value: i32, field: &str, min: i32, max: i32) -> Result<(), String> {
    if value < min || value > max {
        return Err(format!("视频配置 {field} 必须在 {min}-{max} 之间"));
    }
    Ok(())
}

fn validate_video_duration_capability(params: &mut VideoConfigParams) -> Result<(), String> {
    let values = validate_video_duration_capability_values(
        &params.duration_mode,
        params.default_duration_secs,
        params.min_duration_secs,
        params.max_duration_secs,
        params.supported_durations_json.as_ref(),
    )?;
    params.duration_mode = values.duration_mode;
    if params.min_duration_secs != values.min_duration_secs {
        return Err(format!(
            "视频配置 fixed 模式 min_duration_secs 必须等于 supported_durations_json 第一项 {}",
            values.min_duration_secs
        ));
    }
    if params.max_duration_secs != values.max_duration_secs {
        return Err(format!(
            "视频配置 fixed 模式 max_duration_secs 必须等于 supported_durations_json 最后一项 {}",
            values.max_duration_secs
        ));
    }
    params.supported_durations_json = match params.duration_mode.as_str() {
        VIDEO_DURATION_MODE_FIXED => Some(json!(values.supported_durations_secs)),
        _ => None,
    };
    Ok(())
}

fn validate_video_duration_capability_values(
    duration_mode: &str,
    default_duration_secs: i32,
    min_duration_secs: i32,
    max_duration_secs: i32,
    supported_durations_json: Option<&Value>,
) -> Result<VideoDurationCapabilityValues, String> {
    let duration_mode = normalize_video_duration_mode(duration_mode)?;
    match duration_mode.as_str() {
        VIDEO_DURATION_MODE_FIXED => {
            let supported = validate_supported_durations(supported_durations_json)?;
            let Some(first) = supported.first().copied() else {
                return Err("固定时间档位模式必须填写 supported_durations_json".to_string());
            };
            let last = supported.last().copied().unwrap_or(first);
            if !supported.contains(&default_duration_secs) {
                return Err(format!(
                    "视频配置 default_duration_secs={default_duration_secs} 必须包含在 supported_durations_json 中"
                ));
            }
            Ok(VideoDurationCapabilityValues {
                duration_mode,
                min_duration_secs: first,
                max_duration_secs: last,
                supported_durations_secs: supported,
            })
        }
        VIDEO_DURATION_MODE_RANGE => {
            if supported_durations_json.is_some_and(|value| !value.is_null()) {
                return Err(
                    "视频配置 range 模式不能填写 supported_durations_json，请改用 fixed 模式"
                        .to_string(),
                );
            }
            validate_i32_range(min_duration_secs, "min_duration_secs", 1, 60)?;
            validate_i32_range(max_duration_secs, "max_duration_secs", 1, 60)?;
            if min_duration_secs > max_duration_secs {
                return Err("视频配置 min_duration_secs 不能大于 max_duration_secs".to_string());
            }
            if default_duration_secs < min_duration_secs
                || default_duration_secs > max_duration_secs
            {
                return Err(format!(
                    "视频配置 default_duration_secs 必须在模型支持时长 {min_duration_secs}-{max_duration_secs} 秒之间"
                ));
            }
            Ok(VideoDurationCapabilityValues {
                duration_mode,
                min_duration_secs,
                max_duration_secs,
                supported_durations_secs: Vec::new(),
            })
        }
        _ => unreachable!("duration mode should be normalized"),
    }
}

fn normalize_video_duration_mode(value: &str) -> Result<String, String> {
    let trimmed = value.trim().to_ascii_lowercase();
    if trimmed.is_empty() {
        return Err("视频配置 duration_mode 不能为空，必须是 range 或 fixed".to_string());
    }
    match trimmed.as_str() {
        VIDEO_DURATION_MODE_RANGE => Ok(VIDEO_DURATION_MODE_RANGE.to_string()),
        VIDEO_DURATION_MODE_FIXED => Ok(VIDEO_DURATION_MODE_FIXED.to_string()),
        other => Err(format!(
            "视频配置 duration_mode 无效: {other}，必须是 range 或 fixed"
        )),
    }
}

fn validate_supported_durations(value: Option<&Value>) -> Result<Vec<i32>, String> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(Vec::new());
    };
    let items = value
        .as_array()
        .ok_or_else(|| "视频配置 supported_durations_json 必须是整数数组".to_string())?;
    let mut seen = HashSet::with_capacity(items.len());
    let mut durations = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let Some(raw) = item.as_i64() else {
            return Err("视频配置 supported_durations_json 必须只包含整数".to_string());
        };
        let duration = i32::try_from(raw)
            .map_err(|_| "视频配置 supported_durations_json 包含超出 i32 范围的整数".to_string())?;
        validate_i32_range(duration, "supported_durations_json", 1, 60)?;
        if !seen.insert(duration) {
            return Err(format!(
                "视频配置 supported_durations_json 包含重复时长：{duration}"
            ));
        }
        if let Some(previous) = durations.last()
            && duration <= *previous
        {
            return Err(format!(
                "视频配置 supported_durations_json 必须按从小到大填写；第 {} 项 {duration} 不大于前一项 {previous}",
                index + 1
            ));
        }
        durations.push(duration);
    }
    Ok(durations)
}

#[cfg(test)]
mod tests {
    use super::*;

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
            provider_options_json: None,
        }
    }

    fn video_model() -> video_gen_config::Model {
        let params = validate_video_config_params(valid_params()).expect("valid video config");
        let now = chrono::Utc::now().naive_utc();
        video_gen_config::Model {
            id: 1,
            name: params.name,
            backend: params.backend,
            base_url: Some(params.base_url),
            api_key_encrypted: params.api_key_encrypted,
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
            is_default: true,
            created_at: now,
            updated_at: now,
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

        let err =
            validate_video_config_params(params).expect_err("cliproxy duration cap should fail");

        assert!(err.contains("1-15 秒"));
    }

    #[test]
    fn validate_video_config_params_rejects_non_cliproxy_provider_options() {
        let mut params = valid_params();
        params.provider_options_json = Some(json!({
            "grok_reference_mode": "first_frame",
        }));

        let err = validate_video_config_params(params)
            .expect_err("non-cliproxy video backend must not silently drop provider options");

        assert!(err.contains("provider_options_json 仅支持 cliproxy-openai-video"));
    }

    #[test]
    fn validate_video_config_params_rejects_unknown_cliproxy_provider_options() {
        let mut params = valid_params();
        params.backend = CLIPROXY_OPENAI_VIDEO_BACKEND.to_string();
        params.max_duration_secs = 15;
        params.provider_options_json = Some(json!({
            "grok_reference_mode": "reference",
            "legacy_mode": true,
        }));

        let err = validate_video_config_params(params)
            .expect_err("unknown cliproxy video provider option must fail visibly");

        assert!(err.contains("provider_options_json 包含未知字段：legacy_mode"));
    }

    #[test]
    fn validate_video_config_params_normalizes_cliproxy_provider_options() {
        let mut params = valid_params();
        params.backend = CLIPROXY_OPENAI_VIDEO_BACKEND.to_string();
        params.max_duration_secs = 15;
        params.provider_options_json = Some(json!({
            "grok_reference_mode": " first_frame ",
        }));

        let params = validate_video_config_params(params).expect("valid cliproxy video options");

        assert_eq!(
            params.provider_options_json,
            Some(json!({
                "grok_reference_mode": "first_frame",
            }))
        );
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
    fn pick_provider_for_config_requires_api_key_at_request_boundary() {
        let mut cfg = video_model();
        cfg.api_key_encrypted = None;

        let err = match pick_provider_for_config(&cfg) {
            Ok(_) => panic!("provider requests still require a key"),
            Err(err) => err,
        };

        assert!(err.contains("视频生成配置缺少 api_key_encrypted"));
    }

    #[test]
    fn validate_video_config_params_rejects_out_of_range_values() {
        let mut params = valid_params();
        params.max_concurrent = 0;

        let err =
            validate_video_config_params(params).expect_err("max_concurrent should be in range");

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

        let err =
            validate_video_config_params(params).expect_err("duplicate durations should fail");

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

        let err =
            validate_video_config_params(params).expect_err("fixed min/max should be explicit");

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
}
