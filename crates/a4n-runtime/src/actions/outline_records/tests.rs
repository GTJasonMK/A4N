use super::*;
use a4n_orchestrator::outline::required_outline_pacing_score;
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
