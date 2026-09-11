use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use a4n_api_contract::portrait::{
    GenerateMissingPortraitsInput, GeneratePortraitInput, PortraitType, UploadPortraitInput,
};
use a4n_db::{
    DatabaseConnection,
    project::{CreateFreeformVideoProjectParams, create_freeform_video_project_record},
};
use a4n_db_models::models::{
    ai::generated_image,
    character::character_portrait,
    generation::{generation_run, workflow_checkpoint, workflow_run},
};
use a4n_domain::media::GeneratedMediaArtifactFile;
use a4n_error::AppError;
use a4n_prompt::tera_engine::TeraPromptEngine;
use async_trait::async_trait;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};
use serde_json::json;
use tokio_util::sync::CancellationToken;

use crate::{
    OrchestratorContext,
    media::{
        PersistGeneratedImageFileInput, PersistedGeneratedImageFile,
        PrepareGeneratedImageFileInput, PreparedGeneratedImageFile,
    },
    test_support::TestOrchestratorBuilder,
};

use super::{
    DecodedPortraitImage, GeneratedPortraitImage, PortraitGenerationHostPort,
    PortraitImageProviderInput, image::generate_portrait_image,
};

#[derive(Default)]
struct TestPortraitHost {
    fail_generate_calls: usize,
    generate_calls: AtomicUsize,
    decode_calls: AtomicUsize,
    persist_calls: AtomicUsize,
    validate_calls: AtomicUsize,
    remove_calls: AtomicUsize,
}

#[async_trait]
impl PortraitGenerationHostPort for TestPortraitHost {
    async fn generate_image(
        &self,
        input: PortraitImageProviderInput,
        _cancel_token: CancellationToken,
    ) -> Result<GeneratedPortraitImage, AppError> {
        let call = self.generate_calls.fetch_add(1, Ordering::SeqCst);
        if call < self.fail_generate_calls {
            return Err(AppError::InvalidState {
                current: "test_provider_failure".to_string(),
                operation: "生成测试立绘".to_string(),
            });
        }
        Ok(GeneratedPortraitImage {
            bytes: vec![1, 2, 3, 4],
            format: "png".to_string(),
            model: "test-image-model".to_string(),
            provider: "test-image-provider".to_string(),
            request_json: json!({"prompt": input.prompt}),
            width: 512,
            height: 768,
        })
    }

    fn decode_uploaded_image(
        &self,
        _file_name: &str,
        _mime_type: &str,
        _data_base64: &str,
    ) -> Result<DecodedPortraitImage, AppError> {
        self.decode_calls.fetch_add(1, Ordering::SeqCst);
        Ok(DecodedPortraitImage {
            bytes: vec![5, 6, 7, 8],
            format: "png".to_string(),
            model: "external-import".to_string(),
            width: 256,
            height: 512,
        })
    }

    fn prepare_portrait_file(
        &self,
        input: PrepareGeneratedImageFileInput<'_>,
    ) -> Result<PreparedGeneratedImageFile, AppError> {
        let final_path = format!(
            "projects/{}/portraits/portrait_{}_{}.{}",
            input.project_id,
            input.file_extra.unwrap_or("main"),
            input.publication_id,
            input.format
        );
        Ok(PreparedGeneratedImageFile {
            staging_path: format!("{final_path}.part"),
            final_path,
        })
    }

    fn persist_portrait_file(
        &self,
        input: PersistGeneratedImageFileInput<'_>,
        prepared: &PreparedGeneratedImageFile,
    ) -> Result<PersistedGeneratedImageFile, AppError> {
        self.persist_calls.fetch_add(1, Ordering::SeqCst);
        Ok(PersistedGeneratedImageFile {
            relative_path: prepared.final_path.clone(),
            file_size: i64::try_from(input.bytes.len())
                .map_err(|_| AppError::InvalidArgument("测试图片过大".to_string()))?,
        })
    }

    fn inspect_portrait_file(
        &self,
        _relative_path: &str,
        _label: &str,
    ) -> Result<GeneratedMediaArtifactFile, AppError> {
        Ok(GeneratedMediaArtifactFile {
            content_type: "image/png".to_string(),
            sha256: "a".repeat(64),
            byte_size: 4,
        })
    }

    fn validate_portrait_file(&self, _relative_path: &str, _label: &str) -> Result<(), AppError> {
        self.validate_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    async fn remove_portrait_file(
        &self,
        _relative_path: &str,
        _label: &str,
    ) -> Result<(), AppError> {
        self.remove_calls.fetch_add(1, Ordering::SeqCst);
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

fn prompt_engine() -> TeraPromptEngine {
    TeraPromptEngine::from_directory(&prompts_dir()).expect("测试 prompt 模板应可加载")
}

fn prompts_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../prompts")
}

async fn insert_project_and_profile(
    database: &DatabaseConnection,
) -> (
    a4n_db_models::models::project::novel_project::Model,
    a4n_db_models::models::character::protagonist_profile::Model,
) {
    let project = create_freeform_video_project_record(
        database,
        1,
        CreateFreeformVideoProjectParams {
            title: "Portrait application 测试项目".to_string(),
            description: String::new(),
        },
    )
    .await
    .expect("测试项目应创建成功");
    let profile = a4n_db::character::create_profile(
        database,
        project.id,
        "测试角色",
        json!({
            "explicit_attributes": {
                "gender": "女",
                "age": "二十岁",
                "appearance": "短发，青色外套"
            },
            "implicit_attributes": {},
            "social_attributes": {}
        }),
    )
    .await
    .expect("测试角色档案应创建成功");
    (project, profile)
}

#[tokio::test]
async fn portrait_application_rejects_cancellation_before_provider() {
    let (_database, context) = setup_context().await;
    let host = TestPortraitHost::default();
    let cancel_token = CancellationToken::new();
    cancel_token.cancel();

    let error = context
        .generate_portrait_validated(
            1,
            GeneratePortraitInput {
                project_id: 1,
                character_name: "不会被查询".to_string(),
                style: None,
                expression: None,
                pose: None,
                reference_image_ids: Vec::new(),
                width: 1024,
                height: 1536,
            },
            &prompt_engine(),
            &host,
            None,
            cancel_token,
        )
        .await
        .expect_err("取消必须显式失败");

    assert!(matches!(error, AppError::Cancelled));
    assert_eq!(host.generate_calls.load(Ordering::SeqCst), 0);
    assert_eq!(host.persist_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn portrait_application_rejects_invalid_reference_ids_before_host() {
    let (_database, context) = setup_context().await;
    let host = TestPortraitHost::default();

    let error = generate_portrait_image(
        &context,
        &host,
        1,
        "人物三视图提示词",
        1024,
        1536,
        &[0],
        None,
        CancellationToken::new(),
    )
    .await
    .expect_err("非法参考图 ID 必须失败");

    assert!(matches!(error, AppError::InvalidArgument(_)));
    assert_eq!(host.generate_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn portrait_application_rejects_blank_upload_prompt_before_host_io() {
    let (database, context) = setup_context().await;
    let (project, profile) = insert_project_and_profile(database.as_ref()).await;
    let host = TestPortraitHost::default();

    let error = context
        .upload_portrait_validated(
            1,
            UploadPortraitInput {
                project_id: project.id,
                profile_id: profile.id,
                file_name: "portrait.png".to_string(),
                mime_type: "image/png".to_string(),
                data_base64: "ignored".to_string(),
                portrait_type: PortraitType::FullBody,
                prompt: "  \n\t ".to_string(),
                expression: None,
                pose: None,
                set_primary: false,
            },
            &host,
        )
        .await
        .expect_err("空立绘描述必须失败");

    assert!(
        matches!(error, AppError::InvalidArgument(message) if message.contains("立绘描述不能为空"))
    );
    assert_eq!(host.decode_calls.load(Ordering::SeqCst), 0);
    assert_eq!(host.persist_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn portrait_application_persists_complete_missing_portrait_trace() {
    let (database, context) = setup_context().await;
    let (project, profile) = insert_project_and_profile(database.as_ref()).await;
    let host = TestPortraitHost::default();

    let portraits = context
        .generate_missing_portraits_validated(
            1,
            GenerateMissingPortraitsInput {
                project_id: project.id,
                width: 1024,
                height: 1536,
            },
            &host,
            None,
            CancellationToken::new(),
        )
        .await
        .expect("缺失立绘完整 application 应成功");

    assert_eq!(portraits.items.len(), 1);
    assert_eq!(portraits.items[0].summary.profile_id, profile.id);
    assert_eq!(portraits.items[0].summary.project_id, project.id);
    assert_eq!(
        portraits.items[0].summary.portrait_type,
        PortraitType::HalfBody
    );
    assert!(portraits.items[0].summary.is_primary);
    assert_eq!(host.generate_calls.load(Ordering::SeqCst), 1);
    assert_eq!(host.persist_calls.load(Ordering::SeqCst), 1);
    assert_eq!(host.remove_calls.load(Ordering::SeqCst), 0);

    let portrait_count = character_portrait::Entity::find()
        .filter(character_portrait::Column::ProjectId.eq(project.id))
        .count(database.as_ref())
        .await
        .expect("应可统计立绘记录");
    let generated_image_count = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.eq(project.id))
        .filter(generated_image::Column::ImageType.eq("portrait"))
        .count(database.as_ref())
        .await
        .expect("应可统计 generated_image 镜像");
    let generation_runs = generation_run::Entity::find()
        .filter(generation_run::Column::ProjectId.eq(project.id))
        .all(database.as_ref())
        .await
        .expect("应可读取生成运行记录");
    let workflow = workflow_run::Entity::find()
        .filter(workflow_run::Column::ProjectId.eq(project.id))
        .filter(workflow_run::Column::WorkflowKind.eq("missing_portrait_generation"))
        .one(database.as_ref())
        .await
        .expect("应可读取工作流")
        .expect("缺失立绘工作流应存在");
    let checkpoints = workflow_checkpoint::Entity::find()
        .filter(workflow_checkpoint::Column::WorkflowRunId.eq(workflow.id))
        .all(database.as_ref())
        .await
        .expect("应可读取工作流检查点");

    assert_eq!(portrait_count, 1);
    assert_eq!(generated_image_count, 1);
    assert_eq!(generation_runs.len(), 1);
    assert_eq!(generation_runs[0].workflow, "portrait_generation");
    assert_eq!(generation_runs[0].step_key, "generate_portrait_image");
    assert_eq!(generation_runs[0].workflow_run_id, Some(workflow.id));
    assert_eq!(workflow.status, "completed");
    assert_eq!(checkpoints.len(), 1);
    assert_eq!(
        checkpoints[0].step_key,
        format!("missing_portrait_generation.profile.{}", profile.id)
    );
    assert_eq!(checkpoints[0].status, "completed");
}

#[tokio::test]
async fn missing_portrait_batch_continues_after_one_item_fails() {
    let (database, context) = setup_context().await;
    let (project, first_profile) = insert_project_and_profile(database.as_ref()).await;
    let second_profile = a4n_db::character::create_profile(
        database.as_ref(),
        project.id,
        "第二个测试角色",
        json!({
            "explicit_attributes": { "appearance": "长发，黑色外套" },
            "implicit_attributes": {},
            "social_attributes": {}
        }),
    )
    .await
    .expect("第二个测试角色应创建成功");
    let host = TestPortraitHost {
        fail_generate_calls: 1,
        ..Default::default()
    };

    let outcome = context
        .generate_missing_portraits_validated(
            1,
            GenerateMissingPortraitsInput {
                project_id: project.id,
                width: 1024,
                height: 1536,
            },
            &host,
            None,
            CancellationToken::new(),
        )
        .await
        .expect("best-effort 批量应继续执行后续角色");

    assert_eq!(outcome.total, 2);
    assert_eq!(outcome.succeeded(), 1);
    assert_eq!(outcome.failed(), 1);
    assert_eq!(outcome.items[0].summary.profile_id, second_profile.id);
    assert_eq!(
        outcome.failed_item_keys,
        vec![format!("portrait-profile-{}", first_profile.id)]
    );
    assert!(!outcome.interrupted);
}
