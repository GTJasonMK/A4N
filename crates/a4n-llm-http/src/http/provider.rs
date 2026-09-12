mod config;
mod native;
mod request;
mod streaming;
mod trait_impl;

use a4n_llm::provider::{LlmProtocolKind, ModelCapabilities};
use reqwest::Client;
use serde_json::{Map, Value};

pub(in crate::http) struct HttpLlmProvider {
    client: Client,
    base_url: String,
    api_key: Option<String>,
    model: String,
    pub(super) timeout_secs: u64,
    pub(super) connect_timeout_secs: u64,
    extra_body: Option<Map<String, Value>>,
    caps: ModelCapabilities,
    protocol: LlmProtocolKind,
}

impl HttpLlmProvider {
    pub(in crate::http) fn with_protocol(mut self, protocol: LlmProtocolKind) -> Self {
        self.protocol = protocol;
        self
    }
}
