use serde::{Deserialize, Serialize};

/// Input tokens include cache reads, cache writes and ordinary input. Writes
/// are a subset of misses; keeping them separate permits provider-specific cost accounting.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LlmCallUsageTraceInput {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
    pub prompt_cache_hit_tokens: Option<usize>,
    pub prompt_cache_miss_tokens: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_cache_write_tokens: Option<usize>,
}

impl LlmCallUsageTraceInput {
    pub fn normalize_cache_tokens(mut self) -> Result<Self, String> {
        if let Some(hit) = self.prompt_cache_hit_tokens {
            let miss = self.prompt_tokens.checked_sub(hit).ok_or_else(|| {
                format!(
                    "LLM usage 缓存命中 token 数超过 prompt token 数：hit={hit}, prompt={}",
                    self.prompt_tokens
                )
            })?;
            if self
                .prompt_cache_miss_tokens
                .is_some_and(|reported| reported != miss)
            {
                return Err("LLM usage 缓存命中与未命中之和不等于输入 token 数".into());
            }
            self.prompt_cache_miss_tokens = Some(miss);
        }
        let miss = self.prompt_cache_miss_tokens.unwrap_or(self.prompt_tokens);
        if miss > self.prompt_tokens {
            return Err("LLM usage 缓存未命中 token 数超过输入 token 数".into());
        }
        if self
            .prompt_cache_write_tokens
            .is_some_and(|write| write > miss)
        {
            return Err("LLM usage 缓存写入 token 数超过未命中 token 数".into());
        }
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_usage_preserves_unknown_counts_and_separates_writes_from_misses() {
        let unknown = LlmCallUsageTraceInput {
            prompt_tokens: 100,
            ..Default::default()
        }
        .normalize_cache_tokens()
        .unwrap();
        assert_eq!(unknown.prompt_cache_miss_tokens, None);
        let usage = LlmCallUsageTraceInput {
            prompt_tokens: 100,
            prompt_cache_hit_tokens: Some(70),
            prompt_cache_write_tokens: Some(20),
            ..Default::default()
        }
        .normalize_cache_tokens()
        .unwrap();
        assert_eq!(usage.prompt_cache_miss_tokens, Some(30));
        assert_eq!(usage.prompt_cache_write_tokens, Some(20));
        for (hit, miss, write) in [
            (101, None, None),
            (70, Some(29), None),
            (70, None, Some(31)),
        ] {
            assert!(
                LlmCallUsageTraceInput {
                    prompt_tokens: 100,
                    prompt_cache_hit_tokens: Some(hit),
                    prompt_cache_miss_tokens: miss,
                    prompt_cache_write_tokens: write,
                    ..Default::default()
                }
                .normalize_cache_tokens()
                .is_err()
            );
        }
    }
}
