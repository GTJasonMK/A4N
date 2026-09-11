use a4n_domain::ProviderError as AppError;
use a4n_video::VideoGenerationResult;
use base64::Engine as _;
use serde_json::Value;

use super::super::download::download_video_url;

pub(super) async fn decode_result(
    client: &reqwest::Client,
    value: &Value,
    provider_job_id: Option<String>,
) -> Result<Option<VideoGenerationResult>, AppError> {
    let Some(item) = first_data_item(value)? else {
        return Ok(None);
    };
    let format = required_video_format(item)?;

    if let Some(encoded) = optional_string(item, "b64_json", "data[0].b64_json")? {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|error| AppError::Internal(format!("视频 base64 解码失败：{error}")))?;
        return Ok(Some(VideoGenerationResult {
            bytes,
            format,
            provider_job_id,
            metadata: value.clone(),
        }));
    }

    if let Some(url) = optional_string(item, "url", "data[0].url")? {
        let (bytes, format) = download_video_url(client, url, Some(&format)).await?;
        return Ok(Some(VideoGenerationResult {
            bytes,
            format,
            provider_job_id,
            metadata: value.clone(),
        }));
    }

    Ok(None)
}

fn first_data_item(value: &Value) -> Result<Option<&Value>, AppError> {
    let Some(data) = value.get("data") else {
        return Ok(None);
    };
    let items = data
        .as_array()
        .ok_or_else(|| AppError::Internal(format!("视频响应 data 必须是数组")))?;
    let Some(item) = items.first() else {
        return Ok(None);
    };
    if !item.is_object() {
        return Err(AppError::Internal(format!("视频响应 data[0] 必须是对象")));
    }
    Ok(Some(item))
}

fn optional_string<'a>(
    value: &'a Value,
    key: &str,
    label: &str,
) -> Result<Option<&'a str>, AppError> {
    let Some(value) = value.get(key) else {
        return Ok(None);
    };
    let text = value
        .as_str()
        .ok_or_else(|| AppError::Internal(format!("{label} 必须是非空字符串")))?
        .trim();
    if text.is_empty() {
        return Err(AppError::Internal(format!("{label} 必须是非空字符串")));
    }
    Ok(Some(text))
}

pub(super) fn required_string<'a>(
    value: &'a Value,
    pointer: &str,
    label: &str,
) -> Result<&'a str, AppError> {
    let text = value
        .pointer(pointer)
        .ok_or_else(|| AppError::Internal(format!("视频响应缺少 {label}")))?
        .as_str()
        .ok_or_else(|| AppError::Internal(format!("{label} 必须是非空字符串")))?
        .trim();
    if text.is_empty() {
        return Err(AppError::Internal(format!("{label} 必须是非空字符串")));
    }
    Ok(text)
}

pub(super) fn required_video_format(item: &Value) -> Result<String, AppError> {
    let raw = optional_string(item, "format", "data[0].format")?
        .ok_or_else(|| AppError::Internal(format!("视频响应缺少 data[0].format")))?;
    let format = raw.to_ascii_lowercase();
    if matches!(format.as_str(), "mp4" | "webm" | "mov" | "m4v") {
        Ok(format)
    } else {
        Err(AppError::Internal(format!("不支持的视频格式：{format}")))
    }
}

pub(super) fn extract_job_id(value: &Value) -> Result<Option<String>, AppError> {
    let Some(value) = value.get("id") else {
        return Ok(None);
    };
    let id = value
        .as_str()
        .ok_or_else(|| AppError::Internal(format!("视频生成响应 id 必须是非空字符串")))?
        .trim();
    if id.is_empty() {
        return Err(AppError::Internal(format!(
            "视频生成响应 id 必须是非空字符串"
        )));
    }
    Ok(Some(id.to_string()))
}
