use super::contracts::{ChapterOutlineGenerationContext, PartOutlineGenerationContext};
use super::projections::{
    chapter_outline_context_json, has_outline_generation_source, outline_blueprint_context_json,
    outline_formal_materials_context_json,
};
use crate::outline::part_contract::{
    infer_chapters_per_part_from_existing, resolve_part_generation_plan,
};
use a4n_db::outline::{
    OutlineGenerationSourceRows, load_outline_generation_source_rows,
    outline_generation_source_snapshot,
};
use a4n_error::AppError;
use sea_orm::{ConnectionTrait, DatabaseConnection, DatabaseTransaction};
use serde_json::{Value, json};

pub(in crate::outline) async fn load_part_generation_context<C>(
    db: &C,
    project_id: i64,
    requested_part_count: Option<i32>,
    requested_chapters_per_part: Option<i32>,
) -> Result<PartOutlineGenerationContext, AppError>
where
    C: ConnectionTrait,
{
    let rows = load_outline_generation_source_rows(db, project_id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
    let formal_materials = outline_formal_materials_context_json(
        &rows.character_profiles,
        &rows.objects,
        &rows.scenes,
        &rows.foreshadowing,
    );
    if !has_outline_generation_source(
        rows.blueprint.is_some(),
        &rows.project.description,
        &formal_materials,
        !rows.parts.is_empty() || !rows.chapter_outlines.is_empty(),
    ) {
        return Err(AppError::InvalidArgument(format!(
            "项目 {project_id} 缺少可用于生成分部大纲的正式资料；请先录入项目简介、角色、场景、物品、伏笔或章节大纲，或建立正式蓝图"
        )));
    }
    let existing_chapters_per_part = infer_chapters_per_part_from_existing(&rows.parts)?;
    let plan = resolve_part_generation_plan(
        requested_part_count,
        requested_chapters_per_part.or(existing_chapters_per_part),
        rows.project.chapters_per_part,
        rows.blueprint
            .as_ref()
            .and_then(|blueprint| blueprint.chapters_per_part),
    )?;
    let blueprint_payload =
        outline_blueprint_payload(&rows, formal_materials, Some(plan.chapters_per_part));
    let source_snapshot = outline_generation_source_snapshot(&rows);

    Ok(PartOutlineGenerationContext {
        chapters_per_part: plan.chapters_per_part,
        requested_part_count: plan.requested_part_count,
        blueprint_payload,
        parts: rows.parts,
        source_snapshot,
    })
}

pub(in crate::outline) async fn load_chapter_outline_generation_context<C>(
    db: &C,
    project_id: i64,
) -> Result<ChapterOutlineGenerationContext, AppError>
where
    C: ConnectionTrait,
{
    let rows = load_outline_generation_source_rows(db, project_id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
    let formal_materials = outline_formal_materials_context_json(
        &rows.character_profiles,
        &rows.objects,
        &rows.scenes,
        &rows.foreshadowing,
    );
    if !has_outline_generation_source(
        rows.blueprint.is_some(),
        &rows.project.description,
        &formal_materials,
        !rows.parts.is_empty() || !rows.chapter_outlines.is_empty(),
    ) {
        return Err(AppError::InvalidArgument(format!(
            "项目 {project_id} 缺少可用于生成章节大纲的正式资料；请先录入项目简介、角色、场景、物品或伏笔，或建立正式蓝图"
        )));
    }
    let chapters_per_part = rows.project.chapters_per_part.or_else(|| {
        rows.blueprint
            .as_ref()
            .and_then(|blueprint| blueprint.chapters_per_part)
    });
    let blueprint_payload = outline_blueprint_payload(&rows, formal_materials, chapters_per_part);
    let source_snapshot = outline_generation_source_snapshot(&rows);

    Ok(ChapterOutlineGenerationContext {
        blueprint_payload,
        parts: rows.parts,
        chapter_outlines: rows.chapter_outlines,
        source_snapshot,
    })
}

pub(in crate::outline) async fn lock_outline_generation_sources(
    db: &DatabaseConnection,
    project_id: i64,
    expected_source_snapshot: &Value,
) -> Result<DatabaseTransaction, AppError> {
    a4n_db::outline::lock_outline_generation_sources(db, project_id, expected_source_snapshot)
        .await
        .map_err(crate::outline::outline_db_error)
}

fn outline_blueprint_payload(
    rows: &OutlineGenerationSourceRows,
    formal_materials: Value,
    chapters_per_part: Option<i32>,
) -> Value {
    let (characters, relationships) = if rows.blueprint.is_some() {
        (
            rows.blueprint_characters.as_slice(),
            rows.blueprint_relationships.as_slice(),
        )
    } else {
        (&[][..], &[][..])
    };
    let mut payload = outline_blueprint_context_json(
        &rows.project,
        rows.blueprint.as_ref(),
        characters,
        relationships,
        formal_materials,
        chapters_per_part,
    );
    payload
        .as_object_mut()
        .expect("outline_blueprint_context_json returns a JSON object")
        .insert(
            "chapter_outline".to_string(),
            json!(
                rows.chapter_outlines
                    .iter()
                    .map(chapter_outline_context_json)
                    .collect::<Vec<_>>()
            ),
        );
    payload
}
