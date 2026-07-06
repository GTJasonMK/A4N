use a4n_core::error::AppError;
use a4n_core::models::chapter::{chapter_outline, part_outline};
use a4n_core::models::project::{
    blueprint_character, blueprint_relationship, novel_blueprint, novel_project,
};
use sea_orm::*;
use serde_json::{Value, json};

use crate::blueprint::{
    blueprint_character_llm_context, blueprint_model_llm_context,
    blueprint_relationship_llm_context,
};

use super::part_contract::resolve_part_generation_plan;

pub(super) struct ChapterOutlineGenerationContext {
    pub(super) blueprint_payload: Value,
    pub(super) parts: Vec<part_outline::Model>,
}

pub(super) struct PartOutlineGenerationContext {
    pub(super) chapters_per_part: i32,
    pub(super) requested_part_count: i32,
    pub(super) blueprint_payload: Value,
    pub(super) parts: Vec<part_outline::Model>,
}

pub(super) async fn load_part_generation_context(
    db: &DatabaseConnection,
    project_id: i64,
    requested_part_count: Option<i32>,
    requested_chapters_per_part: Option<i32>,
) -> Result<PartOutlineGenerationContext, AppError> {
    let project = novel_project::Entity::find_by_id(project_id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
    let blueprint = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project_id))
        .one(db)
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

pub(super) async fn load_chapter_outline_generation_context(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<ChapterOutlineGenerationContext, AppError> {
    let project = novel_project::Entity::find_by_id(project_id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
    let blueprint = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project_id))
        .one(db)
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

async fn load_blueprint_characters(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<Vec<blueprint_character::Model>, AppError> {
    blueprint_character::Entity::find()
        .filter(blueprint_character::Column::ProjectId.eq(project_id))
        .order_by_asc(blueprint_character::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)
}

async fn load_blueprint_relationships(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<Vec<blueprint_relationship::Model>, AppError> {
    blueprint_relationship::Entity::find()
        .filter(blueprint_relationship::Column::ProjectId.eq(project_id))
        .order_by_asc(blueprint_relationship::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)
}

async fn load_part_outlines(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<Vec<part_outline::Model>, AppError> {
    part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(part_outline::Column::PartNumber)
        .all(db)
        .await
        .map_err(AppError::Database)
}

async fn load_chapter_outlines(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<Vec<chapter_outline::Model>, AppError> {
    chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(chapter_outline::Column::ChapterNumber)
        .all(db)
        .await
        .map_err(AppError::Database)
}

pub(super) fn outline_blueprint_context_json(
    blueprint: &novel_blueprint::Model,
    characters: &[blueprint_character::Model],
    relationships: &[blueprint_relationship::Model],
    chapters_per_part: Option<i32>,
) -> Value {
    let mut value = blueprint_model_llm_context(blueprint);
    let object = value
        .as_object_mut()
        .expect("blueprint_model_llm_context returns a JSON object");
    object.remove("total_chapters");
    object.insert("chapters_per_part".to_string(), json!(chapters_per_part));
    object.insert(
        "blueprint_chapters_per_part".to_string(),
        json!(blueprint.chapters_per_part),
    );
    object.insert(
        "characters".to_string(),
        json!(
            characters
                .iter()
                .map(blueprint_character_llm_context)
                .collect::<Vec<_>>()
        ),
    );
    object.insert(
        "relationships".to_string(),
        json!(
            relationships
                .iter()
                .map(blueprint_relationship_llm_context)
                .collect::<Vec<_>>()
        ),
    );
    value
}

pub(super) fn part_context_json(part: &part_outline::Model) -> Value {
    json!({
        "part_number": part.part_number,
        "title": part.title.clone(),
        "start_chapter": part.start_chapter,
        "end_chapter": part.end_chapter,
        "summary": part.summary.clone(),
        "theme": part.theme.clone(),
        "key_events": part.key_events.clone(),
        "character_focus": part.character_focus.clone(),
    })
}

pub(super) fn chapter_outline_context_json(outline: &chapter_outline::Model) -> Value {
    json!({
        "chapter_number": outline.chapter_number,
        "title": outline.title.clone(),
        "summary": outline.summary.clone(),
        "beats": outline.beats.clone(),
        "key_scenes": outline.key_scenes.clone(),
        "foreshadowing_setup": outline.foreshadowing_setup.clone(),
        "foreshadowing_payoff": outline.foreshadowing_payoff.clone(),
        "characters_involved": outline.characters_involved.clone(),
        "pacing_score": outline.pacing_score,
        "word_count_target": outline.word_count_target,
    })
}
