use std::collections::BTreeSet;

use serde_json::{Value, json};

use super::super::{GalgamePortraitAssetDraft, portrait_variant};
use super::fixtures::{test_generation_spec, test_line, test_line_with, test_spec_input};

const SPEC_STATUS_COMPLETED_FOR_TEST: &str = "completed";

#[test]
fn portrait_variant_requires_expression() {
    let expression_only = test_line(Some("smile"), None);
    let expression_with_pose = test_line(Some("smile"), Some("standing"));
    let pose_only = test_line(None, Some("standing"));

    assert_eq!(
        portrait_variant(&expression_only).map(|variant| variant.key),
        Some("expression:smile".to_string())
    );
    assert_eq!(
        portrait_variant(&expression_with_pose).map(|variant| variant.key),
        Some("expression:smile:pose:standing".to_string())
    );
    assert!(portrait_variant(&pose_only).is_none());
}

#[test]
fn portrait_asset_draft_aggregates_line_samples() {
    let variant =
        portrait_variant(&test_line(Some("smile"), Some("standing"))).expect("portrait variant");
    let mut draft = GalgamePortraitAssetDraft {
        canonical_key: format!("galgame:10:portrait:7:{}", variant.key),
        profile_id: 7,
        variant_key: variant.key,
        expression: variant.expression,
        pose: variant.pose,
        position_samples: BTreeSet::new(),
        source_line_ids: BTreeSet::new(),
        text_anchor_ids: BTreeSet::new(),
    };
    let line_a = test_line_with(10, Some(100), Some("smile"), Some("standing"), Some("left"));
    let line_b = test_line_with(11, None, Some("smile"), Some("standing"), Some("right"));
    let scene_anchor_ids = BTreeSet::from([201, 202]);

    draft.add_line_sample(&line_a, &scene_anchor_ids);
    draft.add_line_sample(&line_b, &scene_anchor_ids);
    let params = draft.params_json();

    assert_eq!(params["source_line_ids"], json!([10, 11]));
    assert_eq!(params["position_samples"], json!(["left", "right"]));
    assert_eq!(draft.text_anchor_ids, BTreeSet::from([100, 201, 202]));
    assert_eq!(params["source_chapter_id"], Value::Null);
}

#[test]
fn generation_spec_content_unchanged_preserves_existing_status() {
    let input = test_spec_input(json!({
        "canonical_key": "galgame:10:background:scene:100",
        "asset_kind": "background",
        "scene": {"id": 100}
    }));
    let source_version_hash =
        a4n_db::galgame::galgame_generation_spec_source_hash_for_value(&input.params_json)
            .expect("source hash");
    let mut existing = test_generation_spec(&input, &source_version_hash);
    existing.status = SPEC_STATUS_COMPLETED_FOR_TEST.to_string();

    assert!(!a4n_db::galgame::galgame_generation_spec_content_changed(
        &existing,
        &input,
        &source_version_hash
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
    let old_hash = a4n_db::galgame::galgame_generation_spec_source_hash_for_value(&old_params)
        .expect("old source hash");
    let new_hash =
        a4n_db::galgame::galgame_generation_spec_source_hash_for_value(&input.params_json)
            .expect("new source hash");
    let mut existing = test_generation_spec(&input, &old_hash);
    existing.params_json = old_params;
    existing.status = SPEC_STATUS_COMPLETED_FOR_TEST.to_string();

    assert!(a4n_db::galgame::galgame_generation_spec_content_changed(
        &existing, &input, &new_hash
    ));
}

#[test]
fn generation_spec_content_change_detects_legacy_canvas_context() {
    let input = test_spec_input(json!({
        "canonical_key": "galgame:10:background:scene:100",
        "asset_kind": "background",
        "scene": {"id": 100}
    }));
    let source_version_hash =
        a4n_db::galgame::galgame_generation_spec_source_hash_for_value(&input.params_json)
            .expect("source hash");
    let mut existing = test_generation_spec(&input, &source_version_hash);
    existing.canvas_text_asset_id = Some(99);

    assert!(a4n_db::galgame::galgame_generation_spec_content_changed(
        &existing,
        &input,
        &source_version_hash
    ));
}
