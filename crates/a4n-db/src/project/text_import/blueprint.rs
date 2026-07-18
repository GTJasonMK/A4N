use super::*;

pub(super) async fn insert_import_text_project_blueprint<C>(
    db: &C,
    project_id: i64,
    blueprint: &ImportTextProjectBlueprint,
    now: NaiveDateTime,
) -> Result<(), ImportTextProjectError>
where
    C: ConnectionTrait,
{
    novel_blueprint::ActiveModel {
        project_id: Set(project_id),
        title: Set(blueprint.title.clone()),
        genre: Set(blueprint.genre.clone()),
        style: Set(blueprint.style.clone()),
        tone: Set(blueprint.tone.clone()),
        target_audience: Set(blueprint.target_audience.clone()),
        one_sentence_summary: Set(blueprint.one_sentence_summary.clone()),
        full_synopsis: Set(blueprint.full_synopsis.clone()),
        world_setting: Set(blueprint.world_setting.clone()),
        current_version: Set(blueprint.current_version.clone()),
        needs_part_outlines: Set(blueprint.needs_part_outlines),
        total_chapters: Set(blueprint.total_chapters),
        chapters_per_part: Set(blueprint.chapters_per_part),
        avatar_svg: Set(blueprint.avatar_svg.clone()),
        avatar_animal: Set(blueprint.avatar_animal.clone()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(|error| ImportTextProjectError::database("导入蓝图失败", error))?;
    Ok(())
}

pub(super) async fn insert_import_text_project_blueprint_versions<C>(
    db: &C,
    project_id: i64,
    versions: &[ImportTextProjectBlueprintVersion],
    now: NaiveDateTime,
) -> Result<(), ImportTextProjectError>
where
    C: ConnectionTrait,
{
    for version in versions {
        blueprint_version::ActiveModel {
            project_id: Set(project_id),
            version: Set(version.version.clone()),
            change_description: Set(version.change_description.clone()),
            blueprint_data: Set(version.blueprint_data.clone()),
            world_setting_snapshot: Set(version.world_setting_snapshot.clone()),
            characters_snapshot: Set(version.characters_snapshot.clone()),
            relationships_snapshot: Set(version.relationships_snapshot.clone()),
            synopsis_snapshot: Set(version.synopsis_snapshot.clone()),
            created_at: Set(now),
            created_by: Set(version.created_by.clone()),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|error| ImportTextProjectError::database("导入蓝图版本失败", error))?;
    }
    Ok(())
}

pub(super) async fn insert_import_text_project_characters<C>(
    db: &C,
    project_id: i64,
    characters: &[ImportTextProjectCharacter],
    now: NaiveDateTime,
) -> Result<(), ImportTextProjectError>
where
    C: ConnectionTrait,
{
    for character in characters {
        blueprint_character::ActiveModel {
            project_id: Set(project_id),
            name: Set(character.name.clone()),
            character_type: Set(character.character_type.clone()),
            gender: Set(character.gender.clone()),
            age: Set(character.age.clone()),
            appearance: Set(character.appearance.clone()),
            personality: Set(character.personality.clone()),
            background: Set(character.background.clone()),
            motivation: Set(character.motivation.clone()),
            arc_summary: Set(character.arc_summary.clone()),
            relationship_to_protagonist: Set(character.relationship_to_protagonist.clone()),
            arc_stages: Set(character.arc_stages.clone()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|error| ImportTextProjectError::database("导入角色失败", error))?;
    }
    Ok(())
}

pub(super) async fn insert_import_text_project_relationships<C>(
    db: &C,
    project_id: i64,
    relationships: &[ImportTextProjectRelationship],
    now: NaiveDateTime,
) -> Result<(), ImportTextProjectError>
where
    C: ConnectionTrait,
{
    for relationship in relationships {
        blueprint_relationship::ActiveModel {
            project_id: Set(project_id),
            relationship_type: Set(relationship.relationship_type.clone()),
            character_a: Set(relationship.character_a.clone()),
            character_b: Set(relationship.character_b.clone()),
            description: Set(relationship.description.clone()),
            strength: Set(relationship.strength),
            evolution: Set(relationship.evolution.clone()),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|error| ImportTextProjectError::database("导入角色关系失败", error))?;
    }
    Ok(())
}
