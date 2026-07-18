use a4n_core::models::chapter::chapter;
use a4n_db::project::ChapterGenerationReadiness;

use super::readiness::{
    validate_chapter_generation_number, validate_chapter_generation_readiness_record,
};
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

fn readiness(
    has_blueprint: bool,
    has_chapter_outline: bool,
    existing_chapter: Option<chapter::Model>,
    previous_chapter: Option<chapter::Model>,
) -> ChapterGenerationReadiness {
    ChapterGenerationReadiness {
        has_blueprint,
        has_chapter_outline,
        existing_chapter,
        previous_chapter,
    }
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

#[test]
fn chapter_generation_readiness_requires_blueprint_before_outline() {
    let record = readiness(false, false, None, None);

    assert_eq!(
        validate_chapter_generation_readiness_record(2, &record),
        Err("请先完成故事蓝图后再生成正文".to_string())
    );
}

#[test]
fn chapter_generation_readiness_requires_target_outline() {
    let record = readiness(true, false, None, None);

    assert_eq!(
        validate_chapter_generation_readiness_record(2, &record),
        Err("第2章章节大纲未找到，请先生成该章章纲后再生成正文".to_string())
    );
}

#[test]
fn chapter_generation_readiness_accepts_available_existing_content() {
    let existing = chapter_record(2, chapter::STATUS_COMPLETED, "已有正文");
    let record = readiness(true, true, Some(existing), None);

    assert_eq!(
        validate_chapter_generation_readiness_record(2, &record),
        Ok(())
    );
}

#[test]
fn chapter_generation_readiness_accepts_first_chapter_without_previous_content() {
    let record = readiness(true, true, None, None);

    assert_eq!(
        validate_chapter_generation_readiness_record(1, &record),
        Ok(())
    );
}

#[test]
fn chapter_generation_readiness_requires_previous_chapter_record() {
    let record = readiness(true, true, None, None);

    assert_eq!(
        validate_chapter_generation_readiness_record(3, &record),
        Err("请先完成第2章正文后，再生成第3章".to_string())
    );
}

#[test]
fn chapter_generation_readiness_requires_available_previous_content() {
    let previous = chapter_record(2, chapter::STATUS_DELETED, "已删除正文");
    let record = readiness(true, true, None, Some(previous));

    assert_eq!(
        validate_chapter_generation_readiness_record(3, &record),
        Err("第2章尚无可用正文，请先生成或导入正文后再生成第3章".to_string())
    );
}

#[test]
fn chapter_generation_readiness_accepts_available_previous_content() {
    let previous = chapter_record(2, chapter::STATUS_IMPORTED, "前序正文");
    let record = readiness(true, true, None, Some(previous));

    assert_eq!(
        validate_chapter_generation_readiness_record(3, &record),
        Ok(())
    );
}
