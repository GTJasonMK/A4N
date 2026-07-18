use super::*;
use chrono::NaiveDate;
use serde_json::json;

fn sample_outline(project_id: i64, pacing_score: Option<f32>) -> chapter_outline::Model {
    let now = NaiveDate::from_ymd_opt(2026, 6, 6)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap();
    chapter_outline::Model {
        id: 42,
        project_id,
        part_id: None,
        chapter_number: 3,
        title: Some("旧标题".to_string()),
        summary: Some("旧概要".to_string()),
        beats: None,
        key_scenes: None,
        pacing_score,
        foreshadowing_setup: None,
        foreshadowing_payoff: None,
        characters_involved: None,
        word_count_target: None,
        status: "draft".to_string(),
        created_at: now,
        updated_at: now,
    }
}

fn sample_part(id: i64, part_number: i32) -> part_outline::Model {
    let now = NaiveDate::from_ymd_opt(2026, 6, 6)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap();
    part_outline::Model {
        id,
        project_id: 1,
        part_number,
        title: format!("第 {part_number} 部"),
        start_chapter: (part_number - 1) * 10 + 1,
        end_chapter: part_number * 10,
        summary: Some("概要".to_string()),
        theme: None,
        key_events: None,
        character_focus: None,
        created_at: now,
        updated_at: now,
    }
}

fn sample_part_range(id: i64, part_number: i32, start: i32, end: i32) -> part_outline::Model {
    let mut part = sample_part(id, part_number);
    part.start_chapter = start;
    part.end_chapter = end;
    part
}

fn progress_outline(id: i64, chapter_number: i32, status: &str) -> chapter_outline::Model {
    let mut outline = sample_outline(1, None);
    outline.id = id;
    outline.chapter_number = chapter_number;
    outline.status = status.to_string();
    outline
}

#[test]
fn update_chapter_outline_input_requires_project_id() {
    let err = serde_json::from_value::<UpdateChapterOutlineInput>(json!({
        "outline_id": 42,
        "title": "新标题"
    }))
    .expect_err("project_id must be required");

    assert!(err.to_string().contains("project_id"));
}

#[test]
fn update_chapter_outline_input_rejects_unknown_fields() {
    let err = serde_json::from_value::<UpdateChapterOutlineInput>(json!({
        "project_id": 7,
        "outline_id": 42,
        "title": "新标题",
        "projectId": 7
    }))
    .expect_err("unknown fields must fail instead of being ignored");

    assert!(err.to_string().contains("unknown field"));
}

#[test]
fn update_chapter_outline_input_accepts_only_valid_provided_fields() {
    let input = serde_json::from_value::<UpdateChapterOutlineInput>(json!({
        "project_id": 7,
        "outline_id": 42,
        "title": "  新标题  ",
        "summary": "新概要",
        "beats": ["推进"],
        "key_scenes": [{"title": "关键场景"}]
    }))
    .expect("valid provided fields should deserialize");

    assert_eq!(input.title.as_deref(), Some("新标题"));
    assert_eq!(input.summary.as_deref(), Some("新概要"));
    assert_eq!(input.beats, Some(json!(["推进"])));
    assert_eq!(input.key_scenes, Some(json!([{"title": "关键场景"}])));
}

#[test]
fn update_chapter_outline_input_rejects_null_or_blank_text_fields() {
    for (field, value, expected) in [
        ("title", json!(null), "不能为 null"),
        ("summary", json!(" \n\t "), "不能为空"),
    ] {
        let mut payload = json!({
            "project_id": 7,
            "outline_id": 42
        });
        payload[field] = value;

        let err = serde_json::from_value::<UpdateChapterOutlineInput>(payload)
            .expect_err("invalid text field must fail");

        assert!(err.to_string().contains(field));
        assert!(err.to_string().contains(expected));
    }
}

#[test]
fn update_chapter_outline_input_rejects_non_array_structured_fields() {
    for (field, value, expected) in [
        ("beats", json!("推进"), "必须是数组"),
        ("key_scenes", json!(null), "不能为 null"),
    ] {
        let mut payload = json!({
            "project_id": 7,
            "outline_id": 42
        });
        payload[field] = value;

        let err = serde_json::from_value::<UpdateChapterOutlineInput>(payload)
            .expect_err("invalid structured field must fail");

        assert!(err.to_string().contains(field));
        assert!(err.to_string().contains(expected));
    }
}

#[test]
fn ensure_chapter_outline_belongs_to_project_rejects_mismatch() {
    let outline = sample_outline(1, None);

    let err = ensure_chapter_outline_belongs_to_project(&outline, 2)
        .expect_err("cross-project update must fail");

    assert!(err.to_string().contains("不属于项目 2"));
}

#[test]
fn required_outline_pacing_score_rejects_missing_or_non_finite_score() {
    let missing = required_outline_pacing_score(None, 3)
        .expect_err("missing pacing score should be rejected");
    assert!(missing.to_string().contains("第 3 章缺少 pacing_score"));

    let invalid = required_outline_pacing_score(Some(f32::NAN), 4)
        .expect_err("non-finite pacing score should be rejected");
    assert!(
        invalid
            .to_string()
            .contains("第 4 章 pacing_score 不是有限数字")
    );
}

#[test]
fn build_chapter_outlines_pacing_accepts_finite_scores() {
    let pacing = build_chapter_outlines_pacing(&[
        sample_outline(1, Some(0.5)),
        sample_outline(1, Some(0.7)),
    ])
    .expect("finite pacing scores should be accepted");

    assert_eq!(pacing["pacing_score"], json!(0.6));
    assert_eq!(pacing["summary"], json!("节奏适中"));
}

#[test]
fn order_parts_for_batch_generation_uses_part_order_not_input_order() {
    let mut parts = vec![
        sample_part(30, 3),
        sample_part(10, 1),
        sample_part(20, 2),
        sample_part(19, 2),
    ];

    order_parts_for_batch_generation(&mut parts);

    let ids = parts.iter().map(|part| part.id).collect::<Vec<_>>();
    assert_eq!(ids, vec![10, 19, 20, 30]);
}

#[test]
fn completed_part_count_requires_full_completed_chapter_range() {
    let parts = vec![
        sample_part_range(1, 1, 1, 2),
        sample_part_range(2, 2, 3, 4),
        sample_part_range(3, 3, 5, 5),
    ];
    let outlines = vec![
        progress_outline(11, 1, chapter_outline::STATUS_COMPLETED),
        progress_outline(12, 2, chapter_outline::STATUS_COMPLETED),
        progress_outline(13, 3, chapter_outline::STATUS_COMPLETED),
        progress_outline(14, 4, chapter_outline::STATUS_DRAFT),
        progress_outline(15, 5, chapter_outline::STATUS_COMPLETED),
    ];

    let completed =
        completed_part_count(&parts, &outlines).expect("valid progress data should count");

    assert_eq!(completed, 2);
}

#[test]
fn completed_part_count_rejects_invalid_progress_state() {
    let duplicate = completed_part_count(
        &[sample_part_range(1, 1, 1, 2)],
        &[
            progress_outline(11, 1, chapter_outline::STATUS_COMPLETED),
            progress_outline(12, 1, chapter_outline::STATUS_COMPLETED),
        ],
    )
    .expect_err("duplicate chapter outlines must be explicit");
    assert!(duplicate.to_string().contains("存在重复章节大纲"));

    let overlapping = completed_part_count(
        &[sample_part_range(1, 1, 1, 2), sample_part_range(2, 2, 2, 3)],
        &[],
    )
    .expect_err("overlapping part ranges must be explicit");
    assert!(overlapping.to_string().contains("被多个分部范围覆盖"));
}
