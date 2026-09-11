use a4n_db_models::models::chapter::chapter::{self, STATUS_DRAFT};
use a4n_db_models::models::project::novel_project;
use sea_orm::*;

use crate::project::{PROJECT_KIND_TEXT, SOURCE_SYNC_CURRENT};

pub(super) async fn setup_db() -> DatabaseConnection {
    a4n_migration::test_support::connect_test_database().await
}

fn fixed_time() -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, 6, 2)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap()
}

pub(super) async fn insert_project(db: &DatabaseConnection) -> novel_project::Model {
    let now = fixed_time();
    novel_project::ActiveModel {
        user_id: Set(1),
        title: Set("章节清理测试".to_string()),
        description: Set("验证空章节不会残留".to_string()),
        project_type: Set("novel_long".to_string()),
        project_kind: Set(PROJECT_KIND_TEXT.to_string()),
        source_sync_status: Set(SOURCE_SYNC_CURRENT.to_string()),
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

pub(super) async fn insert_chapter(db: &DatabaseConnection, project_id: i64) -> chapter::Model {
    let now = fixed_time();
    chapter::ActiveModel {
        project_id: Set(project_id),
        outline_id: Set(None),
        chapter_number: Set(1),
        title: Set(Some("有效章节".to_string())),
        content: Set("已有正文。".to_string()),
        word_count: Set(5),
        status: Set(STATUS_DRAFT.to_string()),
        summary: Set(Some("已有摘要。".to_string())),
        style_tags: Set(None),
        analysis_data: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}
