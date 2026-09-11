use a4n_capabilities::LlmClient;
use std::sync::Arc;

use crate::provider_runtime::DynamicProviderFacade;

pub(crate) struct DynamicLlmProvider {
    pub(in crate::dynamic_llm) facade: DynamicProviderFacade,
    pub(in crate::dynamic_llm) sidecar_client: Option<Arc<dyn LlmClient>>,
}
