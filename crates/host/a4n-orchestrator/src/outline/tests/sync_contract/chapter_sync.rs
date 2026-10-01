use super::*;
use a4n_db_models::models::chapter::{chapter_outline, part_outline};
use sea_orm::TransactionTrait;

#[tokio::test]
async fn sync_chapter_outlines_by_number_preserves_existing_ids_and_inserts_missing() {
    let db = setup_db().await;
    let project_id = 7501;
    insert_project(&db, project_id).await;
    let old_time = fixed_time(1);
    let now = fixed_time(2);
    let part = part_outline::ActiveModel {
        project_id: Set(project_id),
        part_number: Set(1),
        title: Set("第一部".to_string()),
        start_chapter: Set(1),
        end_chapter: Set(5),
        summary: Set(Some("第一部摘要".to_string())),
        created_at: Set(old_time),
        updated_at: Set(old_time),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let existing = chapter_outline::ActiveModel {
        project_id: Set(project_id),
        part_id: Set(None),
        chapter_number: Set(2),
        title: Set(Some("旧第二章".to_string())),
        summary: Set(Some("旧摘要".to_string())),
        status: Set("draft".to_string()),
        created_at: Set(old_time),
        updated_at: Set(old_time),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let transaction = db.begin().await.unwrap();
    let rows = sync_chapter_outlines_by_number_in(
        &transaction,
        project_id,
        Some(part.id),
        vec![
            json!({
                "chapter_number": 2,
                "title": "新第二章",
                "summary": "新摘要",
                "beats": ["承压"],
                "word_count_target": 3200
            }),
            json!({
                "chapter_number": 3,
                "title": "第三章",
                "summary": "新增摘要",
                "pacing_score": 0.75
            }),
        ],
        now,
    )
    .await
    .unwrap();
    transaction.commit().await.unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].id, existing.id);
    assert_eq!(rows[0].created_at, old_time);
    assert_eq!(rows[0].updated_at, now);
    assert_eq!(rows[0].part_id, Some(part.id));
    assert_eq!(rows[0].title.as_deref(), Some("新第二章"));
    assert_eq!(rows[0].beats, Some(json!(["承压"])));
    assert_eq!(rows[0].word_count_target, Some(3200));
    assert_ne!(rows[1].id, existing.id);
    assert_eq!(rows[1].created_at, now);
    assert_eq!(rows[1].updated_at, now);
    assert_eq!(rows[1].part_id, Some(part.id));
    assert_eq!(rows[1].chapter_number, 3);
    assert_eq!(rows[1].pacing_score, Some(0.75));
    let all_rows = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(chapter_outline::Column::ChapterNumber)
        .all(&db)
        .await
        .unwrap();
    assert_eq!(all_rows.len(), 2);
    assert_eq!(all_rows[0].id, existing.id);
    assert_eq!(all_rows[1].chapter_number, 3);
}
