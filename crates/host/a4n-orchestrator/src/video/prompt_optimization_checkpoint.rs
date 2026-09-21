use serde::{Deserialize, Deserializer};
use serde_json::Value;

use crate::structured_llm::{ensure_checkpoint_parsed_output_matches, parse_strict_json_response};

#[derive(PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct OptimizedVideoPromptFields {
    pub(super) visual_prompt: String,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub(super) motion_prompt: Option<String>,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub(super) negative_prompt: Option<String>,
}

pub(super) fn video_prompt_optimization_checkpoint_output(
    parsed_output: &Value,
    response_content: &str,
    call_key: &str,
) -> Result<OptimizedVideoPromptFields, String> {
    let response_parsed = parse_strict_json_response::<OptimizedVideoPromptFields>(
        response_content,
        "视频提示词优化 checkpoint",
    )
    .map_err(|error| error.to_string())?;
    let checkpoint_parsed: OptimizedVideoPromptFields =
        serde_json::from_value(parsed_output.clone())
            .map_err(|err| format!("{call_key}.parsed_output 字段不符合要求: {err}"))?;
    ensure_checkpoint_parsed_output_matches(
        &checkpoint_parsed,
        &response_parsed,
        &format!("视频提示词优化 {call_key}"),
    )
    .map_err(|error| error.to_string())?;
    Ok(response_parsed)
}

fn deserialize_required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn checkpoint_requires_the_raw_response_and_parsed_output_to_match() {
        let checkpoint = json!({
            "response_content": r#"{"visual_prompt":"画面","motion_prompt":null,"negative_prompt":null}"#,
            "parsed_output": {
                "visual_prompt": "另一份画面",
                "motion_prompt": null,
                "negative_prompt": null,
            },
            "usage": null,
            "finish_reason": "stop",
        });

        assert!(
            video_prompt_optimization_checkpoint_output(
                &checkpoint["parsed_output"],
                checkpoint["response_content"].as_str().unwrap(),
                "call",
            )
            .is_err()
        );
    }
}
