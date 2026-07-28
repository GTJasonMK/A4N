use std::future::Future;

pub use crate::actions::request_preview::{ModelRequestPreview, ModelRequestPreviewStep};
pub use a4n_api_contract::config::ModelDiscoveryInput;
pub use a4n_model_discovery::{DiscoveredModel, ModelDiscoveryResult};

pub(super) async fn model_discovery_api_key<F, Fut>(
    provided_api_key: Option<&str>,
    load_saved_api_key: F,
) -> Result<Option<String>, String>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<Option<String>, String>>,
{
    if let Some(api_key) = provided_api_key
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Ok(Some(api_key.to_string()));
    }
    Ok(load_saved_api_key()
        .await?
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty()))
}

pub(super) fn required_existing_config_id(
    config_id: Option<i64>,
    label: &str,
) -> Result<i64, String> {
    config_id
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("{label}模型探测需要有效的已保存配置 ID"))
}

pub(super) fn validate_runtime_snapshot_config_id(
    config_kind: &str,
    snapshot_id: i64,
    config_id: i64,
) -> Result<(), String> {
    if snapshot_id == config_id {
        return Ok(());
    }
    Err(format!(
        "runtime_config_snapshot.{config_kind}.id 必须等于 config_id：snapshot_id={snapshot_id}, config_id={config_id}",
    ))
}

pub(super) fn positive_i32_as_usize(value: i32, label: &str) -> Result<usize, String> {
    usize::try_from(value)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("{label} 必须为正整数"))
}
