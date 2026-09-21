use a4n_db_models::models::chapter::{chapter_outline, part_outline};
use chrono::Utc;
use sea_orm::DatabaseConnection;

use super::*;

fn part(id: i64, part_number: i32) -> part_outline::Model {
    let now = Utc::now().naive_utc();
    part_outline::Model {
        id,
        project_id: 7,
        part_number,
        title: format!("第 {part_number} 部"),
        start_chapter: (part_number - 1) * 10 + 1,
        end_chapter: part_number * 10,
        summary: Some("概要".to_string()),
        theme: None,
        key_events: None,
        character_focus: None,
        created_at: now,
        updated_at: now,
    }
}

fn outline(chapter_number: i32, part_id: Option<i64>) -> chapter_outline::Model {
    chapter_outline::Model {
        id: 100 + i64::from(chapter_number),
        project_id: 7,
        part_id,
        chapter_number,
        title: Some(format!("第 {chapter_number} 章")),
        summary: Some("摘要".to_string()),
        beats: None,
        key_scenes: None,
        pacing_score: None,
        foreshadowing_setup: None,
        foreshadowing_payoff: None,
        characters_involved: None,
        word_count_target: None,
        status: "draft".to_string(),
        created_at: Utc::now().naive_utc(),
        updated_at: Utc::now().naive_utc(),
    }
}

#[test]
fn chapter_outline_ids_in_part_range_selects_only_target_range() {
    let part = part(20, 2);
    let outlines = [8, 12, 20, 21]
        .into_iter()
        .map(|chapter_number| outline(chapter_number, Some(part.id)))
        .collect::<Vec<_>>();
    let deleted_ids = chapter_outline_ids_in_part_range(&outlines, &part);
    assert_eq!(deleted_ids, vec![112, 120]);
}

#[test]
fn chapter_outline_ids_in_part_ranges_selects_by_chapter_range_not_part_id() {
    let first_part = part(20, 2);
    let second_part = part(30, 3);
    let outlines = [
        outline(12, None),
        outline(18, Some(999)),
        outline(25, Some(second_part.id)),
        outline(31, Some(second_part.id)),
    ];
    assert_eq!(
        chapter_outline_ids_in_part_ranges(&outlines, &[first_part, second_part]),
        vec![112, 118, 125]
    );
}

#[tokio::test]
async fn update_part_outline_and_rebind_rejects_cross_project_before_transaction() {
    let existing = part(20, 2);
    let draft = PartOutlineSyncDraft {
        part_number: existing.part_number,
        start_chapter: existing.start_chapter,
        end_chapter: existing.end_chapter,
        title: existing.title.clone(),
        summary: "新概要".to_string(),
        theme: "新主题".to_string(),
        key_events: serde_json::json!([]),
        character_focus: serde_json::json!({}),
    };

    let error = update_part_outline_and_rebind_chapters_transaction(
        &DatabaseConnection::Disconnected,
        8,
        existing,
        draft,
        Utc::now().naive_utc(),
    )
    .await
    .expect_err("cross-project update must fail before database access");

    assert!(
        matches!(error, OutlineDbError::InvalidArgument(message) if message.contains("不属于项目 8"))
    );
}
