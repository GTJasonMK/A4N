mod blueprint;
mod chapter_entries;
mod extraction;
#[cfg(test)]
mod tests;

use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};
use serde_json::{Value, json};

use self::blueprint::blueprint_context;
use self::chapter_entries::{chapter_outline_entry, chapter_summary_for_reverse};
use super::types::{ReverseContentRequest, ReverseContextKind, required_reverse_context_limit};

use a4n_core::models::{
    chapter::{chapter, chapter_outline, part_outline},
    project::novel_project,
};

pub async fn resolve_reverse_content_sample(
    db: &impl ConnectionTrait,
    input: ReverseContentRequest<'_>,
    kind: ReverseContextKind,
) -> Result<String, String> {
    let manual = input.manual_sample.trim();
    if !input.auto_context {
        if manual.is_empty() {
            return Err("请提供反推样本，或启用项目上下文".to_string());
        }
        return Ok(manual.to_string());
    }

    let limit = required_reverse_context_limit(input.context_limit)?;
    let mut context = project_reverse_context(db, input.project_id, kind, limit).await?;
    if !manual.is_empty() {
        if let Some(obj) = context.as_object_mut() {
            obj.insert("user_supplied_sample".to_string(), json!(manual));
        } else {
            context = json!({
                "auto_context": context,
                "user_supplied_sample": manual,
            });
        }
    }
    serde_json::to_string_pretty(&context).map_err(|e| e.to_string())
}

async fn project_reverse_context(
    db: &impl ConnectionTrait,
    project_id: i64,
    kind: ReverseContextKind,
    limit: usize,
) -> Result<Value, String> {
    let project = novel_project::Entity::find_by_id(project_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("项目不存在: project_id={project_id}"))?;
    let chapters = importable_chapters(db, project_id).await?;
    if chapters.is_empty() {
        return Err("项目没有可用于反推的章节正文".to_string());
    }
    let outlines = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(chapter_outline::Column::ChapterNumber)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    let parts = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(part_outline::Column::PartNumber)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;

    match kind {
        ReverseContextKind::Outline => Ok(json!({
            "novel_title": project.title,
            "total_chapters": chapters.len(),
            "summaries": chapters
                .iter()
                .take(limit)
                .map(chapter_summary_for_reverse)
                .collect::<Vec<_>>(),
        })),
        ReverseContextKind::PartOutline => {
            let outline_values = if outlines.is_empty() {
                chapters
                    .iter()
                    .take(limit)
                    .map(chapter_outline_entry)
                    .collect::<Vec<_>>()
            } else {
                outlines
                    .iter()
                    .take(limit)
                    .map(|outline| {
                        json!({
                            "chapter_number": outline.chapter_number,
                            "title": outline.title,
                            "summary": outline.summary,
                            "key_scenes": outline.key_scenes,
                            "characters_involved": outline.characters_involved,
                        })
                    })
                    .collect::<Vec<_>>()
            };
            Ok(json!({
                "novel_title": project.title,
                "total_chapters": chapters.len(),
                "chapter_outlines": outline_values,
            }))
        }
        ReverseContextKind::Blueprint => {
            blueprint_context(db, project, chapters, parts, limit).await
        }
    }
}

async fn importable_chapters(
    db: &impl ConnectionTrait,
    project_id: i64,
) -> Result<Vec<chapter::Model>, String> {
    chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::Status.ne("deleted"))
        .order_by_asc(chapter::Column::ChapterNumber)
        .all(db)
        .await
        .map_err(|e| e.to_string())
        .map(|chapters| {
            chapters
                .into_iter()
                .filter(|ch| !ch.content.trim().is_empty())
                .collect()
        })
}
