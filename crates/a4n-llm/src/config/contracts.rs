use serde_json::Value;

#[derive(Debug)]
pub struct CreateLlmConfigInput<'a> {
    pub name: &'a str,
    pub provider: &'a str,
    pub base_url: &'a str,
    pub api_key_encrypted: Option<&'a str>,
    pub model: &'a str,
    pub max_concurrent: i32,
    pub timeout_secs: i32,
    pub max_context_tokens: i32,
    pub max_output_tokens: i32,
    pub request_options_json: Option<Value>,
}

#[derive(Debug, Default)]
pub struct UpdateLlmConfigInput<'a> {
    pub name: Option<&'a str>,
    pub provider: Option<&'a str>,
    pub base_url: Option<&'a str>,
    pub api_key_encrypted: Option<Option<&'a str>>,
    pub model: Option<&'a str>,
    pub max_concurrent: Option<i32>,
    pub timeout_secs: Option<i32>,
    pub max_context_tokens: Option<i32>,
    pub max_output_tokens: Option<i32>,
    pub request_options_json: Option<Option<Value>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NormalizedLlmConfig {
    pub name: String,
    pub provider: String,
    pub base_url: String,
    pub api_key_encrypted: Option<String>,
    pub model: String,
    pub max_concurrent: i32,
    pub timeout_secs: i32,
    pub max_context_tokens: i32,
    pub max_output_tokens: i32,
    pub request_options_json: Option<Value>,
}
