use super::queries::{source_snapshot_chapter_summary, source_snapshot_id_for_project};
use crate::project::ProjectMutationError;
use a4n_db_models::models::project::{novel_project, source_snapshot_chapter};
use chrono::NaiveDate;
use serde_json::json;

fn project(
    project_kind: &str,
    root_project_id: Option<i64>,
    source_snapshot_id: Option<i64>,
) -> novel_project::Model {
    novel_project::Model {
        id: 7,
        user_id: 1,
        title: "项目".to_string(),
        description: String::new(),
        project_type: "novel_long".to_string(),
        project_kind: project_kind.to_string(),
        root_project_id,
        source_snapshot_id,
        source_sync_status: "current".to_string(),
        status: "draft".to_string(),
        lifecycle_gate: "active".to_string(),
        target_chapters: 10,
        current_chapter: 1,
        total_chapters: Some(10),
        chapters_per_part: None,
        avatar_svg: None,
        avatar_animal: None,
        raw_blueprint: None,
        import_analysis_progress: None,
        export_config: None,
        created_at: naive_datetime(),
        updated_at: naive_datetime(),
    }
}

fn naive_datetime() -> chrono::NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
}

#[test]
fn source_snapshot_id_rejects_text_project() {
    let err = source_snapshot_id_for_project(&project("text", None, None))
        .expect_err("text project should not expose source snapshot chapters");

    assert!(matches!(
        err,
        ProjectMutationError::InvalidArgument(message)
            if message == "文本项目没有源文本快照章节"
    ));
}

#[test]
fn source_snapshot_id_rejects_freeform_project() {
    let err = source_snapshot_id_for_project(&project("video", None, None))
        .expect_err("freeform project should not expose source snapshot chapters");

    assert!(matches!(
        err,
        ProjectMutationError::InvalidArgument(message)
            if message == "自由视频项目没有源文本快照章节"
    ));
}

#[test]
fn source_snapshot_chapter_summary_preserves_content_and_analysis() {
    let summary = source_snapshot_chapter_summary(&source_snapshot_chapter::Model {
        id: 1,
        snapshot_id: 2,
        root_project_id: 3,
        source_chapter_id: 4,
        chapter_number: 5,
        title: Some("标题".to_string()),
        content: "正文".to_string(),
        summary: Some("摘要".to_string()),
        analysis_data_json: Some(json!({"k": "v"})),
        content_hash: "content-hash".to_string(),
        analysis_hash: Some("analysis-hash".to_string()),
        visual_plan_id: None,
        visual_plan_version: None,
        visual_plan_sha256: None,
    });

    assert_eq!(summary.id, 1);
    assert_eq!(summary.snapshot_id, 2);
    assert_eq!(summary.root_project_id, 3);
    assert_eq!(summary.source_chapter_id, 4);
    assert_eq!(summary.chapter_number, 5);
    assert_eq!(summary.title.as_deref(), Some("标题"));
    assert_eq!(summary.content, "正文");
    assert_eq!(summary.summary.as_deref(), Some("摘要"));
    assert_eq!(summary.analysis_data_json, Some(json!({"k": "v"})));
    assert_eq!(summary.content_hash, "content-hash");
    assert_eq!(summary.analysis_hash.as_deref(), Some("analysis-hash"));
}
