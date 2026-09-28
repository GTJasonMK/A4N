use std::collections::BTreeSet;

use serde_json::{Value, json};

use super::super::{
    GalgameFacingV2, GalgameLine, GalgameLinePresentationDocumentV3, GalgameLineType,
    GalgamePortraitAssetDraft, GalgameStageLayerV2, GalgameStagePositionV2,
    GalgameVisibleCastMemberV2, portrait_variant,
};
use super::fixtures::{test_generation_spec, test_spec_input};

const SPEC_STATUS_COMPLETED_FOR_TEST: &str = "completed";

#[test]
fn portrait_variant_uses_canonical_expression_and_pose() {
    let line = projected_line(1, Some(1), GalgameStagePositionV2::Center);

    assert_eq!(
        portrait_variant(&line).map(|variant| variant.key),
        Some("expression:smile:pose:standing".to_string())
    );

    let mut narration = line;
    narration.line_type = GalgameLineType::Narration;
    narration.speaker_profile_id = None;
    narration.expression = None;
    narration.pose = None;
    narration.position = None;
    narration.presentation_json.visible_cast.clear();
    narration.presentation_json.focus_character_profile_id = None;
    assert!(portrait_variant(&narration).is_none());
}

#[test]
fn portrait_asset_draft_aggregates_line_samples() {
    let variant = portrait_variant(&projected_line(1, Some(1), GalgameStagePositionV2::Center))
        .expect("portrait variant");
    let mut draft = GalgamePortraitAssetDraft {
        canonical_key: format!("galgame:10:portrait:7:{}", variant.key),
        profile_id: 7,
        variant_key: variant.key,
        expression: variant.expression,
        pose: variant.pose,
        position_samples: BTreeSet::new(),
        source_line_ids: BTreeSet::new(),
        text_anchor_ids: BTreeSet::new(),
        source_labels: BTreeSet::new(),
    };
    let line_a = projected_line(10, Some(100), GalgameStagePositionV2::Left);
    let line_b = projected_line(11, None, GalgameStagePositionV2::Right);
    let scene_anchor_ids = BTreeSet::from([201, 202]);

    draft.add_line_sample(&line_a, &scene_anchor_ids);
    draft.add_line_sample(&line_b, &scene_anchor_ids);
    let params = draft.params_json();

    assert_eq!(params["source_line_ids"], json!([10, 11]));
    assert_eq!(params["position_samples"], json!(["left", "right"]));
    assert_eq!(draft.text_anchor_ids, BTreeSet::from([100, 201, 202]));
    assert_eq!(params["source_chapter_id"], Value::Null);
}

fn projected_line(
    id: i64,
    text_anchor_id: Option<i64>,
    position: GalgameStagePositionV2,
) -> GalgameLine {
    GalgameLine {
        id,
        scene_id: 1,
        line_index: 1,
        line_type: GalgameLineType::Dialogue,
        speaker_profile_id: Some(7),
        text: "测试台词".to_string(),
        expression: Some("smile".to_string()),
        pose: Some("standing".to_string()),
        position: Some(position),
        presentation_json: GalgameLinePresentationDocumentV3 {
            schema_version: a4n_domain::galgame::GalgameLineDocumentSchemaVersion::V3,
            origin: a4n_domain::media_authoring::AdaptationOrigin::Historical,
            beat_ref: None,
            visible_cast: vec![GalgameVisibleCastMemberV2 {
                character_profile_id: 7,
                expression: "smile".to_string(),
                pose: "standing".to_string(),
                position,
                facing: GalgameFacingV2::Front,
                layer: GalgameStageLayerV2::Front,
            }],
            enter: Vec::new(),
            exit: Vec::new(),
            focus_character_profile_id: Some(7),
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
        },
        text_anchor_id,
    }
}

#[test]
fn generation_spec_content_unchanged_preserves_existing_status() {
    let input = test_spec_input(json!({
        "canonical_key": "galgame:10:background:scene:100",
        "asset_kind": "background",
        "scene": {"id": 100}
    }));
    let source_version_hash = "snapshot-hash";
    let mut existing = test_generation_spec(&input, source_version_hash);
    existing.status = SPEC_STATUS_COMPLETED_FOR_TEST.to_string();

    assert!(!a4n_db::galgame::galgame_generation_spec_content_changed(
        &existing,
        &input,
        source_version_hash
    ));
    assert_eq!(existing.status, SPEC_STATUS_COMPLETED_FOR_TEST);
}

#[test]
fn generation_spec_content_change_resets_existing_spec_to_draft_path() {
    let input = test_spec_input(json!({
        "canonical_key": "galgame:10:background:scene:100",
        "asset_kind": "background",
        "scene": {"id": 100, "background_note": "雨夜月台"}
    }));
    let old_params = json!({
        "canonical_key": "galgame:10:background:scene:100",
        "asset_kind": "background",
        "scene": {"id": 100, "background_note": "晴天月台"}
    });
    let source_version_hash = "same-snapshot-hash";
    let mut existing = test_generation_spec(&input, source_version_hash);
    existing.params_json = old_params;
    existing.status = SPEC_STATUS_COMPLETED_FOR_TEST.to_string();

    assert!(a4n_db::galgame::galgame_generation_spec_content_changed(
        &existing,
        &input,
        source_version_hash,
    ));
}

#[test]
fn generation_spec_content_change_detects_legacy_canvas_context() {
    let input = test_spec_input(json!({
        "canonical_key": "galgame:10:background:scene:100",
        "asset_kind": "background",
        "scene": {"id": 100}
    }));
    let source_version_hash = "snapshot-hash";
    let mut existing = test_generation_spec(&input, source_version_hash);
    existing.canvas_text_asset_id = Some(99);

    assert!(a4n_db::galgame::galgame_generation_spec_content_changed(
        &existing,
        &input,
        source_version_hash
    ));
}
