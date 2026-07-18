use a4n_core::models::chapter::chapter_manga_prompt;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use serde_json::json;

use crate::manga::{
    MANGA_PAGE_PROMPT_TYPE, MangaPipelineDbError, MangaPromptSyncDraft,
    PIPELINE_STATUS_PROMPT_TYPE, clear_manga_pipeline_checkpoint, sync_manga_prompts_by_chapter,
};

async fn setup_db() -> DatabaseConnection {
    a4n_migration::test_support::connect_test_database().await
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
) -> MangaPromptSyncDraft {
    MangaPromptSyncDraft {
        chapter_id,
        project_id,
        page_number,
        panel_number: 0,
        prompt_type: MANGA_PAGE_PROMPT_TYPE.to_string(),
        image_prompt: image_prompt.to_string(),
        dialogue_text: None,
        characters_in_frame: None,
        layout: Some(page_layout()),
        camera_angle: None,
        dialogue_position: None,
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
async fn sync_generated_outputs_preserves_rendered_panel_when_prompt_unchanged() {
    let db = setup_db().await;
    let chapter_id = 8101;
    let project_id = 8201;
    let old_time = fixed_time(1);
    let kept = chapter_manga_prompt::ActiveModel {
        chapter_id: Set(chapter_id),
        project_id: Set(project_id),
        page_number: Set(1),
        panel_number: Set(0),
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
        panel_number: Set(0),
        prompt_type: Set("dsl".to_string()),
        image_prompt: Set("旧镜头".to_string()),
        status: Set("pending".to_string()),
        created_at: Set(old_time),
        updated_at: Set(old_time),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    let rows = sync_manga_prompts_by_chapter(
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
        .order_by_asc(chapter_manga_prompt::Column::PanelNumber)
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
async fn sync_rejects_legacy_prompt_types_and_invalid_positions() {
    let db = setup_db().await;
    let mut legacy = prompt_draft(8401, 8402, 1, "旧提示词");
    legacy.prompt_type = "page_planning".to_string();
    let error = sync_manga_prompts_by_chapter(&db, 8402, 8401, vec![legacy], fixed_time(1))
        .await
        .expect_err("legacy type must fail");
    assert!(matches!(
        error,
        MangaPipelineDbError::InvalidArgument(message)
            if message == "不支持的漫画 prompt_type: page_planning"
    ));

    let mut invalid_page = prompt_draft(8401, 8402, 1, "非法位置");
    invalid_page.panel_number = 1;
    let error = sync_manga_prompts_by_chapter(&db, 8402, 8401, vec![invalid_page], fixed_time(1))
        .await
        .expect_err("page prompt panel number must fail");
    assert!(matches!(
        error,
        MangaPipelineDbError::InvalidArgument(message)
            if message == "page_generation 必须使用 page_number>0 且 panel_number=0"
    ));
}

#[tokio::test]
async fn clear_pipeline_checkpoint_rejects_invalid_status_layout() {
    let db = setup_db().await;
    let chapter_id = 8301;
    let project_id = 8302;
    let now = fixed_time(1);
    chapter_manga_prompt::ActiveModel {
        chapter_id: Set(chapter_id),
        project_id: Set(project_id),
        page_number: Set(0),
        panel_number: Set(0),
        prompt_type: Set(PIPELINE_STATUS_PROMPT_TYPE.to_string()),
        image_prompt: Set("状态".to_string()),
        layout: Set(Some(json!("invalid"))),
        status: Set("generating".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    let error = clear_manga_pipeline_checkpoint(&db, project_id, chapter_id)
        .await
        .expect_err("invalid status layout must fail");

    assert!(matches!(
        error,
        MangaPipelineDbError::InvalidArgument(message)
            if message == "漫画流水线状态.layout 必须是 JSON 对象"
    ));
}
