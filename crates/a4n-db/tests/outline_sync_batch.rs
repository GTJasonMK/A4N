use a4n_core::models::chapter::{chapter_outline, chapter_outline_provenance, part_outline};
use a4n_db::outline::{
    ChapterOutlineSyncDraft, PartOutlineSyncDraft, sync_chapter_outline_records_by_number,
    sync_part_outline_records,
};
use a4n_db::project::{CreateProjectRecordInput, create_project_record, part_outlines_by_ids};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde_json::json;

async fn create_project(db: &sea_orm::DatabaseConnection) -> i64 {
    create_project_record(
        db,
        CreateProjectRecordInput {
            user_id: 1,
            title: "分部同步批量写入".to_string(),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("test project must be created")
    .id
}

fn draft(part_number: i32, title: &str) -> PartOutlineSyncDraft {
    PartOutlineSyncDraft {
        part_number,
        start_chapter: part_number * 10,
        end_chapter: part_number * 10 + 9,
        title: title.to_string(),
        summary: format!("第 {part_number} 部概要"),
        theme: "成长".to_string(),
        key_events: json!([format!("事件 {part_number}")]),
        character_focus: json!({"主角": part_number}),
    }
}

fn chapter_draft(chapter_number: i32, title: &str) -> ChapterOutlineSyncDraft {
    ChapterOutlineSyncDraft {
        chapter_number,
        title: title.to_string(),
        summary: format!("第 {chapter_number} 章概要"),
        beats: Some(json!(["冲突"])),
        key_scenes: Some(json!(["场景"])),
        pacing_score: Some(0.8),
        foreshadowing_setup: None,
        foreshadowing_payoff: None,
        characters_involved: Some(json!(["主角"])),
        word_count_target: Some(2_000),
    }
}

#[tokio::test]
async fn part_outline_sync_batches_updates_inserts_and_stale_deletes() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project_id = create_project(&db).await;
    let now = Utc::now().naive_utc();
    let existing = part_outline::ActiveModel {
        project_id: Set(project_id),
        part_number: Set(1),
        title: Set("旧标题".to_string()),
        start_chapter: Set(10),
        end_chapter: Set(19),
        summary: Set(Some("旧概要".to_string())),
        theme: Set(Some("旧主题".to_string())),
        key_events: Set(Some(json!(["旧事件"]))),
        character_focus: Set(Some(json!({"旧角色": true}))),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("existing part must be inserted");
    let stale = part_outline::ActiveModel {
        project_id: Set(project_id),
        part_number: Set(99),
        title: Set("待删除".to_string()),
        start_chapter: Set(990),
        end_chapter: Set(999),
        summary: Set(Some("待删除".to_string())),
        theme: Set(Some("待删除".to_string())),
        key_events: Set(Some(json!([]))),
        character_focus: Set(Some(json!({}))),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("stale part must be inserted");

    let result = sync_part_outline_records(
        &db,
        project_id,
        vec![draft(1, "更新后的第一部"), draft(2, "新增第二部")],
        now,
    )
    .await
    .expect("batch part sync must succeed");

    assert_eq!(result.len(), 2);
    assert_eq!(result[0].id, existing.id);
    assert_eq!(result[0].title, "更新后的第一部");
    assert_eq!(result[1].part_number, 2);
    assert_ne!(result[1].id, existing.id);
    let loaded = part_outlines_by_ids(&db, &[result[1].id, result[0].id])
        .await
        .expect("batch part lookup must succeed");
    assert_eq!(
        loaded.iter().map(|part| part.id).collect::<Vec<_>>(),
        vec![result[0].id, result[1].id]
    );
    assert!(
        part_outline::Entity::find_by_id(stale.id)
            .one(&db)
            .await
            .expect("stale lookup must succeed")
            .is_none()
    );
}

#[tokio::test]
async fn chapter_outline_sync_batches_updates_inserts_and_provenance_cleanup() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project_id = create_project(&db).await;
    let now = Utc::now().naive_utc();
    let existing = chapter_outline::ActiveModel {
        project_id: Set(project_id),
        part_id: Set(None),
        chapter_number: Set(1),
        title: Set(Some("旧章纲".to_string())),
        summary: Set(Some("旧概要".to_string())),
        beats: Set(None),
        key_scenes: Set(None),
        pacing_score: Set(None),
        foreshadowing_setup: Set(None),
        foreshadowing_payoff: Set(None),
        characters_involved: Set(None),
        word_count_target: Set(None),
        status: Set("completed".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("existing chapter must be inserted");
    chapter_outline_provenance::ActiveModel {
        outline_id: Set(existing.id),
        source_kind: Set("import_analysis".to_string()),
        source_fingerprint_json: Set(json!({"source": "test"})),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(&db)
    .await
    .expect("chapter provenance must be inserted");

    let result = sync_chapter_outline_records_by_number(
        &db,
        project_id,
        None,
        vec![
            chapter_draft(1, "更新后的章纲"),
            chapter_draft(2, "新增章纲"),
        ],
        now,
    )
    .await
    .expect("batch chapter sync must succeed");

    assert_eq!(result.len(), 2);
    assert_eq!(result[0].id, existing.id);
    assert_eq!(result[0].title.as_deref(), Some("更新后的章纲"));
    assert_eq!(result[1].chapter_number, 2);
    assert_ne!(result[1].id, existing.id);
    assert!(
        chapter_outline_provenance::Entity::find_by_id(existing.id)
            .one(&db)
            .await
            .expect("provenance lookup must succeed")
            .is_none()
    );
}
