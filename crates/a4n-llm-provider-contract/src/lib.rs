//! Pure in-process contracts shared by LLM providers and their Host adapters.
//!
//! Context assembly, gateway wire payloads, and provider implementations stay
//! outside this crate so concrete providers do not compile the LLM workflow.

pub fn estimate_context_tokens(text: &str) -> usize {
    // ponytail: UTF-8 bytes conservatively bound text tokens; use model tokenizers
    // when recovering unused context capacity is worth their model data/dependency.
    text.len()
}

pub mod provider;

pub use provider::*;

#[cfg(test)]
mod tests {
    use super::estimate_context_tokens;

    #[test]
    fn context_token_estimate_is_a_conservative_byte_bound() {
        assert_eq!(estimate_context_tokens("你好 world"), "你好 world".len());
        assert_eq!(estimate_context_tokens(""), 0);
    }
}
