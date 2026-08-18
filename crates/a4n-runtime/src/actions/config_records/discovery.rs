use super::shared::{ModelDiscoveryInput, require_config_admin};
use crate::{
    AppState,
    actions::app_error_message,
    sidecar_clients::{ModelDiscoveryKind, discover_models},
};
use a4n_model_discovery::ModelDiscoveryResult;

pub async fn detect_llm_config_models(
    state: &AppState,
    user_id: i64,
    input: ModelDiscoveryInput,
) -> Result<ModelDiscoveryResult, String> {
    detect_config_models(state, user_id, ModelDiscoveryKind::Llm, input).await
}

pub async fn detect_image_config_models(
    state: &AppState,
    user_id: i64,
    input: ModelDiscoveryInput,
) -> Result<ModelDiscoveryResult, String> {
    detect_config_models(state, user_id, ModelDiscoveryKind::Image, input).await
}

pub async fn detect_video_config_models(
    state: &AppState,
    user_id: i64,
    input: ModelDiscoveryInput,
) -> Result<ModelDiscoveryResult, String> {
    detect_config_models(state, user_id, ModelDiscoveryKind::Video, input).await
}

async fn detect_config_models(
    state: &AppState,
    user_id: i64,
    kind: ModelDiscoveryKind,
    input: ModelDiscoveryInput,
) -> Result<ModelDiscoveryResult, String> {
    let api_key = resolve_model_discovery_api_key(state, user_id, kind, &input).await?;
    discover_models(
        state.runtime_profile(),
        state.capabilities(),
        kind,
        &input.provider,
        &input.base_url,
        api_key.as_deref(),
    )
    .await
}

async fn resolve_model_discovery_api_key(
    state: &AppState,
    user_id: i64,
    kind: ModelDiscoveryKind,
    input: &ModelDiscoveryInput,
) -> Result<Option<String>, String> {
    let explicit_api_key = normalized_explicit_api_key(input)?;
    if !input.use_saved_api_key {
        return Ok(explicit_api_key);
    }
    require_config_admin(state, user_id)
        .await
        .map_err(app_error_message)?;
    let config_id = input
        .config_id
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("{}模型探测需要有效的已保存配置 ID", kind.label()))?;
    let saved_api_key = match kind {
        ModelDiscoveryKind::Llm => {
            state
                .orchestrator()
                .llm_config_by_id(config_id)
                .await
                .map_err(app_error_message)?
                .api_key_encrypted
        }
        ModelDiscoveryKind::Image => {
            state
                .orchestrator()
                .image_config_by_id(config_id)
                .await
                .map_err(app_error_message)?
                .api_key_encrypted
        }
        ModelDiscoveryKind::Video => {
            state
                .orchestrator()
                .video_config_by_id(config_id)
                .await
                .map_err(app_error_message)?
                .api_key_encrypted
        }
    };
    let saved_api_key = saved_api_key
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            format!(
                "{}模型探测请求使用已保存密钥，但配置 {config_id} 没有可用 API Key",
                kind.label()
            )
        })?;
    Ok(Some(saved_api_key.to_string()))
}

fn normalized_explicit_api_key(input: &ModelDiscoveryInput) -> Result<Option<String>, String> {
    let api_key = input
        .api_key
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    if input.use_saved_api_key && api_key.is_some() {
        return Err("模型探测不能同时提供 api_key 并请求使用已保存密钥".to_string());
    }
    Ok(api_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_and_saved_discovery_keys_are_mutually_exclusive() {
        let input = ModelDiscoveryInput {
            provider: "openai".to_string(),
            base_url: "https://example.invalid".to_string(),
            api_key: Some(" key ".to_string()),
            config_id: Some(1),
            use_saved_api_key: true,
        };

        assert!(normalized_explicit_api_key(&input).is_err());
    }
}
