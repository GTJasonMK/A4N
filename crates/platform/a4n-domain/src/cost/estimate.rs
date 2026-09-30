use super::contracts::{
    CostConfidence, CostEstimate, MILLION, ModelCapability, ModelUsageMeter, PRICE_MICRO_SCALE,
    PricingPlan,
};

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

pub(super) fn parse_price_micro(value: &str) -> Result<i64, String> {
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
