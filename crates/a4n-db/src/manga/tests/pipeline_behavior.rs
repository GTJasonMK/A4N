use a4n_core::models::chapter::{
    chapter,
    chapter_manga_prompt::{self, MANGA_PAGE_PROMPT_TYPE},
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use serde_json::json;

use crate::manga::{
    MangaPagePromptSyncDraft, MangaPipelineDbError, sync_manga_page_prompts_by_chapter,
};

async fn setup_db() -> DatabaseConnection {
    a4n_migration::test_support::connect_test_database().await
}

async fn project_and_chapter(db: &DatabaseConnection, title: &str) -> (i64, i64) {
    let project = crate::project::create_project_record(
        db,
        crate::project::CreateProjectRecordInput {
            user_id: 1,
            title: title.to_string(),
            description: String::new(),
            project_type: crate::project::PROJECT_TYPE_NOVEL_LONG.to_string(),
        },
    )
    .await
    .unwrap();
    let now = fixed_time(1);
    let chapter = chapter::ActiveModel {
        project_id: Set(project.id),
        chapter_number: Set(1),
        title: Set(Some("第一章".to_string())),
        content: Set("章节正文".to_string()),
        word_count: Set(4),
        status: Set(chapter::STATUS_DRAFT.to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
    (project.id, chapter.id)
}

fn fixed_time(day: u32) -> chrono::NaiveDateTime {
    chrono::NaiveDate::from_ymd_opt(2026, 5, day)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap()
}

fn prompt_draft(
    chapter_id: i64,
    project_id: i64,
    page_number: i32,
    image_prompt: &str,
) -> MangaPagePromptSyncDraft {
    MangaPagePromptSyncDraft {
        chapter_id,
        project_id,
        page_number,
        image_prompt: image_prompt.to_string(),
        dialogue_text: None,
        characters_in_frame: None,
        layout: Some(page_layout()),
        status: "pending".to_string(),
    }
}

fn page_layout() -> serde_json::Value {
    json!({
        "schema_version": "manga_page_generation_v2",
        "shooting_script_id": 71,
        "shooting_script_revision": 2
    })
}

#[tokio::test]
async fn sync_page_prompts_preserves_rendered_image_when_prompt_unchanged() {
    let db = setup_db().await;
    let (project_id, chapter_id) = project_and_chapter(&db, "漫画提示词同步").await;
    let old_time = fixed_time(1);
    let kept = chapter_manga_prompt::ActiveModel {
        chapter_id: Set(chapter_id),
        project_id: Set(project_id),
        page_number: Set(1),
        prompt_type: Set(MANGA_PAGE_PROMPT_TYPE.to_string()),
        image_prompt: Set("镜头一".to_string()),
        layout: Set(Some(page_layout())),
        generated_image_id: Set(Some(9901)),
        status: Set("completed".to_string()),
        created_at: Set(old_time),
        updated_at: Set(old_time),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let stale = chapter_manga_prompt::ActiveModel {
        chapter_id: Set(chapter_id),
        project_id: Set(project_id),
        page_number: Set(9),
        prompt_type: Set(MANGA_PAGE_PROMPT_TYPE.to_string()),
        image_prompt: Set("旧镜头".to_string()),
        status: Set("pending".to_string()),
        created_at: Set(old_time),
        updated_at: Set(old_time),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    let rows = sync_manga_page_prompts_by_chapter(
        &db,
        project_id,
        chapter_id,
        vec![
            prompt_draft(chapter_id, project_id, 1, "镜头一"),
            prompt_draft(chapter_id, project_id, 2, "镜头二"),
        ],
        fixed_time(2),
    )
    .await
    .unwrap();

    assert_eq!(rows.len(), 2);
    let stored = chapter_manga_prompt::Entity::find()
        .filter(chapter_manga_prompt::Column::ChapterId.eq(chapter_id))
        .order_by_asc(chapter_manga_prompt::Column::PageNumber)
        .all(&db)
        .await
        .unwrap();
    assert_eq!(stored.len(), 2);
    assert_eq!(stored[0].id, kept.id);
    assert_eq!(stored[0].generated_image_id, Some(9901));
    assert_eq!(stored[0].status, "completed");
    assert!(!stored.iter().any(|row| row.id == stale.id));
}

#[tokio::test]
async fn sync_rejects_invalid_page_number() {
    let db = setup_db().await;
    let invalid_page = prompt_draft(8401, 8402, 0, "非法位置");
    let error =
        sync_manga_page_prompts_by_chapter(&db, 8402, 8401, vec![invalid_page], fixed_time(1))
            .await
            .expect_err("page number must fail");
    assert!(matches!(
        error,
        MangaPipelineDbError::InvalidArgument(message)
            if message == "page_generation 必须使用 page_number>0"
    ));
}
