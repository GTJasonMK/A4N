use super::{
    CharacterConflictChapterScope,
    context::{
        MAX_CONFLICT_FALLBACK_CONTENT_CHARS, MAX_CONFLICT_TOTAL_CONTEXT_CHARS,
        character_conflict_allowed_source_ids, character_conflict_chapter_context,
        ensure_character_conflict_context_size, ensure_character_conflict_has_comparison_context,
    },
    parsing::{
        parse_conflict_output, parse_conflict_output_with_source_ids,
        restored_character_conflict_checkpoint,
    },
};
use a4n_core::models::chapter::chapter;
use chrono::NaiveDate;
use serde_json::json;
use std::collections::BTreeSet;

fn sample_chapter(summary: Option<&str>, content: &str) -> chapter::Model {
    let now = NaiveDate::from_ymd_opt(2026, 5, 31)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    chapter::Model {
        id: 1,
        project_id: 9,
        outline_id: None,
        chapter_number: 3,
        title: Some("旧城".to_string()),
        content: content.to_string(),
        word_count: content.chars().count() as i32,
        status: "draft".to_string(),
        summary: summary.map(ToOwned::to_owned),
        style_tags: None,
        analysis_data: None,
        created_at: now,
        updated_at: now,
    }
}

#[test]
fn character_conflict_chapter_context_uses_summary_before_content() {
    let chapter = sample_chapter(Some("主角进入旧城。"), "完整正文不应重复塞入");
    let context = character_conflict_chapter_context(&chapter)
        .unwrap()
        .expect("chapter should be included");
    assert_eq!(context["source_id"], json!("chapter:3"));
    assert_eq!(context["summary"], json!("主角进入旧城。"));
    assert!(context.get("content").is_none());
}

#[test]
fn character_conflict_chapter_context_uses_short_content_when_summary_missing() {
    let content = "陈恪走进旧城，看见魏征站在雨里。";
    let context = character_conflict_chapter_context(&sample_chapter(None, content))
        .unwrap()
        .unwrap();
    assert_eq!(context["content"], json!(content));
}

#[test]
fn character_conflict_chapter_context_rejects_long_content_without_summary() {
    let content =
        "陈恪走进旧城，看见魏征站在雨里。".repeat(MAX_CONFLICT_FALLBACK_CONTENT_CHARS / 10 + 10);
    let err = character_conflict_chapter_context(&sample_chapter(None, &content)).unwrap_err();
    assert!(err.to_string().contains("超过角色一致性检查兜底上限"));
}

#[test]
fn character_conflict_allowed_source_ids_uses_only_top_level_inputs() {
    let role = json!({"source_id": "profile"});
    let chapters = vec![json!({
        "source_id": "chapter:3",
        "analysis": {"key_events": [{"source_id": "legacy_event:1"}]},
    })];
    let ids = character_conflict_allowed_source_ids(
        &role,
        &chapters,
        &[json!({"source_id": "state:3"})],
        &[json!({"source_id": "attribute_event:9"})],
    );
    assert!(ids.contains("profile"));
    assert!(ids.contains("chapter:3"));
    assert!(!ids.contains("legacy_event:1"));
}

#[test]
fn restored_character_conflict_checkpoint_revalidates_response_content() {
    let allowed = ["profile".to_string(), "chapter:3".to_string()]
        .into_iter()
        .collect::<BTreeSet<_>>();
    let raw = r#"[{"chapter_number":3,"field":"goal","expected":"留在旧城","actual":"已经离开旧城","expected_source_id":"profile","actual_source_id":"chapter:3","severity":"major"}]"#;
    let output = parse_conflict_output_with_source_ids(raw, &allowed).unwrap();
    let restored = restored_character_conflict_checkpoint(
        json!({"output": output.clone(), "parsed_output": output, "response_content": raw}),
        "character_conflict:1:陈恪",
        &allowed,
    )
    .unwrap();
    assert_eq!(restored.len(), 1);
}

#[test]
fn restored_character_conflict_checkpoint_rejects_mismatched_output() {
    let allowed = ["profile".to_string(), "chapter:3".to_string()]
        .into_iter()
        .collect::<BTreeSet<_>>();
    let raw = r#"[{"chapter_number":3,"field":"goal","expected":"留在旧城","actual":"已经离开旧城","expected_source_id":"profile","actual_source_id":"chapter:3","severity":"major"}]"#;
    let err = restored_character_conflict_checkpoint(
        json!({"output": [], "parsed_output": [], "response_content": raw}),
        "character_conflict:1:陈恪",
        &allowed,
    )
    .unwrap_err();
    assert!(
        err.to_string()
            .contains("checkpoint_parsed_output_mismatch")
    );
}

#[test]
fn character_conflict_requires_comparison_context() {
    let err = ensure_character_conflict_has_comparison_context(&[], &[], &[]).unwrap_err();
    assert!(err.to_string().contains("缺少可对照上下文"));
}

#[test]
fn character_conflict_allows_any_comparison_context() {
    ensure_character_conflict_has_comparison_context(&[json!({})], &[], &[]).unwrap();
    ensure_character_conflict_has_comparison_context(&[], &[json!({})], &[]).unwrap();
    ensure_character_conflict_has_comparison_context(&[], &[], &[json!({})]).unwrap();
}

#[test]
fn character_conflict_allows_context_within_size_limit() {
    ensure_character_conflict_context_size(
        &json!({"source_id": "profile"}),
        &[json!({"source_id": "chapter:3", "summary": "主角进入旧城。"})],
        &[],
        &[],
    )
    .unwrap();
}

#[test]
fn character_conflict_rejects_oversized_total_context() {
    let err = ensure_character_conflict_context_size(
        &json!({"source_id": "profile"}),
        &[json!({"summary": "角色状态摘要过长。".repeat(MAX_CONFLICT_TOTAL_CONTEXT_CHARS)})],
        &[],
        &[],
    )
    .unwrap_err();
    assert!(err.to_string().contains("全量上下文过大"));
}

#[test]
fn character_conflict_chapter_scope_accepts_full_and_range() {
    assert_eq!(
        CharacterConflictChapterScope::new(None, None).unwrap(),
        CharacterConflictChapterScope::default()
    );
    assert_eq!(
        CharacterConflictChapterScope::new(Some(2), Some(5)).unwrap(),
        CharacterConflictChapterScope {
            start_chapter: Some(2),
            end_chapter: Some(5),
        }
    );
}

#[test]
fn character_conflict_chapter_scope_rejects_invalid_range() {
    assert!(CharacterConflictChapterScope::new(Some(0), None).is_err());
    assert!(CharacterConflictChapterScope::new(None, Some(-1)).is_err());
    assert!(CharacterConflictChapterScope::new(Some(5), Some(2)).is_err());
}

#[test]
fn parse_conflict_output_accepts_empty_array() {
    assert!(parse_conflict_output("[]").unwrap().is_empty());
}

#[test]
fn parse_conflict_output_rejects_embedded_json() {
    assert!(parse_conflict_output(r#"结果如下：[{"chapter_number":1}]"#).is_err());
}

#[test]
fn parse_conflict_output_rejects_invalid_item_contracts() {
    let invalid_severity = r#"[{"chapter_number":1,"field":"personality","expected":"冷静","actual":"冲动","expected_source_id":"profile","actual_source_id":"state:1","severity":"critical"}]"#;
    assert!(
        parse_conflict_output(invalid_severity)
            .unwrap_err()
            .to_string()
            .contains("severity")
    );
    let missing_source = r#"[{"chapter_number":1,"field":"personality","expected":"冷静","actual":"冲动","severity":"major"}]"#;
    assert!(
        parse_conflict_output(missing_source)
            .unwrap_err()
            .to_string()
            .contains("expected_source_id")
    );
    let unknown = r#"[{"chapter_number":1,"field":"personality","expected":"冷静","actual":"冲动","expected_source_id":"profile","actual_source_id":"state:1","severity":"major","notes":"多余说明"}]"#;
    assert!(
        parse_conflict_output(unknown)
            .unwrap_err()
            .to_string()
            .contains("未支持字段 `notes`")
    );
}

#[test]
fn parse_conflict_output_with_source_ids_enforces_selected_sources() {
    let allowed = ["profile".to_string(), "state:1".to_string()]
        .into_iter()
        .collect();
    let valid = r#"[{"chapter_number":1,"field":"personality","expected":"冷静","actual":"冲动","expected_source_id":"profile","actual_source_id":"state:1","severity":"major"}]"#;
    assert_eq!(
        parse_conflict_output_with_source_ids(valid, &allowed)
            .unwrap()
            .len(),
        1
    );
    let restricted = ["profile".to_string()].into_iter().collect();
    assert!(parse_conflict_output_with_source_ids(valid, &restricted).is_err());
}

#[test]
fn nested_analysis_source_ids_are_not_allowed() {
    let source_ids = character_conflict_allowed_source_ids(
        &json!({"source_id": "profile"}),
        &[json!({
            "source_id": "chapter:3",
            "analysis": {"key_events": [{"source_id": "legacy_event:1"}]},
        })],
        &[],
        &[],
    );
    let raw = r#"[{"chapter_number":3,"field":"appearance","expected":"黑衣","actual":"白衣","expected_source_id":"legacy_event:1","actual_source_id":"chapter:3","severity":"minor"}]"#;
    assert!(parse_conflict_output_with_source_ids(raw, &source_ids).is_err());
}
