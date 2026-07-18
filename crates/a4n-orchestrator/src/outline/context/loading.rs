use super::contracts::{ChapterOutlineGenerationContext, PartOutlineGenerationContext};
use super::projections::{chapter_outline_context_json, outline_blueprint_context_json};
use crate::outline::part_contract::resolve_part_generation_plan;
use a4n_core::error::AppError;
use a4n_core::models::chapter::{chapter_outline, part_outline};
use a4n_core::models::project::{blueprint_character, blueprint_relationship};
use a4n_db::DatabaseConnectionTrait as ConnectionTrait;
use a4n_db::project as project_db;
use serde_json::json;

pub(in crate::outline) async fn load_part_generation_context<C>(
    db: &C,
    project_id: i64,
    requested_part_count: Option<i32>,
    requested_chapters_per_part: Option<i32>,
) -> Result<PartOutlineGenerationContext, AppError>
where
    C: ConnectionTrait,
{
    let project = project_db::project_by_id_record(db, project_id)
        .await
        .map_err(crate::project::project_mutation_error_to_app_error)?
        .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
    let blueprint = project_db::novel_blueprint_for_project(db, project_id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("项目 {project_id} 缺少蓝图，无法生成分部大纲"))
        })?;
    let plan = resolve_part_generation_plan(
        requested_part_count,
        requested_chapters_per_part,
        project.chapters_per_part,
        blueprint.chapters_per_part,
    )?;

    let characters = load_blueprint_characters(db, project_id).await?;
    let relationships = load_blueprint_relationships(db, project_id).await?;
    let parts = load_part_outlines(db, project_id).await?;
    let blueprint_payload = outline_blueprint_context_json(
        &blueprint,
        &characters,
        &relationships,
        Some(plan.chapters_per_part),
    );

    Ok(PartOutlineGenerationContext {
        chapters_per_part: plan.chapters_per_part,
        requested_part_count: plan.requested_part_count,
        blueprint_payload,
        parts,
    })
}

pub(in crate::outline) async fn load_chapter_outline_generation_context<C>(
    db: &C,
    project_id: i64,
) -> Result<ChapterOutlineGenerationContext, AppError>
where
    C: ConnectionTrait,
{
    let project = project_db::project_by_id_record(db, project_id)
        .await
        .map_err(crate::project::project_mutation_error_to_app_error)?
        .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
    let blueprint = project_db::novel_blueprint_for_project(db, project_id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("项目 {project_id} 缺少蓝图，无法生成章节大纲"))
        })?;
    let characters = load_blueprint_characters(db, project_id).await?;
    let relationships = load_blueprint_relationships(db, project_id).await?;
    let existing_outlines = load_chapter_outlines(db, project_id).await?;
    let parts = load_part_outlines(db, project_id).await?;
    let mut blueprint_payload = outline_blueprint_context_json(
        &blueprint,
        &characters,
        &relationships,
        project.chapters_per_part.or(blueprint.chapters_per_part),
    );
    blueprint_payload
        .as_object_mut()
        .expect("outline_blueprint_context_json returns a JSON object")
        .insert(
            "chapter_outline".to_string(),
            json!(
                existing_outlines
                    .iter()
                    .map(chapter_outline_context_json)
                    .collect::<Vec<_>>()
            ),
        );

    Ok(ChapterOutlineGenerationContext {
        blueprint_payload,
        parts,
    })
}

async fn load_blueprint_characters<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<blueprint_character::Model>, AppError>
where
    C: ConnectionTrait,
{
    project_db::blueprint_characters_for_project(db, project_id)
        .await
        .map_err(AppError::Database)
}

async fn load_blueprint_relationships<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<blueprint_relationship::Model>, AppError>
where
    C: ConnectionTrait,
{
    project_db::blueprint_relationships_for_project(db, project_id)
        .await
        .map_err(AppError::Database)
}

async fn load_part_outlines<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<part_outline::Model>, AppError>
where
    C: ConnectionTrait,
{
    project_db::part_outlines_for_project(db, project_id)
        .await
        .map_err(crate::project::project_outline_record_error_to_app_error)
}

async fn load_chapter_outlines<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<chapter_outline::Model>, AppError>
where
    C: ConnectionTrait,
{
    project_db::chapter_outlines_for_project(db, project_id)
        .await
        .map_err(crate::project::project_outline_record_error_to_app_error)
}
