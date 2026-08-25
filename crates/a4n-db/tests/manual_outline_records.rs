use a4n_core::models::chapter::{
    chapter, chapter_outline, chapter_outline_provenance, part_outline,
};
use a4n_db::{
    import_reverse::{ReverseOutlineApplyRow, apply_reverse_outline_records_transaction},
    outline::{PartOutlineSyncDraft, update_part_outline_and_rebind_chapters_transaction},
    project::{
        CreateProjectRecordInput, DeleteLatestOutlineRecordsInput, ManualChapterOutlineSaveInput,
        ManualPartOutlineSaveInput, OutlineDeleteTarget, ProjectOutlineRecordError,
        create_project_record, delete_latest_chapter_outline_records,
        delete_latest_part_outline_records, save_manual_chapter_outline, save_manual_part_outline,
    },
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde_json::json;

async fn create_project(db: &sea_orm::DatabaseConnection, title: &str) -> i64 {
    create_project_record(
        db,
        CreateProjectRecordInput {
            user_id: 1,
            title: title.to_string(),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .expect("test project must be created")
    .id
}

fn chapter_input(project_id: i64, chapter_number: i32) -> ManualChapterOutlineSaveInput {
    ManualChapterOutlineSaveInput {
        project_id,
        outline_id: None,
        expected_updated_at: None,
        chapter_number,
        title: Some(format!("第 {chapter_number} 章")),
        summary: Some("用户计划".to_string()),
    }
}

#[tokio::test]
async fn manual_chapter_outline_rejects_stale_form() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project_id = create_project(&db, "章纲旧表单保护").await;
    let created = save_manual_chapter_outline(&db, chapter_input(project_id, 7))
        .await
        .expect("manual outline must be created");

    let updated = save_manual_chapter_outline(
        &db,
        ManualChapterOutlineSaveInput {
            project_id,
            outline_id: Some(created.id),
            expected_updated_at: Some(created.updated_at.to_string()),
            chapter_number: 7,
            title: Some("用户新标题".to_string()),
            summary: Some("用户新计划".to_string()),
        },
    )
    .await
    .expect("current form must save");
    assert!(updated.updated_at > created.updated_at);

    let error = save_manual_chapter_outline(
        &db,
        ManualChapterOutlineSaveInput {
            project_id,
            outline_id: Some(created.id),
            expected_updated_at: Some(created.updated_at.to_string()),
            chapter_number: 7,
            title: Some("过期覆盖".to_string()),
            summary: Some("不应保存".to_string()),
        },
    )
    .await
    .expect_err("stale form must be rejected");
    assert!(matches!(
        error,
        ProjectOutlineRecordError::OutlineSourceChanged { .. }
    ));

    let stored = chapter_outline::Entity::find_by_id(created.id)
        .one(&db)
        .await
        .expect("outline lookup must succeed")
        .expect("outline must remain");
    assert_eq!(stored.title.as_deref(), Some("用户新标题"));
    assert_eq!(stored.summary.as_deref(), Some("用户新计划"));
}

#[tokio::test]
async fn manual_part_outline_allows_gaps_and_rebinds_without_deleting_chapters() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project_id = create_project(&db, "稀疏分部归属").await;
    let chapter_7 = save_manual_chapter_outline(&db, chapter_input(project_id, 7))
        .await
        .expect("chapter 7 outline must be created");
    let chapter_20 = save_manual_chapter_outline(&db, chapter_input(project_id, 20))
        .await
        .expect("chapter 20 outline must be created");

    let first_part = save_manual_part_outline(
        &db,
        ManualPartOutlineSaveInput {
            project_id,
            part_id: None,
            expected_updated_at: None,
            part_number: 1,
            start_chapter: 1,
            end_chapter: 10,
            title: Some("开端".to_string()),
            summary: Some("建立冲突".to_string()),
            theme: None,
        },
    )
    .await
    .expect("first sparse part must save");

    let stored_7 = chapter_outline::Entity::find_by_id(chapter_7.id)
        .one(&db)
        .await
        .expect("chapter 7 lookup must succeed")
        .expect("chapter 7 must remain");
    let stored_20 = chapter_outline::Entity::find_by_id(chapter_20.id)
        .one(&db)
        .await
        .expect("chapter 20 lookup must succeed")
        .expect("chapter 20 must remain");
    assert_eq!(stored_7.part_id, Some(first_part.id));
    assert_eq!(stored_20.part_id, None);

    let second_part = save_manual_part_outline(
        &db,
        ManualPartOutlineSaveInput {
            project_id,
            part_id: None,
            expected_updated_at: None,
            part_number: 2,
            start_chapter: 20,
            end_chapter: 25,
            title: Some("远端章节".to_string()),
            summary: None,
            theme: None,
        },
    )
    .await
    .expect("second sparse part must save");
    assert_eq!(
        chapter_outline::Entity::find_by_id(chapter_20.id)
            .one(&db)
            .await
            .expect("chapter 20 lookup must succeed")
            .expect("chapter 20 must remain")
            .part_id,
        Some(second_part.id)
    );

    let updated_first_part = save_manual_part_outline(
        &db,
        ManualPartOutlineSaveInput {
            project_id,
            part_id: Some(first_part.id),
            expected_updated_at: Some(first_part.updated_at.to_string()),
            part_number: 1,
            start_chapter: 1,
            end_chapter: 8,
            title: Some("缩短后的开端".to_string()),
            summary: None,
            theme: None,
        },
    )
    .await
    .expect("current part form must save");
    assert!(updated_first_part.updated_at > first_part.updated_at);

    let error = save_manual_part_outline(
        &db,
        ManualPartOutlineSaveInput {
            project_id,
            part_id: Some(first_part.id),
            expected_updated_at: Some(first_part.updated_at.to_string()),
            part_number: 1,
            start_chapter: 1,
            end_chapter: 9,
            title: Some("过期分部".to_string()),
            summary: None,
            theme: None,
        },
    )
    .await
    .expect_err("stale part form must be rejected");
    assert!(matches!(
        error,
        ProjectOutlineRecordError::OutlineSourceChanged { .. }
    ));
    assert_eq!(
        part_outline::Entity::find()
            .filter(part_outline::Column::ProjectId.eq(project_id))
            .all(&db)
            .await
            .expect("parts must load")
            .len(),
        2
    );
}

#[tokio::test]
async fn confirmed_reverse_outline_becomes_formal_and_invalidates_old_form() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project_id = create_project(&db, "用户确认反推章纲").await;
    let now = Utc::now().naive_utc();
    let automatic = chapter_outline::ActiveModel {
        project_id: Set(project_id),
        part_id: Set(None),
        chapter_number: Set(3),
        title: Set(Some("自动标题".to_string())),
        summary: Set(Some("自动摘要".to_string())),
        beats: Set(None),
        key_scenes: Set(None),
        pacing_score: Set(None),
        foreshadowing_setup: Set(None),
        foreshadowing_payoff: Set(None),
        characters_involved: Set(None),
        word_count_target: Set(None),
        status: Set(chapter_outline::STATUS_COMPLETED.to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("automatic outline must be inserted");
    chapter_outline_provenance::ActiveModel {
        outline_id: Set(automatic.id),
        source_kind: Set(
            chapter_outline_provenance::SOURCE_KIND_RETROSPECTIVE_ANALYSIS.to_string(),
        ),
        source_fingerprint_json: Set(json!({"source": "chapter analysis"})),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(&db)
    .await
    .expect("automatic provenance must be inserted");

    apply_reverse_outline_records_transaction(
        &db,
        project_id,
        &[ReverseOutlineApplyRow {
            chapter_number: 3,
            title: "用户确认标题".to_string(),
            summary: "用户确认计划".to_string(),
            word_count_target: Some(2_500),
        }],
    )
    .await
    .expect("confirmed reverse outline must save");

    let stored = chapter_outline::Entity::find_by_id(automatic.id)
        .one(&db)
        .await
        .expect("outline lookup must succeed")
        .expect("outline must remain");
    assert_eq!(stored.title.as_deref(), Some("用户确认标题"));
    assert!(stored.updated_at > automatic.updated_at);
    assert!(
        chapter_outline_provenance::Entity::find_by_id(automatic.id)
            .one(&db)
            .await
            .expect("provenance lookup must succeed")
            .is_none()
    );

    let error = save_manual_chapter_outline(
        &db,
        ManualChapterOutlineSaveInput {
            project_id,
            outline_id: Some(automatic.id),
            expected_updated_at: Some(automatic.updated_at.to_string()),
            chapter_number: 3,
            title: Some("过期覆盖".to_string()),
            summary: Some("不应保存".to_string()),
        },
    )
    .await
    .expect_err("confirmation must invalidate a form opened before it");
    assert!(matches!(
        error,
        ProjectOutlineRecordError::OutlineSourceChanged { .. }
    ));
}

#[tokio::test]
async fn deleting_part_preserves_chapter_outlines_and_rejects_stale_confirmation() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project_id = create_project(&db, "删除分部保留章纲").await;
    let chapter_7 = save_manual_chapter_outline(&db, chapter_input(project_id, 7))
        .await
        .expect("chapter 7 outline must be created");
    let chapter_20 = save_manual_chapter_outline(&db, chapter_input(project_id, 20))
        .await
        .expect("chapter 20 outline must be created");
    let first_part = save_manual_part_outline(
        &db,
        ManualPartOutlineSaveInput {
            project_id,
            part_id: None,
            expected_updated_at: None,
            part_number: 1,
            start_chapter: 1,
            end_chapter: 10,
            title: Some("开端".to_string()),
            summary: None,
            theme: None,
        },
    )
    .await
    .expect("first part must save");
    let second_part = save_manual_part_outline(
        &db,
        ManualPartOutlineSaveInput {
            project_id,
            part_id: None,
            expected_updated_at: None,
            part_number: 2,
            start_chapter: 20,
            end_chapter: 25,
            title: Some("远端章节".to_string()),
            summary: None,
            theme: None,
        },
    )
    .await
    .expect("second part must save");
    let updated_second_part = update_part_outline_and_rebind_chapters_transaction(
        &db,
        project_id,
        second_part.clone(),
        PartOutlineSyncDraft {
            part_number: 2,
            start_chapter: 20,
            end_chapter: 25,
            title: "远端章节已更新".to_string(),
            summary: "更新后的分部概要".to_string(),
            theme: "远端主题".to_string(),
            key_events: serde_json::json!([]),
            character_focus: serde_json::json!([]),
        },
        Utc::now().naive_utc(),
    )
    .await
    .expect("regenerating a part must not delete its chapter outlines");
    assert!(
        chapter_outline::Entity::find_by_id(chapter_20.id)
            .one(&db)
            .await
            .expect("chapter 20 lookup after part update must succeed")
            .is_some()
    );

    let stale_error = delete_latest_part_outline_records(
        &db,
        DeleteLatestOutlineRecordsInput {
            project_id,
            targets: vec![OutlineDeleteTarget {
                id: second_part.id,
                expected_updated_at: second_part.updated_at.to_string(),
            }],
        },
    )
    .await
    .expect_err("stale delete confirmation must fail");
    assert!(matches!(
        stale_error,
        ProjectOutlineRecordError::OutlineSourceChanged { .. }
    ));

    delete_latest_part_outline_records(
        &db,
        DeleteLatestOutlineRecordsInput {
            project_id,
            targets: vec![OutlineDeleteTarget {
                id: updated_second_part.id,
                expected_updated_at: updated_second_part.updated_at.to_string(),
            }],
        },
    )
    .await
    .expect("current delete confirmation must succeed");

    assert!(
        part_outline::Entity::find_by_id(second_part.id)
            .one(&db)
            .await
            .expect("part lookup must succeed")
            .is_none()
    );
    assert_eq!(
        chapter_outline::Entity::find_by_id(chapter_7.id)
            .one(&db)
            .await
            .expect("chapter 7 lookup must succeed")
            .expect("chapter 7 outline must remain")
            .part_id,
        Some(first_part.id)
    );
    assert_eq!(
        chapter_outline::Entity::find_by_id(chapter_20.id)
            .one(&db)
            .await
            .expect("chapter 20 lookup must succeed")
            .expect("chapter 20 outline must remain")
            .part_id,
        None
    );
}

#[tokio::test]
async fn deleting_part_outline_unbinds_multiple_chapter_outlines() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project_id = create_project(&db, "批量解绑分部大纲").await;
    let part = save_manual_part_outline(
        &db,
        ManualPartOutlineSaveInput {
            project_id,
            part_id: None,
            expected_updated_at: None,
            part_number: 1,
            start_chapter: 1,
            end_chapter: 3,
            title: Some("开端".to_string()),
            summary: None,
            theme: None,
        },
    )
    .await
    .expect("part must save");
    let mut outlines = Vec::new();
    for chapter_number in 1..=3 {
        outlines.push(
            save_manual_chapter_outline(&db, chapter_input(project_id, chapter_number))
                .await
                .expect("chapter outline must save"),
        );
    }

    delete_latest_part_outline_records(
        &db,
        DeleteLatestOutlineRecordsInput {
            project_id,
            targets: vec![OutlineDeleteTarget {
                id: part.id,
                expected_updated_at: part.updated_at.to_string(),
            }],
        },
    )
    .await
    .expect("current part delete must succeed");

    for outline in outlines {
        let current = chapter_outline::Entity::find_by_id(outline.id)
            .one(&db)
            .await
            .unwrap()
            .expect("chapter outline must remain");
        assert_eq!(current.part_id, None);
        assert!(current.updated_at > outline.updated_at);
    }
}

#[tokio::test]
async fn deleting_chapter_outline_preserves_body_and_requires_the_latest_target() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project_id = create_project(&db, "删除章纲保留正文").await;
    let chapter_7_outline = save_manual_chapter_outline(&db, chapter_input(project_id, 7))
        .await
        .expect("chapter 7 outline must be created");
    let chapter_20_outline = save_manual_chapter_outline(&db, chapter_input(project_id, 20))
        .await
        .expect("chapter 20 outline must be created");
    let now = Utc::now().naive_utc();
    let stored_chapter = chapter::ActiveModel {
        project_id: Set(project_id),
        outline_id: Set(Some(chapter_20_outline.id)),
        chapter_number: Set(20),
        title: Set(Some("已有正文".to_string())),
        content: Set("这段正文不能因为删除章纲而消失。".to_string()),
        word_count: Set(17),
        status: Set(chapter::STATUS_IMPORTED.to_string()),
        summary: Set(None),
        style_tags: Set(None),
        analysis_data: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .expect("chapter body must be inserted");

    let wrong_suffix_error = delete_latest_chapter_outline_records(
        &db,
        DeleteLatestOutlineRecordsInput {
            project_id,
            targets: vec![OutlineDeleteTarget {
                id: chapter_7_outline.id,
                expected_updated_at: chapter_7_outline.updated_at.to_string(),
            }],
        },
    )
    .await
    .expect_err("non-latest target must fail");
    assert!(matches!(
        wrong_suffix_error,
        ProjectOutlineRecordError::OutlineSourceChanged { .. }
    ));

    delete_latest_chapter_outline_records(
        &db,
        DeleteLatestOutlineRecordsInput {
            project_id,
            targets: vec![OutlineDeleteTarget {
                id: chapter_20_outline.id,
                expected_updated_at: chapter_20_outline.updated_at.to_string(),
            }],
        },
    )
    .await
    .expect("latest outline must be deleted");

    assert!(
        chapter_outline::Entity::find_by_id(chapter_20_outline.id)
            .one(&db)
            .await
            .expect("outline lookup must succeed")
            .is_none()
    );
    let preserved = chapter::Entity::find_by_id(stored_chapter.id)
        .one(&db)
        .await
        .expect("chapter lookup must succeed")
        .expect("chapter body must remain");
    assert_eq!(preserved.outline_id, None);
    assert_eq!(preserved.content, "这段正文不能因为删除章纲而消失。");
}
