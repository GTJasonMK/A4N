use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use a4n_api_contract::project::DuplicateProjectInput;
use a4n_db::{
    generation::{
        GENERATION_SPEC_TARGET_ENTITY_KIND_CANVAS_NODE, GENERATION_SPEC_TARGET_KIND_CANVAS_VIDEO,
    },
    project::{
        CreateFreeformVideoProjectParams, PROJECT_KIND_TEXT, SOURCE_SYNC_CURRENT,
        create_freeform_video_project_record,
    },
};
use a4n_db_models::models::{
    ai::generated_video,
    canvas::{canvas_graph, canvas_node},
    generation::generation_spec,
    media::media_asset,
    project::novel_project,
};
use a4n_domain::canvas_transfer_runtime_identity_key;
use a4n_domain::project::{ProjectMediaCopyError, ProjectMediaFileCopier};
use a4n_domain::video_workflow::WORKFLOW_MODE_CANVAS_VIDEO;
use a4n_error::AppError;
use async_trait::async_trait;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde_json::json;

use crate::{OrchestratorContext, test_support::TestOrchestratorBuilder};

use super::ProjectDuplicationHostPort;

#[derive(Default)]
struct TestProjectDuplicationHost {
    permission_calls: AtomicUsize,
    copy_calls: AtomicUsize,
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
        self.copy_calls.fetch_add(1, Ordering::SeqCst);
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
    let source = insert_project(database.as_ref(), 7, PROJECT_KIND_TEXT).await;
    let mut host = TestProjectDuplicationHost::default();

    let duplicated = context
        .duplicate_project_validated(
            19,
            DuplicateProjectInput {
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
    assert_eq!(host.copy_calls.load(Ordering::SeqCst), 0);
    assert_eq!(host.rollback_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn duplicate_missing_project_is_not_found_before_permission_check() {
    let (context, _) = setup().await;
    let mut host = TestProjectDuplicationHost::default();

    let error = context
        .duplicate_project_validated(
            19,
            DuplicateProjectInput {
                project_id: i64::MAX,
                new_title: "不存在的副本".to_string(),
            },
            &mut host,
        )
        .await
        .expect_err("不存在的项目必须返回 NotFound");

    assert!(matches!(
        error,
        AppError::NotFound(message) if message.contains(&i64::MAX.to_string())
    ));
    assert_eq!(host.permission_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn duplicate_freeform_application_rolls_back_host_files_on_copy_failure() {
    let (context, database) = setup().await;
    let source = insert_freeform_project(database.as_ref(), 8).await;
    insert_media_asset(database.as_ref(), source.id).await;
    let mut host = TestProjectDuplicationHost::failing_copy();
    let before = novel_project::Entity::find()
        .all(database.as_ref())
        .await
        .unwrap()
        .len();

    let error = context
        .duplicate_project_validated(
            23,
            DuplicateProjectInput {
                project_id: source.id,
                new_title: "自由画布副本".to_string(),
            },
            &mut host,
        )
        .await
        .expect_err("媒体复制失败必须使 freeform 复制失败");
    let after = novel_project::Entity::find()
        .all(database.as_ref())
        .await
        .unwrap()
        .len();

    assert!(error.to_string().contains("test copy failure"));
    assert_eq!(after, before);
    assert_eq!(host.permission_calls.load(Ordering::SeqCst), 1);
    assert_eq!(host.copy_calls.load(Ordering::SeqCst), 1);
    assert_eq!(host.rollback_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn duplicate_freeform_application_rejects_non_terminal_video_before_copying_files() {
    let (context, database) = setup().await;
    let source = insert_freeform_project(database.as_ref(), 18).await;
    insert_generated_video(
        database.as_ref(),
        source.id,
        "running",
        json!({"workflow_run_id": 99}),
    )
    .await;
    let mut host = TestProjectDuplicationHost::default();

    let error = context
        .duplicate_project_validated(
            24,
            DuplicateProjectInput {
                project_id: source.id,
                new_title: "运行中自由画布副本".to_string(),
            },
            &mut host,
        )
        .await
        .expect_err("非终态视频必须阻止 freeform 复制");

    assert!(error.to_string().contains("仍处于 running 状态"));
    assert_eq!(host.permission_calls.load(Ordering::SeqCst), 1);
    assert_eq!(host.copy_calls.load(Ordering::SeqCst), 0);
    assert_eq!(host.rollback_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn duplicate_freeform_application_remaps_source_material_and_clears_runtime_identity() {
    let (context, database) = setup().await;
    let source = insert_freeform_project(database.as_ref(), 25).await;
    let now = Utc::now().naive_utc();
    let graph = canvas_graph::ActiveModel {
        project_id: Set(source.id),
        name: Set("主画布".to_string()),
        graph_kind: Set("video".to_string()),
        version: Set(1),
        viewport_json: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(database.as_ref())
    .await
    .expect("测试画布应可写入");
    let node = canvas_node::ActiveModel {
        graph_id: Set(graph.id),
        node_kind: Set("video_generation".to_string()),
        ref_type: Set(Some("inline".to_string())),
        ref_id: Set(None),
        config_json: Set(json!({})),
        position_json: Set(json!({"x":0,"y":0})),
        state: Set("idle".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(database.as_ref())
    .await
    .expect("测试画布节点应可写入");
    let video = insert_generated_video(
        database.as_ref(),
        source.id,
        "completed",
        json!({
            "canvas": {
                "source_node_ids": [node.id],
                "workflow_run_id": 91
            },
            "provider_job_id": "metadata-job"
        }),
    )
    .await;
    a4n_db::canvas::upsert_freeform_canvas_generation_spec_record(
        database.as_ref(),
        a4n_db::canvas::FreeformCanvasGenerationSpecUpsertInput {
            project_id: source.id,
            target_entity_id: node.id,
            canvas_text_asset_id: None,
            target_kind: GENERATION_SPEC_TARGET_KIND_CANVAS_VIDEO.to_string(),
            target_entity_kind: GENERATION_SPEC_TARGET_ENTITY_KIND_CANVAS_NODE.to_string(),
            status: "completed".to_string(),
            params_json: json!({
                "source_material": {
                    "source_node_ids": [node.id],
                    "generated_video_id": video.id,
                    "config_id": 92
                },
                "workflow_run_id": 93
            }),
        },
    )
    .await
    .expect("测试生成意图应可写入");
    let mut host = TestProjectDuplicationHost::default();

    let duplicated = context
        .duplicate_project_validated(
            26,
            DuplicateProjectInput {
                project_id: source.id,
                new_title: "终态自由画布副本".to_string(),
            },
            &mut host,
        )
        .await
        .expect("终态自由画布项目应可复制");

    let copied_video = generated_video::Entity::find()
        .filter(generated_video::Column::ProjectId.eq(duplicated.id))
        .one(database.as_ref())
        .await
        .unwrap()
        .expect("复制后视频应存在");
    let copied_graph = canvas_graph::Entity::find()
        .filter(canvas_graph::Column::ProjectId.eq(duplicated.id))
        .one(database.as_ref())
        .await
        .unwrap()
        .expect("复制后画布应存在");
    let copied_node = canvas_node::Entity::find()
        .filter(canvas_node::Column::GraphId.eq(copied_graph.id))
        .one(database.as_ref())
        .await
        .unwrap()
        .expect("复制后画布节点应存在");
    let copied_spec = generation_spec::Entity::find()
        .filter(generation_spec::Column::ProjectId.eq(duplicated.id))
        .one(database.as_ref())
        .await
        .unwrap()
        .expect("复制后生成意图应存在");

    assert_eq!(copied_video.provider_job_id, None);
    assert_eq!(
        copied_video
            .metadata_json
            .as_ref()
            .and_then(canvas_transfer_runtime_identity_key),
        None
    );
    assert_eq!(
        copied_video.metadata_json.as_ref().unwrap()["canvas"]["source_node_ids"],
        json!([copied_node.id])
    );
    assert_eq!(copied_spec.target_entity_id, Some(copied_node.id));
    assert_eq!(
        copied_spec.params_json["source_material"]["source_node_ids"],
        json!([copied_node.id])
    );
    assert_eq!(
        copied_spec.params_json["source_material"]["generated_video_id"],
        json!(copied_video.id)
    );
    assert_eq!(
        canvas_transfer_runtime_identity_key(&copied_spec.params_json),
        None
    );
    assert_eq!(
        Some(copied_spec.source_version_hash),
        generation_spec::freeform_source_version_hash(&copied_spec.params_json)
    );
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

async fn insert_generated_video(
    database: &a4n_db::DatabaseConnection,
    project_id: i64,
    status: &str,
    metadata_json: serde_json::Value,
) -> generated_video::Model {
    let now = Utc::now().naive_utc();
    generated_video::ActiveModel {
        project_id: Set(project_id),
        chapter_id: Set(None),
        video_type: Set(a4n_domain::video_workflow::VIDEO_TYPE_CANVAS.to_string()),
        workflow_mode: Set(Some(WORKFLOW_MODE_CANVAS_VIDEO.to_string())),
        file_path: Set(None),
        thumbnail_path: Set(None),
        prompt: Set(format!("{status} video")),
        negative_prompt: Set(None),
        model: Set("test".to_string()),
        width: Set(1280),
        height: Set(720),
        duration_secs: Set(3.0),
        fps: Set(24),
        file_size: Set(None),
        source_video_id: Set(None),
        source_image_id: Set(None),
        status: Set(status.to_string()),
        provider_job_id: Set(Some("provider-job".to_string())),
        error_message: Set(None),
        metadata_json: Set(Some(metadata_json)),
        is_favorite: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(database)
    .await
    .expect("测试生成视频应可写入")
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
) -> novel_project::Model {
    let now = Utc::now().naive_utc();
    novel_project::ActiveModel {
        user_id: Set(user_id),
        title: Set(format!("source-{user_id}")),
        description: Set("project duplication application test".to_string()),
        project_type: Set("novel_long".to_string()),
        project_kind: Set(project_kind.to_string()),
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
