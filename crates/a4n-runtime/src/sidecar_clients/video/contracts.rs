use a4n_core::models::ai::video_gen_config;
use a4n_orchestrator::video::validate_video_config_model;
use a4n_video::{VideoGenerationRequest, VideoGenerationResult};
use base64::Engine as _;
use serde::de::Error as _;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub(in crate::sidecar_clients::video) fn video_provider_payload(
    config: &video_gen_config::Model,
    request: VideoGenerationRequest,
    provider_idempotency_key_sha256: Option<String>,
    project_id: Option<i64>,
) -> Result<VideoProviderPayload, String> {
    let params = validate_video_config_model(config)?;
    let api_key = params
        .api_key_encrypted
        .ok_or_else(|| "视频生成配置缺少 api_key_encrypted".to_string())?;
    Ok(VideoProviderPayload {
        backend: params.backend,
        base_url: params.base_url,
        api_key,
        provider_options_json: params.provider_options_json,
        provider_idempotency_key_sha256,
        project_id,
        request,
    })
}

pub(in crate::sidecar_clients::video) fn video_generation_result(
    output: VideoGenerationOutput,
) -> Result<VideoGenerationResult, String> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(output.bytes_base64)
        .map_err(|err| format!("video sidecar 返回的 bytes_base64 无效: {err}"))?;
    Ok(VideoGenerationResult {
        bytes,
        format: output.format,
        provider_job_id: output.provider_job_id,
        metadata: output.metadata,
    })
}

#[derive(Debug, Serialize)]
pub(in crate::sidecar_clients::video) struct VideoProviderPayload {
    pub(in crate::sidecar_clients::video) backend: String,
    pub(in crate::sidecar_clients::video) base_url: String,
    pub(in crate::sidecar_clients::video) api_key: String,
    pub(in crate::sidecar_clients::video) provider_options_json: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in crate::sidecar_clients::video) provider_idempotency_key_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in crate::sidecar_clients::video) project_id: Option<i64>,
    pub(in crate::sidecar_clients::video) request: VideoGenerationRequest,
}

#[derive(Debug, Serialize)]
pub(in crate::sidecar_clients::video) struct VideoPollPayload {
    pub(in crate::sidecar_clients::video) backend: String,
    pub(in crate::sidecar_clients::video) base_url: String,
    pub(in crate::sidecar_clients::video) api_key: String,
    pub(in crate::sidecar_clients::video) provider_options_json: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(in crate::sidecar_clients::video) project_id: Option<i64>,
    pub(in crate::sidecar_clients::video) job_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::sidecar_clients::video) struct VideoGenerationOutput {
    pub(in crate::sidecar_clients::video) bytes_base64: String,
    pub(in crate::sidecar_clients::video) format: String,
    pub(in crate::sidecar_clients::video) provider_job_id: Option<String>,
    pub(in crate::sidecar_clients::video) metadata: Value,
}

#[derive(Debug)]
pub(in crate::sidecar_clients::video) enum VideoJobOutput {
    Pending,
    Running,
    Completed { result: VideoGenerationOutput },
    Failed { message: String },
}

#[derive(Debug)]
pub(in crate::sidecar_clients::video) enum VideoStatusOutput {
    Pending,
    Running,
    Completed,
    Failed { message: String },
    Cancelled,
    NotFound,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case", deny_unknown_fields)]
pub(in crate::sidecar_clients::video) enum VideoDeleteOutput {
    Deleted,
    NotFound,
}

impl<'de> Deserialize<'de> for VideoStatusOutput {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        let mut object = value
            .as_object()
            .cloned()
            .ok_or_else(|| D::Error::custom("video status output 必须是 JSON 对象"))?;
        let state = object
            .remove("state")
            .and_then(|value| value.as_str().map(str::to_string))
            .ok_or_else(|| D::Error::custom("video status output 缺少字符串 state"))?;
        let output = match state.as_str() {
            "pending" => Self::Pending,
            "running" => Self::Running,
            "completed" => Self::Completed,
            "cancelled" => Self::Cancelled,
            "not_found" => Self::NotFound,
            "failed" => {
                let message = object
                    .remove("message")
                    .and_then(|value| value.as_str().map(str::to_string))
                    .ok_or_else(|| {
                        D::Error::custom("failed video status output 缺少字符串 message")
                    })?;
                Self::Failed { message }
            }
            other => {
                return Err(D::Error::custom(format!(
                    "未知 video status state: {other}"
                )));
            }
        };
        if let Some(field) = object.keys().next() {
            return Err(D::Error::custom(format!(
                "video status output 包含未知字段 `{field}`"
            )));
        }
        Ok(output)
    }
}

impl<'de> Deserialize<'de> for VideoJobOutput {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        let mut object = value
            .as_object()
            .cloned()
            .ok_or_else(|| D::Error::custom("video job output 必须是 JSON 对象"))?;
        let state = object
            .remove("state")
            .and_then(|value| value.as_str().map(str::to_string))
            .ok_or_else(|| D::Error::custom("video job output 缺少字符串 state"))?;

        let output = match state.as_str() {
            "pending" => Self::Pending,
            "running" => Self::Running,
            "completed" => {
                let result = object
                    .remove("result")
                    .ok_or_else(|| D::Error::custom("completed video job output 缺少 result"))?;
                Self::Completed {
                    result: serde_json::from_value(result).map_err(D::Error::custom)?,
                }
            }
            "failed" => {
                let message = object
                    .remove("message")
                    .and_then(|value| value.as_str().map(str::to_string))
                    .ok_or_else(|| {
                        D::Error::custom("failed video job output 缺少字符串 message")
                    })?;
                Self::Failed { message }
            }
            other => return Err(D::Error::custom(format!("未知 video job state: {other}"))),
        };

        if let Some(field) = object.keys().next() {
            return Err(D::Error::custom(format!(
                "video job output 包含未知字段 `{field}`"
            )));
        }
        Ok(output)
    }
}

#[cfg(test)]
mod strict_output_tests {
    use super::*;

    #[test]
    fn video_outputs_reject_unknown_fields() {
        let error = serde_json::from_value::<VideoGenerationOutput>(serde_json::json!({
            "bytes_base64": "dmlkZW8=",
            "format": "mp4",
            "provider_job_id": null,
            "metadata": {},
            "legacy_task_id": "task"
        }))
        .expect_err("video output must reject unknown fields");
        assert!(error.to_string().contains("legacy_task_id"));

        let error = serde_json::from_value::<VideoJobOutput>(serde_json::json!({
            "state": "pending",
            "legacy_status": true
        }))
        .expect_err("video job output must reject unknown fields");
        assert!(error.to_string().contains("legacy_status"));

        let error = serde_json::from_value::<VideoStatusOutput>(serde_json::json!({
            "state": "completed",
            "result": {}
        }))
        .expect_err("status-only output must reject artifact payloads");
        assert!(error.to_string().contains("result"));
    }
}
