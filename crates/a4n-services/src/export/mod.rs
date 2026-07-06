//! 导出服务：将项目数据导出为多种格式
//!
//! 对应文档 19-数据导出系统

pub mod epub;

use std::sync::Arc;

use sea_orm::*;

use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter;
use a4n_core::models::project::novel_project;

#[derive(Debug, Clone)]
pub struct ExportChapter {
    pub id: i64,
    pub chapter_number: i32,
    pub title: Option<String>,
    pub content: String,
}

#[derive(Clone)]
pub struct ExportService {
    db: Arc<DatabaseConnection>,
}

impl ExportService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    async fn ensure_project_exists(&self, project_id: i64) -> Result<(), AppError> {
        novel_project::Entity::find_by_id(project_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .map(|_| ())
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))
    }

    /// 导出章节内容为纯文本
    pub async fn export_txt(&self, project_id: i64) -> Result<String, AppError> {
        let chapters = self.export_chapters(project_id).await?;
        Ok(render_txt_document(&chapters))
    }

    /// 导出章节内容为 Markdown
    pub async fn export_markdown(
        &self,
        project_id: i64,
        title: &str,
        author: &str,
    ) -> Result<String, AppError> {
        let chapters = self.export_chapters(project_id).await?;
        Ok(render_markdown_document(&chapters, title, author))
    }

    /// 获取所有可导出的章节。chapter.content 是正文真源。
    pub async fn export_chapters(&self, project_id: i64) -> Result<Vec<ExportChapter>, AppError> {
        self.ensure_project_exists(project_id).await?;

        let chapters = chapter::Entity::find()
            .filter(chapter::Column::ProjectId.eq(project_id))
            .filter(chapter::Column::Status.ne("deleted"))
            .order_by_asc(chapter::Column::ChapterNumber)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;

        let exportable = chapters
            .into_iter()
            .filter_map(|ch| {
                let content = ch.content.trim_start_matches('\u{feff}').trim().to_string();
                if content.is_empty() {
                    None
                } else {
                    Some(ExportChapter {
                        id: ch.id,
                        chapter_number: ch.chapter_number,
                        title: ch.title,
                        content,
                    })
                }
            })
            .collect();

        Ok(exportable)
    }

    /// 导出大纲为 JSON
    pub async fn export_outline_json(
        &self,
        project_id: i64,
    ) -> Result<serde_json::Value, AppError> {
        self.ensure_project_exists(project_id).await?;

        let parts = a4n_core::models::chapter::part_outline::Entity::find()
            .filter(a4n_core::models::chapter::part_outline::Column::ProjectId.eq(project_id))
            .order_by_asc(a4n_core::models::chapter::part_outline::Column::PartNumber)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;

        let chapter_outlines = a4n_core::models::chapter::chapter_outline::Entity::find()
            .filter(a4n_core::models::chapter::chapter_outline::Column::ProjectId.eq(project_id))
            .order_by_asc(a4n_core::models::chapter::chapter_outline::Column::ChapterNumber)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;

        Ok(serde_json::json!({
            "project_id": project_id,
            "parts": parts,
            "chapter_outlines": chapter_outlines,
        }))
    }

    /// 导出角色数据为 JSON
    pub async fn export_characters_json(
        &self,
        project_id: i64,
    ) -> Result<serde_json::Value, AppError> {
        self.ensure_project_exists(project_id).await?;

        let profiles = a4n_core::models::character::protagonist_profile::Entity::find()
            .filter(
                a4n_core::models::character::protagonist_profile::Column::ProjectId.eq(project_id),
            )
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;

        Ok(serde_json::json!({
            "project_id": project_id,
            "profiles": profiles,
        }))
    }
}

fn render_txt_document(chapters: &[ExportChapter]) -> String {
    let mut output = String::new();
    for ch in chapters {
        output.push_str("\n\n");
        output.push_str(&chapter_heading(ch.chapter_number, ch.title.as_deref()));
        output.push_str("\n\n");
        output.push_str(&ch.content);
        output.push('\n');
    }
    output
}

fn render_markdown_document(chapters: &[ExportChapter], title: &str, author: &str) -> String {
    let mut md = format!("# {title}\n\n作者：{author}\n\n---\n");
    for ch in chapters {
        md.push_str("\n## ");
        md.push_str(&chapter_heading(ch.chapter_number, ch.title.as_deref()));
        md.push_str("\n\n");
        md.push_str(&ch.content);
        md.push('\n');
    }
    md
}

pub fn chapter_heading(chapter_number: i32, title: Option<&str>) -> String {
    match title.map(str::trim).filter(|title| !title.is_empty()) {
        Some(title) => format!("第{chapter_number}章 {title}"),
        None => format!("第{chapter_number}章"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sea_orm::{DatabaseConnection, Set};
    use std::sync::Arc;

    async fn setup_db() -> DatabaseConnection {
        a4n_migration::test_support::connect_test_database().await
    }

    async fn insert_project(db: &DatabaseConnection) -> novel_project::Model {
        let now = Utc::now().naive_utc();
        novel_project::ActiveModel {
            user_id: Set(1),
            title: Set("导出测试项目".to_string()),
            description: Set("验证导出边界行为".to_string()),
            project_type: Set("novel_long".to_string()),
            status: Set("writing".to_string()),
            target_chapters: Set(10),
            current_chapter: Set(0),
            total_chapters: Set(Some(10)),
            chapters_per_part: Set(Some(5)),
            avatar_svg: Set(None),
            avatar_animal: Set(None),
            raw_blueprint: Set(None),
            import_analysis_progress: Set(None),
            export_config: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap()
    }

    fn service(db: DatabaseConnection) -> ExportService {
        ExportService::new(Arc::new(db))
    }

    #[test]
    fn chapter_heading_omits_missing_title_without_padding() {
        assert_eq!(chapter_heading(7, None), "第7章");
    }

    #[test]
    fn chapter_heading_omits_blank_title_without_padding() {
        assert_eq!(chapter_heading(8, Some("  \t ")), "第8章");
    }

    #[test]
    fn chapter_heading_includes_explicit_title() {
        assert_eq!(chapter_heading(9, Some(" 风起 ")), "第9章 风起");
    }

    #[tokio::test]
    async fn export_rejects_missing_project_instead_of_returning_empty_results() {
        let service = service(setup_db().await);
        let missing_project_id = 404;

        for result in [
            service
                .export_chapters(missing_project_id)
                .await
                .map(|_| ()),
            service.export_txt(missing_project_id).await.map(|_| ()),
            service
                .export_markdown(missing_project_id, "测试书名", "测试作者")
                .await
                .map(|_| ()),
            service
                .export_outline_json(missing_project_id)
                .await
                .map(|_| ()),
            service
                .export_characters_json(missing_project_id)
                .await
                .map(|_| ()),
        ] {
            assert!(
                matches!(result, Err(AppError::NotFound(message)) if message == "项目不存在: 404")
            );
        }
    }

    #[tokio::test]
    async fn export_existing_empty_project_returns_empty_exports() {
        let db = setup_db().await;
        let project = insert_project(&db).await;
        let service = service(db);

        assert_eq!(service.export_chapters(project.id).await.unwrap().len(), 0);
        assert_eq!(service.export_txt(project.id).await.unwrap(), "");

        let markdown = service
            .export_markdown(project.id, "测试书名", "测试作者")
            .await
            .unwrap();
        assert_eq!(markdown, "# 测试书名\n\n作者：测试作者\n\n---\n");

        let outline = service.export_outline_json(project.id).await.unwrap();
        assert_eq!(outline["project_id"], project.id);
        assert_eq!(outline["parts"].as_array().unwrap().len(), 0);
        assert_eq!(outline["chapter_outlines"].as_array().unwrap().len(), 0);

        let characters = service.export_characters_json(project.id).await.unwrap();
        assert_eq!(characters["project_id"], project.id);
        assert_eq!(characters["profiles"].as_array().unwrap().len(), 0);
    }
}
