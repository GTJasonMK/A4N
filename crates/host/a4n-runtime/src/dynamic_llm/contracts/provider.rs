use std::sync::Arc;

use a4n_llm_client::LlmClient;

use crate::provider_runtime::DynamicProviderFacade;

pub(crate) struct DynamicLlmProvider {
    pub(in crate::dynamic_llm) facade: DynamicProviderFacade,
    pub(in crate::dynamic_llm) sidecar_client: Option<Arc<dyn LlmClient>>,
}
