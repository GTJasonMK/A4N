use super::super::contracts::BlueprintStageSpec;
use a4n_core::error::AppError;
use a4n_llm::provider::ChatRequest;
use serde_json::{Value, json};

const BLUEPRINT_STAGE_PREVIEW_PROMPT_NAME: &str =
    "blueprint + blueprint_stage_system + blueprint_stage_user";

pub(in crate::blueprint::generation) fn blueprint_stage_call_key(
    spec: &BlueprintStageSpec,
) -> String {
    format!("blueprint_generation.{}", spec.key)
}

pub(in crate::blueprint::generation) fn blueprint_stage_call_fingerprint_json(
    spec: &BlueprintStageSpec,
    request: &ChatRequest,
) -> Result<Value, AppError> {
    Ok(json!({
        "schema_version": 1,
        "workflow": "blueprint_generation",
        "call_key": blueprint_stage_call_key(spec),
        "stage_key": spec.key,
        "prompt_name": BLUEPRINT_STAGE_PREVIEW_PROMPT_NAME,
        "messages": request.messages,
        "context_manifest": request.context_manifest_json,
        "max_tokens_profile": request.max_tokens_profile,
        "response_format": request.response_format,
        "output_schema": spec.output_schema,
        "expected_top_level_keys": spec.expected_top_level_keys,
    }))
}

#[cfg(test)]
mod tests {
    use crate::blueprint::generation::contracts::BLUEPRINT_GENERATION_STAGES;
    use a4n_llm::provider::{ChatMessage, ChatRequest};

    use super::*;

    #[test]
    fn builds_stable_stage_call_key() {
        assert_eq!(
            blueprint_stage_call_key(&BLUEPRINT_GENERATION_STAGES[1]),
            "blueprint_generation.world"
        );
    }

    #[test]
    fn fingerprint_contains_request_and_stage_identity() {
        let request = ChatRequest::new(
            vec![ChatMessage::user("生成世界观")],
            Some(0.7),
            None,
            Some("blueprint".to_string()),
        )
        .with_json_response_format();
        let fingerprint =
            blueprint_stage_call_fingerprint_json(&BLUEPRINT_GENERATION_STAGES[1], &request)
                .expect("fingerprint should build");

        assert_eq!(fingerprint["schema_version"], 1);
        assert_eq!(fingerprint["workflow"], "blueprint_generation");
        assert_eq!(fingerprint["call_key"], "blueprint_generation.world");
        assert_eq!(fingerprint["stage_key"], "world");
        assert_eq!(fingerprint["max_tokens_profile"], "blueprint");
        assert_eq!(fingerprint["expected_top_level_keys"][0], "world_setting");
    }
}
