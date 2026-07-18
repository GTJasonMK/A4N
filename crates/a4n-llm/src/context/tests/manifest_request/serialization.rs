use super::super::super::*;
use super::super::support::pack_ref;

#[test]
fn context_plan_serialization_skips_hydrated_pack_content() {
    let plan = LlmContextPlan {
        model_context_tokens: 1_000,
        output_token_reserve: 200,
        safety_margin_tokens: 100,
        input_utilization_percent: 100,
        raw_available_input_tokens: 700,
        available_input_tokens: 700,
        stable_prefix_tokens: 10,
        dynamic_delta_tokens: 20,
        selected_compression_level: COMPRESSION_FULL_RAW.to_string(),
        selected_pack_refs: vec![pack_ref("chapter", COMPRESSION_FULL_RAW, 10)],
        selected_packs: vec![PlannedContextPack {
            pack_ref: pack_ref("chapter", COMPRESSION_FULL_RAW, 10),
            stable_prefix: true,
            canonical_content_text: Some("full content should not be serialized".to_string()),
        }],
        dropped_optional_pack_refs: Vec::new(),
        estimated_input_tokens: 30,
    };

    let serialized = serde_json::to_value(&plan).expect("serialize plan");

    assert!(serialized.get("selected_packs").is_none());
    assert!(
        !serialized
            .to_string()
            .contains("full content should not be serialized")
    );
}
