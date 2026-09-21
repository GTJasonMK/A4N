use a4n_domain::project::{CHAPTERS_PER_PART, PART_OUTLINE_THRESHOLD};
use chrono::NaiveDateTime;

use super::blueprint::resolve_reverse_blueprint_apply_record;
use super::*;

mod database;

fn draft(
    incoming_total_chapters: Option<i32>,
    incoming_needs_part_outlines: Option<bool>,
) -> ReverseBlueprintApplyDraft {
    ReverseBlueprintApplyDraft {
        title: "星潮档案".to_string(),
        genre: "科幻".to_string(),
        style: "冷峻".to_string(),
        tone: "悬疑".to_string(),
        target_audience: "成人".to_string(),
        one_sentence_summary: "调查星潮真相。".to_string(),
        full_synopsis: "主角追查星潮背后的阴谋。".to_string(),
        world_setting: serde_json::json!({"premise": "星潮改写记忆"}),
        incoming_total_chapters,
        incoming_needs_part_outlines,
        characters: Vec::new(),
        relationships: Vec::new(),
    }
}

fn applied_at() -> NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, 7, 10)
        .unwrap()
        .and_hms_opt(20, 0, 0)
        .unwrap()
}

#[test]
fn reverse_blueprint_plan_preserves_existing_chapter_count() {
    let record = resolve_reverse_blueprint_apply_record(
        &draft(None, None),
        &ReverseBlueprintApplyPlanRecord {
            project_total_chapters: Some(100),
            existing_blueprint_total_chapters: Some(51),
            next_version: "1.1".to_string(),
        },
        applied_at(),
    )
    .expect("existing chapter plan should be preserved");

    assert_eq!(record.total_chapters, 51);
    assert!(record.needs_part_outlines);
    assert_eq!(record.chapters_per_part, Some(CHAPTERS_PER_PART));
    assert_eq!(record.current_version, "1.1");
}

#[test]
fn reverse_blueprint_plan_rejects_conflicting_chapter_count() {
    let error = resolve_reverse_blueprint_apply_record(
        &draft(Some(51), None),
        &ReverseBlueprintApplyPlanRecord {
            project_total_chapters: Some(100),
            existing_blueprint_total_chapters: Some(50),
            next_version: "1.1".to_string(),
        },
        applied_at(),
    )
    .expect_err("conflicting chapter plan must fail");

    assert!(error.to_string().contains("total_chapters"));
}

#[test]
fn reverse_blueprint_plan_rejects_chapter_count_outside_domain_contract() {
    let error = resolve_reverse_blueprint_apply_record(
        &draft(Some(10_001), None),
        &ReverseBlueprintApplyPlanRecord {
            project_total_chapters: None,
            existing_blueprint_total_chapters: None,
            next_version: "1.0".to_string(),
        },
        applied_at(),
    )
    .expect_err("reverse blueprint must use the canonical persisted chapter range");

    assert!(error.to_string().contains("1 到 10000"));
}

#[test]
fn reverse_blueprint_plan_requires_a_chapter_count_source() {
    let error = resolve_reverse_blueprint_apply_record(
        &draft(None, None),
        &ReverseBlueprintApplyPlanRecord {
            project_total_chapters: None,
            existing_blueprint_total_chapters: None,
            next_version: "1.0".to_string(),
        },
        applied_at(),
    )
    .expect_err("reverse blueprint snapshots require total_chapters");

    assert!(error.to_string().contains("total_chapters 不能为空"));
}

#[test]
fn reverse_blueprint_plan_rejects_mismatched_part_outline_flag() {
    let error = resolve_reverse_blueprint_apply_record(
        &draft(Some(51), Some(false)),
        &ReverseBlueprintApplyPlanRecord {
            project_total_chapters: None,
            existing_blueprint_total_chapters: None,
            next_version: "1.0".to_string(),
        },
        applied_at(),
    )
    .expect_err("mismatched part outline flag must fail");

    assert!(error.to_string().contains("needs_part_outlines"));
}

#[test]
fn reverse_blueprint_plan_uses_shared_part_outline_threshold() {
    let short = resolve_reverse_blueprint_apply_record(
        &draft(Some(PART_OUTLINE_THRESHOLD), None),
        &ReverseBlueprintApplyPlanRecord {
            project_total_chapters: None,
            existing_blueprint_total_chapters: None,
            next_version: "1.0".to_string(),
        },
        applied_at(),
    )
    .unwrap();
    let long = resolve_reverse_blueprint_apply_record(
        &draft(Some(PART_OUTLINE_THRESHOLD + 1), None),
        &ReverseBlueprintApplyPlanRecord {
            project_total_chapters: None,
            existing_blueprint_total_chapters: None,
            next_version: "1.0".to_string(),
        },
        applied_at(),
    )
    .unwrap();

    assert!(!short.needs_part_outlines);
    assert_eq!(short.chapters_per_part, None);
    assert!(long.needs_part_outlines);
    assert_eq!(long.chapters_per_part, Some(CHAPTERS_PER_PART));
}
