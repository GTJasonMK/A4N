use a4n_core::models::chapter::{
    chapter,
    chapter_manga_prompt::{self, MANGA_PAGE_PROMPT_TYPE},
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use serde_json::json;

use crate::manga::{
    MangaPagePromptSyncDraft, MangaPipelineDbError, manga_page_prompt_statuses_by_chapter,
    manga_page_prompt_statuses_by_chapters, sync_manga_page_prompts_by_chapter,
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
            project_type: "novel_long".to_string(),
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
        layout: Some(page_layout(page_number)),
        status: "pending".to_string(),
    }
}

fn page_layout(page_number: i32) -> serde_json::Value {
    json!({
        "schema_version": "manga_page_generation_v2",
        "page_plan": {
            "page_number": page_number,
            "beat_refs": [{"scene_index": 1, "shot_index": 1, "beat_index": 1}],
            "role": "setup",
            "pacing": "medium",
            "importance": "normal",
            "reading_flow": "top_to_bottom_left_to_right",
            "page_goal": "推进场景",
            "visual_thesis": "角色对峙",
            "entry_state": "平静",
            "exit_state": "紧张",
            "dominant_panel_number": 1,
            "panels": [{
                "panel_number": 1,
                "beat_refs": [{"scene_index": 1, "shot_index": 1, "beat_index": 1}],
                "function": "dialogue",
                "size": "dominant",
                "shot_scale": "medium",
                "camera_angle": "eye_level",
                "participants": ["hero"],
                "composition": "单人构图",
                "observable_action": "抬头",
                "reaction": null,
                "spatial_relationship": "中央",
                "continuity_from_previous": null,
                "text_layers": []
            }]
        },
        "drawing_prompt": {
            "schema_version": "manga_page_drawing_prompt_v2",
            "page_number": page_number,
            "image_prompt": "整页"
        },
        "reference_bindings": [],
        "prompt_fingerprint": {"hash": "sha256"},
        "source": {
            "root_project_id": 1,
            "source_chapter_id": 2,
            "source_snapshot_id": null,
            "source_snapshot_chapter_id": null,
            "source_revision_key": "live:2:content-sha256",
            "content_sha256": "content-sha256",
            "shooting_script_id": 71,
            "shooting_script_revision": 2,
            "shooting_script_sha256": "script-sha256"
        }
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
        layout: Set(Some(page_layout(1))),
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

    let mut statuses = manga_page_prompt_statuses_by_chapter(&db, project_id, chapter_id)
        .await
        .unwrap();
    statuses.sort_unstable();
    assert_eq!(
        statuses,
        vec![
            (stored[0].id, "completed".to_string()),
            (stored[1].id, "pending".to_string())
        ]
    );
    let batch_statuses = manga_page_prompt_statuses_by_chapters(&db, project_id, &[chapter_id])
        .await
        .unwrap();
    assert_eq!(
        batch_statuses,
        vec![
            (chapter_id, stored[0].id, "completed".to_string()),
            (chapter_id, stored[1].id, "pending".to_string()),
        ]
    );
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
