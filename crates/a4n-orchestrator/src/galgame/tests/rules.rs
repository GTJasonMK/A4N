use serde_json::json;

use super::super::{
    LINE_TYPE_DIALOGUE, LINE_TYPE_MONOLOGUE, SCENE_ENTRY_BRANCH, SCENE_ENTRY_MAIN,
    event_cg_directive, validate_line_semantics, validate_presentation_json,
    validate_scene_entry_kind,
};

#[test]
fn line_semantics_require_bound_dialogue_expression() {
    let err = validate_line_semantics(LINE_TYPE_DIALOGUE, Some(1), None).expect_err("expression");

    assert!(err.to_string().contains("必须显式提供 expression"));
}

#[test]
fn line_semantics_reject_invalid_expression() {
    let err = validate_line_semantics(LINE_TYPE_DIALOGUE, Some(1), Some("ecstatic"))
        .expect_err("invalid expression");

    assert!(err.to_string().contains("表情无效"));
}

#[test]
fn line_semantics_accept_bound_dialogue_expression() {
    validate_line_semantics(LINE_TYPE_DIALOGUE, Some(1), Some("neutral"))
        .expect("neutral expression");
    validate_line_semantics(LINE_TYPE_MONOLOGUE, Some(1), Some("smile")).expect("smile expression");
}

#[test]
fn scene_entry_kind_rejects_unknown_value() {
    validate_scene_entry_kind(SCENE_ENTRY_MAIN).expect("main scene entry");
    validate_scene_entry_kind(SCENE_ENTRY_BRANCH).expect("branch scene entry");

    let err = validate_scene_entry_kind("side_path").expect_err("unknown scene entry kind");

    assert!(err.to_string().contains("entry_kind 无效"));
}

#[test]
fn event_cg_directive_requires_note_when_enabled() {
    let err = event_cg_directive(&json!({"cg": true}), 10)
        .expect_err("enabled event CG must provide a concrete note");

    assert!(err.to_string().contains("必须提供 cg_note"));
}

#[test]
fn event_cg_directive_rejects_invalid_importance() {
    let err = event_cg_directive(
        &json!({
            "cg": true,
            "cg_note": "雨中回头",
            "cg_importance": "major"
        }),
        10,
    )
    .expect_err("invalid importance must fail visibly");

    assert!(err.to_string().contains("cg_importance 无效"));
}

#[test]
fn event_cg_directive_accepts_note_and_importance() {
    let directive = event_cg_directive(
        &json!({
            "cg": true,
            "cg_note": "雨中回头",
            "cg_importance": "critical"
        }),
        10,
    )
    .expect("event CG directive")
    .expect("enabled directive");

    assert_eq!(directive.note, "雨中回头");
    assert_eq!(directive.importance, "critical");
}

#[test]
fn presentation_json_save_contract_rejects_cg_without_note() {
    let err = validate_presentation_json(&json!({"cg": true}), "Galgame 新增行")
        .expect_err("save layer must reject empty event CG");

    assert!(err.to_string().contains("必须提供 cg_note"));
}

#[test]
fn presentation_json_save_contract_rejects_invalid_cg_importance() {
    let err = validate_presentation_json(
        &json!({
            "cg_note": "雨中回头",
            "cg_importance": "major"
        }),
        "Galgame 新增行",
    )
    .expect_err("save layer must reject invalid event CG importance");

    assert!(err.to_string().contains("cg_importance 无效"));
}

#[test]
fn presentation_json_save_contract_accepts_valid_event_cg() {
    validate_presentation_json(
        &json!({
            "cg": true,
            "cg_note": "雨中回头",
            "cg_importance": "critical"
        }),
        "Galgame 新增行",
    )
    .expect("valid event CG presentation json");
}
