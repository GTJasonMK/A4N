use super::super::contracts::BlueprintStageSpec;
use a4n_core::{
    blueprint::{
        BLUEPRINT_GENERATION_KIND, BLUEPRINT_GENERATION_PROMPTS, blueprint_stage_call_key,
    },
    error::AppError,
};
use a4n_llm::provider::ChatRequest;
use serde_json::{Value, json};

pub(in crate::blueprint::generation) fn blueprint_stage_call_fingerprint_json(
    spec: &BlueprintStageSpec,
    request: &ChatRequest,
) -> Result<Value, AppError> {
    Ok(json!({
        "schema_version": 1,
        "workflow": BLUEPRINT_GENERATION_KIND,
        "call_key": blueprint_stage_call_key(spec.key),
        "stage_key": spec.key,
        "prompt_name": BLUEPRINT_GENERATION_PROMPTS.join(" + "),
        "messages": request.messages,
        "context_manifest": request.context_manifest_json,
        "max_tokens_profile": request.max_tokens_profile,
        "response_format": request.response_format,
        "output_schema": spec.output_schema,
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
            blueprint_stage_call_key(BLUEPRINT_GENERATION_STAGES[1].key),
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
        assert_eq!(fingerprint["workflow"], BLUEPRINT_GENERATION_KIND);
        assert_eq!(fingerprint["call_key"], "blueprint_generation.world");
        assert_eq!(fingerprint["stage_key"], "world");
        assert_eq!(fingerprint["max_tokens_profile"], "blueprint");
        assert!(
            fingerprint["output_schema"]
                .as_str()
                .is_some_and(|schema| schema.contains("world_setting"))
        );
    }
}
