mod config;
mod request;
mod streaming;
mod trait_impl;

use a4n_llm::provider::ModelCapabilities;
use reqwest::Client;
use serde_json::{Map, Value};

pub(super) const DEFAULT_TIMEOUT_SECS: u64 = 120;
const CONNECTION_TEST_MAX_TOKENS: usize = 8;

pub struct HttpLlmProvider {
    client: Client,
    base_url: String,
    api_key: Option<String>,
    model: String,
    pub(super) timeout_secs: u64,
    pub(super) connect_timeout_secs: u64,
    request_options: Option<Map<String, Value>>,
    caps: ModelCapabilities,
}
