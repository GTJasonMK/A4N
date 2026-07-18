use super::contracts::ProjectFamilyDbError;
use super::manga_sources::snapshot_chapter_manga_source;
use a4n_core::models::project::source_snapshot_chapter;

fn snapshot_chapter(content: &str) -> source_snapshot_chapter::Model {
    source_snapshot_chapter::Model {
        id: 1,
        snapshot_id: 2,
        root_project_id: 3,
        source_chapter_id: 4,
        chapter_number: 5,
        title: Some("标题".to_string()),
        content: content.to_string(),
        summary: Some("摘要".to_string()),
        analysis_data_json: None,
        content_hash: "content-hash".to_string(),
        analysis_hash: None,
    }
}

#[test]
fn snapshot_manga_source_requires_content_when_requested() {
    let error = snapshot_chapter_manga_source(snapshot_chapter("  "), true)
        .expect_err("blank snapshot content must be rejected");

    assert!(matches!(
        error,
        ProjectFamilyDbError::InvalidArgument(message)
            if message == "源快照章节正文为空，无法生成整页漫画提示词"
    ));

    let source = snapshot_chapter_manga_source(snapshot_chapter("  "), false)
        .expect("content is optional for metadata-only reads");
    assert_eq!(source.chapter_id, 4);
    assert_eq!(source.chapter_number, 5);
    assert_eq!(source.status, "completed");
}
