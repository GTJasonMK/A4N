use serde_json::Value;

/// Returns the prompt identity stored in the canonical generated-video metadata.
pub fn generated_video_prompt_id(metadata: Option<&Value>) -> Option<i64> {
    metadata
        .and_then(|value| value.get("prompt_id"))
        .and_then(Value::as_i64)
        .filter(|id| *id > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_only_the_canonical_root_prompt_id() {
        assert_eq!(
            generated_video_prompt_id(Some(&json!({"prompt_id": 11}))),
            Some(11)
        );
        assert_eq!(
            generated_video_prompt_id(Some(&json!({"metadata": {"prompt_id": 12}}))),
            None
        );
        assert_eq!(
            generated_video_prompt_id(Some(&json!({"generation_context": {"prompt_id": 13}}))),
            None
        );
        assert_eq!(
            generated_video_prompt_id(Some(&json!({"prompt_id": 0}))),
            None
        );
    }
}
