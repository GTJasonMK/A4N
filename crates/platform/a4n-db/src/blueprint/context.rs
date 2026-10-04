//! Database model to LLM context projection.
//!
//! ORM metadata stays inside the persistence adapter; callers receive only
//! the deliberately public JSON projection.

use serde::Serialize;
use serde_json::{Value, json};

pub fn blueprint_llm_context<B, C, R>(blueprint: &B, characters: &[C], relationships: &[R]) -> Value
where
    B: Serialize,
    C: Serialize,
    R: Serialize,
{
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

pub fn blueprint_model_llm_context(blueprint: &impl Serialize) -> Value {
    let source = serde_json::to_value(blueprint).expect("blueprint record must serialize");
    let fields = [
        "title",
        "genre",
        "style",
        "tone",
        "target_audience",
        "one_sentence_summary",
        "full_synopsis",
        "world_setting",
        "needs_part_outlines",
        "total_chapters",
        "chapters_per_part",
    ];
    Value::Object(
        fields
            .into_iter()
            .map(|field| {
                (
                    field.to_string(),
                    source
                        .get(field)
                        .cloned()
                        .expect("blueprint context field must exist"),
                )
            })
            .collect(),
    )
}

pub fn blueprint_character_llm_context(character: &impl Serialize) -> Value {
    serialize_fields(
        character,
        &[
            "name",
            "character_type",
            "gender",
            "age",
            "appearance",
            "personality",
            "background",
            "motivation",
            "arc_summary",
            "relationship_to_protagonist",
            "arc_stages",
        ],
    )
}

pub fn blueprint_relationship_llm_context(relationship: &impl Serialize) -> Value {
    serialize_fields(
        relationship,
        &[
            "relationship_type",
            "character_a",
            "character_b",
            "description",
            "strength",
            "evolution",
        ],
    )
}

fn serialize_fields(record: &impl Serialize, fields: &[&str]) -> Value {
    let value = serde_json::to_value(record).expect("blueprint context record must serialize");
    Value::Object(
        fields
            .iter()
            .map(|field| {
                (
                    (*field).to_string(),
                    value
                        .get(field)
                        .cloned()
                        .expect("blueprint context field must exist"),
                )
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, NaiveDateTime};
    use serde_json::json;

    use super::{blueprint_llm_context, blueprint_model_llm_context};
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
        let record: a4n_domain::project::BlueprintRecord =
            serde_json::from_value(serde_json::to_value(&blueprint).unwrap()).unwrap();
        assert_eq!(
            blueprint_model_llm_context(&blueprint),
            blueprint_model_llm_context(&record)
        );
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

        let characters = [crate::project::blueprint_character_record(character)];
        let relationships = [crate::project::blueprint_relationship_record(relationship)];
        let context = blueprint_llm_context(&blueprint, &characters, &relationships);
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
