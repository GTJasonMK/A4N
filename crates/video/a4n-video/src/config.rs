//! 视频 provider 配置的纯合同与校验。

use crate::video_provider_capabilities;
use a4n_domain::ProviderError as AppError;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashSet;

pub const VIDEO_DURATION_MODE_RANGE: &str = "range";
pub const VIDEO_DURATION_MODE_FIXED: &str = "fixed";

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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VideoDurationCapability {
    pub model: Option<String>,
    pub duration_mode: String,
    pub default_duration_secs: i32,
    pub min_duration_secs: i32,
    pub max_duration_secs: i32,
    pub supported_durations_secs: Vec<i32>,
}

impl VideoDurationCapability {
    pub fn validate_duration(&self, duration_secs: i32) -> Result<(), AppError> {
        let fixed_mode = self.duration_mode == VIDEO_DURATION_MODE_FIXED;
        if duration_secs < self.min_duration_secs || duration_secs > self.max_duration_secs {
            return Err(AppError::InvalidArgument(format!(
                "视频时长 {duration_secs}s 不在当前模型支持范围 {}-{}s 内",
                self.min_duration_secs, self.max_duration_secs
            )));
        }
        if fixed_mode && !self.supported_durations_secs.contains(&duration_secs) {
            return Err(AppError::InvalidArgument(format!(
                "视频时长 {duration_secs}s 不在当前模型固定时长档位 {:?} 内",
                self.supported_durations_secs
            )));
        }
        Ok(())
    }

    pub fn validate_duration_options(&self, duration_secs_options: &[i32]) -> Result<(), AppError> {
        for duration_secs in duration_secs_options {
            self.validate_duration(*duration_secs)?;
        }
        Ok(())
    }

    pub fn to_prompt_json(&self) -> Value {
        let fixed_mode = self.duration_mode == VIDEO_DURATION_MODE_FIXED;
        json!({
            "model": self.model,
            "duration_mode": self.duration_mode,
            "default_duration_secs": self.default_duration_secs,
            "min_duration_secs": self.min_duration_secs,
            "max_duration_secs": self.max_duration_secs,
            "supported_durations_secs": self.supported_durations_secs,
            "rule": if fixed_mode {
                format!(
                    "每个镜头 duration_secs 必须从 supported_durations_secs 中选择，优先使用 default_duration_secs={}",
                    self.default_duration_secs
                )
            } else {
                format!(
                    "每个镜头 duration_secs 可以使用 {}-{} 秒范围内的任意整数，优先使用 default_duration_secs",
                    self.min_duration_secs, self.max_duration_secs
                )
            }
        })
    }
}

pub fn validate_video_config_params(
    mut params: VideoConfigParams,
) -> Result<VideoConfigParams, String> {
    params.name = required_video_config_value(params.name, "name")?;
    params.backend = required_video_config_value(params.backend, "backend")?;
    params.model = required_video_config_value(params.model, "model")?;

    let capabilities = video_provider_capabilities(&params.backend)
        .ok_or_else(|| format!("未知视频生成后端: {}", params.backend))?;
    params.api_key_encrypted = params
        .api_key_encrypted
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    params.base_url = a4n_domain::config::validate_http_base_url(
        &params.base_url,
        "视频生成配置",
        params.api_key_encrypted.as_deref(),
    )?;

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

pub fn duration_capability_for_params(
    params: &VideoConfigParams,
) -> Result<VideoDurationCapability, String> {
    let capabilities = video_provider_capabilities(&params.backend)
        .ok_or_else(|| format!("未知视频生成后端: {}", params.backend))?;
    validate_video_duration_capability_values(
        &params.duration_mode,
        params.default_duration_secs,
        params.min_duration_secs,
        params.max_duration_secs,
        params.supported_durations_json.as_ref(),
        capabilities.min_duration_secs,
        capabilities.max_duration_secs,
    )
    .map(|values| VideoDurationCapability {
        model: Some(params.model.clone()),
        duration_mode: values.duration_mode,
        default_duration_secs: params.default_duration_secs,
        min_duration_secs: values.min_duration_secs,
        max_duration_secs: values.max_duration_secs,
        supported_durations_secs: values.supported_durations_secs,
    })
}

pub fn validate_video_duration_for_params_secs(
    params: &VideoConfigParams,
    duration_secs: i32,
) -> Result<(), String> {
    duration_capability_for_params(params)?
        .validate_duration(duration_secs)
        .map_err(|error| error.to_string())
}

pub fn validate_video_max_concurrent(value: i32) -> Result<(), String> {
    validate_i32_range(value, "max_concurrent", 1, 8)
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

struct VideoDurationCapabilityValues {
    duration_mode: String,
    min_duration_secs: i32,
    max_duration_secs: i32,
    supported_durations_secs: Vec<i32>,
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
            let last = supported
                .last()
                .copied()
                .expect("fixed duration list is non-empty");
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
        VIDEO_DURATION_MODE_RANGE | VIDEO_DURATION_MODE_FIXED => Ok(trimmed),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NEWAPI_VIDEO_BACKEND;

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
    fn validates_and_normalizes_video_config_without_database_types() {
        let params = validate_video_config_params(valid_params()).expect("valid config");
        assert_eq!(params.name, "默认视频");
        assert_eq!(params.api_key_encrypted.as_deref(), Some("secret"));
        assert!(validate_video_duration_for_params_secs(&params, 5).is_ok());
    }

    #[test]
    fn video_configs_reject_invalid_and_credential_leaking_endpoints() {
        let mut params = valid_params();
        params.base_url = "not-a-url".to_string();
        assert!(
            validate_video_config_params(params.clone())
                .unwrap_err()
                .contains("不是有效 URL")
        );

        params.base_url = "http://video.example/v1".to_string();
        assert!(
            validate_video_config_params(params.clone())
                .unwrap_err()
                .contains("必须使用 HTTPS")
        );

        params.base_url = "http://127.0.0.1:8080/v1".to_string();
        assert!(validate_video_config_params(params).is_ok());
    }

    #[test]
    fn rejects_invalid_fixed_duration_lists() {
        let mut params = valid_params();
        params.duration_mode = VIDEO_DURATION_MODE_FIXED.to_string();
        params.default_duration_secs = 10;
        params.min_duration_secs = 5;
        params.max_duration_secs = 15;
        params.supported_durations_json = Some(json!([5, 10, 10]));
        let error = validate_video_config_params(params).expect_err("duplicate duration");
        assert!(error.contains("包含重复时长"));
    }
}
