use super::*;
use a4n_db_models::models::project::novel_project;
use a4n_document_export_core::{chapter_heading, render_markdown_document, render_txt_document};

async fn setup_db() -> DatabaseConnection {
    a4n_migration::test_support::connect_test_database().await
}

async fn insert_project(db: &DatabaseConnection) -> novel_project::Model {
    crate::project::create_project_record(
        db,
        crate::project::CreateProjectRecordInput {
            user_id: 1,
            title: "导出测试项目".to_string(),
            description: "验证导出边界行为".to_string(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .unwrap()
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
    let db = setup_db().await;
    let missing_project_id = 404;

    for result in [
        export_document(&db, missing_project_id).await.map(|_| ()),
        export_outline_json(&db, missing_project_id)
            .await
            .map(|_| ()),
        export_characters_json(&db, missing_project_id)
            .await
            .map(|_| ()),
    ] {
        assert!(matches!(
            result,
            Err(AppError::NotFound(message)) if message == "项目不存在: 404"
        ));
    }
}

#[tokio::test]
async fn export_existing_empty_project_returns_empty_exports() {
    let db = setup_db().await;
    let project = insert_project(&db).await;

    let document = export_document(&db, project.id).await.unwrap();
    assert_eq!(document.title, project.title);
    assert!(document.chapters.is_empty());
    assert_eq!(render_txt_document(&document.chapters), "");

    let markdown = render_markdown_document(&document.chapters, "测试书名", "测试作者");
    assert_eq!(markdown, "# 测试书名\n\n作者：测试作者\n\n---\n");

    let outline = export_outline_json(&db, project.id).await.unwrap();
    assert_eq!(outline["project_id"], project.id);
    assert_eq!(outline["parts"].as_array().unwrap().len(), 0);
    assert_eq!(outline["chapter_outlines"].as_array().unwrap().len(), 0);

    let characters = export_characters_json(&db, project.id).await.unwrap();
    assert_eq!(characters["project_id"], project.id);
    assert_eq!(characters["profiles"].as_array().unwrap().len(), 0);
}
