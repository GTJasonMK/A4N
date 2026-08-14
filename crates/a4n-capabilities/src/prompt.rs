use super::contracts::{CapabilityClient, CapabilityResult, PromptClient};
use super::transport::{HttpClientFactory, HttpSidecarTransport};
use a4n_prompt::{
    contracts::{
        CountOutput, PromptEmptyPayload, PromptHydrateOverridesPayload, PromptListOutput,
        PromptNamePayload, PromptRenderAbOutput, PromptRenderOutput, PromptRenderPayload,
        PromptSetProjectOverridePayload, PromptUpdateManyPayload, PromptUpdatePayload,
        PromptValidateOutput, PromptValidatePayload, PromptVersionDiffOutput,
        PromptVersionDiffPayload, PromptVersionHistoryOutput,
    },
    engine::{PromptCategory, PromptEntry},
};
use a4n_service_protocol::{PROMPT_SERVICE_NAME, ServiceHealth, ServiceRequest, ServiceResponse};
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

    pub fn new_with_auth_token(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            transport: HttpSidecarTransport::new(base_url)
                .with_expected_service(PROMPT_SERVICE_NAME)
                .with_auth_token(token),
        }
    }

    pub fn with_http_client_factory(mut self, factory: HttpClientFactory) -> Self {
        self.transport = self.transport.with_http_client_factory(factory);
        self
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
    async fn list(
        &self,
        request: ServiceRequest<PromptEmptyPayload>,
    ) -> CapabilityResult<ServiceResponse<PromptListOutput>> {
        self.transport.post_typed("/prompt/list", request).await
    }

    async fn get(
        &self,
        request: ServiceRequest<PromptNamePayload>,
    ) -> CapabilityResult<ServiceResponse<PromptEntry>> {
        self.transport.post_typed("/prompt/get", request).await
    }

    async fn render(
        &self,
        request: ServiceRequest<PromptRenderPayload>,
    ) -> CapabilityResult<ServiceResponse<PromptRenderOutput>> {
        self.transport.post_typed("/prompt/render", request).await
    }

    async fn validate(
        &self,
        request: ServiceRequest<PromptValidatePayload>,
    ) -> CapabilityResult<ServiceResponse<PromptValidateOutput>> {
        self.transport.post_typed("/prompt/validate", request).await
    }

    async fn render_ab(
        &self,
        request: ServiceRequest<PromptRenderPayload>,
    ) -> CapabilityResult<ServiceResponse<PromptRenderAbOutput>> {
        self.transport
            .post_typed("/prompt/render_ab", request)
            .await
    }

    async fn hydrate_project_overrides(
        &self,
        request: ServiceRequest<PromptHydrateOverridesPayload>,
    ) -> CapabilityResult<ServiceResponse<()>> {
        self.transport
            .post_typed("/prompt/override/hydrate", request)
            .await
    }

    async fn set_project_override(
        &self,
        request: ServiceRequest<PromptSetProjectOverridePayload>,
    ) -> CapabilityResult<ServiceResponse<()>> {
        self.transport
            .post_typed("/prompt/override/set", request)
            .await
    }

    async fn remove_project_override(
        &self,
        request: ServiceRequest<PromptNamePayload>,
    ) -> CapabilityResult<ServiceResponse<()>> {
        self.transport
            .post_typed("/prompt/override/remove", request)
            .await
    }

    async fn load_defaults(
        &self,
        request: ServiceRequest<PromptEmptyPayload>,
    ) -> CapabilityResult<ServiceResponse<CountOutput>> {
        self.transport
            .post_typed("/prompt/load_defaults", request)
            .await
    }

    async fn update(
        &self,
        request: ServiceRequest<PromptUpdatePayload>,
    ) -> CapabilityResult<ServiceResponse<PromptEntry>> {
        self.transport.post_typed("/prompt/update", request).await
    }

    async fn update_many(
        &self,
        request: ServiceRequest<PromptUpdateManyPayload>,
    ) -> CapabilityResult<ServiceResponse<Vec<PromptEntry>>> {
        self.transport
            .post_typed("/prompt/update_many", request)
            .await
    }

    async fn reset(
        &self,
        request: ServiceRequest<PromptNamePayload>,
    ) -> CapabilityResult<ServiceResponse<()>> {
        self.transport.post_typed("/prompt/reset", request).await
    }

    async fn version_history(
        &self,
        request: ServiceRequest<PromptNamePayload>,
    ) -> CapabilityResult<ServiceResponse<PromptVersionHistoryOutput>> {
        self.transport
            .post_typed("/prompt/version_history", request)
            .await
    }

    async fn diff(
        &self,
        request: ServiceRequest<PromptVersionDiffPayload>,
    ) -> CapabilityResult<ServiceResponse<PromptVersionDiffOutput>> {
        self.transport
            .post_typed("/prompt/version_diff", request)
            .await
    }

    async fn categories(
        &self,
        request: ServiceRequest<PromptEmptyPayload>,
    ) -> CapabilityResult<ServiceResponse<Vec<PromptCategory>>> {
        self.transport
            .post_typed("/prompt/categories", request)
            .await
    }
}
