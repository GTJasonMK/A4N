use super::super::sync_preflight::{
    analysis_character_profile_sync_preflight_failures,
    report_analysis_character_profile_sync_preflight_failures,
};
use super::fixtures::{character_sync_context_for_test, discovery_chapter};
use serde_json::json;

#[test]
fn analysis_sync_preflight_rejects_missing_required_chapter_before_llm() {
    let mut context = character_sync_context_for_test("陈恪");
    context.profile.last_synced_chapter = 2;
    let chapters = vec![
        discovery_chapter(1, "completed", "第一章正文"),
        discovery_chapter(2, "completed", "第二章正文"),
        discovery_chapter(4, "completed", "第四章正文"),
        discovery_chapter(5, "completed", "第五章正文"),
    ];

    let failures =
        analysis_character_profile_sync_preflight_failures(&[context.profile], &chapters, 5);

    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].name, "陈恪");
    assert!(failures[0].error.contains("角色档案同步范围不完整"));
    assert!(failures[0].error.contains("第 3 章"));
}

#[test]
fn analysis_sync_preflight_skips_profile_first_seen_after_target() {
    let mut context = character_sync_context_for_test("西市胡商");
    context.profile.base_attributes = json!({
        "explicit_attributes": {
            "first_seen_chapter": 6
        },
        "implicit_attributes": {},
        "social_attributes": {}
    });
    context.profile.current_state = context.profile.base_attributes.clone();
    context.profile.last_synced_chapter = 0;
    let chapters = vec![
        discovery_chapter(1, "completed", "第一章正文"),
        discovery_chapter(2, "completed", "第二章正文"),
        discovery_chapter(4, "completed", "第四章正文"),
        discovery_chapter(5, "completed", "第五章正文"),
    ];

    let failures =
        analysis_character_profile_sync_preflight_failures(&[context.profile], &chapters, 5);

    assert!(failures.is_empty());
}

#[test]
fn analysis_sync_preflight_report_does_not_claim_partial_writes() {
    let mut context = character_sync_context_for_test("陈恪");
    context.profile.last_synced_chapter = 2;
    let chapters = vec![
        discovery_chapter(1, "completed", "第一章正文"),
        discovery_chapter(2, "completed", "第二章正文"),
        discovery_chapter(4, "completed", "第四章正文"),
        discovery_chapter(5, "completed", "第五章正文"),
    ];
    let failures =
        analysis_character_profile_sync_preflight_failures(&[context.profile], &chapters, 5);

    let err = report_analysis_character_profile_sync_preflight_failures(failures)
        .expect_err("preflight failure should be visible");

    assert!(err.contains("章节分析前角色档案同步预检失败"));
    assert!(err.contains("陈恪"));
    assert!(!err.contains("已成功写入"));
}
