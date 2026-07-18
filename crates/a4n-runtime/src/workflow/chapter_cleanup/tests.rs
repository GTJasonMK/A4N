use super::cascade::{latest_chapter_outlines_to_delete, validate_chapter_media_file_paths};
use super::preflight::profiles_missing_cleanup_snapshots;
use a4n_core::models::chapter::chapter_outline;
use a4n_core::models::character::protagonist_profile;
use a4n_orchestrator::project::{
    chapter_cleanup_profile_state_should_restore_base, chapter_cleanup_requires_snapshot,
};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .expect("system clock must be after UNIX_EPOCH");
    std::env::temp_dir().join(format!("{prefix}_{}_{}", std::process::id(), stamp))
}

fn profile_for_context(last_synced_chapter: i32) -> protagonist_profile::Model {
    let now = chrono::Utc::now().naive_utc();
    protagonist_profile::Model {
        id: 1,
        project_id: 1,
        name: "林岚".to_string(),
        gender: None,
        age: None,
        base_attributes: serde_json::json!({}),
        current_state: serde_json::json!({
            "explicit_attributes": {
                "appearance": "穿青色劲装，左腕有旧伤",
                "weapon": "短刀"
            },
            "implicit_attributes": {
                "personality": "谨慎但会为弱者冒险"
            },
            "social_attributes": {
                "身份": "边城医馆学徒"
            }
        }),
        character_arc: None,
        voice_fingerprint: None,
        appearance: None,
        background: None,
        personality: None,
        last_synced_chapter,
        created_at: now,
        updated_at: now,
    }
}

fn outline_for_delete(id: i64, chapter_number: i32) -> chapter_outline::Model {
    let now = chrono::Utc::now().naive_utc();
    chapter_outline::Model {
        id,
        project_id: 1,
        part_id: None,
        chapter_number,
        title: Some(format!("第 {chapter_number} 章")),
        summary: Some("章节摘要".to_string()),
        beats: None,
        key_scenes: None,
        pacing_score: None,
        foreshadowing_setup: None,
        foreshadowing_payoff: None,
        characters_involved: None,
        word_count_target: None,
        status: "draft".to_string(),
        created_at: now,
        updated_at: now,
    }
}

#[test]
fn latest_chapter_outlines_to_delete_selects_highest_chapter_numbers() {
    let outlines = [
        outline_for_delete(11, 1),
        outline_for_delete(12, 3),
        outline_for_delete(13, 2),
    ];

    let selected = latest_chapter_outlines_to_delete(&outlines, 2)
        .expect("valid outlines should select latest chapter numbers");

    let selected_numbers = selected
        .iter()
        .map(|outline| outline.chapter_number)
        .collect::<Vec<_>>();
    assert_eq!(selected_numbers, vec![3, 2]);
}

#[test]
fn latest_chapter_outlines_to_delete_rejects_duplicate_chapter_number() {
    let outlines = [
        outline_for_delete(11, 1),
        outline_for_delete(12, 2),
        outline_for_delete(13, 2),
    ];

    let err = latest_chapter_outlines_to_delete(&outlines, 1)
        .expect_err("duplicate chapter outlines must fail before cascade cleanup");

    assert!(err.contains("第 2 章已有重复章节大纲"));
}

#[test]
fn latest_chapter_outlines_to_delete_rejects_invalid_chapter_number() {
    let outlines = [outline_for_delete(11, 0)];

    let err = latest_chapter_outlines_to_delete(&outlines, 1)
        .expect_err("invalid chapter number must fail before cascade cleanup");

    assert!(err.contains("章节号无效"));
}

#[test]
fn profile_state_restore_base_before_first_seen_chapter() {
    let mut profile = profile_for_context(2);
    profile.base_attributes = serde_json::json!({
        "explicit_attributes": {
            "first_seen_chapter": 2,
            "role": "西市胡商"
        },
        "implicit_attributes": {},
        "social_attributes": {}
    });

    assert!(chapter_cleanup_profile_state_should_restore_base(
        &profile, 0
    ));
    assert!(chapter_cleanup_profile_state_should_restore_base(
        &profile, 1
    ));
    assert!(!chapter_cleanup_profile_state_should_restore_base(
        &profile, 2
    ));
}

#[test]
fn profile_first_seen_chapter_extracts_text_derived_origin() {
    let mut profile = profile_for_context(0);
    profile.base_attributes = serde_json::json!({
        "explicit_attributes": {
            "first_seen_chapter": 5
        },
        "implicit_attributes": {},
        "social_attributes": {}
    });

    assert_eq!(profile.first_seen_chapter(), Some(5));
}

#[test]
fn profile_state_restore_base_without_first_seen_only_for_initial_state() {
    let profile = profile_for_context(2);

    assert!(chapter_cleanup_profile_state_should_restore_base(
        &profile, 0
    ));
    assert!(!chapter_cleanup_profile_state_should_restore_base(
        &profile, 1
    ));
}

#[test]
fn cleanup_preflight_requires_snapshot_when_rewriting_synced_chapter() {
    let profile = profile_for_context(5);

    assert!(chapter_cleanup_requires_snapshot(&profile, 3));
    assert!(!chapter_cleanup_requires_snapshot(&profile, 6));
}

#[test]
fn cleanup_preflight_reports_profiles_missing_required_snapshot() {
    let profile = profile_for_context(5);
    let missing = profiles_missing_cleanup_snapshots(&[profile], &BTreeSet::new(), 3);

    assert_eq!(missing, vec!["林岚".to_string()]);
}

#[test]
fn cleanup_preflight_accepts_existing_required_snapshot() {
    let profile = profile_for_context(5);
    let mut existing = BTreeSet::new();
    existing.insert(profile.id);

    let missing = profiles_missing_cleanup_snapshots(&[profile], &existing, 3);

    assert!(missing.is_empty());
}

#[test]
fn media_file_preflight_accepts_existing_and_missing_data_files() {
    let data_dir = unique_temp_dir("a4n_chapter_cleanup_media_ok");
    let existing_path = data_dir.join("projects/1/images/ref.png");
    std::fs::create_dir_all(existing_path.parent().expect("nested parent"))
        .expect("create nested dir");
    std::fs::write(&existing_path, b"demo").expect("write existing file");

    validate_chapter_media_file_paths(
        &data_dir,
        &[
            "projects/1/images/ref.png".to_string(),
            "projects/1/images/missing.png".to_string(),
        ],
    )
    .expect("existing and missing in-data-dir files should pass preflight");

    assert!(existing_path.exists(), "preflight must not delete files");
    let _ = std::fs::remove_dir_all(&data_dir);
}

#[test]
fn media_file_preflight_rejects_paths_outside_data_dir() {
    let root = unique_temp_dir("a4n_chapter_cleanup_media_bad");
    let data_dir = root.join("data");
    std::fs::create_dir_all(&data_dir).expect("create data dir");

    let err = validate_chapter_media_file_paths(&data_dir, &["../outside.png".to_string()])
        .expect_err("traversal media path must fail before DB commit");

    assert!(err.contains("不在应用数据目录内"));
    let _ = std::fs::remove_dir_all(&root);
}
