use a4n_core::models::project::novel_project;
use chrono::Utc;
use sea_orm::DatabaseConnection;

use crate::project::{
    PROJECT_KIND_TEXT, PROJECT_KIND_VIDEO,
    duplicate::{ProjectDuplicateError, ProjectMediaCopyError, ProjectMediaFileCopier},
};

use super::execution::copy_derived_project_graph_record;

struct NoopMediaFileCopier;

impl ProjectMediaFileCopier for NoopMediaFileCopier {
    fn copy_media_file(
        &mut self,
        _new_project_id: i64,
        _source_media_asset_id: i64,
        _field: &str,
        _relative_path: Option<&str>,
    ) -> Result<Option<String>, ProjectMediaCopyError> {
        Ok(None)
    }
}

fn project(project_kind: &str) -> novel_project::Model {
    let now = Utc::now().naive_utc();
    novel_project::Model {
        id: 7,
        user_id: 1,
        title: "派生项目".to_string(),
        description: String::new(),
        project_type: "novel_long".to_string(),
        project_kind: project_kind.to_string(),
        source_mode: "derived".to_string(),
        root_project_id: Some(1),
        source_snapshot_id: Some(2),
        source_sync_status: "current".to_string(),
        status: "draft".to_string(),
        lifecycle_gate: "active".to_string(),
        target_chapters: 10,
        current_chapter: 0,
        total_chapters: Some(10),
        chapters_per_part: None,
        avatar_svg: None,
        avatar_animal: None,
        raw_blueprint: None,
        import_analysis_progress: None,
        export_config: None,
        created_at: now,
        updated_at: now,
    }
}

async fn copy_error(project: &novel_project::Model) -> ProjectDuplicateError {
    copy_derived_project_graph_record(
        &DatabaseConnection::Disconnected,
        project,
        1,
        "复制项目",
        Utc::now().naive_utc(),
        &mut NoopMediaFileCopier,
    )
    .await
    .unwrap_err()
}

#[tokio::test]
async fn derived_snapshot_copy_rejects_text_project_before_database_access() {
    let error = copy_error(&project(PROJECT_KIND_TEXT)).await;

    assert!(matches!(
        error,
        ProjectDuplicateError::InvalidArgument(message)
            if message == "复制派生项目不能接收文本项目"
    ));
}

#[tokio::test]
async fn derived_snapshot_copy_requires_root_project_before_database_access() {
    let mut project = project(PROJECT_KIND_VIDEO);
    project.root_project_id = None;
    let error = copy_error(&project).await;

    assert!(matches!(
        error,
        ProjectDuplicateError::InvalidState { current, operation }
            if current == "派生项目 7 缺少 root_project_id" && operation == "复制派生项目"
    ));
}

#[tokio::test]
async fn derived_snapshot_copy_requires_source_snapshot_before_database_access() {
    let mut project = project(PROJECT_KIND_VIDEO);
    project.source_snapshot_id = None;
    let error = copy_error(&project).await;

    assert!(matches!(
        error,
        ProjectDuplicateError::InvalidState { current, operation }
            if current == "派生项目 7 缺少 source_snapshot_id" && operation == "复制派生项目"
    ));
}
