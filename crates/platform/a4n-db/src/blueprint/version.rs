use a4n_db_models::models::project::{
    blueprint_character, blueprint_relationship, blueprint_version, novel_blueprint,
};
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};
use serde_json::{Value, json};

use super::{BlueprintDbError, InsertBlueprintVersionInput};

/// 蓝图版本与项目 raw_blueprint 共用的唯一规范快照。
pub fn blueprint_version_snapshot(
    blueprint: &novel_blueprint::Model,
    characters: &[blueprint_character::Model],
    relationships: &[blueprint_relationship::Model],
) -> Value {
    let characters = characters
        .iter()
        .map(|character| {
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
        })
        .collect::<Vec<_>>();
    let relationships = relationships
        .iter()
        .map(|relationship| {
            json!({
                "relationship_type": relationship.relationship_type,
                "character_a": relationship.character_a,
                "character_b": relationship.character_b,
                "description": relationship.description,
                "strength": relationship.strength,
                "evolution": relationship.evolution,
            })
        })
        .collect::<Vec<_>>();

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
        "avatar_svg": blueprint.avatar_svg,
        "avatar_animal": blueprint.avatar_animal,
        "characters": characters,
        "relationships": relationships,
    })
}

/// 插入蓝图版本快照记录。
pub async fn insert_blueprint_version_record<C>(
    db: &C,
    input: InsertBlueprintVersionInput,
) -> Result<blueprint_version::Model, BlueprintDbError>
where
    C: ConnectionTrait,
{
    if input.project_id <= 0 {
        return Err(BlueprintDbError::InvalidArgument(
            "project_id 必须大于 0".to_string(),
        ));
    }
    crate::project::validate_blueprint_version_snapshot(&input.blueprint_data)
        .map_err(|error| BlueprintDbError::InvalidArgument(error.to_string()))?;
    let version = canonical_version_text(input.version, "version")?;
    let change_description =
        canonical_version_text(input.change_description, "change_description")?;
    let created_by = canonical_version_text(input.created_by, "created_by")?;
    Ok(blueprint_version::ActiveModel {
        project_id: Set(input.project_id),
        version: Set(version),
        change_description: Set(Some(change_description)),
        blueprint_data: Set(input.blueprint_data),
        created_at: Set(input.now),
        created_by: Set(Some(created_by)),
        ..Default::default()
    }
    .insert(db)
    .await?)
}

fn canonical_version_text(value: String, field: &str) -> Result<String, BlueprintDbError> {
    if value.is_empty() || value.trim() != value {
        Err(BlueprintDbError::InvalidArgument(format!(
            "{field} 必须是非空规范文本"
        )))
    } else {
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use serde_json::json;

    use super::{blueprint_version_snapshot, canonical_version_text};
    use a4n_db_models::models::project::{
        blueprint_character, blueprint_relationship, novel_blueprint,
    };

    #[test]
    fn blueprint_version_metadata_requires_canonical_text() {
        assert_eq!(
            canonical_version_text("system".to_string(), "created_by").unwrap(),
            "system"
        );
        for value in ["", " system "] {
            assert!(canonical_version_text(value.to_string(), "created_by").is_err());
        }
    }

    #[test]
    fn blueprint_version_snapshot_omits_retired_and_database_fields() {
        let now = NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let blueprint = novel_blueprint::Model {
            id: 11,
            project_id: 7,
            title: "雾城".to_string(),
            genre: "悬疑".to_string(),
            style: Some("冷峻".to_string()),
            tone: None,
            target_audience: None,
            one_sentence_summary: None,
            full_synopsis: Some("追查倒流迷雾".to_string()),
            world_setting: Some(json!({"premise": "迷雾倒流"})),
            current_version: "2.0".to_string(),
            needs_part_outlines: false,
            total_chapters: Some(30),
            chapters_per_part: None,
            avatar_svg: None,
            avatar_animal: None,
            created_at: now,
            updated_at: now,
        };
        let character = blueprint_character::Model {
            id: 13,
            project_id: 7,
            name: "林澈".to_string(),
            character_type: "protagonist".to_string(),
            gender: None,
            age: None,
            appearance: None,
            personality: Some("克制".to_string()),
            background: None,
            motivation: None,
            arc_summary: None,
            relationship_to_protagonist: None,
            arc_stages: None,
            created_at: now,
            updated_at: now,
        };
        let relationship = blueprint_relationship::Model {
            id: 17,
            project_id: 7,
            relationship_type: "rival".to_string(),
            character_a: "林澈".to_string(),
            character_b: "许灯".to_string(),
            description: None,
            strength: Some(7),
            evolution: None,
            created_at: now,
        };

        let snapshot = blueprint_version_snapshot(&blueprint, &[character], &[relationship]);
        assert_eq!(snapshot.as_object().unwrap().len(), 15);
        for retired in ["current_version", "chapter_outline", "optimization_patch"] {
            assert!(snapshot.get(retired).is_none(), "{retired}");
        }
        let character = snapshot["characters"][0].as_object().unwrap();
        assert_eq!(character.len(), 11);
        for database_field in ["id", "project_id", "created_at", "updated_at"] {
            assert!(!character.contains_key(database_field), "{database_field}");
        }
        let relationship = snapshot["relationships"][0].as_object().unwrap();
        assert_eq!(relationship.len(), 6);
        for database_field in ["id", "project_id", "created_at"] {
            assert!(
                !relationship.contains_key(database_field),
                "{database_field}"
            );
        }
    }
}
