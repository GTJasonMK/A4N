use a4n_core::models::project::{
    blueprint_character, blueprint_relationship, blueprint_version, novel_blueprint,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};

pub(super) struct BlueprintGraph {
    blueprint: novel_blueprint::Model,
    versions: Vec<blueprint_version::Model>,
    characters: Vec<blueprint_character::Model>,
    relationships: Vec<blueprint_relationship::Model>,
}

pub(super) async fn load_blueprint_graph<C>(
    db: &C,
    project_id: i64,
) -> Result<Option<BlueprintGraph>, String>
where
    C: ConnectionTrait,
{
    let Some(blueprint) = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project_id))
        .one(db)
        .await
        .map_err(|e| e.to_string())?
    else {
        return Ok(None);
    };
    let versions = blueprint_version::Entity::find()
        .filter(blueprint_version::Column::ProjectId.eq(project_id))
        .order_by_asc(blueprint_version::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    let characters = blueprint_character::Entity::find()
        .filter(blueprint_character::Column::ProjectId.eq(project_id))
        .order_by_asc(blueprint_character::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    let relationships = blueprint_relationship::Entity::find()
        .filter(blueprint_relationship::Column::ProjectId.eq(project_id))
        .order_by_asc(blueprint_relationship::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(Some(BlueprintGraph {
        blueprint,
        versions,
        characters,
        relationships,
    }))
}

pub(super) async fn copy_blueprint_graph<C>(
    db: &C,
    target_project_id: i64,
    graph: &BlueprintGraph,
    now: chrono::NaiveDateTime,
) -> Result<(), String>
where
    C: ConnectionTrait,
{
    let bp = &graph.blueprint;
    novel_blueprint::ActiveModel {
        project_id: Set(target_project_id),
        title: Set(bp.title.clone()),
        genre: Set(bp.genre.clone()),
        style: Set(bp.style.clone()),
        tone: Set(bp.tone.clone()),
        target_audience: Set(bp.target_audience.clone()),
        one_sentence_summary: Set(bp.one_sentence_summary.clone()),
        full_synopsis: Set(bp.full_synopsis.clone()),
        world_setting: Set(bp.world_setting.clone()),
        current_version: Set(bp.current_version.clone()),
        needs_part_outlines: Set(bp.needs_part_outlines),
        total_chapters: Set(bp.total_chapters),
        chapters_per_part: Set(bp.chapters_per_part),
        avatar_svg: Set(bp.avatar_svg.clone()),
        avatar_animal: Set(bp.avatar_animal.clone()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(|e| e.to_string())?;

    for version in &graph.versions {
        blueprint_version::ActiveModel {
            project_id: Set(target_project_id),
            version: Set(version.version.clone()),
            change_description: Set(version.change_description.clone()),
            blueprint_data: Set(version.blueprint_data.clone()),
            world_setting_snapshot: Set(version.world_setting_snapshot.clone()),
            characters_snapshot: Set(version.characters_snapshot.clone()),
            relationships_snapshot: Set(version.relationships_snapshot.clone()),
            synopsis_snapshot: Set(version.synopsis_snapshot.clone()),
            created_at: Set(version.created_at),
            created_by: Set(version.created_by.clone()),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|e| e.to_string())?;
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
        .map_err(|e| e.to_string())?;
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
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub(super) fn blueprint_graph_raw_blueprint(graph: &BlueprintGraph) -> Result<String, String> {
    serde_json::to_string_pretty(&serde_json::json!({
        "blueprint": a4n_services::blueprint::blueprint_model_llm_context(&graph.blueprint),
        "characters": graph
            .characters
            .iter()
            .map(a4n_services::blueprint::blueprint_character_llm_context)
            .collect::<Vec<_>>(),
        "relationships": graph
            .relationships
            .iter()
            .map(a4n_services::blueprint::blueprint_relationship_llm_context)
            .collect::<Vec<_>>(),
    }))
    .map_err(|e| e.to_string())
}
