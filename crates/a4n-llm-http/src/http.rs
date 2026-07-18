//! OpenAI-compatible HTTP LLM provider.

mod errors;
mod provider;
mod response;
mod sse;
mod stream;
#[cfg(test)]
mod tests;

pub use provider::HttpLlmProvider;
