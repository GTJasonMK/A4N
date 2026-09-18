use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const PRICE_MICRO_SCALE: i64 = 1_000_000;
pub const MILLION: u64 = 1_000_000;
/// Runtime snapshots use this sentinel for a server-managed provider template.
/// It is never a persisted database configuration ID.
pub const SYSTEM_DEFAULT_PROVIDER_CONFIG_ID: i64 = -1;

/// 手工维护的 Provider 价格。价格使用货币单位的十进制字符串，避免浮点数
/// 在配置和历史快照之间产生不可见的舍入误差。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct PricingPlan {
    pub currency: String,
    pub llm_input_per_million: Option<String>,
    pub llm_output_per_million: Option<String>,
    pub llm_cache_read_per_million: Option<String>,
    pub llm_cache_write_per_million: Option<String>,
    pub embedding_input_per_million: Option<String>,
    pub image_per_generation: Option<String>,
    pub video_per_second: Option<String>,
}

impl Default for PricingPlan {
    fn default() -> Self {
        Self {
            currency: "USD".to_string(),
            llm_input_per_million: None,
            llm_output_per_million: None,
            llm_cache_read_per_million: None,
            llm_cache_write_per_million: None,
            embedding_input_per_million: None,
            image_per_generation: None,
            video_per_second: None,
        }
    }
}

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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ModelCapability {
    Llm,
    Embedding,
    Image,
    Video,
}

impl ModelCapability {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Llm => "llm",
            Self::Embedding => "embedding",
            Self::Image => "image",
            Self::Video => "video",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct ModelUsageMeter {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cache_read_tokens: Option<u64>,
    pub cache_write_tokens: Option<u64>,
    pub image_count: Option<u64>,
    pub duration_seconds: Option<String>,
    pub estimated: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CostConfidence {
    Exact,
    Estimated,
    Partial,
    Unpriced,
    Unknown,
}

impl CostConfidence {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Estimated => "estimated",
            Self::Partial => "partial",
            Self::Unpriced => "unpriced",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CostEstimate {
    pub amount_micros: Option<i64>,
    pub confidence: CostConfidence,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsageRecordStart {
    pub request_id: String,
    pub user_id: Option<i64>,
    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
    pub workflow_run_id: Option<i64>,
    pub generation_run_id: Option<i64>,
    pub workflow: String,
    pub step_key: String,
    pub call_key: Option<String>,
    pub source: Option<String>,
    pub capability: ModelCapability,
    pub provider_config_id: Option<i64>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub pricing: PricingPlan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsageRecordCompletion {
    pub status: String,
    pub meter: ModelUsageMeter,
    pub error_kind: Option<String>,
    pub error_message: Option<String>,
}

pub fn estimate_cost(
    capability: ModelCapability,
    plan: &PricingPlan,
    meter: &ModelUsageMeter,
) -> Result<CostEstimate, String> {
    let plan = plan.clone().validate()?;
    match capability {
        ModelCapability::Llm => estimate_llm_cost(&plan, meter),
        ModelCapability::Embedding => estimate_token_cost(
            meter.input_tokens,
            plan.embedding_input_per_million.as_deref(),
            meter.estimated,
        ),
        ModelCapability::Image => estimate_count_cost(
            meter.image_count,
            plan.image_per_generation.as_deref(),
            true,
        ),
        ModelCapability::Video => estimate_duration_cost(
            meter.duration_seconds.as_deref(),
            plan.video_per_second.as_deref(),
            true,
        ),
    }
}

fn estimate_llm_cost(plan: &PricingPlan, meter: &ModelUsageMeter) -> Result<CostEstimate, String> {
    let Some(input_tokens) = meter.input_tokens else {
        return Ok(CostEstimate {
            amount_micros: None,
            confidence: CostConfidence::Unknown,
        });
    };
    let Some(output_tokens) = meter.output_tokens else {
        return Ok(CostEstimate {
            amount_micros: None,
            confidence: CostConfidence::Unknown,
        });
    };

    let mut amount = 0i128;
    let mut partial = meter.estimated;
    let cache_read = meter.cache_read_tokens.unwrap_or(0);
    let cache_write = meter.cache_write_tokens.unwrap_or(0);
    let cache_known = meter.cache_read_tokens.is_some() || meter.cache_write_tokens.is_some();
    let ordinary_input = input_tokens
        .checked_sub(cache_read)
        .and_then(|value| value.checked_sub(cache_write));
    let Some(ordinary_input) = ordinary_input else {
        return Err("LLM 成本计量中的缓存 token 数超过输入 token 数".to_string());
    };

    amount += token_amount(ordinary_input, plan.llm_input_per_million.as_deref())?
        .map(i128::from)
        .unwrap_or_else(|| {
            partial = true;
            0
        });
    amount += token_amount(cache_read, plan.llm_cache_read_per_million.as_deref())?
        .map(i128::from)
        .unwrap_or_else(|| {
            if cache_read > 0 {
                partial = true;
            }
            0
        });
    amount += token_amount(cache_write, plan.llm_cache_write_per_million.as_deref())?
        .map(i128::from)
        .unwrap_or_else(|| {
            if cache_write > 0 {
                partial = true;
            }
            0
        });
    amount += token_amount(output_tokens, plan.llm_output_per_million.as_deref())?
        .map(i128::from)
        .unwrap_or_else(|| {
            partial = true;
            0
        });
    if !cache_known
        && (plan.llm_cache_read_per_million.is_some() || plan.llm_cache_write_per_million.is_some())
    {
        partial = true;
    }
    let confidence = if plan.llm_input_per_million.is_none()
        && plan.llm_output_per_million.is_none()
        && plan.llm_cache_read_per_million.is_none()
        && plan.llm_cache_write_per_million.is_none()
    {
        CostConfidence::Unpriced
    } else if partial {
        CostConfidence::Partial
    } else {
        CostConfidence::Exact
    };
    Ok(CostEstimate {
        amount_micros: if matches!(confidence, CostConfidence::Unpriced) {
            None
        } else {
            Some(i64::try_from(amount).map_err(|_| "成本超出 i64 范围".to_string())?)
        },
        confidence,
    })
}

fn estimate_token_cost(
    tokens: Option<u64>,
    price: Option<&str>,
    estimated: bool,
) -> Result<CostEstimate, String> {
    let Some(tokens) = tokens else {
        return Ok(CostEstimate {
            amount_micros: None,
            confidence: CostConfidence::Unknown,
        });
    };
    let Some(price) = price else {
        return Ok(CostEstimate {
            amount_micros: None,
            confidence: CostConfidence::Unpriced,
        });
    };
    Ok(CostEstimate {
        amount_micros: Some(token_amount(tokens, Some(price))?.expect("price is present")),
        confidence: if estimated {
            CostConfidence::Estimated
        } else {
            CostConfidence::Exact
        },
    })
}

fn estimate_count_cost(
    count: Option<u64>,
    price: Option<&str>,
    estimated: bool,
) -> Result<CostEstimate, String> {
    let Some(count) = count else {
        return Ok(CostEstimate {
            amount_micros: None,
            confidence: CostConfidence::Unknown,
        });
    };
    let Some(price) = price else {
        return Ok(CostEstimate {
            amount_micros: None,
            confidence: CostConfidence::Unpriced,
        });
    };
    let unit = parse_price_micro(price)?;
    let amount = i128::from(unit)
        .checked_mul(i128::from(count))
        .ok_or_else(|| "成本超出 i64 范围".to_string())?;
    Ok(CostEstimate {
        amount_micros: Some(i64::try_from(amount).map_err(|_| "成本超出 i64 范围".to_string())?),
        confidence: if estimated {
            CostConfidence::Estimated
        } else {
            CostConfidence::Exact
        },
    })
}

fn estimate_duration_cost(
    duration: Option<&str>,
    price: Option<&str>,
    estimated: bool,
) -> Result<CostEstimate, String> {
    let Some(duration) = duration else {
        return Ok(CostEstimate {
            amount_micros: None,
            confidence: CostConfidence::Unknown,
        });
    };
    let Some(price) = price else {
        return Ok(CostEstimate {
            amount_micros: None,
            confidence: CostConfidence::Unpriced,
        });
    };
    let duration_micro = parse_decimal_micro(duration)?;
    let unit = parse_price_micro(price)?;
    let amount = i128::from(duration_micro)
        .checked_mul(i128::from(unit))
        .and_then(|value| value.checked_div(i128::from(PRICE_MICRO_SCALE)))
        .ok_or_else(|| "成本超出 i64 范围".to_string())?;
    Ok(CostEstimate {
        amount_micros: Some(i64::try_from(amount).map_err(|_| "成本超出 i64 范围".to_string())?),
        confidence: if estimated {
            CostConfidence::Estimated
        } else {
            CostConfidence::Exact
        },
    })
}

fn token_amount(tokens: u64, price: Option<&str>) -> Result<Option<i64>, String> {
    let Some(price) = price else {
        return Ok(None);
    };
    let unit = i128::from(parse_price_micro(price)?);
    let amount = unit
        .checked_mul(i128::from(tokens))
        .and_then(|value| value.checked_div(i128::from(MILLION)))
        .ok_or_else(|| "成本超出 i64 范围".to_string())?;
    Ok(Some(
        i64::try_from(amount).map_err(|_| "成本超出 i64 范围".to_string())?,
    ))
}

fn parse_price_micro(value: &str) -> Result<i64, String> {
    parse_decimal_micro(value)
}

fn parse_decimal_micro(value: &str) -> Result<i64, String> {
    let value = value.trim();
    if value.is_empty() || value.starts_with('-') {
        return Err("必须是非负十进制数".to_string());
    }
    let mut parts = value.split('.');
    let whole = parts.next().unwrap_or_default();
    let fraction = parts.next().unwrap_or_default();
    if parts.next().is_some()
        || whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
        || fraction.len() > 6
    {
        return Err("必须是最多 6 位小数的非负十进制数".to_string());
    }
    let whole = whole.parse::<i128>().map_err(|_| "数值过大".to_string())?;
    let fraction_value = if fraction.is_empty() {
        0
    } else {
        fraction
            .parse::<i128>()
            .map_err(|_| "小数无效".to_string())?
            * 10i128.pow(6 - fraction.len() as u32)
    };
    let micros = whole
        .checked_mul(i128::from(PRICE_MICRO_SCALE))
        .and_then(|value| value.checked_add(fraction_value))
        .ok_or_else(|| "数值过大".to_string())?;
    i64::try_from(micros).map_err(|_| "数值过大".to_string())
}

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
