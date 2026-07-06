use a4n_core::error::AppError;
use a4n_core::models::ai::{embedding_config, image_gen_config, llm_config};
use a4n_engine::embedding::fastembed::fastembed_model_dimension;
use a4n_engine::llm::http::normalize_request_options_json;
use serde_json::Value;

use super::{CreateEmbeddingConfigParams, CreateImageConfigParams};

pub fn validate_image_config_params(
    mut params: CreateImageConfigParams,
) -> Result<CreateImageConfigParams, AppError> {
    params.name = normalize_config_name(params.name, "图片配置")?;
    params.backend = required_config_value(&params.backend, "图片生成配置", "backend")?;
    params.model = required_config_value(&params.model, "图片生成配置", "model")?;
    params.base_url = required_config_value(&params.base_url, "图片生成配置", "base_url")?;

    match params.backend.as_str() {
        "openai-compatible-image"
        | "openai-compatible-image-edit"
        | "openai-chat-image"
        | "cliproxy-image" => {
            params.api_key_encrypted = normalize_optional_config_value(params.api_key_encrypted);
        }
        "stable-diffusion" | "comfyui" => {
            params.api_key_encrypted = None;
        }
        other => {
            return Err(AppError::InvalidArgument(format!(
                "未知图片生成后端: {other}"
            )));
        }
    }

    if params.max_concurrent <= 0 {
        return Err(AppError::InvalidArgument(
            "图片配置 max_concurrent 必须大于 0".into(),
        ));
    }
    if params.default_width <= 0 {
        return Err(AppError::InvalidArgument(
            "图片配置 default_width 必须大于 0".into(),
        ));
    }
    if params.default_height <= 0 {
        return Err(AppError::InvalidArgument(
            "图片配置 default_height 必须大于 0".into(),
        ));
    }
    params.provider_options_json =
        normalize_image_provider_options_json(&params.backend, params.provider_options_json)?;

    Ok(params)
}

pub fn validate_image_config_model(
    model: &image_gen_config::Model,
) -> Result<CreateImageConfigParams, AppError> {
    validate_image_config_params(CreateImageConfigParams {
        name: model.name.clone(),
        backend: model.backend.clone(),
        base_url: model
            .base_url
            .clone()
            .ok_or_else(|| AppError::InvalidArgument("图片生成配置缺少 base_url".into()))?,
        api_key_encrypted: model.api_key_encrypted.clone(),
        model: model.model.clone(),
        max_concurrent: model.max_concurrent,
        default_width: model.default_width,
        default_height: model.default_height,
        provider_options_json: model.provider_options_json.clone(),
    })
}

fn normalize_image_provider_options_json(
    backend: &str,
    value: Option<Value>,
) -> Result<Option<Value>, AppError> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    if backend != "cliproxy-image" {
        return Err(AppError::InvalidArgument(format!(
            "图片 provider_options_json 仅支持 cliproxy-image 后端，当前后端为 {backend}"
        )));
    }
    if let Value::Object(map) = &value
        && let Some(key) = map.keys().find(|key| key.as_str() != "grok_reference_mode")
    {
        return Err(AppError::InvalidArgument(format!(
            "图片 provider_options_json 包含未知字段：{key}"
        )));
    }
    normalize_grok_reference_mode_options_json(true, "图片", Some(value))
}

pub(crate) fn normalize_grok_reference_mode_options_json(
    enabled: bool,
    kind: &str,
    value: Option<Value>,
) -> Result<Option<Value>, AppError> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    if !enabled {
        return Ok(None);
    }
    let Value::Object(mut map) = value else {
        return Err(AppError::InvalidArgument(format!(
            "{kind} provider_options_json 必须是 JSON 对象"
        )));
    };
    let Some(mode) = map
        .remove("grok_reference_mode")
        .filter(|value| !value.is_null())
    else {
        return Ok(None);
    };
    let mode = mode.as_str().map(str::trim).ok_or_else(|| {
        AppError::InvalidArgument(format!(
            "{kind} provider_options_json.grok_reference_mode 必须是字符串"
        ))
    })?;
    let normalized = match mode {
        "reference" | "first_frame" => mode,
        other => {
            return Err(AppError::InvalidArgument(format!(
                "{kind} provider_options_json.grok_reference_mode 无效：{other}"
            )));
        }
    };
    Ok(Some(serde_json::json!({
        "grok_reference_mode": normalized,
    })))
}

pub fn validate_embedding_config_params(
    mut params: CreateEmbeddingConfigParams,
) -> Result<CreateEmbeddingConfigParams, AppError> {
    params.name = normalize_config_name(params.name, "嵌入配置")?;
    params.backend = normalize_embedding_backend(&params.backend)?;
    params.model = required_config_value(&params.model, "嵌入配置", "model")?;
    params.dimension = positive_i32(params.dimension, "嵌入配置 dimension")?;
    params.batch_size = positive_i32(params.batch_size, "嵌入配置 batch_size")?;
    params.base_url = normalize_optional_config_value(params.base_url);
    params.api_key_encrypted = normalize_optional_config_value(params.api_key_encrypted);
    params.local_model_path = normalize_optional_config_value(params.local_model_path);

    if params.backend == "api" && params.base_url.is_none() {
        return Err(AppError::InvalidArgument(
            "api 嵌入后端必须填写 base_url".into(),
        ));
    }
    if params.backend == "fastembed" {
        let actual = fastembed_model_dimension(&params.model).map_err(|err| {
            AppError::InvalidArgument(format!("fastembed 模型 {} 不可用：{err}", params.model))
        })?;
        let configured = usize::try_from(params.dimension)
            .map_err(|_| AppError::InvalidArgument("嵌入配置 dimension 必须为正整数".into()))?;
        if configured != actual {
            return Err(AppError::InvalidArgument(format!(
                "fastembed 模型 {} 实际输出 {actual} 维，当前配置为 {configured} 维",
                params.model
            )));
        }
    }

    Ok(params)
}

pub(super) fn validate_embedding_config_model(
    model: &embedding_config::Model,
) -> Result<(), AppError> {
    validate_embedding_config_params(CreateEmbeddingConfigParams {
        name: model.name.clone(),
        backend: model.backend.clone(),
        base_url: model.base_url.clone(),
        api_key_encrypted: model.api_key_encrypted.clone(),
        model: model.model.clone(),
        dimension: model.dimension,
        batch_size: model.batch_size,
        local_model_path: model.local_model_path.clone(),
    })
    .map(|_| ())
}

pub(super) fn validate_llm_config_model(model: &llm_config::Model) -> Result<(), AppError> {
    normalize_config_name(&model.name, "LLM配置")?;
    required_config_value(&model.provider, "LLM配置", "provider")?;
    required_config_value(&model.base_url, "LLM配置", "base_url")?;
    required_config_value(&model.model, "LLM配置", "model")?;
    positive_i32(model.max_concurrent, "LLM配置 max_concurrent")?;
    positive_i32(model.timeout_secs, "LLM配置 timeout_secs")?;
    positive_i32(model.max_context_tokens, "LLM配置 max_context_tokens")?;
    positive_i32(model.max_output_tokens, "LLM配置 max_output_tokens")?;
    if model.max_output_tokens >= model.max_context_tokens {
        return Err(AppError::InvalidArgument(
            "LLM配置 max_output_tokens 必须小于 max_context_tokens".into(),
        ));
    }
    normalize_llm_request_options_json(model.request_options_json.clone())?;
    Ok(())
}

pub(super) fn normalize_llm_request_options_json(
    value: Option<Value>,
) -> Result<Option<Value>, AppError> {
    normalize_request_options_json(value).map_err(|err| {
        AppError::InvalidArgument(format!(
            "LLM配置 request_options_json 无效：{}",
            app_error_plain_message(err)
        ))
    })
}

fn app_error_plain_message(err: AppError) -> String {
    match err {
        AppError::InvalidArgument(message)
        | AppError::ConfigError(message)
        | AppError::LlmError(message)
        | AppError::Timeout(message) => message,
        other => other.to_string(),
    }
}

fn normalize_embedding_backend(value: &str) -> Result<String, AppError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "api" => Ok("api".to_string()),
        "fastembed" => Ok("fastembed".to_string()),
        "local_hash" => Ok("local_hash".to_string()),
        "" => Err(AppError::InvalidArgument("嵌入后端不能为空".into())),
        other => Err(AppError::InvalidArgument(format!(
            "不支持的嵌入后端: {other}"
        ))),
    }
}

pub(super) fn required_config_value(
    value: &str,
    kind: &str,
    field: &str,
) -> Result<String, AppError> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(AppError::InvalidArgument(format!("{kind}缺少 {field}")));
    }
    Ok(value)
}

fn normalize_optional_config_value(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(super) fn positive_i32(value: i32, field: &str) -> Result<i32, AppError> {
    if value <= 0 {
        return Err(AppError::InvalidArgument(format!("{field} 必须大于 0")));
    }
    Ok(value)
}

pub(super) fn normalize_config_name(
    value: impl AsRef<str>,
    kind: &str,
) -> Result<String, AppError> {
    let value = value.as_ref().trim().to_string();
    if value.is_empty() {
        return Err(AppError::InvalidArgument(format!("{kind}名称不能为空")));
    }
    Ok(value)
}

pub(super) fn ensure_name_available(
    kind: &str,
    name: &str,
    existing_count: u64,
) -> Result<(), AppError> {
    if existing_count > 0 {
        return Err(AppError::InvalidArgument(format!(
            "{kind}名称已存在: {name}"
        )));
    }
    Ok(())
}
