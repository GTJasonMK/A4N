use crate::provider::ChatRequest;

pub use a4n_llm_sidecar_contract::LlmGatewayProviderConfig;

pub type LlmGatewayPayload = a4n_llm_sidecar_contract::LlmGatewayPayload<ChatRequest>;
pub type LlmProbePayload = a4n_llm_sidecar_contract::LlmProbePayload<ChatRequest>;
