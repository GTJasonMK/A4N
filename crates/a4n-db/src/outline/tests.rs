use a4n_core::models::chapter::{chapter_outline, part_outline};
use chrono::Utc;
use sea_orm::DatabaseConnection;

use super::ranges::latest_part_ids_to_delete;
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

#[test]
fn latest_part_ids_to_delete_rejects_zero_count() {
    let err = latest_part_ids_to_delete(&[part(1, 1)], 0).expect_err("zero delete count must fail");
    assert!(err.to_string().contains("删除分部数量必须大于 0"));
}

#[test]
fn latest_part_ids_to_delete_rejects_count_greater_than_existing_parts() {
    let err = latest_part_ids_to_delete(&[part(1, 1), part(2, 2)], 3)
        .expect_err("oversized delete count must fail");
    assert!(err.to_string().contains("删除分部数量超过现有分部数量"));
}

#[test]
fn latest_part_ids_to_delete_selects_latest_parts_by_list_order() {
    let ids = latest_part_ids_to_delete(&[part(11, 1), part(12, 2), part(13, 3)], 2)
        .expect("valid count should select latest parts");
    assert_eq!(ids, vec![13, 12]);
}

#[test]
fn latest_part_ids_to_delete_rejects_invalid_existing_parts() {
    let err = latest_part_ids_to_delete(&[part(11, 1), part(12, 1)], 1)
        .expect_err("ambiguous latest part state must fail before delete");
    assert!(err.to_string().contains("重复分部大纲"));
    assert!(err.to_string().contains("请先清理分部状态后再删除"));
}

#[test]
fn chapter_outline_ids_in_part_range_selects_only_target_range() {
    let part = part(20, 2);
    let outlines = [8, 12, 20, 21]
        .into_iter()
        .map(|chapter_number| chapter_outline::Model {
            id: 100 + chapter_number as i64,
            project_id: 7,
            part_id: Some(part.id),
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
        })
        .collect::<Vec<_>>();
    let deleted_ids = chapter_outline_ids_in_part_range(&outlines, &part);
    assert_eq!(deleted_ids, vec![112, 120]);
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
