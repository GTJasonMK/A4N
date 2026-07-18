use a4n_core::error::AppError;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

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
        let fixed_mode = self.duration_mode == "fixed";
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
        let fixed_mode = self.duration_mode == "fixed";
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
