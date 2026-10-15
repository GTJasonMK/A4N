//! Typed client for the prompt sidecar service.

use a4n_prompt_contract::{
    CountOutput, PROMPT_RENDER_CAPABILITY_VERSION, PROMPT_SERVICE_NAME, PromptCategory,
    PromptEmptyPayload, PromptEntry, PromptHydrateOverridesPayload, PromptListOutput,
    PromptNamePayload, PromptRenderAbOutput, PromptRenderAbPayload, PromptRenderOutput,
    PromptRenderPayload, PromptSetProjectOverridePayload, PromptUpdateManyPayload,
    PromptUpdatePayload, PromptValidateOutput, PromptValidatePayload, PromptVersionDiffOutput,
    PromptVersionDiffPayload, PromptVersionHistoryOutput,
};
use a4n_service_client::{
    HttpClientFactory, HttpSidecarTransport, ServiceClient, ServiceClientConfig,
    ServiceClientResult,
};
use a4n_service_protocol::{
    ServiceHealth, ServiceRequest, ServiceResponse, sidecar_resource_policy,
};
use async_trait::async_trait;

pub struct HttpPromptClient {
    transport: HttpSidecarTransport,
}

impl HttpPromptClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            transport: HttpSidecarTransport::new(base_url, service_client_config()),
        }
    }

    pub fn new_with_auth_token(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            transport: HttpSidecarTransport::new(base_url, service_client_config())
                .with_auth_token(token),
        }
    }

    pub fn with_http_client_factory(mut self, factory: HttpClientFactory) -> Self {
        self.transport = self.transport.with_http_client_factory(factory);
        self
    }
}

fn service_client_config() -> ServiceClientConfig {
    ServiceClientConfig::new(
        PROMPT_SERVICE_NAME,
        PROMPT_RENDER_CAPABILITY_VERSION,
        sidecar_resource_policy(PROMPT_SERVICE_NAME)
            .expect("prompt service must have a resource policy"),
    )
}

#[async_trait]
impl ServiceClient for HttpPromptClient {
    async fn health(&self) -> ServiceClientResult<ServiceHealth> {
        self.transport.health().await
    }
}

#[async_trait]
pub trait PromptClient: ServiceClient {
    async fn list(
        &self,
        request: ServiceRequest<PromptEmptyPayload>,
    ) -> ServiceClientResult<ServiceResponse<PromptListOutput>>;
    async fn get(
        &self,
        request: ServiceRequest<PromptNamePayload>,
    ) -> ServiceClientResult<ServiceResponse<PromptEntry>>;
    async fn render(
        &self,
        request: ServiceRequest<PromptRenderPayload>,
    ) -> ServiceClientResult<ServiceResponse<PromptRenderOutput>>;
    async fn validate(
        &self,
        request: ServiceRequest<PromptValidatePayload>,
    ) -> ServiceClientResult<ServiceResponse<PromptValidateOutput>>;
    async fn render_ab(
        &self,
        request: ServiceRequest<PromptRenderAbPayload>,
    ) -> ServiceClientResult<ServiceResponse<PromptRenderAbOutput>>;
    async fn hydrate_project_overrides(
        &self,
        request: ServiceRequest<PromptHydrateOverridesPayload>,
    ) -> ServiceClientResult<ServiceResponse<()>>;
    async fn set_project_override(
        &self,
        request: ServiceRequest<PromptSetProjectOverridePayload>,
    ) -> ServiceClientResult<ServiceResponse<()>>;
    async fn remove_project_override(
        &self,
        request: ServiceRequest<PromptNamePayload>,
    ) -> ServiceClientResult<ServiceResponse<()>>;
    async fn load_defaults(
        &self,
        request: ServiceRequest<PromptEmptyPayload>,
    ) -> ServiceClientResult<ServiceResponse<CountOutput>>;
    async fn update(
        &self,
        request: ServiceRequest<PromptUpdatePayload>,
    ) -> ServiceClientResult<ServiceResponse<PromptEntry>>;
    async fn update_many(
        &self,
        request: ServiceRequest<PromptUpdateManyPayload>,
    ) -> ServiceClientResult<ServiceResponse<Vec<PromptEntry>>>;
    async fn reset(
        &self,
        request: ServiceRequest<PromptNamePayload>,
    ) -> ServiceClientResult<ServiceResponse<PromptEntry>>;
    async fn version_history(
        &self,
        request: ServiceRequest<PromptNamePayload>,
    ) -> ServiceClientResult<ServiceResponse<PromptVersionHistoryOutput>>;
    async fn diff(
        &self,
        request: ServiceRequest<PromptVersionDiffPayload>,
    ) -> ServiceClientResult<ServiceResponse<PromptVersionDiffOutput>>;
    async fn categories(
        &self,
        request: ServiceRequest<PromptEmptyPayload>,
    ) -> ServiceClientResult<ServiceResponse<Vec<PromptCategory>>>;
}

#[async_trait]
impl PromptClient for HttpPromptClient {
    async fn list(
        &self,
        request: ServiceRequest<PromptEmptyPayload>,
    ) -> ServiceClientResult<ServiceResponse<PromptListOutput>> {
        self.transport.post("/prompt/list", request).await
    }

    async fn get(
        &self,
        request: ServiceRequest<PromptNamePayload>,
    ) -> ServiceClientResult<ServiceResponse<PromptEntry>> {
        self.transport.post("/prompt/get", request).await
    }

    async fn render(
        &self,
        request: ServiceRequest<PromptRenderPayload>,
    ) -> ServiceClientResult<ServiceResponse<PromptRenderOutput>> {
        self.transport.post("/prompt/render", request).await
    }

    async fn validate(
        &self,
        request: ServiceRequest<PromptValidatePayload>,
    ) -> ServiceClientResult<ServiceResponse<PromptValidateOutput>> {
        self.transport.post("/prompt/validate", request).await
    }

    async fn render_ab(
        &self,
        request: ServiceRequest<PromptRenderAbPayload>,
    ) -> ServiceClientResult<ServiceResponse<PromptRenderAbOutput>> {
        self.transport.post("/prompt/render_ab", request).await
    }

    async fn hydrate_project_overrides(
        &self,
        request: ServiceRequest<PromptHydrateOverridesPayload>,
    ) -> ServiceClientResult<ServiceResponse<()>> {
        self.transport
            .post("/prompt/override/hydrate", request)
            .await
    }

    async fn set_project_override(
        &self,
        request: ServiceRequest<PromptSetProjectOverridePayload>,
    ) -> ServiceClientResult<ServiceResponse<()>> {
        self.transport.post("/prompt/override/set", request).await
    }

    async fn remove_project_override(
        &self,
        request: ServiceRequest<PromptNamePayload>,
    ) -> ServiceClientResult<ServiceResponse<()>> {
        self.transport
            .post("/prompt/override/remove", request)
            .await
    }

    async fn load_defaults(
        &self,
        request: ServiceRequest<PromptEmptyPayload>,
    ) -> ServiceClientResult<ServiceResponse<CountOutput>> {
        self.transport.post("/prompt/load_defaults", request).await
    }

    async fn update(
        &self,
        request: ServiceRequest<PromptUpdatePayload>,
    ) -> ServiceClientResult<ServiceResponse<PromptEntry>> {
        self.transport.post("/prompt/update", request).await
    }

    async fn update_many(
        &self,
        request: ServiceRequest<PromptUpdateManyPayload>,
    ) -> ServiceClientResult<ServiceResponse<Vec<PromptEntry>>> {
        self.transport.post("/prompt/update_many", request).await
    }

    async fn reset(
        &self,
        request: ServiceRequest<PromptNamePayload>,
    ) -> ServiceClientResult<ServiceResponse<PromptEntry>> {
        self.transport.post("/prompt/reset", request).await
    }

    async fn version_history(
        &self,
        request: ServiceRequest<PromptNamePayload>,
    ) -> ServiceClientResult<ServiceResponse<PromptVersionHistoryOutput>> {
        self.transport
            .post("/prompt/version_history", request)
            .await
    }

    async fn diff(
        &self,
        request: ServiceRequest<PromptVersionDiffPayload>,
    ) -> ServiceClientResult<ServiceResponse<PromptVersionDiffOutput>> {
        self.transport.post("/prompt/version_diff", request).await
    }

    async fn categories(
        &self,
        request: ServiceRequest<PromptEmptyPayload>,
    ) -> ServiceClientResult<ServiceResponse<Vec<PromptCategory>>> {
        self.transport.post("/prompt/categories", request).await
    }
}
