use super::fixtures::{character_profile_for_test, discovery_chapter};
use super::prelude::*;
use serde_json::json;
use std::collections::BTreeSet;

#[test]
fn chapter_analysis_character_names_use_appearing_character_fields_only() {
    let analysis = json!({
        "metadata": {
            "characters": ["林岚", "沈砚"],
            "locations": ["北城门"]
        },
        "character_states": {
            "魏青": {
                "location": "北城门",
                "status": "受伤",
                "changes": [],
                "emotional_state": "警惕"
            }
        },
        "foreshadowing": {
            "planted": [
                { "related_entities": ["沈砚", "玉佩"] }
            ],
            "resolved": [
                { "related_entities": "林岚" }
            ],
            "tensions": ["城门守军动向未明"]
        },
        "key_events": [
            { "involved_characters": ["魏青", "  "] }
        ]
    });

    let names = character_names_from_chapter_analysis(&analysis);

    assert!(names.contains("林岚"));
    assert!(names.contains("沈砚"));
    assert!(names.contains("魏青"));
    assert!(!names.contains("玉佩"));
    assert!(!names.contains("北城门"));
    assert!(!names.contains(""));
}

#[test]
fn analysis_sync_character_names_include_discovery_result() {
    let analysis_names = ["陈恪", "魏征"]
        .into_iter()
        .map(ToOwned::to_owned)
        .collect::<BTreeSet<_>>();
    let discovery = json!({
        "created": [
            {
                "name": "胡商",
                "base_attributes": {
                    "explicit_attributes": {
                        "aliases": ["西市胡商"]
                    }
                },
                "current_state": {
                    "explicit_attributes": {
                        "aliases": ["卖图胡商"]
                    }
                }
            }
        ],
        "existing": [
            {
                "name": "刘先生",
                "aliases": ["刘姓旅人"]
            }
        ],
        "skipped": []
    });
    let discovery_names = character_names_from_discovery_result(&discovery);
    let sync_names = analysis_sync_character_names(&analysis_names, &discovery_names);

    assert!(sync_names.contains("陈恪"));
    assert!(sync_names.contains("魏征"));
    assert!(sync_names.contains("胡商"));
    assert!(sync_names.contains("西市胡商"));
    assert!(sync_names.contains("卖图胡商"));
    assert!(sync_names.contains("刘先生"));
    assert!(sync_names.contains("刘姓旅人"));
}

#[test]
fn profile_sync_skip_reason_skips_future_synced_profile() {
    let mut profile = character_profile_for_test("陈恪");
    profile.last_synced_chapter = 3;

    assert_eq!(
        profile_sync_skip_reason(&profile, 2),
        Some("already_synced_past_target")
    );
    let summary = sync_skip_summary(&profile, 2);
    assert_eq!(summary["synced_chapter"], 3);
    assert_eq!(summary["target_chapter"], 2);
    assert_eq!(summary["reason"], "already_synced_past_target");
}

#[test]
fn analysis_sync_range_resyncs_already_covered_target_chapter() {
    let mut profile = character_profile_for_test("陈恪");
    profile.last_synced_chapter = 5;

    assert_eq!(character_sync_start_chapter(&profile, 2), None);
    assert_eq!(
        profile_sync_skip_reason(&profile, 2),
        Some("already_synced_past_target")
    );
    assert_eq!(analysis_profile_sync_skip_reason(&profile, 2), None);
    assert_eq!(
        character_sync_range(
            &profile,
            2,
            CharacterSyncRangeMode::AnalysisResyncFromTarget,
        ),
        Some(CharacterSyncRange {
            start_chapter: 2,
            clear_to_chapter: 5,
        })
    );
}

#[test]
fn character_sync_start_chapter_respects_first_seen_chapter() {
    let mut profile = character_profile_for_test("西市胡商");
    profile.base_attributes = json!({
        "explicit_attributes": {
            "first_seen_chapter": 2
        },
        "implicit_attributes": {},
        "social_attributes": {}
    });
    profile.current_state = profile.base_attributes.clone();
    profile.last_synced_chapter = 0;

    assert_eq!(profile_first_seen_chapter(&profile), Some(2));
    assert_eq!(character_sync_start_chapter(&profile, 2), Some(2));
    assert_eq!(
        character_sync_range(
            &profile,
            2,
            CharacterSyncRangeMode::AnalysisResyncFromTarget,
        ),
        Some(CharacterSyncRange {
            start_chapter: 2,
            clear_to_chapter: 2,
        })
    );
}

#[test]
fn character_sync_start_chapter_skips_target_before_first_seen() {
    let mut profile = character_profile_for_test("西市胡商");
    profile.base_attributes = json!({
        "explicit_attributes": {
            "first_seen_chapter": 2
        },
        "implicit_attributes": {},
        "social_attributes": {}
    });
    profile.current_state = profile.base_attributes.clone();
    profile.last_synced_chapter = 0;

    assert_eq!(
        profile_sync_skip_reason(&profile, 1),
        Some("target_before_first_seen")
    );
    assert_eq!(character_sync_start_chapter(&profile, 1), None);
    let summary = sync_skip_summary(&profile, 1);
    assert_eq!(summary["first_seen_chapter"], 2);
    assert_eq!(summary["reason"], "target_before_first_seen");
    assert_eq!(
        analysis_profile_sync_skip_reason(&profile, 1),
        Some("target_before_first_seen")
    );
    assert_eq!(
        character_sync_range(
            &profile,
            1,
            CharacterSyncRangeMode::AnalysisResyncFromTarget,
        ),
        None
    );
}

#[test]
fn validate_character_sync_target_chapter_accepts_explicit_available_chapter() {
    let chapters = vec![
        discovery_chapter(1, "completed", "有效正文"),
        discovery_chapter(3, "completed", "另一章正文"),
    ];

    assert_eq!(
        validate_character_sync_target_chapter(&chapters, 1, 3),
        Ok(3)
    );
}

#[test]
fn validate_character_sync_target_chapter_rejects_missing_or_invalid_chapter() {
    let chapters = vec![
        discovery_chapter(1, "completed", "有效正文"),
        discovery_chapter(3, "completed", "另一章正文"),
    ];

    let zero = validate_character_sync_target_chapter(&chapters, 1, 0)
        .expect_err("sync chapter must be explicit and positive");
    assert!(zero.contains("同步目标章节必须大于 0"));

    let gap = validate_character_sync_target_chapter(&chapters, 1, 2)
        .expect_err("missing chapter in range should fail");
    assert!(gap.contains("没有可同步正文"));

    let beyond = validate_character_sync_target_chapter(&chapters, 1, 4)
        .expect_err("chapter beyond latest should fail");
    assert!(beyond.contains("当前最新章节为第 3 章"));
}

#[test]
fn validate_character_sync_target_chapter_rejects_empty_project_scope() {
    let err = validate_character_sync_target_chapter(&[], 9, 1)
        .expect_err("empty available chapter list should fail");

    assert!(err.contains("项目还没有可同步章节"));
    assert!(err.contains("project_id=9"));
}

#[test]
fn validate_contiguous_sync_range_accepts_complete_range() {
    validate_contiguous_character_sync_range(2, 4, &[2, 3, 4])
        .expect("complete sync range should pass");
}

#[test]
fn validate_contiguous_sync_range_rejects_missing_chapters() {
    let err = validate_contiguous_character_sync_range(1, 4, &[1, 3])
        .expect_err("missing chapters should fail");

    assert!(err.contains("同步范围不完整"));
    assert!(err.contains("第 2 章"));
    assert!(err.contains("第 4 章"));
}
