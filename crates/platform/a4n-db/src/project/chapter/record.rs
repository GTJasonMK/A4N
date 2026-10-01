use a4n_db_models::models::chapter::chapter;
use a4n_domain::chapter::ChapterRecord;

pub(crate) fn chapter_record(model: chapter::Model) -> ChapterRecord {
    ChapterRecord {
        id: model.id,
        project_id: model.project_id,
        outline_id: model.outline_id,
        chapter_number: model.chapter_number,
        title: model.title,
        content: model.content,
        word_count: model.word_count,
        status: model.status,
        summary: model.summary,
        style_tags: model.style_tags,
        analysis_data: model.analysis_data,
        created_at: model.created_at,
        updated_at: model.updated_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn chapter_record_preserves_content_and_analysis_fields() {
        let timestamp = NaiveDate::from_ymd_opt(2026, 9, 27)
            .unwrap()
            .and_hms_opt(8, 30, 0)
            .unwrap();
        let model = chapter::Model {
            id: 1,
            project_id: 2,
            outline_id: Some(3),
            chapter_number: 4,
            title: Some("章节".to_string()),
            content: "正文".to_string(),
            word_count: 2,
            status: "completed".to_string(),
            summary: Some("摘要".to_string()),
            style_tags: Some(serde_json::json!(["冷峻"])),
            analysis_data: Some(serde_json::json!({"key_events": []})),
            created_at: timestamp,
            updated_at: timestamp,
        };

        let record = chapter_record(model.clone());

        assert_eq!(record.id, model.id);
        assert_eq!(record.analysis_data, model.analysis_data);
        assert_eq!(record.updated_at, model.updated_at);
    }
}
