#[derive(Debug, Clone)]
pub struct LlmConcurrencyLimit {
    pub key: String,
    pub max_concurrent: usize,
}
