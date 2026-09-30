use super::contracts::PricingPlan;
use super::estimate::parse_price_micro;

impl PricingPlan {
    pub fn validate(mut self) -> Result<Self, String> {
        self.currency = self.currency.trim().to_ascii_uppercase();
        if self.currency.len() != 3 || !self.currency.bytes().all(|byte| byte.is_ascii_uppercase())
        {
            return Err("价格币种必须是 3 位大写 ISO 货币代码".to_string());
        }
        for (field, value) in [
            ("llm_input_per_million", &self.llm_input_per_million),
            ("llm_output_per_million", &self.llm_output_per_million),
            (
                "llm_cache_read_per_million",
                &self.llm_cache_read_per_million,
            ),
            (
                "llm_cache_write_per_million",
                &self.llm_cache_write_per_million,
            ),
            (
                "embedding_input_per_million",
                &self.embedding_input_per_million,
            ),
            ("image_per_generation", &self.image_per_generation),
            ("video_per_second", &self.video_per_second),
        ] {
            if let Some(value) = value {
                parse_price_micro(value).map_err(|error| format!("{field} {error}"))?;
            }
        }
        Ok(self)
    }

    pub fn normalized_snapshot(&self) -> Result<Self, String> {
        self.clone().validate()
    }
}
