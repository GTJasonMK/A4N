use a4n_core::{error::AppError, models::project::novel_blueprint};
use a4n_db::blueprint::{
    BlueprintRefinePatchInput, InsertBlueprintVersionInput, apply_blueprint_refine_patch_record,
    insert_blueprint_version_record,
};
use a4n_db::project::{
    blueprint_characters_for_project, blueprint_relationships_for_project,
    sync_blueprint_characters, sync_blueprint_relationships,
};
use chrono::Utc;
use sea_orm::DatabaseTransaction;
use serde_json::json;

use super::contracts::BlueprintRefinementPersistenceInput;
use crate::blueprint::persistence::{blueprint_character_drafts, blueprint_relationship_drafts};
use crate::blueprint::{blueprint_db_error, blueprint_sync_error, character_profile_seed_error};

pub(super) struct BlueprintRefinementCompletionOutcome {
    pub(super) blueprint: novel_blueprint::Model,
}

pub(super) async fn persist_blueprint_refinement_in_transaction(
    tx: &DatabaseTransaction,
    input: BlueprintRefinementPersistenceInput,
) -> Result<BlueprintRefinementCompletionOutcome, AppError> {
    let BlueprintRefinementPersistenceInput {
        project_id,
        blueprint,
        next_version,
        parsed,
    } = input;
    let optimization_patch = parsed.raw.clone();
    let now = Utc::now().naive_utc();
    let updated = apply_blueprint_refine_patch_record(
        tx,
        blueprint,
        BlueprintRefinePatchInput {
            style: parsed.style,
            full_synopsis: parsed.full_synopsis,
            world_setting: parsed.world_setting,
            current_version: next_version.clone(),
            now,
        },
    )
    .await
    .map_err(blueprint_db_error)?;
    if let Some(characters) = parsed.characters {
        sync_blueprint_characters(tx, project_id, blueprint_character_drafts(&characters), now)
            .await
            .map_err(blueprint_sync_error)?;
    }
    if let Some(relationships) = parsed.relationships {
        sync_blueprint_relationships(
            tx,
            project_id,
            blueprint_relationship_drafts(&relationships),
            now,
        )
        .await
        .map_err(blueprint_sync_error)?;
    }
    a4n_db::character::sync_character_profiles_from_blueprint(tx, project_id, now)
        .await
        .map_err(character_profile_seed_error)?;
    let updated_characters = blueprint_characters_for_project(tx, project_id).await?;
    let updated_relationships = blueprint_relationships_for_project(tx, project_id).await?;
    let snapshot = json!({
        "title": &updated.title,
        "genre": &updated.genre,
        "style": &updated.style,
        "tone": &updated.tone,
        "target_audience": &updated.target_audience,
        "one_sentence_summary": &updated.one_sentence_summary,
        "full_synopsis": &updated.full_synopsis,
        "world_setting": &updated.world_setting,
        "needs_part_outlines": updated.needs_part_outlines,
        "total_chapters": updated.total_chapters,
        "chapters_per_part": updated.chapters_per_part,
        "characters": updated_characters,
        "relationships": updated_relationships,
        "optimization_patch": optimization_patch,
    });
    insert_blueprint_version_record(
        tx,
        InsertBlueprintVersionInput {
            project_id,
            version: next_version,
            change_description: Some("优化蓝图".to_string()),
            blueprint_data: snapshot,
            now,
            created_by: Some("system".to_string()),
        },
    )
    .await
    .map_err(blueprint_db_error)?;
    Ok(BlueprintRefinementCompletionOutcome { blueprint: updated })
}
