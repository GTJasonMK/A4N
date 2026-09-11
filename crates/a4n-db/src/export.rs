use a4n_db_models::models::chapter::{chapter, chapter_outline, part_outline};
use a4n_db_models::models::character::protagonist_profile;
use a4n_db_models::models::project::novel_project;
use a4n_error::AppError;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect};

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub struct ExportChapter {
    pub chapter_number: i32,
    pub title: Option<String>,
    pub content: String,
}

#[derive(Debug)]
pub struct ExportDocument {
    pub title: String,
    pub chapters: Vec<ExportChapter>,
}

async fn export_project_title(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<String, AppError> {
    novel_project::Entity::find_by_id(project_id)
        .select_only()
        .column(novel_project::Column::Title)
        .into_tuple::<String>()
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))
}

async fn ensure_export_project(db: &DatabaseConnection, project_id: i64) -> Result<(), AppError> {
    novel_project::Entity::find_by_id(project_id)
        .select_only()
        .column(novel_project::Column::Id)
        .into_tuple::<i64>()
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
    Ok(())
}

async fn export_chapters(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<Vec<(i32, Option<String>, String)>, AppError> {
    chapter::Entity::find()
        .select_only()
        .column(chapter::Column::ChapterNumber)
        .column(chapter::Column::Title)
        .column(chapter::Column::Content)
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::Status.ne("deleted"))
        .order_by_asc(chapter::Column::ChapterNumber)
        .into_tuple::<(i32, Option<String>, String)>()
        .all(db)
        .await
        .map_err(AppError::Database)
}

async fn export_parts(
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

async fn export_chapter_outlines(
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

pub async fn export_document(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<ExportDocument, AppError> {
    let (title_result, chapters_result) = futures::join!(
        export_project_title(db, project_id),
        export_chapters(db, project_id),
    );
    let title = title_result?;
    let chapters = chapters_result?;

    let exportable = chapters
        .into_iter()
        .filter_map(|(chapter_number, title, raw_content)| {
            let content = raw_content
                .trim_start_matches('\u{feff}')
                .trim()
                .to_string();
            if content.is_empty() {
                None
            } else {
                Some(ExportChapter {
                    chapter_number,
                    title,
                    content,
                })
            }
        })
        .collect();

    Ok(ExportDocument {
        title,
        chapters: exportable,
    })
}

pub async fn export_outline_json(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<serde_json::Value, AppError> {
    ensure_export_project(db, project_id).await?;
    let (parts_result, chapter_outlines_result) = futures::join!(
        export_parts(db, project_id),
        export_chapter_outlines(db, project_id),
    );
    let parts = parts_result?;
    let chapter_outlines = chapter_outlines_result?;

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
    ensure_export_project(db, project_id).await?;

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
