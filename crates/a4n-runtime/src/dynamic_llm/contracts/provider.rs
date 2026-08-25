use a4n_capabilities::LlmClient;
use a4n_orchestrator::DynamicProviderFacade;
use std::sync::Arc;

pub(crate) struct DynamicLlmProvider {
    pub(in crate::dynamic_llm) facade: DynamicProviderFacade,
    pub(in crate::dynamic_llm) sidecar_client: Option<Arc<dyn LlmClient>>,
}
