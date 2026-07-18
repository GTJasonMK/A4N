use a4n_core::models::scene::scene_profile;
use chrono::NaiveDateTime;
use serde_json::json;

use super::rules::{SceneProfileDraftTracker, merge_scene_profile_record_input};
use crate::scene_profile::{SceneProfileDbError, SceneProfileRecordDraft, SceneProfileRecordInput};

fn input(name: &str, normalized_name: &str) -> SceneProfileRecordInput {
    SceneProfileRecordInput {
        name: name.to_string(),
        normalized_name: normalized_name.to_string(),
        aliases_json: json!([]),
        scene_type: "location".to_string(),
        importance: "major".to_string(),
        parent_scene_id: None,
        base_description: String::new(),
        visual_description: String::new(),
        spatial_layout: String::new(),
        atmosphere: String::new(),
        entry_points_json: json!([]),
        rules_json: json!([]),
        first_seen_chapter: 3,
        last_seen_chapter: 8,
        evidence_json: json!(["共有证据", " 新证据 ", ""]),
        image_prompt_seed: String::new(),
    }
}

fn draft(
    scene_ref: &str,
    name: &str,
    normalized_name: &str,
    scene_profile_id: Option<i64>,
) -> SceneProfileRecordDraft {
    SceneProfileRecordDraft {
        scene_ref: scene_ref.to_string(),
        scene_profile_id,
        input: input(name, normalized_name),
    }
}

#[test]
fn profile_merge_preserves_chapter_range_and_deduplicates_evidence() {
    let existing = scene_profile::Model {
        id: 1,
        project_id: 2,
        name: "旧场景".to_string(),
        normalized_name: "旧场景".to_string(),
        aliases_json: json!([]),
        scene_type: "location".to_string(),
        importance: "major".to_string(),
        parent_scene_id: None,
        base_description: String::new(),
        visual_description: String::new(),
        spatial_layout: String::new(),
        atmosphere: String::new(),
        entry_points_json: json!([]),
        rules_json: json!([]),
        current_state_json: json!({}),
        first_seen_chapter: 2,
        last_seen_chapter: 5,
        evidence_json: json!(["旧证据", "共有证据"]),
        image_prompt_seed: String::new(),
        created_at: NaiveDateTime::MIN,
        updated_at: NaiveDateTime::MIN,
    };

    let merged = merge_scene_profile_record_input(&existing, input("新场景", "新场景"));

    assert_eq!(merged.first_seen_chapter, 2);
    assert_eq!(merged.last_seen_chapter, 8);
    assert_eq!(
        merged.evidence_json,
        json!(["共有证据", "新证据", "旧证据"])
    );
}

#[test]
fn draft_tracker_rejects_duplicate_ref_before_other_fields() {
    let mut tracker = SceneProfileDraftTracker::default();
    tracker
        .accept(&draft("scene-a", "场景 A", "场景a", Some(1)))
        .expect("first scene draft must pass");
    let error = tracker
        .accept(&draft("scene-a", "场景 A2", "场景a2", Some(2)))
        .expect_err("duplicate scene ref must fail");

    assert!(matches!(
        error,
        SceneProfileDbError::InvalidArgument(message)
            if message == "场景同步存在重复 scene_ref: scene-a"
    ));
}

#[test]
fn draft_tracker_rejects_duplicate_name_and_explicit_profile_id() {
    let mut name_tracker = SceneProfileDraftTracker::default();
    name_tracker
        .accept(&draft("scene-a", "场景 A", "共享场景", Some(1)))
        .expect("first scene draft must pass");
    assert!(matches!(
        name_tracker.accept(&draft("scene-b", "场景 B", "共享场景", Some(2))),
        Err(SceneProfileDbError::InvalidArgument(message))
            if message == "场景同步存在重复场景名称: 场景 B"
    ));

    let mut id_tracker = SceneProfileDraftTracker::default();
    id_tracker
        .accept(&draft("scene-a", "场景 A", "场景a", Some(9)))
        .expect("first scene draft must pass");
    assert!(matches!(
        id_tracker.accept(&draft("scene-b", "场景 B", "场景b", Some(9))),
        Err(SceneProfileDbError::InvalidArgument(message))
            if message == "场景同步重复引用同一场景档案: 9"
    ));
}
