use super::*;
use a4n_core::models::project::novel_project;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, Set};

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
        export_chapters(&db, missing_project_id).await.map(|_| ()),
        export_txt(&db, missing_project_id).await.map(|_| ()),
        export_markdown(&db, missing_project_id, "测试书名", "测试作者")
            .await
            .map(|_| ()),
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

    assert_eq!(export_chapters(&db, project.id).await.unwrap().len(), 0);
    assert_eq!(export_txt(&db, project.id).await.unwrap(), "");

    let markdown = export_markdown(&db, project.id, "测试书名", "测试作者")
        .await
        .unwrap();
    assert_eq!(markdown, "# 测试书名\n\n作者：测试作者\n\n---\n");

    let outline = export_outline_json(&db, project.id).await.unwrap();
    assert_eq!(outline["project_id"], project.id);
    assert_eq!(outline["parts"].as_array().unwrap().len(), 0);
    assert_eq!(outline["chapter_outlines"].as_array().unwrap().len(), 0);

    let characters = export_characters_json(&db, project.id).await.unwrap();
    assert_eq!(characters["project_id"], project.id);
    assert_eq!(characters["profiles"].as_array().unwrap().len(), 0);
}
