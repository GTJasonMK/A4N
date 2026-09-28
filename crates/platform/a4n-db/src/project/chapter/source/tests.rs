use a4n_db_models::models::{chapter::chapter, project::source_snapshot_chapter};
use a4n_domain::source_text_hash;
use chrono::Utc;

use super::{live::live_source_from_chapter, snapshot::snapshot_source_from_chapter};

fn chapter(content: &str) -> chapter::Model {
    let now = Utc::now().naive_utc();
    chapter::Model {
        id: 9,
        project_id: 4,
        outline_id: None,
        chapter_number: 3,
        title: Some("第三章".to_string()),
        content: content.to_string(),
        word_count: 4,
        status: chapter::STATUS_COMPLETED.to_string(),
        summary: Some("摘要".to_string()),
        style_tags: None,
        analysis_data: None,
        created_at: now,
        updated_at: now,
    }
}

#[test]
fn live_revision_identity_contains_content_hash() {
    let source = live_source_from_chapter(4, chapter("正文"));

    assert_eq!(
        source.source_revision_key,
        format!("live:9:{}", source.content_sha256)
    );
    assert!(source.source_snapshot_id.is_none());
}

#[test]
fn snapshot_revision_identity_contains_snapshot_chapter_and_hash() {
    let content = "冻结正文";
    let source = snapshot_source_from_chapter(
        8,
        4,
        6,
        source_snapshot_chapter::Model {
            id: 12,
            snapshot_id: 6,
            root_project_id: 4,
            source_chapter_id: 9,
            chapter_number: 3,
            title: None,
            content: content.to_string(),
            summary: None,
            analysis_data_json: None,
            content_hash: source_text_hash(content),
            analysis_hash: None,
        },
    );

    assert_eq!(
        source.source_revision_key,
        format!("snapshot:6:chapter:12:{}", source.content_sha256)
    );
    assert_eq!(source.source_snapshot_chapter_id, Some(12));
}
