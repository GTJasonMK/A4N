use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use a4n_core::{
    error::AppError,
    models::{media::media_asset, project::novel_project},
};
use a4n_db::project::{
    CreateFreeformVideoProjectParams, PROJECT_KIND_TEXT, ProjectMediaCopyError,
    ProjectMediaFileCopier, SOURCE_MODE_SELF, SOURCE_SYNC_CURRENT,
    create_freeform_video_project_record,
};
use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, Set};

use crate::{OrchestratorContext, test_support::TestOrchestratorBuilder};

use super::{
    BranchProjectApplicationInput, DuplicateProjectApplicationInput, ProjectDuplicationHostPort,
};

#[derive(Default)]
struct TestProjectDuplicationHost {
    permission_calls: AtomicUsize,
    rollback_calls: AtomicUsize,
    fail_copy: bool,
}

impl TestProjectDuplicationHost {
    fn failing_copy() -> Self {
        Self {
            fail_copy: true,
            ..Self::default()
        }
    }

    fn copied_path(
        &self,
        relative_path: Option<&str>,
    ) -> Result<Option<String>, ProjectMediaCopyError> {
        if self.fail_copy {
            return Err(ProjectMediaCopyError::InvalidSourcePath(
                "test copy failure".to_string(),
            ));
        }
        Ok(relative_path.map(str::to_string))
    }
}

impl ProjectMediaFileCopier for TestProjectDuplicationHost {
    fn copy_media_file(
        &mut self,
        _new_project_id: i64,
        _source_media_asset_id: i64,
        _field: &str,
        relative_path: Option<&str>,
    ) -> Result<Option<String>, ProjectMediaCopyError> {
        self.copied_path(relative_path)
    }
}

#[async_trait]
impl ProjectDuplicationHostPort for TestProjectDuplicationHost {
    async fn verify_project_editor(&self, _user_id: i64, _project_id: i64) -> Result<(), AppError> {
        self.permission_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn rollback_copied_media_files(&mut self, _label: &str) -> Result<(), String> {
        self.rollback_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[tokio::test]
async fn duplicate_text_application_owns_lookup_permission_validation_and_dispatch() {
    let (context, database) = setup().await;
    let source = insert_project(database.as_ref(), 7, PROJECT_KIND_TEXT, SOURCE_MODE_SELF).await;
    let mut host = TestProjectDuplicationHost::default();

    let duplicated = context
        .duplicate_project_validated(
            DuplicateProjectApplicationInput {
                user_id: 19,
                project_id: source.id,
                new_title: "  文本项目副本  ".to_string(),
            },
            &mut host,
        )
        .await
        .expect("文本项目复制 application 应成功");

    assert_eq!(duplicated.user_id, 19);
    assert_eq!(duplicated.title, "文本项目副本");
    assert_eq!(duplicated.project_kind, PROJECT_KIND_TEXT);
    assert_eq!(host.permission_calls.load(Ordering::SeqCst), 1);
    assert_eq!(host.rollback_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn duplicate_freeform_application_rolls_back_host_files_on_copy_failure() {
    let (context, database) = setup().await;
    let source = insert_freeform_project(database.as_ref(), 8).await;
    insert_media_asset(database.as_ref(), source.id).await;
    let mut host = TestProjectDuplicationHost::failing_copy();

    let error = context
        .duplicate_project_validated(
            DuplicateProjectApplicationInput {
                user_id: 23,
                project_id: source.id,
                new_title: "自由画布副本".to_string(),
            },
            &mut host,
        )
        .await
        .expect_err("媒体复制失败必须使 freeform 复制失败");

    assert!(error.to_string().contains("test copy failure"));
    assert_eq!(host.permission_calls.load(Ordering::SeqCst), 1);
    assert_eq!(host.rollback_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn branch_application_rejects_non_text_after_permission_gate() {
    let (context, database) = setup().await;
    let video = insert_freeform_project(database.as_ref(), 9).await;
    let host = TestProjectDuplicationHost::default();

    let error = context
        .branch_project_validated(
            BranchProjectApplicationInput {
                user_id: 29,
                project_id: video.id,
                branch_title: "视频分支".to_string(),
                at_status: "draft".to_string(),
            },
            &host,
        )
        .await
        .expect_err("非文本项目不得创建文本分支");

    assert!(error.to_string().contains("仅支持文本项目"));
    assert_eq!(host.permission_calls.load(Ordering::SeqCst), 1);
    assert_eq!(host.rollback_calls.load(Ordering::SeqCst), 0);
}

async fn insert_freeform_project(
    database: &a4n_db::DatabaseConnection,
    user_id: i64,
) -> novel_project::Model {
    create_freeform_video_project_record(
        database,
        user_id,
        CreateFreeformVideoProjectParams {
            title: format!("freeform-{user_id}"),
            description: "project duplication application test".to_string(),
        },
    )
    .await
    .expect("测试 freeform 视频项目应可写入")
}

async fn insert_media_asset(database: &a4n_db::DatabaseConnection, project_id: i64) {
    let now = Utc::now().naive_utc();
    media_asset::ActiveModel {
        project_id: Set(project_id),
        root_project_id: Set(project_id),
        media_kind: Set("video".to_string()),
        origin: Set("imported".to_string()),
        file_path: Set(Some("projects/source/video.mp4".to_string())),
        status: Set("completed".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(database)
    .await
    .expect("测试媒体资产应可写入");
}

async fn setup() -> (OrchestratorContext, Arc<a4n_db::DatabaseConnection>) {
    let database = Arc::new(a4n_migration::test_support::connect_test_database().await);
    let context = TestOrchestratorBuilder::new(Arc::clone(&database))
        .build()
        .await;
    (context, database)
}

async fn insert_project(
    database: &a4n_db::DatabaseConnection,
    user_id: i64,
    project_kind: &str,
    source_mode: &str,
) -> novel_project::Model {
    let now = Utc::now().naive_utc();
    novel_project::ActiveModel {
        user_id: Set(user_id),
        title: Set(format!("source-{user_id}")),
        description: Set("project duplication application test".to_string()),
        project_type: Set("novel_long".to_string()),
        project_kind: Set(project_kind.to_string()),
        source_mode: Set(source_mode.to_string()),
        source_sync_status: Set(SOURCE_SYNC_CURRENT.to_string()),
        status: Set("draft".to_string()),
        target_chapters: Set(12),
        current_chapter: Set(0),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(database)
    .await
    .expect("测试项目应可写入")
}
