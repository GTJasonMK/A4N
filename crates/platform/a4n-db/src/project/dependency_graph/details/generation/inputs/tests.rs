use serde_json::json;

use super::{
    projection::{GenerationRunInputRecord, apply_generation_runs},
    rules::{generation_target_key, generation_target_table, should_expect_generation_inputs},
};
use crate::project::dependency_graph::{AssetLineageRecord, AssetLineageRef};

fn lineage_record(asset_kind: &str, origin: &str) -> AssetLineageRecord {
    AssetLineageRecord {
        id: format!("{asset_kind}:7"),
        label: asset_kind.to_string(),
        asset_kind: asset_kind.to_string(),
        asset_id: Some(7),
        status: None,
        origin: origin.to_string(),
        source_refs: Vec::new(),
        prompt_sources: Vec::new(),
        media_sources: Vec::new(),
        request_snapshot: None,
        input_status: "pending".to_string(),
        generation_inputs: Vec::new(),
        result: json!({}),
        warnings: Vec::new(),
    }
}

fn generation_run_with_inputs(include_inputs: bool) -> GenerationRunInputRecord {
    GenerationRunInputRecord {
        id: 9,
        rendered_messages_json: include_inputs.then(|| json!([{"role": "user"}])),
        rendered_prompts_json: None,
        input_variables_json: include_inputs.then(|| json!({"chapter_id": 1})),
        source_refs_json: None,
        media_refs_json: None,
        request_json: Some(serde_json::Value::Null),
        response_summary_json: None,
    }
}

#[test]
fn generation_target_rules_preserve_table_and_expectation_contract() {
    assert_eq!(
        generation_target_table("novel_blueprint"),
        Some("novel_blueprint")
    );
    assert_eq!(
        generation_target_table("manga_page_image"),
        Some("generated_image")
    );
    assert_eq!(
        generation_target_table("video_frame_prompt"),
        Some("chapter_video_prompt")
    );
    assert_eq!(generation_target_table("unknown"), None);

    let generated = lineage_record("novel_blueprint", "llm_generated");
    assert_eq!(
        generation_target_key(&generated),
        Some(("novel_blueprint", 7))
    );
    assert!(should_expect_generation_inputs(&generated));

    let manual = lineage_record("novel_blueprint", "manual");
    assert!(!should_expect_generation_inputs(&manual));
    let unsupported = lineage_record("media_asset", "image_generated");
    assert!(!should_expect_generation_inputs(&unsupported));
}

#[test]
fn generation_run_projection_preserves_input_order_and_five_statuses() {
    let run_with_inputs = generation_run_with_inputs(true);
    let runs = vec![run_with_inputs.clone()];
    let mut complete = lineage_record("novel_blueprint", "llm_generated");
    apply_generation_runs(&mut complete, &[0], &runs);
    assert_eq!(complete.input_status, "complete");
    assert_eq!(complete.generation_inputs.len(), 2);
    assert_eq!(complete.generation_inputs[0].kind, "rendered_messages");
    assert_eq!(complete.generation_inputs[1].kind, "input_variables");
    assert_eq!(
        complete.generation_inputs[1].value,
        json!({"chapter_id": 1})
    );
    assert_eq!(
        serde_json::to_value(&complete.generation_inputs[0]).unwrap(),
        json!({
            "group": "prompt",
            "kind": "rendered_messages",
            "label": "渲染消息",
            "value": [{"role": "user"}],
        })
    );
    assert!(
        complete
            .generation_inputs
            .iter()
            .all(|input| input.kind != "request")
    );

    let run_without_inputs = generation_run_with_inputs(false);
    let runs = vec![run_without_inputs];
    let mut without_inputs = lineage_record("novel_blueprint", "llm_generated");
    apply_generation_runs(&mut without_inputs, &[0], &runs);
    assert_eq!(without_inputs.input_status, "run_without_inputs");
    assert!(without_inputs.warnings[0].contains("缺少生成输入快照"));

    let mut missing = lineage_record("novel_blueprint", "llm_generated");
    apply_generation_runs(&mut missing, &[], &[]);
    assert_eq!(missing.input_status, "missing");
    assert!(missing.warnings[0].contains("缺少 generation_run"));

    let mut structural = lineage_record("media_asset", "manual");
    structural.source_refs.push(AssetLineageRef {
        id: "source:1".to_string(),
        label: "源记录".to_string(),
        kind: "text".to_string(),
        asset_type: "chapter".to_string(),
        record_id: Some(1),
    });
    apply_generation_runs(&mut structural, &[], &[]);
    assert_eq!(structural.input_status, "structural");

    let mut not_applicable = lineage_record("media_asset", "manual");
    apply_generation_runs(&mut not_applicable, &[], &[]);
    assert_eq!(not_applicable.input_status, "not_applicable");
}
