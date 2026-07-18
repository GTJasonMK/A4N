use super::construction::{blueprint_relationship_seed_map, profile_seed_from_blueprint_character};
use super::contracts::CharacterProfileSeedError;
use super::persistence::insert_or_update_profile_seed;
use a4n_core::models::project::{blueprint_character, blueprint_relationship};
use chrono::NaiveDateTime;
use sea_orm::ConnectionTrait;

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
    let relationship_map = blueprint_relationship_seed_map(characters, relationships);
    for character in characters {
        if character.name.trim().is_empty() {
            continue;
        }
        let seed = profile_seed_from_blueprint_character(
            character,
            relationship_map
                .get(&character.name)
                .map(Vec::as_slice)
                .unwrap_or(&[]),
        );
        insert_or_update_profile_seed(db, project_id, character, &seed, now).await?;
    }
    Ok(())
}
