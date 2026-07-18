use a4n_core::{error::AppError, models::project::novel_blueprint};
use a4n_db::blueprint::{BlueprintRefinePatchInput, apply_blueprint_refine_patch_record};
use chrono::Utc;
use serde_json::json;

use crate::OrchestratorContext;

use super::super::super::{blueprint_db_error, character_profile_seed_error};
use super::contracts::BlueprintRefinementPersistenceInput;

pub(super) async fn persist_blueprint_refinement(
    context: &OrchestratorContext,
    input: BlueprintRefinementPersistenceInput,
) -> Result<novel_blueprint::Model, AppError> {
    let BlueprintRefinementPersistenceInput {
        project_id,
        blueprint,
        next_version,
        parsed,
    } = input;
    let optimization_patch = parsed.raw.clone();
    let now = Utc::now().naive_utc();
    let updated = apply_blueprint_refine_patch_record(
        context.db.connection(),
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
        context
            .replace_blueprint_characters(project_id, &characters)
            .await?;
    }
    if let Some(relationships) = parsed.relationships {
        context
            .replace_blueprint_relationships(project_id, &relationships)
            .await?;
    }
    a4n_db::character::sync_character_profiles_from_blueprint(
        context.db.connection(),
        project_id,
        now,
    )
    .await
    .map_err(character_profile_seed_error)?;
    let updated_characters = context.load_blueprint_characters(project_id).await?;
    let updated_relationships = context.load_blueprint_relationships(project_id).await?;
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
    context
        .save_blueprint_version(
            project_id,
            next_version,
            Some("优化蓝图".to_string()),
            &snapshot,
        )
        .await?;
    Ok(updated)
}
