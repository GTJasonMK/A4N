mod contracts;
mod defaults;
mod profiles;
mod validation;

pub use contracts::LlmMaxTokensConfig;
pub use profiles::llm_token_profiles;

#[cfg(test)]
mod tests;
