use std::sync::Arc;

use crate::provider_runtime::DynamicProviderFacade;
use a4n_capabilities::A4nCapabilities;
use a4n_error::AppError;
use a4n_llm::provider::LlmProvider;

use crate::state::RuntimeProfile;

pub(crate) fn create_llm_provider(
    facade: DynamicProviderFacade,
    profile: RuntimeProfile,
    capabilities: &A4nCapabilities,
) -> Result<Arc<dyn LlmProvider>, AppError> {
    let sidecar_client = required_sidecar_client(profile, capabilities.llm.as_ref(), "LLM")?;
    if let Some(client) = sidecar_client {
        return Ok(Arc::new(
            crate::dynamic_llm::DynamicLlmProvider::new_with_sidecar_client(facade, client),
        ));
    }

    Ok(Arc::new(crate::dynamic_llm::DynamicLlmProvider::new(
        facade,
    )))
}

pub(super) fn required_sidecar_client<T: Clone>(
    profile: RuntimeProfile,
    client: Option<&T>,
    capability: &str,
) -> Result<Option<T>, AppError> {
    if profile.uses_sidecars() {
        return client
            .cloned()
            .map(Some)
            .ok_or_else(|| AppError::ConfigError(missing_sidecar_capability_message(capability)));
    }
    Ok(None)
}

pub(super) fn missing_sidecar_capability_message(capability: &str) -> String {
    format!("{capability} sidecar capability 未配置，无法启用 sidecar profile")
}
