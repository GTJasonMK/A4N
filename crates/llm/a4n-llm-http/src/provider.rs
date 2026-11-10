mod config;
mod native;
mod request;
mod streaming;
mod trait_impl;

use a4n_llm_provider_contract::{LlmProtocolKind, ModelCapabilities};
use reqwest::Client;
use serde_json::{Map, Value};

pub(super) struct HttpLlmProvider {
    client: Client,
    base_url: String,
    api_key: Option<String>,
    model: String,
    pub(super) timeout_secs: u64,
    pub(super) connect_timeout_secs: u64,
    extra_body: Option<Map<String, Value>>,
    caps: ModelCapabilities,
    protocol: LlmProtocolKind,
    replay_reasoning: bool,
}

impl HttpLlmProvider {
    #[cfg(test)]
    pub(crate) fn with_test_client(mut self, client: Client) -> Self {
        self.client = client;
        self
    }

    pub(super) fn with_reasoning_replay(mut self, enabled: bool) -> Self {
        self.replay_reasoning = enabled;
        self
    }

    pub(super) fn with_protocol(mut self, protocol: LlmProtocolKind) -> Self {
        self.protocol = protocol;
        self
    }
}
