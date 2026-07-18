use super::contracts::{
    CapabilityClient, CapabilityResult, JsonRequest, JsonResponse, PromptClient,
};
use super::transport::HttpSidecarTransport;
use a4n_service_protocol::{PROMPT_SERVICE_NAME, ServiceHealth};
use async_trait::async_trait;

pub struct HttpPromptClient {
    pub(super) transport: HttpSidecarTransport,
}

impl HttpPromptClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            transport: HttpSidecarTransport::new(base_url)
                .with_expected_service(PROMPT_SERVICE_NAME),
        }
    }
}

#[async_trait]
impl CapabilityClient for HttpPromptClient {
    async fn health(&self) -> CapabilityResult<ServiceHealth> {
        self.transport.health().await
    }
}

#[async_trait]
impl PromptClient for HttpPromptClient {
    async fn list(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport.post_json("/prompt/list", request).await
    }

    async fn get(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport.post_json("/prompt/get", request).await
    }

    async fn render(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport.post_json("/prompt/render", request).await
    }

    async fn validate(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport.post_json("/prompt/validate", request).await
    }

    async fn render_ab(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport.post_json("/prompt/render_ab", request).await
    }

    async fn hydrate_project_overrides(
        &self,
        request: JsonRequest,
    ) -> CapabilityResult<JsonResponse> {
        self.transport
            .post_json("/prompt/override/hydrate", request)
            .await
    }

    async fn set_project_override(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport
            .post_json("/prompt/override/set", request)
            .await
    }

    async fn remove_project_override(
        &self,
        request: JsonRequest,
    ) -> CapabilityResult<JsonResponse> {
        self.transport
            .post_json("/prompt/override/remove", request)
            .await
    }

    async fn load_defaults(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport
            .post_json("/prompt/load_defaults", request)
            .await
    }

    async fn update(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport.post_json("/prompt/update", request).await
    }

    async fn update_many(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport
            .post_json("/prompt/update_many", request)
            .await
    }

    async fn reset(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport.post_json("/prompt/reset", request).await
    }

    async fn version_history(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport
            .post_json("/prompt/version_history", request)
            .await
    }

    async fn get_version(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport
            .post_json("/prompt/version_get", request)
            .await
    }

    async fn diff(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport
            .post_json("/prompt/version_diff", request)
            .await
    }

    async fn dependency_graph(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport
            .post_json("/prompt/dependency_graph", request)
            .await
    }

    async fn categories(&self, request: JsonRequest) -> CapabilityResult<JsonResponse> {
        self.transport
            .post_json("/prompt/categories", request)
            .await
    }
}
