mod document;
mod relationship;
mod world;

pub use document::{blueprint_metadata_rag_record, blueprint_rag_record};
pub use relationship::relationship_rag_record;
pub use world::blueprint_world_setting_rag_records;

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_domain::project::BlueprintRecord;
    use chrono::Utc;

    #[test]
    fn manual_blueprint_indexes_only_present_optional_fields() {
        let now = Utc::now().naive_utc();
        let mut blueprint = BlueprintRecord {
            id: 1,
            project_id: 2,
            title: "雨夜来信".into(),
            genre: "悬疑".into(),
            style: None,
            tone: None,
            target_audience: None,
            one_sentence_summary: Some("一封来信改变命运".into()),
            full_synopsis: None,
            world_setting: None,
            current_version: "1.0".into(),
            needs_part_outlines: false,
            total_chapters: Some(1),
            chapters_per_part: None,
            avatar_svg: None,
            avatar_animal: None,
            created_at: now,
            updated_at: now,
        };
        let record = blueprint_rag_record(2, &blueprint).unwrap();
        assert!(record.text.contains("雨夜来信"));
        assert!(record.text.contains("一封来信改变命运"));
        assert!(!record.text.contains("完整梗概:"));
        assert!(
            blueprint_world_setting_rag_records(2, &blueprint)
                .unwrap()
                .is_empty()
        );
        blueprint.world_setting = Some(serde_json::Value::Null);
        assert!(blueprint_world_setting_rag_records(2, &blueprint).is_err());
        blueprint.genre.clear();
        assert!(blueprint_rag_record(2, &blueprint).is_err());
    }
}
