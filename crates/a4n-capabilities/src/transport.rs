mod errors;
mod events;
mod health;
mod json;

use a4n_service_protocol::expected_capability_version_for_service;

#[cfg(test)]
pub(super) use events::{drain_event_lines, parse_event_line};
#[cfg(test)]
pub(super) use health::{validate_health_expectation, validate_service_health};

#[derive(Debug, Clone)]
pub struct HttpSidecarTransport {
    base_url: String,
    pub(super) client: reqwest::Client,
    pub(super) expected_service_name: Option<&'static str>,
    pub(super) expected_capability_version: Option<&'static str>,
}

impl HttpSidecarTransport {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            client: reqwest::Client::new(),
            expected_service_name: None,
            expected_capability_version: None,
        }
    }

    pub(super) fn with_expected_service(mut self, service_name: &'static str) -> Self {
        self.expected_service_name = Some(service_name);
        self.expected_capability_version = expected_capability_version_for_service(service_name);
        self
    }

    pub(super) fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }
}
