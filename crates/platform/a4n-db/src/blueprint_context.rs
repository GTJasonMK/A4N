//! Database model to LLM context projection.
//!
//! ORM metadata stays inside the persistence adapter; callers receive only
//! the deliberately public JSON projection.

use a4n_db_models::models::project::{
    blueprint_character, blueprint_relationship, novel_blueprint,
};
use serde_json::{Value, json};

pub fn blueprint_llm_context(
    blueprint: &novel_blueprint::Model,
    characters: &[blueprint_character::Model],
    relationships: &[blueprint_relationship::Model],
) -> Value {
    json!({
        "blueprint": blueprint_model_llm_context(blueprint),
        "characters": characters
            .iter()
            .map(blueprint_character_llm_context)
            .collect::<Vec<_>>(),
        "relationships": relationships
            .iter()
            .map(blueprint_relationship_llm_context)
            .collect::<Vec<_>>(),
    })
}

pub fn blueprint_model_llm_context(blueprint: &novel_blueprint::Model) -> Value {
    json!({
        "title": blueprint.title,
        "genre": blueprint.genre,
        "style": blueprint.style,
        "tone": blueprint.tone,
        "target_audience": blueprint.target_audience,
        "one_sentence_summary": blueprint.one_sentence_summary,
        "full_synopsis": blueprint.full_synopsis,
        "world_setting": blueprint.world_setting,
        "needs_part_outlines": blueprint.needs_part_outlines,
        "total_chapters": blueprint.total_chapters,
        "chapters_per_part": blueprint.chapters_per_part,
    })
}

pub fn blueprint_character_llm_context(character: &blueprint_character::Model) -> Value {
    json!({
        "name": character.name,
        "character_type": character.character_type,
        "gender": character.gender,
        "age": character.age,
        "appearance": character.appearance,
        "personality": character.personality,
        "background": character.background,
        "motivation": character.motivation,
        "arc_summary": character.arc_summary,
        "relationship_to_protagonist": character.relationship_to_protagonist,
        "arc_stages": character.arc_stages,
    })
}

pub fn blueprint_relationship_llm_context(relationship: &blueprint_relationship::Model) -> Value {
    json!({
        "relationship_type": relationship.relationship_type,
        "character_a": relationship.character_a,
        "character_b": relationship.character_b,
        "description": relationship.description,
        "strength": relationship.strength,
        "evolution": relationship.evolution,
    })
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, NaiveDateTime};
    use serde_json::json;

    use super::blueprint_llm_context;
    use a4n_db_models::models::project::{
        blueprint_character, blueprint_relationship, novel_blueprint,
    };

    #[test]
    fn projects_story_fields_without_database_metadata() {
        let now: NaiveDateTime = NaiveDate::from_ymd_opt(2026, 5, 26)
            .unwrap()
            .and_hms_opt(10, 0, 0)
            .unwrap();
        let blueprint = novel_blueprint::Model {
            id: 10,
            project_id: 20,
            title: "星潮档案".to_string(),
            genre: "科幻".to_string(),
            style: Some("冷峻".to_string()),
            tone: Some("悬疑".to_string()),
            target_audience: Some("成人".to_string()),
            one_sentence_summary: Some("调查星潮真相".to_string()),
            full_synopsis: Some("完整主线".to_string()),
            world_setting: Some(json!({"premise": "星潮复苏"})),
            current_version: "1.0.0".to_string(),
            needs_part_outlines: true,
            total_chapters: Some(100),
            chapters_per_part: Some(25),
            avatar_svg: Some("<svg />".to_string()),
            avatar_animal: Some("crest".to_string()),
            created_at: now,
            updated_at: now,
        };
        let character = blueprint_character::Model {
            id: 11,
            project_id: 20,
            name: "林烬".to_string(),
            character_type: "protagonist".to_string(),
            gender: Some("男".to_string()),
            age: Some("二十岁".to_string()),
            appearance: Some("灰色制服".to_string()),
            personality: Some("克制".to_string()),
            background: Some("调查员".to_string()),
            motivation: Some("查明真相".to_string()),
            arc_summary: Some("从逃避到承担".to_string()),
            relationship_to_protagonist: Some("本人".to_string()),
            arc_stages: Some(json!({"start": "逃避"})),
            created_at: now,
            updated_at: now,
        };
        let relationship = blueprint_relationship::Model {
            id: 12,
            project_id: 20,
            relationship_type: "rival".to_string(),
            character_a: "林烬".to_string(),
            character_b: "沈澜".to_string(),
            description: Some("互相试探".to_string()),
            strength: Some(7),
            evolution: Some(json!({"start": "敌意"})),
            created_at: now,
        };

        let context = blueprint_llm_context(&blueprint, &[character], &[relationship]);
        let text = serde_json::to_string(&context).unwrap();

        assert_eq!(context["blueprint"]["title"], json!("星潮档案"));
        assert_eq!(context["characters"][0]["motivation"], json!("查明真相"));
        assert_eq!(context["relationships"][0]["strength"], json!(7));
        assert!(!text.contains("\"project_id\""));
        assert!(!text.contains("\"created_at\""));
        assert!(!text.contains("\"updated_at\""));
        assert!(!text.contains("\"avatar_svg\""));
    }
}
