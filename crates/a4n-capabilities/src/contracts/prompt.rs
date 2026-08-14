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
use a4n_service_protocol::{ServiceRequest, ServiceResponse};
use async_trait::async_trait;

use super::{CapabilityClient, CapabilityResult};

#[async_trait]
pub trait PromptClient: CapabilityClient {
    async fn list(
        &self,
        request: ServiceRequest<PromptEmptyPayload>,
    ) -> CapabilityResult<ServiceResponse<PromptListOutput>>;
    async fn get(
        &self,
        request: ServiceRequest<PromptNamePayload>,
    ) -> CapabilityResult<ServiceResponse<PromptEntry>>;
    async fn render(
        &self,
        request: ServiceRequest<PromptRenderPayload>,
    ) -> CapabilityResult<ServiceResponse<PromptRenderOutput>>;
    async fn validate(
        &self,
        request: ServiceRequest<PromptValidatePayload>,
    ) -> CapabilityResult<ServiceResponse<PromptValidateOutput>>;
    async fn render_ab(
        &self,
        request: ServiceRequest<PromptRenderPayload>,
    ) -> CapabilityResult<ServiceResponse<PromptRenderAbOutput>>;
    async fn hydrate_project_overrides(
        &self,
        request: ServiceRequest<PromptHydrateOverridesPayload>,
    ) -> CapabilityResult<ServiceResponse<()>>;
    async fn set_project_override(
        &self,
        request: ServiceRequest<PromptSetProjectOverridePayload>,
    ) -> CapabilityResult<ServiceResponse<()>>;
    async fn remove_project_override(
        &self,
        request: ServiceRequest<PromptNamePayload>,
    ) -> CapabilityResult<ServiceResponse<()>>;
    async fn load_defaults(
        &self,
        request: ServiceRequest<PromptEmptyPayload>,
    ) -> CapabilityResult<ServiceResponse<CountOutput>>;
    async fn update(
        &self,
        request: ServiceRequest<PromptUpdatePayload>,
    ) -> CapabilityResult<ServiceResponse<PromptEntry>>;
    async fn update_many(
        &self,
        request: ServiceRequest<PromptUpdateManyPayload>,
    ) -> CapabilityResult<ServiceResponse<Vec<PromptEntry>>>;
    async fn reset(
        &self,
        request: ServiceRequest<PromptNamePayload>,
    ) -> CapabilityResult<ServiceResponse<()>>;
    async fn version_history(
        &self,
        request: ServiceRequest<PromptNamePayload>,
    ) -> CapabilityResult<ServiceResponse<PromptVersionHistoryOutput>>;
    async fn diff(
        &self,
        request: ServiceRequest<PromptVersionDiffPayload>,
    ) -> CapabilityResult<ServiceResponse<PromptVersionDiffOutput>>;
    async fn categories(
        &self,
        request: ServiceRequest<PromptEmptyPayload>,
    ) -> CapabilityResult<ServiceResponse<Vec<PromptCategory>>>;
}
