use super::source_revision;
use a4n_db_models::models::chapter::chapter;
use chrono::Utc;
use serde_json::json;

#[test]
fn legacy_analysis_is_not_reported_as_new_fact_feedback() {
    let now = Utc::now().naive_utc();
    let mut chapter = chapter::Model {
        id: 1,
        project_id: 2,
        chapter_number: 1,
        outline_id: None,
        title: None,
        content: "施法会损失记忆。".into(),
        word_count: 9,
        summary: None,
        style_tags: None,
        status: "draft".into(),
        analysis_data: Some(json!({"summaries": {}})),
        created_at: now,
        updated_at: now,
    };
    assert!(source_revision(&chapter).is_none());
    chapter.analysis_data = Some(json!({"world_facts": []}));
    let first = source_revision(&chapter).unwrap();
    chapter.analysis_data.as_mut().unwrap()["_meta"] = json!({"attempt": 2});
    assert_eq!(source_revision(&chapter), Some(first));
}
