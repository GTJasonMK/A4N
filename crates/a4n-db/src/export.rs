use a4n_core::error::AppError;
use a4n_core::models::chapter::{chapter, chapter_outline, part_outline};
use a4n_core::models::character::protagonist_profile;
use a4n_core::models::project::novel_project;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone)]
pub struct ExportChapter {
    pub id: i64,
    pub chapter_number: i32,
    pub title: Option<String>,
    pub content: String,
}

pub async fn ensure_project_exists(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<(), AppError> {
    novel_project::Entity::find_by_id(project_id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .map(|_| ())
        .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))
}

pub async fn export_txt(db: &DatabaseConnection, project_id: i64) -> Result<String, AppError> {
    let chapters = export_chapters(db, project_id).await?;
    Ok(render_txt_document(&chapters))
}

pub async fn export_markdown(
    db: &DatabaseConnection,
    project_id: i64,
    title: &str,
    author: &str,
) -> Result<String, AppError> {
    let chapters = export_chapters(db, project_id).await?;
    Ok(render_markdown_document(&chapters, title, author))
}

pub async fn export_chapters(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<Vec<ExportChapter>, AppError> {
    ensure_project_exists(db, project_id).await?;

    let chapters = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::Status.ne("deleted"))
        .order_by_asc(chapter::Column::ChapterNumber)
        .all(db)
        .await
        .map_err(AppError::Database)?;

    let exportable = chapters
        .into_iter()
        .filter_map(|chapter| {
            let content = chapter
                .content
                .trim_start_matches('\u{feff}')
                .trim()
                .to_string();
            if content.is_empty() {
                None
            } else {
                Some(ExportChapter {
                    id: chapter.id,
                    chapter_number: chapter.chapter_number,
                    title: chapter.title,
                    content,
                })
            }
        })
        .collect();

    Ok(exportable)
}

pub async fn export_outline_json(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<serde_json::Value, AppError> {
    ensure_project_exists(db, project_id).await?;

    let parts = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(part_outline::Column::PartNumber)
        .all(db)
        .await
        .map_err(AppError::Database)?;

    let chapter_outlines = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(chapter_outline::Column::ChapterNumber)
        .all(db)
        .await
        .map_err(AppError::Database)?;

    Ok(serde_json::json!({
        "project_id": project_id,
        "parts": parts,
        "chapter_outlines": chapter_outlines,
    }))
}

pub async fn export_characters_json(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<serde_json::Value, AppError> {
    ensure_project_exists(db, project_id).await?;

    let profiles = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .all(db)
        .await
        .map_err(AppError::Database)?;

    Ok(serde_json::json!({
        "project_id": project_id,
        "profiles": profiles,
    }))
}

pub fn render_txt_document(chapters: &[ExportChapter]) -> String {
    let mut output = String::new();
    for chapter in chapters {
        output.push_str("\n\n");
        output.push_str(&chapter_heading(
            chapter.chapter_number,
            chapter.title.as_deref(),
        ));
        output.push_str("\n\n");
        output.push_str(&chapter.content);
        output.push('\n');
    }
    output
}

pub fn render_markdown_document(chapters: &[ExportChapter], title: &str, author: &str) -> String {
    let mut markdown = format!("# {title}\n\n作者：{author}\n\n---\n");
    for chapter in chapters {
        markdown.push_str("\n## ");
        markdown.push_str(&chapter_heading(
            chapter.chapter_number,
            chapter.title.as_deref(),
        ));
        markdown.push_str("\n\n");
        markdown.push_str(&chapter.content);
        markdown.push('\n');
    }
    markdown
}

pub fn chapter_heading(chapter_number: i32, title: Option<&str>) -> String {
    match title.map(str::trim).filter(|title| !title.is_empty()) {
        Some(title) => format!("第{chapter_number}章 {title}"),
        None => format!("第{chapter_number}章"),
    }
}
