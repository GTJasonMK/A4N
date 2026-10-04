use a4n_db_models::models::project::{
    blueprint_character, blueprint_relationship, novel_blueprint,
};
use a4n_domain::project::{BlueprintCharacterRecord, BlueprintRecord, BlueprintRelationshipRecord};

pub(crate) fn blueprint_character_record(
    model: blueprint_character::Model,
) -> BlueprintCharacterRecord {
    BlueprintCharacterRecord {
        id: model.id,
        project_id: model.project_id,
        name: model.name,
        character_type: model.character_type,
        gender: model.gender,
        age: model.age,
        appearance: model.appearance,
        personality: model.personality,
        background: model.background,
        motivation: model.motivation,
        arc_summary: model.arc_summary,
        relationship_to_protagonist: model.relationship_to_protagonist,
        arc_stages: model.arc_stages,
        created_at: model.created_at,
        updated_at: model.updated_at,
    }
}

pub(crate) fn blueprint_relationship_record(
    model: blueprint_relationship::Model,
) -> BlueprintRelationshipRecord {
    BlueprintRelationshipRecord {
        id: model.id,
        project_id: model.project_id,
        relationship_type: model.relationship_type,
        character_a: model.character_a,
        character_b: model.character_b,
        description: model.description,
        strength: model.strength,
        evolution: model.evolution,
        created_at: model.created_at,
    }
}

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

pub(crate) fn blueprint_model_from_record(record: BlueprintRecord) -> novel_blueprint::Model {
    novel_blueprint::Model {
        id: record.id,
        project_id: record.project_id,
        title: record.title,
        genre: record.genre,
        style: record.style,
        tone: record.tone,
        target_audience: record.target_audience,
        one_sentence_summary: record.one_sentence_summary,
        full_synopsis: record.full_synopsis,
        world_setting: record.world_setting,
        current_version: record.current_version,
        needs_part_outlines: record.needs_part_outlines,
        total_chapters: record.total_chapters,
        chapters_per_part: record.chapters_per_part,
        avatar_svg: record.avatar_svg,
        avatar_animal: record.avatar_animal,
        created_at: record.created_at,
        updated_at: record.updated_at,
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
        assert_eq!(
            serde_json::to_value(record).unwrap(),
            serde_json::to_value(model).unwrap()
        );
    }
}
