use super::super::{
    GalgameCgImportance, GalgameLinePresentationDocumentV3, LINE_TYPE_DIALOGUE,
    LINE_TYPE_MONOLOGUE, SCENE_ENTRY_BRANCH, SCENE_ENTRY_MAIN, validate_and_project_presentation,
    validate_line_semantics, validate_scene_entry_kind,
};

fn empty_presentation() -> GalgameLinePresentationDocumentV3 {
    GalgameLinePresentationDocumentV3 {
        schema_version: a4n_domain::galgame::GalgameLineDocumentSchemaVersion::V3,
        origin: a4n_domain::media_authoring::AdaptationOrigin::Added,
        beat_ref: None,
        visible_cast: Vec::new(),
        enter: Vec::new(),
        exit: Vec::new(),
        focus_character_profile_id: None,
        hold: false,
        player_knowledge: Vec::new(),
        cg: None,
        cg_note: None,
        cg_importance: None,
        transition: None,
        branch: None,
        branch_id: None,
        returns_to_scene_index: None,
        after_beat_ref: None,
    }
}

#[test]
fn line_semantics_require_bound_dialogue_expression() {
    let err = validate_line_semantics(LINE_TYPE_DIALOGUE, Some(1), None, Some("standing"))
        .expect_err("expression");

    assert!(err.to_string().contains("必须显式提供 expression"));
}

#[test]
fn line_semantics_reject_invalid_expression() {
    let err = validate_line_semantics(
        LINE_TYPE_DIALOGUE,
        Some(1),
        Some("ecstatic"),
        Some("standing"),
    )
    .expect_err("invalid expression");

    assert!(err.to_string().contains("表情无效"));
}

#[test]
fn line_semantics_require_pose_and_accept_complete_speaking_lines() {
    let err = validate_line_semantics(LINE_TYPE_DIALOGUE, Some(1), Some("neutral"), None)
        .expect_err("pose");
    assert!(err.to_string().contains("必须显式提供 pose"));

    validate_line_semantics(
        LINE_TYPE_DIALOGUE,
        Some(1),
        Some("neutral"),
        Some("standing"),
    )
    .expect("neutral expression");
    validate_line_semantics(LINE_TYPE_MONOLOGUE, Some(1), Some("smile"), Some("sitting"))
        .expect("smile expression");
}

#[test]
fn scene_entry_kind_rejects_unknown_value() {
    validate_scene_entry_kind(SCENE_ENTRY_MAIN).expect("main scene entry");
    validate_scene_entry_kind(SCENE_ENTRY_BRANCH).expect("branch scene entry");

    let err = validate_scene_entry_kind("side_path").expect_err("unknown scene entry kind");

    assert!(err.to_string().contains("entry_kind 无效"));
}

#[test]
fn presentation_requires_complete_enabled_event_cg() {
    let mut presentation = empty_presentation();
    presentation.cg = Some(true);
    let err = validate_and_project_presentation(&presentation, None, "test presentation")
        .expect_err("enabled event CG must provide a concrete note");

    assert!(err.to_string().contains("cg_note 不能为空"));

    presentation.cg_note = Some("雨中回头".to_string());
    let err = validate_and_project_presentation(&presentation, None, "test presentation")
        .expect_err("enabled event CG must provide an explicit importance");
    assert!(err.to_string().contains("cg_importance 不能为空"));
}

#[test]
fn presentation_rejects_disabled_event_cg_metadata() {
    let mut presentation = empty_presentation();
    presentation.cg_note = Some("雨中回头".to_string());
    let err = validate_and_project_presentation(&presentation, None, "test presentation")
        .expect_err("disabled event CG must not retain metadata");

    assert!(err.to_string().contains("cg 未启用时不能保留"));
}

#[test]
fn presentation_accepts_complete_event_cg() {
    let mut presentation = empty_presentation();
    presentation.cg = Some(true);
    presentation.cg_note = Some("雨中回头".to_string());
    presentation.cg_importance = Some(GalgameCgImportance::Critical);

    validate_and_project_presentation(&presentation, None, "test presentation")
        .expect("complete event CG presentation");
}
