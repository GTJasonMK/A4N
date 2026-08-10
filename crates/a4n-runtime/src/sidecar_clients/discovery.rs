use std::{future::Future, sync::Arc};

use a4n_capabilities::{CapabilityResult, JsonRequest, JsonResponse};
use a4n_model_discovery::ModelDiscoveryResult;

use crate::state::RuntimeProfile;

use super::{global_provider_service_request, response_payload, service_error_string};

pub(crate) async fn discover_models_with_profile<C, F, Fut>(
    profile: RuntimeProfile,
    client: Option<Arc<C>>,
    capability_label: &str,
    service_name: &str,
    capability_version: &str,
    provider: &str,
    base_url: &str,
    api_key: Option<&str>,
    discover: F,
) -> Result<ModelDiscoveryResult, String>
where
    C: ?Sized,
    F: FnOnce(Arc<C>, JsonRequest) -> Fut,
    Fut: Future<Output = CapabilityResult<JsonResponse>>,
{
    if profile.uses_sidecars() {
        let client = client.ok_or_else(|| {
            format!("{capability_label} sidecar capability 未配置，无法探测模型列表")
        })?;
        let request = global_provider_service_request(
            service_name,
            capability_version,
            model_discovery_payload(provider, base_url, api_key),
        )?;
        let response = discover(client, request)
            .await
            .map_err(service_error_string)?;
        return response_payload(response, service_name);
    }

    a4n_model_discovery::discover_models(provider, base_url, api_key).await
}

fn model_discovery_payload(
    provider: &str,
    base_url: &str,
    api_key: Option<&str>,
) -> a4n_model_discovery::ModelDiscoveryRequest {
    a4n_model_discovery::ModelDiscoveryRequest {
        provider: provider.to_string(),
        base_url: base_url.to_string(),
        api_key: api_key.map(str::to_string),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use a4n_service_protocol::ServiceResponse;
    use serde_json::json;

    use super::*;

    #[tokio::test]
    async fn sidecar_discovery_route_builds_and_decodes_the_shared_request() {
        let result = discover_models_with_profile(
            RuntimeProfile::Sidecar,
            Some(Arc::new(())),
            "image",
            "image",
            "image-v1",
            "openai-compatible-image",
            "https://provider.example/v1",
            Some("secret"),
            |_, request| async move {
                assert_eq!(request.capability_version, "image-v1");
                assert_eq!(request.payload["provider"], "openai-compatible-image");
                assert_eq!(request.payload["api_key"], "secret");
                Ok(ServiceResponse::ok(
                    "trace-1".to_string(),
                    json!({"models": [{"id": "image-model", "name": null, "owned_by": null}]}),
                ))
            },
        )
        .await
        .expect("sidecar response should decode");

        assert_eq!(result.models[0].id, "image-model");
    }
}
