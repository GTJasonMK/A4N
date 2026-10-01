use a4n_db_models::models::project::novel_blueprint;
use a4n_domain::project::BlueprintRecord;

pub(crate) fn blueprint_record(model: novel_blueprint::Model) -> BlueprintRecord {
    BlueprintRecord {
        id: model.id,
        project_id: model.project_id,
        title: model.title,
        genre: model.genre,
        style: model.style,
        tone: model.tone,
        target_audience: model.target_audience,
        one_sentence_summary: model.one_sentence_summary,
        full_synopsis: model.full_synopsis,
        world_setting: model.world_setting,
        current_version: model.current_version,
        needs_part_outlines: model.needs_part_outlines,
        total_chapters: model.total_chapters,
        chapters_per_part: model.chapters_per_part,
        avatar_svg: model.avatar_svg,
        avatar_animal: model.avatar_animal,
        created_at: model.created_at,
        updated_at: model.updated_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn blueprint_record_preserves_storage_fields() {
        let timestamp = NaiveDate::from_ymd_opt(2026, 9, 27)
            .unwrap()
            .and_hms_opt(8, 30, 0)
            .unwrap();
        let model = novel_blueprint::Model {
            id: 1,
            project_id: 2,
            title: "蓝图".to_string(),
            genre: "科幻".to_string(),
            style: Some("冷峻".to_string()),
            tone: None,
            target_audience: Some("成年读者".to_string()),
            one_sentence_summary: Some("一句话".to_string()),
            full_synopsis: None,
            world_setting: Some(serde_json::json!({"premise": "星门"})),
            current_version: "1.0.0".to_string(),
            needs_part_outlines: true,
            total_chapters: Some(12),
            chapters_per_part: Some(4),
            avatar_svg: None,
            avatar_animal: Some("owl".to_string()),
            created_at: timestamp,
            updated_at: timestamp,
        };

        let record = blueprint_record(model.clone());

        assert_eq!(record.id, model.id);
        assert_eq!(record.world_setting, model.world_setting);
        assert_eq!(record.updated_at, model.updated_at);
    }
}
