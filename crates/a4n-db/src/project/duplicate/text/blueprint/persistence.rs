use a4n_core::models::project::{
    blueprint_character, blueprint_relationship, blueprint_version, novel_blueprint,
};
use chrono::NaiveDateTime;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};

use super::ProjectBlueprintGraph;
use crate::project::duplicate::ProjectDuplicateError;

pub(in crate::project::duplicate::text) async fn copy_project_blueprint_graph<C>(
    db: &C,
    target_project_id: i64,
    graph: &ProjectBlueprintGraph,
    now: NaiveDateTime,
) -> Result<(), ProjectDuplicateError>
where
    C: ConnectionTrait,
{
    let blueprint = &graph.blueprint;
    novel_blueprint::ActiveModel {
        project_id: Set(target_project_id),
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
    .map_err(ProjectDuplicateError::Database)?;

    for version in &graph.versions {
        blueprint_version::ActiveModel {
            project_id: Set(target_project_id),
            version: Set(version.version.clone()),
            change_description: Set(version.change_description.clone()),
            blueprint_data: Set(version.blueprint_data.clone()),
            created_at: Set(version.created_at),
            created_by: Set(version.created_by.clone()),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    }

    for character in &graph.characters {
        blueprint_character::ActiveModel {
            project_id: Set(target_project_id),
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
        .map_err(ProjectDuplicateError::Database)?;
    }

    for relationship in &graph.relationships {
        blueprint_relationship::ActiveModel {
            project_id: Set(target_project_id),
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
        .map_err(ProjectDuplicateError::Database)?;
    }

    Ok(())
}
