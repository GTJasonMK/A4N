mod contracts;
mod estimate;
mod pricing;
mod usage;

pub use contracts::{
    CostConfidence, CostEstimate, MILLION, ModelCapability, ModelUsageMeter, PRICE_MICRO_SCALE,
    PricingPlan, SYSTEM_DEFAULT_PROVIDER_CONFIG_ID,
};
pub use estimate::estimate_cost;
pub use usage::{ModelUsageRecordCompletion, ModelUsageRecordStart};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prices_are_decimal_and_cache_costs_are_separated() {
        let plan = PricingPlan {
            currency: " usd ".into(),
            llm_input_per_million: Some("1.00".into()),
            llm_output_per_million: Some("2".into()),
            llm_cache_read_per_million: Some("0.10".into()),
            llm_cache_write_per_million: Some("0.20".into()),
            ..Default::default()
        }
        .validate()
        .unwrap();
        let estimate = estimate_cost(
            ModelCapability::Llm,
            &plan,
            &ModelUsageMeter {
                input_tokens: Some(1_000_000),
                output_tokens: Some(500_000),
                cache_read_tokens: Some(200_000),
                cache_write_tokens: Some(100_000),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(estimate.amount_micros, Some(1_740_000));
        assert_eq!(estimate.confidence, CostConfidence::Exact);
    }

    #[test]
    fn missing_price_is_not_free() {
        let estimate = estimate_cost(
            ModelCapability::Image,
            &PricingPlan::default(),
            &ModelUsageMeter {
                image_count: Some(1),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(estimate.amount_micros, None);
        assert_eq!(estimate.confidence, CostConfidence::Unpriced);
    }

    #[test]
    fn missing_llm_price_is_not_zero_and_partial_price_is_visible() {
        let meter = ModelUsageMeter {
            input_tokens: Some(1_000_000),
            output_tokens: Some(500_000),
            ..Default::default()
        };
        let unpriced =
            estimate_cost(ModelCapability::Llm, &PricingPlan::default(), &meter).unwrap();
        assert_eq!(unpriced.amount_micros, None);
        assert_eq!(unpriced.confidence, CostConfidence::Unpriced);

        let partial = estimate_cost(
            ModelCapability::Llm,
            &PricingPlan {
                llm_input_per_million: Some("1".into()),
                ..Default::default()
            },
            &meter,
        )
        .unwrap();
        assert_eq!(partial.amount_micros, Some(1_000_000));
        assert_eq!(partial.confidence, CostConfidence::Partial);
    }
}
