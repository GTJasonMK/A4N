use async_trait::async_trait;

use super::{CapabilityClient, CapabilityResult, JsonRequest, JsonResponse};

#[async_trait]
pub trait PromptClient: CapabilityClient {
    async fn list(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
    async fn get(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
    async fn render(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
    async fn validate(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
    async fn render_ab(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
    async fn hydrate_project_overrides(
        &self,
        request: JsonRequest,
    ) -> CapabilityResult<JsonResponse>;
    async fn set_project_override(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
    async fn remove_project_override(&self, request: JsonRequest)
    -> CapabilityResult<JsonResponse>;
    async fn load_defaults(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
    async fn update(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
    async fn update_many(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
    async fn reset(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
    async fn version_history(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
    async fn get_version(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
    async fn diff(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
    async fn dependency_graph(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
    async fn categories(&self, request: JsonRequest) -> CapabilityResult<JsonResponse>;
}
