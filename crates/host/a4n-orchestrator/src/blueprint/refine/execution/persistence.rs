use a4n_db::blueprint::{
    BlueprintRefinePatchInput, InsertBlueprintVersionInput, SyncProjectAfterBlueprintInput,
    apply_blueprint_refine_patch_record, blueprint_version_snapshot,
    insert_blueprint_version_record, next_blueprint_version_record,
    sync_project_after_blueprint_record,
};
use a4n_db::inspiration::INSPIRATION_HISTORY_LIMIT;
use a4n_db::project::{
    blueprint_characters_for_project, blueprint_relationships_for_project,
    merge_blueprint_characters, merge_blueprint_relationships,
};
use a4n_db_models::models::project::novel_blueprint;
use a4n_domain::project::ProjectStatus;
use a4n_error::AppError;
use chrono::Utc;
use sea_orm::DatabaseTransaction;
use serde_json::Value;

use super::contracts::blueprint_refine_source_snapshot;
use crate::blueprint::persistence::{blueprint_character_drafts, blueprint_relationship_drafts};
use crate::blueprint::{blueprint_db_error, blueprint_sync_error, character_profile_seed_error};
use crate::context::inspiration_db_error_to_app;

pub(super) async fn persist_blueprint_refinement_in_transaction(
    tx: &DatabaseTransaction,
    project_id: i64,
    expected_source_snapshot: Value,
    parsed: a4n_domain::project::ParsedBlueprintOptimization,
) -> Result<novel_blueprint::Model, AppError> {
    let locked_sources =
        a4n_db::outline::load_outline_generation_source_rows_for_update(tx, project_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
    if locked_sources.project.lifecycle_gate != a4n_db::generation::PROJECT_GATE_ACTIVE {
        return Err(AppError::InvalidState {
            current: locked_sources.project.lifecycle_gate.clone(),
            operation: "保存蓝图优化结果".to_string(),
        });
    }
    let inspiration_history = a4n_db::inspiration::inspiration_messages_for_connection(
        tx,
        project_id,
        INSPIRATION_HISTORY_LIMIT,
    )
    .await
    .map_err(inspiration_db_error_to_app)?;
    if blueprint_refine_source_snapshot(&locked_sources, &inspiration_history)
        != expected_source_snapshot
    {
        return Err(AppError::InvalidState {
            current: "blueprint_refinement_source_changed".to_string(),
            operation: "保存蓝图优化结果（项目正式资料已变化，请重新优化）".to_string(),
        });
    }
    let blueprint = locked_sources
        .blueprint
        .clone()
        .ok_or_else(|| AppError::NotFound("蓝图不存在".to_string()))?;
    let next_version =
        next_blueprint_version_record(tx, project_id, Some(blueprint.current_version.as_str()))
            .await
            .map_err(blueprint_db_error)?;
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
        merge_blueprint_characters(tx, project_id, blueprint_character_drafts(&characters), now)
            .await
            .map_err(blueprint_sync_error)?;
    }
    if let Some(relationships) = parsed.relationships {
        merge_blueprint_relationships(
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
    let snapshot =
        blueprint_version_snapshot(&updated, &updated_characters, &updated_relationships);
    insert_blueprint_version_record(
        tx,
        InsertBlueprintVersionInput {
            project_id,
            version: next_version,
            change_description: "优化蓝图".to_string(),
            blueprint_data: snapshot.clone(),
            now,
            created_by: "system".to_string(),
        },
    )
    .await
    .map_err(blueprint_db_error)?;
    let project_status = ProjectStatus::parse_storage(&locked_sources.project.status)?;
    let total_chapters = updated.total_chapters.ok_or_else(|| {
        AppError::InvalidArgument("蓝图缺少 total_chapters，无法同步项目".to_string())
    })?;
    sync_project_after_blueprint_record(
        tx,
        SyncProjectAfterBlueprintInput {
            project_id,
            title: updated.title.clone(),
            status: project_status,
            target_chapters: total_chapters,
            total_chapters: Some(total_chapters),
            chapters_per_part: updated.chapters_per_part,
            raw_blueprint: serde_json::to_string_pretty(&snapshot)?,
            now,
        },
    )
    .await
    .map_err(blueprint_db_error)?;
    Ok(updated)
}
