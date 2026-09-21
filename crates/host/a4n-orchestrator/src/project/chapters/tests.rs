use a4n_db_models::models::chapter::chapter;

use super::readiness::validate_chapter_generation_number;
use super::records::is_character_sync_available_chapter;

fn chapter_record(chapter_number: i32, status: &str, content: &str) -> chapter::Model {
    let now = chrono::Utc::now().naive_utc();
    chapter::Model {
        id: 1,
        project_id: 1,
        outline_id: None,
        chapter_number,
        title: Some(format!("第{chapter_number}章")),
        content: content.to_string(),
        word_count: content.chars().count() as i32,
        status: status.to_string(),
        summary: None,
        style_tags: None,
        analysis_data: None,
        created_at: now,
        updated_at: now,
    }
}

fn chapter_for_sync(status: &str, content: &str) -> chapter::Model {
    chapter_record(1, status, content)
}

#[test]
fn character_sync_available_chapter_excludes_deleted_content() {
    let deleted = chapter_for_sync(chapter::STATUS_DELETED, "已删除正文");

    assert!(!is_character_sync_available_chapter(&deleted));
}

#[test]
fn character_sync_available_chapter_requires_non_empty_non_deleted_content() {
    let imported = chapter_for_sync(chapter::STATUS_IMPORTED, "有效正文");
    let empty = chapter_for_sync(chapter::STATUS_IMPORTED, "  ");

    assert!(is_character_sync_available_chapter(&imported));
    assert!(!is_character_sync_available_chapter(&empty));
}

#[test]
fn chapter_generation_readiness_rejects_non_positive_chapter_number() {
    assert_eq!(
        validate_chapter_generation_number(0),
        Err("章节序号必须大于 0".to_string())
    );
}
