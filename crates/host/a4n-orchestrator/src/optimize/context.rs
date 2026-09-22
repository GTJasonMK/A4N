mod manifest;
mod projection;

#[cfg(test)]
mod tests;

use a4n_db::blueprint::context::blueprint_llm_context;
use a4n_db::optimize::OptimizationContextReadInput;
use a4n_error::AppError;
use sea_orm::ConnectionTrait;
use serde_json::Value;

use super::{OptimizationAgentPermissions, optimize_db_error};

pub(super) use manifest::optimization_context_manifest;
use projection::{
    optimization_chapter_context, optimization_chapter_summary_context,
    optimization_character_profile_context, optimization_foreshadowing_context,
    optimization_outline_context, optimization_part_outline_context, optimization_project_context,
};

pub(super) async fn build_content_optimization_context<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    permissions: &OptimizationAgentPermissions,
) -> Result<Value, AppError>
where
    C: ConnectionTrait,
{
    let rows = a4n_db::optimize::optimization_context_rows(
        db,
        OptimizationContextReadInput {
            project_id,
            chapter_id,
            read_project_detail: permissions.read_project_detail,
            read_blueprint: permissions.read_blueprint,
            read_outlines: permissions.read_outlines,
            read_character_profiles: permissions.read_character_profiles,
            read_foreshadowing: permissions.read_foreshadowing,
        },
    )
    .await
    .map_err(optimize_db_error)?
    .ok_or_else(|| AppError::NotFound(format!("章节不存在: {chapter_id}")))?;

    let mut context = serde_json::Map::new();
    context.insert(
        "current_chapter".to_string(),
        optimization_chapter_context(&rows.current_chapter),
    );

    if let Some(outline) = rows.current_chapter_outline.as_ref() {
        context.insert(
            "current_chapter_outline".to_string(),
            optimization_outline_context(outline),
        );
    }

    if let Some(project) = rows.project.as_ref() {
        context.insert(
            "project_detail".to_string(),
            optimization_project_context(project),
        );
    }

    if let Some(blueprint) = rows.blueprint.as_ref() {
        context.insert(
            "blueprint".to_string(),
            blueprint_llm_context(
                &blueprint.blueprint,
                &blueprint.characters,
                &blueprint.relationships,
            ),
        );
    }

    if !rows.part_outlines.is_empty() {
        let parts = rows
            .part_outlines
            .iter()
            .map(optimization_part_outline_context)
            .collect::<Vec<_>>();
        context.insert("part_outlines".to_string(), Value::Array(parts));
    }

    if !rows.chapter_outlines.is_empty() {
        let outlines = rows
            .chapter_outlines
            .iter()
            .map(optimization_outline_context)
            .collect::<Vec<_>>();
        context.insert("chapter_outlines".to_string(), Value::Array(outlines));
    }

    if !rows.character_profiles.is_empty() {
        let profiles = rows
            .character_profiles
            .iter()
            .map(optimization_character_profile_context)
            .collect::<Vec<_>>();
        context.insert("character_profiles".to_string(), Value::Array(profiles));
    }

    if !rows.foreshadowing_index.is_empty() {
        let foreshadowing = rows
            .foreshadowing_index
            .iter()
            .map(optimization_foreshadowing_context)
            .collect::<Vec<_>>();
        context.insert(
            "foreshadowing_index".to_string(),
            Value::Array(foreshadowing),
        );
    }

    let previous = rows
        .previous_chapters
        .iter()
        .map(optimization_chapter_summary_context)
        .collect::<Vec<_>>();
    if !previous.is_empty() {
        context.insert("previous_chapters".to_string(), Value::Array(previous));
    }

    let next = rows
        .next_chapters
        .iter()
        .map(optimization_chapter_summary_context)
        .collect::<Vec<_>>();
    if !next.is_empty() {
        context.insert("next_chapters".to_string(), Value::Array(next));
    }

    Ok(Value::Object(context))
}
