use super::checkpoints::{GalgamePreviewCheckpointInputs, galgame_preview_checkpoint_output};
use super::contracts::{GALGAME_CHECKPOINT_LINES_DRAFT, GALGAME_CHECKPOINT_SCENES};
use a4n_orchestrator::galgame::GalgamePipelineStartStage;
use serde_json::{Value, json};

#[test]
fn galgame_preview_checkpoint_output_rejects_mismatched_parsed_output() {
    let output = valid_shooting_script_projection_checkpoint_output();
    let checkpoint_json = json!({
        "output": output,
        "parsed_output": { "scenes": [] }
    });

    let err = galgame_preview_checkpoint_output(GALGAME_CHECKPOINT_SCENES, Some(&checkpoint_json))
        .expect_err("mismatched output and parsed_output must not preview");

    assert!(err.contains("output 与 parsed_output 不一致"));
}

#[test]
fn galgame_preview_checkpoint_output_rejects_legacy_parsed_output_only() {
    let checkpoint_json = json!({
        "parsed_output": valid_script_adaptation_checkpoint_output()
    });

    let err =
        galgame_preview_checkpoint_output(GALGAME_CHECKPOINT_LINES_DRAFT, Some(&checkpoint_json))
            .expect_err("legacy parsed_output-only checkpoint must not preview");

    assert!(err.contains("缺少 output"));
}

#[test]
fn galgame_preview_checkpoint_output_accepts_matching_output_contract() {
    let output = valid_script_adaptation_checkpoint_output();
    let checkpoint_json = json!({
        "output": output,
        "parsed_output": valid_script_adaptation_checkpoint_output()
    });

    let parsed =
        galgame_preview_checkpoint_output(GALGAME_CHECKPOINT_LINES_DRAFT, Some(&checkpoint_json))
            .expect("matching output checkpoint should preview");

    assert_eq!(
        parsed["scene_lines"][0]["lines"][0]["text"],
        json!("你终于来了。")
    );
}

#[test]
fn galgame_preview_checkpoint_output_rejects_missing_output() {
    let checkpoint_json = json!({
        "response_content": "{}"
    });

    let err = galgame_preview_checkpoint_output(GALGAME_CHECKPOINT_SCENES, Some(&checkpoint_json))
        .expect_err("completed Galgame checkpoint without output must not preview");

    assert!(err.contains("缺少 output"));
}

#[test]
fn galgame_preview_checkpoint_output_rejects_malformed_output() {
    let checkpoint_json = json!({
        "output": { "scenes": [] }
    });

    let err = galgame_preview_checkpoint_output(GALGAME_CHECKPOINT_SCENES, Some(&checkpoint_json))
        .expect_err("malformed Galgame checkpoint output must not preview");

    assert!(err.contains("scenes"));
}

#[test]
fn galgame_preview_checkpoint_output_rejects_unknown_key() {
    let checkpoint_json = json!({
        "output": valid_shooting_script_projection_checkpoint_output()
    });

    let err = galgame_preview_checkpoint_output("unknown_stage", Some(&checkpoint_json))
        .expect_err("unknown Galgame checkpoint key must not be accepted");

    assert!(err.contains("checkpoint key 不支持"));
}

#[test]
fn galgame_preview_checkpoint_inputs_require_prior_stage_for_middle_start() {
    let inputs = GalgamePreviewCheckpointInputs {
        required_prior_stage: Some(GalgamePipelineStartStage::CharacterStaging),
        ..Default::default()
    };

    let err = inputs
        .script_adaptation_input()
        .expect_err("start_from_stage=character_staging requires lines_draft checkpoint");

    assert!(err.contains("缺少上游 checkpoint"));
    assert!(err.contains("required_stage=script_adaptation"));
}

#[test]
fn galgame_preview_checkpoint_inputs_do_not_create_placeholder_upstream() {
    let inputs = GalgamePreviewCheckpointInputs::default();

    assert!(inputs.shooting_script_projection_input().unwrap().is_none());
    assert!(inputs.script_adaptation_input().unwrap().is_none());
    assert!(inputs.character_staging_input().unwrap().is_none());
}

#[test]
fn galgame_preview_checkpoint_inputs_return_real_completed_upstream() {
    let inputs = GalgamePreviewCheckpointInputs {
        scenes: Some(valid_shooting_script_projection_checkpoint_output()),
        lines_draft: Some(valid_script_adaptation_checkpoint_output()),
        ..Default::default()
    };

    let scenes = inputs
        .shooting_script_projection_input()
        .expect("scene input")
        .expect("completed scene checkpoint");
    let lines = inputs
        .script_adaptation_input()
        .expect("script input")
        .expect("completed script checkpoint");

    assert_eq!(scenes["scenes"][0]["title"], json!("雨夜重逢"));
    assert_eq!(
        lines["scene_lines"][0]["lines"][0]["text"],
        json!("你终于来了。")
    );
}

#[test]
fn galgame_preview_checkpoint_inputs_require_visual_plan_for_persist_start() {
    let inputs = GalgamePreviewCheckpointInputs {
        scenes: Some(json!({})),
        lines_draft: Some(json!({})),
        lines_staged: Some(json!({})),
        required_prior_stage: Some(GalgamePipelineStartStage::PersistScript),
        ..Default::default()
    };

    let err = inputs
        .validate_required_prior_checkpoints()
        .expect_err("start_from_stage=persist_script requires visual_plan checkpoint");

    assert!(err.contains("缺少上游 checkpoint"));
    assert!(err.contains("required_stage=visual_planning"));
}

fn valid_shooting_script_projection_checkpoint_output() -> Value {
    json!({
        "scenes": [
            {
                "index": 1,
                "title": "雨夜重逢",
                "location": "车站",
                "time_of_day": "夜晚",
                "participants": ["林晓雨"],
                "summary": "两人重逢",
                "source_span_ids": ["ch1:ta1"]
            }
        ]
    })
}

fn valid_script_adaptation_checkpoint_output() -> Value {
    json!({
        "scene_lines": [
            {
                "scene_index": 1,
                "lines": [
                    {
                        "index": 1,
                        "line_type": "dialogue",
                        "speaker_name": "林晓雨",
                        "text": "你终于来了。",
                        "source_span_ids": ["ch1:ta1"]
                    }
                ]
            }
        ]
    })
}
