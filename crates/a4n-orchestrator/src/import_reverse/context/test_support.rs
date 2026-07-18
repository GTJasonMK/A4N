use a4n_core::models::chapter::chapter;
use chrono::NaiveDate;
use serde_json::Value;

pub(super) fn sample_chapter(analysis_data: Option<Value>) -> chapter::Model {
    sample_chapter_with_number(2, analysis_data)
}

pub(super) fn sample_chapter_with_number(
    chapter_number: i32,
    analysis_data: Option<Value>,
) -> chapter::Model {
    let now = NaiveDate::from_ymd_opt(2026, 5, 26)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap();
    chapter::Model {
        id: 9,
        project_id: 3,
        outline_id: None,
        chapter_number,
        title: Some("第二章".into()),
        content: "第一段\n\n第二段".into(),
        word_count: 6,
        status: "completed".into(),
        summary: None,
        style_tags: None,
        analysis_data,
        created_at: now,
        updated_at: now,
    }
}
