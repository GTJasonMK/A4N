use super::contracts::LlmMaxTokensConfig;

impl LlmMaxTokensConfig {
    pub fn validate(&self) -> Result<(), String> {
        for (label, value) in self.profile_values() {
            if value == 0 {
                return Err(format!("{label} 必须大于 0"));
            }
            if value > 262_144 {
                return Err(format!("{label} 不能超过 262144"));
            }
        }
        Ok(())
    }

    pub fn validate_against_model_output_cap(
        &self,
        model_max_output_tokens: usize,
    ) -> Result<(), String> {
        if model_max_output_tokens == 0 {
            return Err("模型最大输出必须大于 0".to_string());
        }
        let over_limit: Vec<String> = self
            .profile_values()
            .into_iter()
            .filter(|(_, value)| *value > model_max_output_tokens)
            .map(|(label, value)| format!("{label}={value}"))
            .collect();
        if over_limit.is_empty() {
            return Ok(());
        }
        Err(format!(
            "场景请求输出预算不能超过当前默认模型最大输出 {model_max_output_tokens} tokens：{}",
            over_limit.join(", ")
        ))
    }
}
