use super::construction::{blueprint_relationship_seed_map, profile_seed_from_blueprint_character};
use super::contracts::CharacterProfileSeedError;
use super::persistence::{
    insert_or_update_profile_seed_with_existing, merge_profile_seed_model,
    update_profile_seed_models_batch,
};
use a4n_core::models::{
    character::protagonist_profile,
    project::{blueprint_character, blueprint_relationship},
};
use chrono::NaiveDateTime;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use std::collections::{BTreeSet, HashMap};

/// 从蓝图角色和关系播种或补齐角色档案。
pub async fn sync_character_profiles_from_blueprint<C>(
    db: &C,
    project_id: i64,
    now: NaiveDateTime,
) -> Result<(), CharacterProfileSeedError>
where
    C: ConnectionTrait,
{
    use crate::project::{blueprint_characters_for_project, blueprint_relationships_for_project};

    let characters = blueprint_characters_for_project(db, project_id).await?;
    let relationships = blueprint_relationships_for_project(db, project_id).await?;
    sync_character_profiles_from_blueprint_rows(db, project_id, &characters, &relationships, now)
        .await
}

async fn sync_character_profiles_from_blueprint_rows<C>(
    db: &C,
    project_id: i64,
    characters: &[blueprint_character::Model],
    relationships: &[blueprint_relationship::Model],
    now: NaiveDateTime,
) -> Result<(), CharacterProfileSeedError>
where
    C: ConnectionTrait,
{
    let profile_names = characters
        .iter()
        .map(|character| character.name.trim())
        .filter(|name| !name.is_empty())
        .collect::<BTreeSet<_>>();
    let mut profiles_by_name = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .filter(protagonist_profile::Column::Name.is_in(profile_names.iter().copied()))
        .all(db)
        .await?
        .into_iter()
        .map(|profile| (profile.name.clone(), profile))
        .collect::<HashMap<_, _>>();
    let relationship_map = blueprint_relationship_seed_map(characters, relationships);
    let mut pending_updates = HashMap::<i64, protagonist_profile::Model>::new();
    for character in characters {
        let name = character.name.trim();
        if name.is_empty() {
            continue;
        }
        let seed = profile_seed_from_blueprint_character(
            character,
            relationship_map
                .get(&character.name)
                .map(Vec::as_slice)
                .unwrap_or(&[]),
        );
        if let Some(existing) = profiles_by_name.get(name).cloned() {
            if let Some(profile) = merge_profile_seed_model(existing, &seed, now) {
                profiles_by_name.insert(name.to_string(), profile.clone());
                pending_updates.insert(profile.id, profile);
            }
            continue;
        }
        if let Some(profile) =
            insert_or_update_profile_seed_with_existing(db, project_id, character, &seed, now, None)
                .await?
        {
            profiles_by_name.insert(name.to_string(), profile);
        }
    }
    let pending_updates = pending_updates.into_values().collect::<Vec<_>>();
    update_profile_seed_models_batch(db, project_id, &pending_updates).await?;
    Ok(())
}
