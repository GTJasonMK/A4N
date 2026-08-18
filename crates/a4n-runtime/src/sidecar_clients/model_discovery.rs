use a4n_capabilities::A4nCapabilities;
use a4n_model_discovery::{ModelDiscoveryRequest, ModelDiscoveryResult};
use a4n_service_protocol::{
    IMAGE_SERVICE_NAME, LLM_SERVICE_NAME, VIDEO_SERVICE_NAME,
    expected_capability_version_for_service,
};

use crate::state::RuntimeProfile;

use super::{global_provider_service_request, response_payload, service_error_string};

#[derive(Clone, Copy)]
pub(crate) enum ModelDiscoveryKind {
    Llm,
    Image,
    Video,
}

pub(crate) async fn discover_models(
    profile: RuntimeProfile,
    capabilities: &A4nCapabilities,
    kind: ModelDiscoveryKind,
    provider: &str,
    base_url: &str,
    api_key: Option<&str>,
) -> Result<ModelDiscoveryResult, String> {
    if !profile.uses_sidecars() {
        return a4n_model_discovery::discover_models(provider, base_url, api_key).await;
    }

    let service_name = kind.service_name();
    let request = global_provider_service_request(
        expected_capability_version_for_service(service_name)
            .expect("model discovery service must have a capability version"),
        ModelDiscoveryRequest {
            provider: provider.to_string(),
            base_url: base_url.to_string(),
            api_key: api_key.map(str::to_string),
        },
    )?;
    let response = match kind {
        ModelDiscoveryKind::Llm => {
            capabilities
                .llm
                .as_ref()
                .ok_or_else(|| kind.missing_capability_message())?
                .discover_models(request)
                .await
        }
        ModelDiscoveryKind::Image => {
            capabilities
                .image
                .as_ref()
                .ok_or_else(|| kind.missing_capability_message())?
                .discover_models(request)
                .await
        }
        ModelDiscoveryKind::Video => {
            capabilities
                .video
                .as_ref()
                .ok_or_else(|| kind.missing_capability_message())?
                .discover_models(request)
                .await
        }
    }
    .map_err(service_error_string)?;
    response_payload(response, service_name)
}

impl ModelDiscoveryKind {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Llm => "LLM",
            Self::Image => "图片",
            Self::Video => "视频",
        }
    }

    fn service_name(self) -> &'static str {
        match self {
            Self::Llm => LLM_SERVICE_NAME,
            Self::Image => IMAGE_SERVICE_NAME,
            Self::Video => VIDEO_SERVICE_NAME,
        }
    }

    fn missing_capability_message(self) -> String {
        format!(
            "{} sidecar capability 未配置，无法探测模型列表",
            self.label()
        )
    }
}
