use super::*;

#[tokio::test]
async fn sync_part_outlines_preserves_ids_and_rebinds_chapter_outlines() {
    let db = setup_db().await;
    let project_id = 7401;
    let old_time = fixed_time(1);
    let now = fixed_time(2);
    let kept = part_outline::ActiveModel {
        project_id: Set(project_id),
        part_number: Set(1),
        title: Set("旧第一部".to_string()),
        start_chapter: Set(1),
        end_chapter: Set(10),
        summary: Set(Some("旧摘要".to_string())),
        theme: Set(Some("旧主题".to_string())),
        key_events: Set(Some(json!(["旧事件"]))),
        character_focus: Set(Some(json!({"主角": "旧"}))),
        created_at: Set(old_time),
        updated_at: Set(old_time),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let stale = part_outline::ActiveModel {
        project_id: Set(project_id),
        part_number: Set(3),
        title: Set("旧第三部".to_string()),
        start_chapter: Set(21),
        end_chapter: Set(30),
        created_at: Set(old_time),
        updated_at: Set(old_time),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    for (chapter_number, part_id) in [(5, None), (15, None)] {
        chapter_outline::ActiveModel {
            project_id: Set(project_id),
            part_id: Set(part_id),
            chapter_number: Set(chapter_number),
            title: Set(Some(format!("第 {chapter_number} 章"))),
            summary: Set(Some("摘要".to_string())),
            status: Set("draft".to_string()),
            created_at: Set(old_time),
            updated_at: Set(old_time),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
    }
    let parts = sync_part_outlines(
        &db,
        project_id,
        vec![
            PartOutlineDraft {
                part_number: 1,
                start_chapter: 1,
                end_chapter: 10,
                data: part_json(1, "新第一部"),
            },
            PartOutlineDraft {
                part_number: 2,
                start_chapter: 11,
                end_chapter: 20,
                data: part_json(2, "第二部"),
            },
        ],
        now,
    )
    .await
    .unwrap();
    sync_chapter_outline_part_assignments(&db, project_id, &parts, now)
        .await
        .unwrap();
    let rows = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(part_outline::Column::PartNumber)
        .all(&db)
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    let first = rows.iter().find(|part| part.part_number == 1).unwrap();
    assert_eq!(first.id, kept.id);
    assert_eq!(first.created_at, old_time);
    assert_eq!(first.updated_at, now);
    assert_eq!(first.title, "新第一部");
    assert!(!rows.iter().any(|part| part.id == stale.id));
    let outlines = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(chapter_outline::Column::ChapterNumber)
        .all(&db)
        .await
        .unwrap();
    let second_part_id = rows.iter().find(|part| part.part_number == 2).unwrap().id;
    assert_eq!(outlines[0].part_id, Some(kept.id));
    assert_eq!(outlines[1].part_id, Some(second_part_id));
}
#[tokio::test]
async fn sync_part_outlines_rejects_out_of_range_existing_chapter_outline() {
    let db = setup_db().await;
    let project_id = 7402;
    let old_time = fixed_time(1);
    let now = fixed_time(2);
    part_outline::ActiveModel {
        project_id: Set(project_id),
        part_number: Set(1),
        title: Set("旧第一部".to_string()),
        start_chapter: Set(1),
        end_chapter: Set(10),
        summary: Set(Some("旧摘要".to_string())),
        created_at: Set(old_time),
        updated_at: Set(old_time),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    chapter_outline::ActiveModel {
        project_id: Set(project_id),
        part_id: Set(None),
        chapter_number: Set(25),
        title: Set(Some("第 25 章".to_string())),
        summary: Set(Some("旧章纲".to_string())),
        status: Set("draft".to_string()),
        created_at: Set(old_time),
        updated_at: Set(old_time),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let parts = sync_part_outlines(
        &db,
        project_id,
        vec![PartOutlineDraft {
            part_number: 1,
            start_chapter: 1,
            end_chapter: 10,
            data: part_json(1, "新第一部"),
        }],
        now,
    )
    .await
    .unwrap();
    let err = sync_chapter_outline_part_assignments(&db, project_id, &parts, now)
        .await
        .expect_err("out-of-range outline must fail instead of being unbound");
    assert!(err.to_string().contains("第 25 章章纲不在任何分部范围内"));
}
