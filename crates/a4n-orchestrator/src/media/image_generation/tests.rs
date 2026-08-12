use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use a4n_core::{
    error::AppError,
    models::{
        ai::{generated_image, image_gen_config},
        chapter::{chapter, chapter_manga_prompt},
        generation::{generation_run, workflow_checkpoint, workflow_run},
        project::novel_project,
    },
};
use a4n_db::{
    DatabaseConnection,
    project::{
        CreateDerivedProjectParams, CreateFreeformVideoProjectParams, DerivedChapterScope,
        PROJECT_KIND_COMIC, create_derived_project_record, create_freeform_video_project_record,
    },
};
use a4n_image::{ImageGenerationRequest, ImageReference};
use async_trait::async_trait;
use base64::Engine as _;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, Set};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use crate::{
    OrchestratorContext,
    media::{BatchImageExecutionItem, BatchImageGenerationApplicationInput},
    test_support::TestOrchestratorBuilder,
};

use super::{
    ImageGenerationExecutionInput, ImageGenerationHostPort, ImageGenerationPostCommitAction,
    ImageGenerationProgress, ImageGenerationProviderInput, ImageGenerationProviderOutput,
    ImageGenerationWorkflowBinding, PersistGeneratedImageFileInput, PersistedGeneratedImageFile,
    application::{check_progress_cancelled, collect_bound_reference_images},
    image_generation_request_trace_json,
    trace::test_image_request_aspect_ratio,
};

const PNG_1X1_B64: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=";

fn png_1x1_bytes() -> Vec<u8> {
    base64::engine::general_purpose::STANDARD
        .decode(PNG_1X1_B64)
        .expect("测试 PNG 应有效")
}

#[test]
fn cancelled_image_progress_preserves_cancelled_error() {
    let token = CancellationToken::new();
    token.cancel();

    let error = check_progress_cancelled(&ImageGenerationProgress::new(None, Some(&token)))
        .expect_err("已取消的图片任务必须返回 cancelled");

    assert!(matches!(error, AppError::Cancelled));
}

#[test]
fn profile_image_inputs_enforce_documented_contract() {
    let object_error = serde_json::from_value::<super::ObjectProfileImageInput>(json!({
        "object_profile_id": 1,
        "style": "写实",
        "set_primary": true
    }))
    .expect_err("object profile input must reject unknown fields");
    assert!(object_error.to_string().contains("unknown field"));

    let scene_error = serde_json::from_value::<super::SceneProfileImageInput>(json!({
        "scene_profile_id": 1,
        "style": "写实",
        "legacy_style_id": "old"
    }))
    .expect_err("scene profile input must reject unknown fields");
    assert!(scene_error.to_string().contains("unknown field"));

    let object = serde_json::from_value::<super::ObjectProfileImageInput>(json!({
        "object_profile_id": 1,
        "chapter_id": 2,
        "style": "写实",
        "config_id": 3,
        "style_template_id": "template",
        "style_template_name": "模板",
        "style_template_version": "1",
        "custom_style": null
    }))
    .expect("documented object profile fields should parse");
    assert_eq!(object.object_profile_id, 1);
    assert_eq!(object.chapter_id, Some(2));

    let scene = serde_json::from_value::<super::SceneProfileImageInput>(json!({
        "scene_profile_id": 4,
        "chapter_id": null,
        "style": null,
        "config_id": null,
        "style_template_id": null,
        "style_template_name": null,
        "style_template_version": null,
        "custom_style": "手绘"
    }))
    .expect("documented scene profile fields should parse");
    assert_eq!(scene.scene_profile_id, 4);
    assert_eq!(scene.custom_style.as_deref(), Some("手绘"));
}

#[test]
fn manga_page_reference_bindings_require_an_asset_for_every_slot() {
    let host = TestImageGenerationHost::default();
    let mut bindings = json!([{
        "character_name": "林岚",
        "bound_asset": null
    }]);

    let error =
        collect_bound_reference_images(&host, &bindings).expect_err("参考图未绑定时必须拒绝成图");
    assert!(error.to_string().contains("参考图“林岚”尚未绑定"));

    bindings[0]["bound_asset"] = json!({
        "asset_kind": "generated_image",
        "asset_id": 7,
        "image_path": "generated/lin-lan.png"
    });
    let references =
        collect_bound_reference_images(&host, &bindings).expect("bound_asset 应是唯一绑定状态");
    assert_eq!(references.len(), 1);
    assert_eq!(references[0].label, "林岚");
}

#[derive(Default)]
struct TestImageGenerationHost {
    generate_calls: AtomicUsize,
    persist_calls: AtomicUsize,
    validate_calls: AtomicUsize,
    remove_calls: AtomicUsize,
    lineage_calls: AtomicUsize,
    removed_paths: Mutex<Vec<String>>,
    fail_generate: bool,
    fail_generate_call: Option<usize>,
    fail_persist: bool,
    fail_validate: bool,
    fail_remove: bool,
    fail_lineage: bool,
    delete_workflow_on_generate: Option<Arc<DatabaseConnection>>,
}

#[async_trait]
impl ImageGenerationHostPort for TestImageGenerationHost {
    async fn resolve_image_config(
        &self,
        _config_id: Option<i64>,
    ) -> Result<image_gen_config::Model, AppError> {
        Ok(image_config())
    }

    fn runtime_config_snapshot(&self) -> Option<Value> {
        Some(json!({"profile": "image-generation-test"}))
    }

    fn read_reference_image(
        &self,
        label: &str,
        _relative_path: &str,
    ) -> Result<ImageReference, AppError> {
        Ok(ImageReference {
            label: label.to_string(),
            data_uri: "data:image/png;base64,AQ==".to_string(),
        })
    }

    async fn generate_image(
        &self,
        input: ImageGenerationProviderInput,
        _progress: &ImageGenerationProgress<'_>,
    ) -> Result<ImageGenerationProviderOutput, AppError> {
        let call = self.generate_calls.fetch_add(1, Ordering::SeqCst) + 1;
        if let Some(database) = &self.delete_workflow_on_generate {
            workflow_run::Entity::delete_by_id(input.workflow_run_id)
                .exec(database.as_ref())
                .await?;
        }
        if self.fail_generate || self.fail_generate_call == Some(call) {
            return Err(AppError::InvalidState {
                current: "provider".to_string(),
                operation: "测试 provider 失败".to_string(),
            });
        }
        Ok(ImageGenerationProviderOutput {
            bytes: png_1x1_bytes(),
            format: "png".to_string(),
            model: "test-provider-model".to_string(),
        })
    }

    fn persist_generated_image_file(
        &self,
        input: PersistGeneratedImageFileInput<'_>,
    ) -> Result<PersistedGeneratedImageFile, AppError> {
        self.persist_calls.fetch_add(1, Ordering::SeqCst);
        if self.fail_persist {
            return Err(AppError::InvalidState {
                current: "file".to_string(),
                operation: "测试图片写入失败".to_string(),
            });
        }
        Ok(PersistedGeneratedImageFile {
            relative_path: format!(
                "generated/project-{}/{}-{}.{}",
                input.project_id,
                input.image_type,
                input.file_extra.unwrap_or("main"),
                input.format
            ),
            file_size: i64::try_from(input.bytes.len())
                .map_err(|_| AppError::InvalidArgument("测试图片过大".to_string()))?,
        })
    }

    fn validate_generated_image_file(
        &self,
        _relative_path: &str,
        _label: &str,
    ) -> Result<(), AppError> {
        self.validate_calls.fetch_add(1, Ordering::SeqCst);
        if self.fail_validate {
            return Err(AppError::InvalidState {
                current: "checkpoint".to_string(),
                operation: "测试断点文件校验失败".to_string(),
            });
        }
        Ok(())
    }

    fn inspect_generated_image_file(
        &self,
        _relative_path: &str,
        _label: &str,
    ) -> Result<super::GeneratedImageArtifactFile, AppError> {
        Ok(super::GeneratedImageArtifactFile {
            content_type: "image/png".to_string(),
            sha256: "test-sha256".to_string(),
            byte_size: png_1x1_bytes().len() as u64,
        })
    }

    async fn remove_generated_image_file(
        &self,
        relative_path: &str,
        _label: &str,
    ) -> Result<(), AppError> {
        self.remove_calls.fetch_add(1, Ordering::SeqCst);
        self.removed_paths
            .lock()
            .expect("removed paths lock")
            .push(relative_path.to_string());
        if self.fail_remove {
            return Err(AppError::InvalidState {
                current: "file".to_string(),
                operation: "测试图片清理失败".to_string(),
            });
        }
        Ok(())
    }

    async fn sync_project_asset_lineage(&self, _project_id: i64) -> Result<(), AppError> {
        self.lineage_calls.fetch_add(1, Ordering::SeqCst);
        if self.fail_lineage {
            return Err(AppError::InvalidState {
                current: "lineage".to_string(),
                operation: "测试 lineage 同步失败".to_string(),
            });
        }
        Ok(())
    }
}

async fn setup_context() -> (Arc<DatabaseConnection>, OrchestratorContext) {
    let database = Arc::new(a4n_migration::test_support::connect_test_database().await);
    let context = TestOrchestratorBuilder::new(Arc::clone(&database))
        .build()
        .await;
    (database, context)
}

async fn insert_project(database: &DatabaseConnection) -> i64 {
    create_freeform_video_project_record(
        database,
        1,
        CreateFreeformVideoProjectParams {
            title: "Image Generation application 测试项目".to_string(),
            description: String::new(),
        },
    )
    .await
    .expect("测试项目应创建成功")
    .id
}

async fn image_workflow_and_checkpoint(
    database: &DatabaseConnection,
    project_id: i64,
) -> (workflow_run::Model, workflow_checkpoint::Model) {
    let workflow = workflow_run::Entity::find()
        .filter(workflow_run::Column::ProjectId.eq(project_id))
        .filter(workflow_run::Column::WorkflowKind.eq("image_generation"))
        .one(database)
        .await
        .expect("应可读取 workflow")
        .expect("图片 workflow 应存在");
    let checkpoint = workflow_checkpoint::Entity::find()
        .filter(workflow_checkpoint::Column::WorkflowRunId.eq(workflow.id))
        .one(database)
        .await
        .expect("应可读取 workflow checkpoint")
        .expect("图片 workflow checkpoint 应存在");
    (workflow, checkpoint)
}

fn bind_existing_item(
    input: &mut ImageGenerationExecutionInput,
    workflow: &workflow_run::Model,
    checkpoint: &workflow_checkpoint::Model,
) {
    input.workflow = ImageGenerationWorkflowBinding::ExistingItem {
        workflow_run_id: workflow.id,
        step_key: checkpoint.step_key.clone(),
        step_order: checkpoint.step_order,
        fingerprint_json: checkpoint
            .input_fingerprint_json
            .clone()
            .expect("checkpoint 指纹应存在"),
    };
}

fn image_config() -> image_gen_config::Model {
    let now = chrono::Utc::now().naive_utc();
    image_gen_config::Model {
        id: 7,
        name: "测试图片配置".to_string(),
        backend: "test-image-provider".to_string(),
        base_url: Some("http://127.0.0.1:47645".to_string()),
        api_key_encrypted: None,
        model: "configured-image-model".to_string(),
        max_concurrent: 1,
        default_width: 1024,
        default_height: 576,
        is_default: true,
        created_at: now,
        updated_at: now,
    }
}

fn image_request() -> ImageGenerationRequest {
    ImageGenerationRequest {
        prompt: "雨夜中的仓库入口，电影感构图".to_string(),
        width: 1024,
        height: 576,
        model: "requested-image-model".to_string(),
        reference_images: vec![ImageReference {
            label: "角色参考".to_string(),
            data_uri: "data:image/png;base64,AAECAwQ=".to_string(),
        }],
    }
}

fn execution_input(project_id: i64) -> ImageGenerationExecutionInput {
    ImageGenerationExecutionInput {
        project_id,
        chapter_id: None,
        image_type: "scene".to_string(),
        command: "image_scene".to_string(),
        file_extra: Some("scene-1".to_string()),
        file_context: "场景图片".to_string(),
        config: image_config(),
        request: image_request(),
        width: 1024,
        height: 576,
        panel_id: Some("scene-1".to_string()),
        metadata_json: Some(json!({
            "reference_image_ids": [11, 12],
            "source": "test",
        })),
        runtime_config_snapshot: Some(json!({
            "backend": "sidecar",
            "max_concurrent": 1,
        })),
        workflow: ImageGenerationWorkflowBinding::Owned,
        post_commit: ImageGenerationPostCommitAction::None,
    }
}

#[test]
fn image_request_trace_records_aspect_ratio_and_redacts_reference_payload() {
    let request = image_request();
    let trace = image_generation_request_trace_json(&image_config(), &request);
    let serialized = serde_json::to_string(&trace).expect("trace 应可序列化");

    assert_eq!(test_image_request_aspect_ratio(1024, 576), "16:9");
    assert_eq!(trace["request"]["aspect_ratio"], "16:9");
    assert_eq!(trace["request"]["reference_image_count"], 1);
    assert_eq!(
        trace["request"]["reference_images"][0]["media_type"],
        "image/png"
    );
    assert_eq!(
        trace["request"]["reference_images"][0]["data_uri_length"],
        request.reference_images[0].data_uri.len()
    );
    assert!(
        trace["request"]["reference_images"][0]["data_uri_sha256"]
            .as_str()
            .is_some_and(|value| value.len() == 64)
    );
    assert!(!serialized.contains(&request.reference_images[0].data_uri));
}

#[tokio::test]
async fn image_generation_application_persists_complete_trace() {
    let (database, context) = setup_context().await;
    let project_id = insert_project(database.as_ref()).await;
    let host = TestImageGenerationHost::default();

    let outcome = context
        .run_image_generation_validated(
            &host,
            execution_input(project_id),
            &ImageGenerationProgress::none(),
        )
        .await
        .expect("单图完整 application 应成功");

    assert!(!outcome.restored);
    assert_eq!(outcome.summary.project_id, project_id);
    assert_eq!(outcome.summary.model, "test-provider-model");
    assert_eq!((outcome.summary.width, outcome.summary.height), (1, 1));
    assert_eq!(host.generate_calls.load(Ordering::SeqCst), 1);
    assert_eq!(host.persist_calls.load(Ordering::SeqCst), 1);
    assert_eq!(host.remove_calls.load(Ordering::SeqCst), 0);

    let image = generated_image::Entity::find_by_id(outcome.summary.id)
        .one(database.as_ref())
        .await
        .expect("应可读取生成图片")
        .expect("生成图片应存在");
    let runs = generation_run::Entity::find()
        .filter(generation_run::Column::ProjectId.eq(project_id))
        .all(database.as_ref())
        .await
        .expect("应可读取 generation run");
    let workflow = workflow_run::Entity::find()
        .filter(workflow_run::Column::ProjectId.eq(project_id))
        .filter(workflow_run::Column::WorkflowKind.eq("image_generation"))
        .one(database.as_ref())
        .await
        .expect("应可读取 workflow")
        .expect("图片 workflow 应存在");
    let checkpoints = workflow_checkpoint::Entity::find()
        .filter(workflow_checkpoint::Column::WorkflowRunId.eq(workflow.id))
        .all(database.as_ref())
        .await
        .expect("应可读取 workflow checkpoint");
    let metadata = image.metadata_json.expect("生成 metadata 应存在");

    assert_eq!((image.width, image.height), (1, 1));
    assert_eq!(metadata["metadata_schema_version"], 1);
    assert_eq!(metadata["reference_image_ids"], json!([11, 12]));
    assert_eq!(metadata["generation"]["command"], "image_scene");
    assert_eq!(metadata["generation"]["width"], 1);
    assert_eq!(metadata["generation"]["height"], 1);
    assert_eq!(
        metadata["generation"]["runtime_config_snapshot"]["backend"],
        "sidecar"
    );
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].workflow, "image_generation");
    assert_eq!(runs[0].status, "completed");
    assert_eq!(runs[0].workflow_run_id, Some(workflow.id));
    assert_eq!(
        runs[0].media_refs_json,
        Some(json!({"reference_image_ids": [11, 12]}))
    );
    assert_eq!(workflow.status, "completed");
    assert_eq!(checkpoints.len(), 1);
    assert_eq!(checkpoints[0].step_key, "image_generation.image_scene");
    assert_eq!(checkpoints[0].status, "completed");
}

#[tokio::test]
async fn batch_image_generation_keeps_successes_after_item_failure() {
    let (database, context) = setup_context().await;
    let project_id = insert_project(database.as_ref()).await;
    let host = TestImageGenerationHost {
        fail_generate_call: Some(1),
        ..Default::default()
    };
    let items = (0..2)
        .map(|index| BatchImageExecutionItem {
            index,
            request: image_request(),
            image_type: "scene".to_string(),
            chapter_id: None,
            panel_id: Some(format!("scene-{index}")),
            metadata_json: None,
            config: image_config(),
        })
        .collect();

    let outcome = context
        .run_batch_image_generation_validated(
            &host,
            BatchImageGenerationApplicationInput {
                project_id,
                requests_json: json!([{"prompt": "first"}, {"prompt": "second"}]),
                items,
                runtime_config_snapshot: None,
            },
            &ImageGenerationProgress::none(),
        )
        .await
        .expect("best-effort batch 应保留后续成功项");

    assert_eq!(host.generate_calls.load(Ordering::SeqCst), 2);
    assert_eq!(outcome.succeeded(), 1);
    assert_eq!(outcome.failed_item_keys, ["image-request-1"]);
    assert_eq!(outcome.items[0].item_key, "image-request-2");
}

#[tokio::test]
async fn image_generation_application_restores_without_provider_or_persist() {
    let (database, context) = setup_context().await;
    let project_id = insert_project(database.as_ref()).await;
    let input = execution_input(project_id);
    let first_host = TestImageGenerationHost::default();

    let first = context
        .run_image_generation_validated(
            &first_host,
            input.clone(),
            &ImageGenerationProgress::none(),
        )
        .await
        .expect("首次生成应成功");
    let (workflow, checkpoint) = image_workflow_and_checkpoint(database.as_ref(), project_id).await;
    let mut restore_input = input;
    bind_existing_item(&mut restore_input, &workflow, &checkpoint);
    let restore_host = TestImageGenerationHost::default();
    let restored = context
        .run_image_generation_validated(
            &restore_host,
            restore_input,
            &ImageGenerationProgress::none(),
        )
        .await
        .expect("相同输入应从断点恢复");

    assert!(restored.restored);
    assert_eq!(restored.summary.id, first.summary.id);
    assert_eq!(restore_host.generate_calls.load(Ordering::SeqCst), 0);
    assert_eq!(restore_host.persist_calls.load(Ordering::SeqCst), 0);
    assert_eq!(restore_host.validate_calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        generated_image::Entity::find()
            .filter(generated_image::Column::ProjectId.eq(project_id))
            .count(database.as_ref())
            .await
            .expect("应可统计图片"),
        1
    );
}

#[tokio::test]
async fn restored_checkpoint_rejects_image_from_another_project() {
    let (database, context) = setup_context().await;
    let source_project_id = insert_project(database.as_ref()).await;
    let target_project_id = insert_project(database.as_ref()).await;
    let host = TestImageGenerationHost::default();

    context
        .run_image_generation_validated(
            &host,
            execution_input(source_project_id),
            &ImageGenerationProgress::none(),
        )
        .await
        .expect("首次生成应成功");
    let (workflow, checkpoint) =
        image_workflow_and_checkpoint(database.as_ref(), source_project_id).await;
    let mut restore_input = execution_input(target_project_id);
    bind_existing_item(&mut restore_input, &workflow, &checkpoint);
    let restore_host = TestImageGenerationHost::default();

    let error = context
        .run_image_generation_validated(
            &restore_host,
            restore_input,
            &ImageGenerationProgress::none(),
        )
        .await
        .expect_err("其他项目图片断点必须拒绝");

    assert!(error.to_string().contains("指向其他项目图片"));
    assert_eq!(restore_host.generate_calls.load(Ordering::SeqCst), 0);
    assert_eq!(restore_host.validate_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn restored_checkpoint_rejects_file_path_mismatch() {
    let (database, context) = setup_context().await;
    let project_id = insert_project(database.as_ref()).await;
    let host = TestImageGenerationHost::default();

    context
        .run_image_generation_validated(
            &host,
            execution_input(project_id),
            &ImageGenerationProgress::none(),
        )
        .await
        .expect("首次生成应成功");
    let (workflow, checkpoint) = image_workflow_and_checkpoint(database.as_ref(), project_id).await;
    let mut checkpoint_json = checkpoint
        .checkpoint_json
        .clone()
        .expect("checkpoint payload 应存在");
    checkpoint_json["file_path"] = json!("generated/tampered.png");
    let mut active: workflow_checkpoint::ActiveModel = checkpoint.clone().into();
    active.checkpoint_json = Set(Some(checkpoint_json));
    active
        .update(database.as_ref())
        .await
        .expect("测试 checkpoint 应可更新");
    let mut restore_input = execution_input(project_id);
    bind_existing_item(&mut restore_input, &workflow, &checkpoint);
    let restore_host = TestImageGenerationHost::default();

    let error = context
        .run_image_generation_validated(
            &restore_host,
            restore_input,
            &ImageGenerationProgress::none(),
        )
        .await
        .expect_err("断点与 DB 图片路径不一致必须拒绝");

    assert!(error.to_string().contains("checkpoint 图片路径不一致"));
    assert_eq!(restore_host.generate_calls.load(Ordering::SeqCst), 0);
    assert_eq!(restore_host.validate_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn restored_checkpoint_requires_host_file_validation() {
    let (database, context) = setup_context().await;
    let project_id = insert_project(database.as_ref()).await;
    let host = TestImageGenerationHost::default();

    context
        .run_image_generation_validated(
            &host,
            execution_input(project_id),
            &ImageGenerationProgress::none(),
        )
        .await
        .expect("首次生成应成功");
    let (workflow, checkpoint) = image_workflow_and_checkpoint(database.as_ref(), project_id).await;
    let mut restore_input = execution_input(project_id);
    bind_existing_item(&mut restore_input, &workflow, &checkpoint);
    let restore_host = TestImageGenerationHost {
        fail_validate: true,
        ..Default::default()
    };

    let error = context
        .run_image_generation_validated(
            &restore_host,
            restore_input,
            &ImageGenerationProgress::none(),
        )
        .await
        .expect_err("断点物理文件校验失败必须拒绝");

    assert!(error.to_string().contains("测试断点文件校验失败"));
    assert_eq!(restore_host.generate_calls.load(Ordering::SeqCst), 0);
    assert_eq!(restore_host.validate_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn image_generation_application_rejects_non_object_metadata() {
    let (database, context) = setup_context().await;
    let project_id = insert_project(database.as_ref()).await;
    let host = TestImageGenerationHost::default();
    let mut input = execution_input(project_id);
    input.metadata_json = Some(json!(["invalid"]));

    let error = context
        .run_image_generation_validated(&host, input, &ImageGenerationProgress::none())
        .await
        .expect_err("非对象 metadata 必须失败");

    assert!(error.to_string().contains("metadata_json 必须是对象"));
    assert_eq!(host.generate_calls.load(Ordering::SeqCst), 1);
    assert_eq!(host.persist_calls.load(Ordering::SeqCst), 0);
    let workflow = workflow_run::Entity::find()
        .filter(workflow_run::Column::ProjectId.eq(project_id))
        .filter(workflow_run::Column::WorkflowKind.eq("image_generation"))
        .one(database.as_ref())
        .await
        .expect("应可读取 workflow")
        .expect("失败 workflow 应存在");
    assert_eq!(workflow.status, "failed");
}

#[tokio::test]
async fn provider_failure_marks_owned_workflow_failed() {
    let (database, context) = setup_context().await;
    let project_id = insert_project(database.as_ref()).await;
    let host = TestImageGenerationHost {
        fail_generate: true,
        ..Default::default()
    };

    let error = context
        .run_image_generation_validated(
            &host,
            execution_input(project_id),
            &ImageGenerationProgress::none(),
        )
        .await
        .expect_err("provider 失败必须显式返回");

    assert!(error.to_string().contains("测试 provider 失败"));
    assert_eq!(host.persist_calls.load(Ordering::SeqCst), 0);
    let workflow = workflow_run::Entity::find()
        .filter(workflow_run::Column::ProjectId.eq(project_id))
        .filter(workflow_run::Column::WorkflowKind.eq("image_generation"))
        .one(database.as_ref())
        .await
        .expect("应可读取 workflow")
        .expect("失败 workflow 应存在");
    assert_eq!(workflow.status, "failed");
    assert!(
        workflow
            .error_message
            .as_deref()
            .is_some_and(|message| message.contains("测试 provider 失败"))
    );
}

#[tokio::test]
async fn cancellation_before_provider_marks_owned_workflow_cancelled() {
    let (database, context) = setup_context().await;
    let project_id = insert_project(database.as_ref()).await;
    let host = TestImageGenerationHost::default();
    let cancel_token = CancellationToken::new();
    cancel_token.cancel();
    let progress = ImageGenerationProgress::new(None, Some(&cancel_token));

    let error = context
        .run_image_generation_validated(&host, execution_input(project_id), &progress)
        .await
        .expect_err("provider 前取消必须显式失败");

    assert!(error.to_string().contains("图片生成已停止"));
    assert_eq!(host.generate_calls.load(Ordering::SeqCst), 0);
    assert_eq!(host.persist_calls.load(Ordering::SeqCst), 0);
    let workflow = workflow_run::Entity::find()
        .filter(workflow_run::Column::ProjectId.eq(project_id))
        .filter(workflow_run::Column::WorkflowKind.eq("image_generation"))
        .one(database.as_ref())
        .await
        .expect("应可读取 workflow")
        .expect("取消 workflow 应存在");
    assert_eq!(workflow.status, "cancelled");
}

#[tokio::test]
async fn primary_and_finalize_failures_are_both_returned() {
    let (database, context) = setup_context().await;
    let project_id = insert_project(database.as_ref()).await;
    let host = TestImageGenerationHost {
        delete_workflow_on_generate: Some(Arc::clone(&database)),
        ..Default::default()
    };

    let error = context
        .run_image_generation_validated(
            &host,
            execution_input(project_id),
            &ImageGenerationProgress::none(),
        )
        .await
        .expect_err("主错误和 finalize 错误必须同时返回");
    let message = error.to_string();

    assert!(message.contains("工作流失败状态写入失败"));
    assert!(message.contains("未找到") || message.contains("数据库"));
    assert_eq!(host.persist_calls.load(Ordering::SeqCst), 1);
    assert_eq!(host.remove_calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        generated_image::Entity::find()
            .filter(generated_image::Column::ProjectId.eq(project_id))
            .count(database.as_ref())
            .await
            .expect("应可统计已提交图片"),
        1
    );
}

#[tokio::test]
async fn database_failure_after_file_persist_removes_created_file() {
    let (_database, context) = setup_context().await;
    let host = TestImageGenerationHost::default();
    let mut input = execution_input(9_999_999);
    input.workflow = ImageGenerationWorkflowBinding::Attached { workflow_run_id: 7 };

    context
        .run_image_generation_validated(&host, input, &ImageGenerationProgress::none())
        .await
        .expect_err("数据库失败必须显式返回");

    assert_eq!(host.generate_calls.load(Ordering::SeqCst), 1);
    assert_eq!(host.persist_calls.load(Ordering::SeqCst), 1);
    assert_eq!(host.remove_calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        host.removed_paths
            .lock()
            .expect("removed paths lock")
            .as_slice(),
        ["generated/project-9999999/scene-scene-1.png"]
    );
}

#[tokio::test]
async fn database_and_file_cleanup_failures_are_both_returned() {
    let (_database, context) = setup_context().await;
    let host = TestImageGenerationHost {
        fail_remove: true,
        ..Default::default()
    };
    let mut input = execution_input(9_999_998);
    input.workflow = ImageGenerationWorkflowBinding::Attached { workflow_run_id: 8 };

    let error = context
        .run_image_generation_validated(&host, input, &ImageGenerationProgress::none())
        .await
        .expect_err("数据库和文件清理双失败必须显式返回");
    let AppError::InvalidState { current, operation } = error else {
        panic!("双失败必须合并为 InvalidState");
    };

    assert!(!current.trim().is_empty());
    assert!(current.contains("项目") || current.contains("生成运行"));
    assert!(operation.contains("清理已写出的图片文件失败"));
    assert!(operation.contains("测试图片清理失败"));
    assert_eq!(host.remove_calls.load(Ordering::SeqCst), 1);
}

async fn insert_manga_page_fixture(
    database: &DatabaseConnection,
) -> (
    i64,
    chapter::Model,
    chapter_manga_prompt::Model,
    generated_image::Model,
) {
    let now = chrono::Utc::now().naive_utc();
    let root_project = novel_project::ActiveModel {
        user_id: Set(1),
        title: Set("漫画根文本项目".to_string()),
        description: Set("漫画页面测试".to_string()),
        project_type: Set("novel_long".to_string()),
        project_kind: Set("text".to_string()),
        source_sync_status: Set("current".to_string()),
        status: Set("writing".to_string()),
        target_chapters: Set(1),
        current_chapter: Set(1),
        total_chapters: Set(Some(1)),
        chapters_per_part: Set(Some(1)),
        avatar_svg: Set(None),
        avatar_animal: Set(None),
        raw_blueprint: Set(None),
        import_analysis_progress: Set(None),
        export_config: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(database)
    .await
    .expect("漫画根文本项目应创建成功");
    let chapter = chapter::ActiveModel {
        project_id: Set(root_project.id),
        outline_id: Set(None),
        chapter_number: Set(1),
        title: Set(Some("测试章节".to_string())),
        content: Set("漫画页面测试正文".to_string()),
        word_count: Set(8),
        status: Set("completed".to_string()),
        summary: Set(None),
        style_tags: Set(None),
        analysis_data: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(database)
    .await
    .expect("测试章节应创建成功");
    let derived_project = create_derived_project_record(
        database,
        1,
        CreateDerivedProjectParams {
            root_project_id: root_project.id,
            derived_kind: PROJECT_KIND_COMIC.to_string(),
            title: "漫画派生项目".to_string(),
            description: String::new(),
            chapter_scope: DerivedChapterScope {
                start_chapter: 1,
                end_chapter: 1,
            },
            creation_options_json: json!({}),
        },
    )
    .await
    .expect("漫画派生项目应创建成功");
    let project_id = derived_project.id;
    let old_image = generated_image::ActiveModel {
        project_id: Set(project_id),
        image_type: Set("manga_page".to_string()),
        file_path: Set("generated/old-page1.png".to_string()),
        prompt: Set("旧漫画页提示词".to_string()),
        model: Set("old-model".to_string()),
        width: Set(1024),
        height: Set(1536),
        file_size: Set(Some(10)),
        ai_score: Set(None),
        is_favorite: Set(false),
        chapter_id: Set(Some(chapter.id)),
        panel_id: Set(Some("page1".to_string())),
        metadata_json: Set(Some(json!({"page_number": 1}))),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(database)
    .await
    .expect("旧漫画页图片应创建成功");
    let page_prompt = chapter_manga_prompt::ActiveModel {
        chapter_id: Set(chapter.id),
        project_id: Set(project_id),
        page_number: Set(1),
        prompt_type: Set("page_generation".to_string()),
        image_prompt: Set("新漫画页提示词".to_string()),
        dialogue_text: Set(None),
        characters_in_frame: Set(None),
        layout: Set(Some(json!({"page_number": 1}))),
        generated_image_id: Set(Some(old_image.id)),
        status: Set("generating".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(database)
    .await
    .expect("漫画页提示词应创建成功");
    (project_id, chapter, page_prompt, old_image)
}

fn manga_page_execution_input(
    project_id: i64,
    chapter_id: i64,
    page_prompt_id: i64,
) -> ImageGenerationExecutionInput {
    let mut input = execution_input(project_id);
    input.chapter_id = Some(chapter_id);
    input.image_type = "manga_page".to_string();
    input.command = "generate_manga_page".to_string();
    input.file_extra = Some("page1".to_string());
    input.file_context = "漫画第 1 页".to_string();
    input.panel_id = Some("page1".to_string());
    input.width = 1024;
    input.height = 1536;
    input.request.width = 1024;
    input.request.height = 1536;
    input.post_commit = ImageGenerationPostCommitAction::MangaPage {
        page_prompt_id,
        page_number: 1,
    };
    input
}

#[tokio::test]
async fn manga_page_post_commit_binds_new_image_and_removes_old_image() {
    let (database, context) = setup_context().await;
    let (project_id, chapter, page_prompt, old_image) =
        insert_manga_page_fixture(database.as_ref()).await;
    let host = TestImageGenerationHost::default();

    let outcome = context
        .run_image_generation_validated(
            &host,
            manga_page_execution_input(project_id, chapter.id, page_prompt.id),
            &ImageGenerationProgress::none(),
        )
        .await
        .expect("漫画页 post-commit 应成功");

    let updated_prompt = chapter_manga_prompt::Entity::find_by_id(page_prompt.id)
        .one(database.as_ref())
        .await
        .expect("应可读取漫画页提示词")
        .expect("漫画页提示词应存在");
    let old_image_row = generated_image::Entity::find_by_id(old_image.id)
        .one(database.as_ref())
        .await
        .expect("应可查询旧图片");
    let new_image_row = generated_image::Entity::find_by_id(outcome.summary.id)
        .one(database.as_ref())
        .await
        .expect("应可查询新图片");

    assert_eq!(updated_prompt.generated_image_id, Some(outcome.summary.id));
    assert_eq!(updated_prompt.status, "completed");
    assert!(old_image_row.is_none());
    assert!(new_image_row.is_some());
    assert_eq!(host.remove_calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        host.removed_paths
            .lock()
            .expect("removed paths lock")
            .as_slice(),
        ["generated/old-page1.png"]
    );
    assert_eq!(host.lineage_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn manga_page_lineage_failure_keeps_committed_workflow_completed() {
    let (database, context) = setup_context().await;
    let (project_id, chapter, page_prompt, _old_image) =
        insert_manga_page_fixture(database.as_ref()).await;
    let host = TestImageGenerationHost {
        fail_lineage: true,
        ..Default::default()
    };

    let outcome = context
        .run_image_generation_validated(
            &host,
            manga_page_execution_input(project_id, chapter.id, page_prompt.id),
            &ImageGenerationProgress::none(),
        )
        .await
        .expect("资产谱系是可回填投影，不得覆盖已提交结果");

    assert!(outcome.summary.id > 0);
    let workflow = workflow_run::Entity::find()
        .filter(workflow_run::Column::ProjectId.eq(project_id))
        .filter(workflow_run::Column::WorkflowKind.eq("image_generation"))
        .one(database.as_ref())
        .await
        .expect("应可读取 workflow")
        .expect("完成 workflow 应存在");
    assert_eq!(workflow.status, "completed");
    assert_eq!(host.lineage_calls.load(Ordering::SeqCst), 1);
}

#[test]
fn generated_image_summary_contract_remains_json_compatible() {
    let summary = super::GeneratedImageSummary {
        id: 1,
        project_id: 2,
        image_type: "scene".to_string(),
        file_path: "generated/scene.png".to_string(),
        prompt: "测试".to_string(),
        model: "model".to_string(),
        width: 1024,
        height: 576,
        file_size: Some(5),
        ai_score: None,
        is_favorite: false,
        is_system_managed: true,
        chapter_id: None,
        panel_id: None,
        metadata_json: Some(Value::Object(Default::default())),
        created_at: "2026-01-01 00:00:00".to_string(),
    };

    let value = serde_json::to_value(summary).expect("summary 应可序列化");
    assert_eq!(value["image_type"], "scene");
    assert_eq!(value["width"], 1024);
}
