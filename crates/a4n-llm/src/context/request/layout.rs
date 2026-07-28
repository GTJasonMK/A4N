use a4n_core::{error::AppError, source_spans::source_text_hash};
use serde_json::{Value, json};

use super::super::contracts::LlmContextPlan;

pub fn context_plan_json_with_message_layout(
    plan: &LlmContextPlan,
    base_system_tokens: usize,
    stable_system_suffix: Option<&str>,
    dynamic_user_content: &str,
    count_tokens: impl Fn(&str) -> usize,
) -> Result<Value, AppError> {
    let stable_system_suffix_tokens = stable_system_suffix.map(&count_tokens).unwrap_or(0);
    let dynamic_message_tokens = count_tokens(dynamic_user_content);
    let expected_cacheable_prefix_tokens =
        base_system_tokens.saturating_add(stable_system_suffix_tokens);
    let rendered_input_tokens =
        expected_cacheable_prefix_tokens.saturating_add(dynamic_message_tokens);
    let cache_prefix_coverage = if rendered_input_tokens == 0 {
        0.0
    } else {
        expected_cacheable_prefix_tokens as f64 / rendered_input_tokens as f64
    };
    let mut value = serde_json::to_value(plan).map_err(AppError::Serialization)?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| AppError::InvalidState {
            current: "context_plan_not_object".to_string(),
            operation: "记录 LLM 上下文消息布局".to_string(),
        })?;
    object.insert("message_layout_version".to_string(), json!(3));
    object.insert(
        "stable_system_suffix_sha256".to_string(),
        stable_system_suffix
            .map(source_text_hash)
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    object.insert(
        "dynamic_message_sha256".to_string(),
        Value::String(source_text_hash(dynamic_user_content)),
    );
    object.insert(
        "stable_system_suffix_tokens".to_string(),
        json!(stable_system_suffix_tokens),
    );
    object.insert(
        "dynamic_message_tokens".to_string(),
        json!(dynamic_message_tokens),
    );
    object.insert(
        "expected_cacheable_prefix_tokens".to_string(),
        json!(expected_cacheable_prefix_tokens),
    );
    object.insert(
        "cache_prefix_coverage".to_string(),
        json!(cache_prefix_coverage),
    );
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{COMPRESSION_FULL_RAW, LlmContextPlan};

    #[test]
    fn context_plan_records_cacheable_message_boundary() {
        let plan = LlmContextPlan {
            model_context_tokens: 10_000,
            output_token_reserve: 2_000,
            safety_margin_tokens: 500,
            input_utilization_percent: 100,
            raw_available_input_tokens: 7_500,
            available_input_tokens: 7_500,
            stable_prefix_tokens: 100,
            dynamic_delta_tokens: 20,
            selected_compression_level: COMPRESSION_FULL_RAW.to_string(),
            selected_pack_refs: Vec::new(),
            selected_packs: Vec::new(),
            dropped_optional_pack_refs: Vec::new(),
            estimated_input_tokens: 120,
        };

        let value =
            context_plan_json_with_message_layout(&plan, 30, Some("stable"), "dynamic", str::len)
                .expect("layout metrics");

        assert_eq!(value["message_layout_version"], json!(3));
        assert_eq!(value["stable_system_suffix_tokens"], json!(6));
        assert_eq!(value["dynamic_message_tokens"], json!(7));
        assert_eq!(value["expected_cacheable_prefix_tokens"], json!(36));
        assert!(value["stable_system_suffix_sha256"].as_str().is_some());
        assert!(value["dynamic_message_sha256"].as_str().is_some());
    }
}
