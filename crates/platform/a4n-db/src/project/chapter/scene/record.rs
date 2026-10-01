use a4n_db_models::models::chapter::chapter_scene;
use a4n_domain::chapter::ChapterSceneRecord;

pub(super) fn chapter_scene_record(model: chapter_scene::Model) -> ChapterSceneRecord {
    ChapterSceneRecord {
        id: model.id,
        chapter_id: model.chapter_id,
        scene_number: model.scene_number,
        title: model.title,
        content: model.content,
        word_count: model.word_count,
        summary: model.summary,
        scene_type: model.scene_type,
        characters_present: model.characters_present,
        location: model.location,
        timeline_position: model.timeline_position,
        transition_from_previous: model.transition_from_previous,
        created_at: model.created_at,
        updated_at: model.updated_at,
    }
}

pub(super) fn chapter_scene_model_from_record(record: &ChapterSceneRecord) -> chapter_scene::Model {
    chapter_scene::Model {
        id: record.id,
        chapter_id: record.chapter_id,
        scene_number: record.scene_number,
        title: record.title.clone(),
        content: record.content.clone(),
        word_count: record.word_count,
        summary: record.summary.clone(),
        scene_type: record.scene_type.clone(),
        characters_present: record.characters_present.clone(),
        location: record.location.clone(),
        timeline_position: record.timeline_position.clone(),
        transition_from_previous: record.transition_from_previous.clone(),
        created_at: record.created_at,
        updated_at: record.updated_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use serde_json::json;

    #[test]
    fn chapter_scene_record_preserves_storage_shape() {
        let timestamp = NaiveDate::from_ymd_opt(2026, 9, 27)
            .unwrap()
            .and_hms_opt(8, 30, 0)
            .unwrap();
        let model = chapter_scene::Model {
            id: 1,
            chapter_id: 2,
            scene_number: 3,
            title: Some("场景标题".to_string()),
            content: "场景正文".to_string(),
            word_count: 4,
            summary: Some("场景摘要".to_string()),
            scene_type: "development".to_string(),
            characters_present: Some(json!(["角色一"])),
            location: Some("旧屋".to_string()),
            timeline_position: Some("夜晚".to_string()),
            transition_from_previous: Some("推门而入".to_string()),
            created_at: timestamp,
            updated_at: timestamp,
        };

        let record = chapter_scene_record(model.clone());

        assert_eq!(chapter_scene_model_from_record(&record), model);
    }
}
