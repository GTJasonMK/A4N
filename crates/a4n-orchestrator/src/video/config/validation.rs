use super::super::VideoDurationCapability;
use super::contracts::{
    VIDEO_DURATION_MODE_FIXED, VIDEO_DURATION_MODE_RANGE, VideoConfigParams,
    VideoDurationCapabilityValues,
};
use a4n_core::models::ai::video_gen_config;
use a4n_video::video_provider_capabilities;
use serde_json::{Value, json};
use std::collections::HashSet;

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
    })
}

pub fn validate_video_config_params(
    mut params: VideoConfigParams,
) -> Result<VideoConfigParams, String> {
    params.name = required_video_config_value(params.name, "name")?;
    params.backend = required_video_config_value(params.backend, "backend")?;
    params.base_url = required_video_config_value(params.base_url, "base_url")?;
    params.model = required_video_config_value(params.model, "model")?;

    let capabilities = video_provider_capabilities(&params.backend)
        .ok_or_else(|| format!("未知视频生成后端: {}", params.backend))?;
    params.api_key_encrypted = params
        .api_key_encrypted
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    validate_video_max_concurrent(params.max_concurrent)?;
    validate_i32_positive(params.default_width, "default_width")?;
    validate_i32_positive(params.default_height, "default_height")?;
    validate_i32_range(
        params.default_duration_secs,
        "default_duration_secs",
        capabilities.min_duration_secs,
        capabilities.max_duration_secs,
    )?;
    validate_video_duration_capability(
        &mut params,
        capabilities.min_duration_secs,
        capabilities.max_duration_secs,
    )?;
    validate_i32_range(params.default_fps, "default_fps", 1, 120)?;
    Ok(params)
}

pub fn duration_capability_for_config(
    cfg: &video_gen_config::Model,
) -> Result<VideoDurationCapability, String> {
    let capabilities = video_provider_capabilities(&cfg.backend)
        .ok_or_else(|| format!("未知视频生成后端: {}", cfg.backend))?;
    validate_video_duration_capability_values(
        &cfg.duration_mode,
        cfg.default_duration_secs,
        cfg.min_duration_secs,
        cfg.max_duration_secs,
        cfg.supported_durations_json.as_ref(),
        capabilities.min_duration_secs,
        capabilities.max_duration_secs,
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

fn validate_video_duration_capability(
    params: &mut VideoConfigParams,
    min_duration_secs: i32,
    max_duration_secs: i32,
) -> Result<(), String> {
    let values = validate_video_duration_capability_values(
        &params.duration_mode,
        params.default_duration_secs,
        params.min_duration_secs,
        params.max_duration_secs,
        params.supported_durations_json.as_ref(),
        min_duration_secs,
        max_duration_secs,
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
    provider_min_duration_secs: i32,
    provider_max_duration_secs: i32,
) -> Result<VideoDurationCapabilityValues, String> {
    let duration_mode = normalize_video_duration_mode(duration_mode)?;
    match duration_mode.as_str() {
        VIDEO_DURATION_MODE_FIXED => {
            let supported = validate_supported_durations(
                supported_durations_json,
                provider_min_duration_secs,
                provider_max_duration_secs,
            )?;
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
            validate_i32_range(
                min_duration_secs,
                "min_duration_secs",
                provider_min_duration_secs,
                provider_max_duration_secs,
            )?;
            validate_i32_range(
                max_duration_secs,
                "max_duration_secs",
                provider_min_duration_secs,
                provider_max_duration_secs,
            )?;
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

fn validate_supported_durations(
    value: Option<&Value>,
    min_duration_secs: i32,
    max_duration_secs: i32,
) -> Result<Vec<i32>, String> {
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
        validate_i32_range(
            duration,
            "supported_durations_json",
            min_duration_secs,
            max_duration_secs,
        )?;
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
