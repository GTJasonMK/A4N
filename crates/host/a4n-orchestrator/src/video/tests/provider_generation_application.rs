//! 章节 / 批量 / 参考媒体 provider generation 的 Orchestrator 定向测试。
//!
//! 不经过 Server/Tauri process；使用本地 HostPort 假实现与 PostgreSQL 测试库。

use a4n_error::AppError;

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use a4n_api_contract::video::{
    GenerateChapterVideoInput, GenerateReferenceMediaVideoInput, ReferenceMediaVideoInput,
};
use a4n_db_models::models::{ai::generated_video, generation::generation_run, user::user};
use a4n_domain::{media::GeneratedMediaArtifactFile, source_text_hash};
use a4n_provider_contract::VideoProviderConfig;
use a4n_video::{
    ProviderImageInputTransport, VideoGenerationRequest, VideoGenerationResult, VideoJobState,
};
use a4n_video_contract::{
    VIDEO_SEGMENT_EXECUTION_RECIPE_V1, VideoReferenceMediaBindingV1, VideoReferenceMediaRecipeV1,
    VideoSegmentExecutionRecipeV1, VideoSegmentGuidanceModeV1, VideoSegmentWorkflowModeV1,
};
use a4n_video_host_contract::{
    ChapterVideoGenerationHostPort, ChapterVideoImageInput, ChapterVideoPreviewHostPort,
    PersistedVideoResult, ReferenceMediaVideoGenerationHostPort,
    ReferenceMediaVideoPreviewHostPort, ResolvedChapterVideoImageInputs, VideoCancellationHandle,
    VideoGenerationProgressPort, VideoProviderRequestBody, VideoProviderRequestInput,
    VideoProviderResultTarget,
};
use async_trait::async_trait;
use base64::Engine as _;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio_util::sync::CancellationToken;

use crate::test_support::TestOrchestratorBuilder;
use crate::video::{
    GenerateChapterVideosBatchInput, SEEDANCE2_VIDEO_BACKEND, VideoGenerationProgress,
};
use a4n_video_host_contract::VideoProviderHostPort;

use super::*;

#[derive(Default)]
struct TestVideoProviderHost {
    persist_calls: AtomicUsize,
    remove_calls: AtomicUsize,
    fail_persist: bool,
    cancel_on_persist: Option<CancellationToken>,
    interrupt_on_persist: Option<Arc<a4n_db::DatabaseConnection>>,
}

#[async_trait]
impl VideoProviderHostPort for TestVideoProviderHost {
    fn inspect_video_file(
        &self,
        _relative_path: &str,
        _label: &str,
    ) -> Result<GeneratedMediaArtifactFile, AppError> {
        Ok(GeneratedMediaArtifactFile {
            content_type: "video/mp4".to_string(),
            sha256: "provider-gen-sha256".to_string(),
            byte_size: 8,
        })
    }

    async fn poll_video_job(
        &self,
        _project_id: i64,
        _workflow_run_id: i64,
        _config: &VideoProviderConfig,
        _job_id: &str,
        _cancel_token: &VideoCancellationHandle,
    ) -> Result<VideoJobState, AppError> {
        Err(AppError::InvalidState {
            current: "poll".to_string(),
            operation: "定向测试不应进入轮询".to_string(),
        })
    }

    async fn persist_video_result(
        &self,
        target: &VideoProviderResultTarget,
        prompt_id: Option<i64>,
        result: VideoGenerationResult,
    ) -> Result<PersistedVideoResult, AppError> {
        self.persist_calls.fetch_add(1, Ordering::SeqCst);
        if self.fail_persist {
            return Err(AppError::InvalidState {
                current: "file".to_string(),
                operation: "测试视频落盘失败".to_string(),
            });
        }
        if let Some(cancel_token) = &self.cancel_on_persist {
            cancel_token.cancel();
        }
        if let Some(database) = &self.interrupt_on_persist {
            a4n_db::video::mark_generated_video_error_status_and_prompt_record(
                database.as_ref(),
                target.video_id,
                prompt_id,
                a4n_domain::video_workflow::VIDEO_STATUS_INTERRUPTED,
                "测试模拟外部取消",
            )
            .await
            .map_err(|error| AppError::Internal(anyhow::anyhow!(error)))?;
        }
        Ok(PersistedVideoResult {
            video_id: target.video_id,
            project_id: target.project_id,
            file_path: format!("generated/videos/{}.{}", target.video_id, result.format),
            thumbnail_path: Some(format!("generated/videos/{}.jpg", target.video_id)),
            properties: a4n_domain::media::VideoFileProperties {
                file_size: i64::try_from(result.bytes.len())
                    .map_err(|_| AppError::InvalidArgument("测试视频过大".to_string()))?,
                width: 16,
                height: 16,
                duration_secs: 1.0,
                fps: 24,
            },
            metadata_json: Some(json!({
                "provider": "test-video-host",
                "format": result.format,
            })),
        })
    }

    async fn remove_video_result(&self, _persisted: &PersistedVideoResult) -> Result<(), AppError> {
        self.remove_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[derive(Default)]
struct TestReferenceMediaHost {
    provider: TestVideoProviderHost,
    submit_calls: AtomicUsize,
    body_calls: AtomicUsize,
    fail_submit: bool,
    cancel_during_submit: bool,
    return_empty_bytes: bool,
    last_request: Mutex<Option<VideoGenerationRequest>>,
}

#[async_trait]
impl ReferenceMediaVideoGenerationHostPort for TestReferenceMediaHost {
    async fn resolve_video_config(
        &self,
        config_id: Option<i64>,
    ) -> Result<VideoProviderConfig, AppError> {
        Ok(seedance_config(config_id.unwrap_or(42)))
    }

    async fn reference_media_provider_request_body(
        &self,
        input: VideoProviderRequestInput,
    ) -> Result<VideoProviderRequestBody, AppError> {
        self.body_calls.fetch_add(1, Ordering::SeqCst);
        Ok(VideoProviderRequestBody {
            request: json!({
                "model": input.config.model,
                "prompt": input.request.prompt,
                "media": input.request.media,
            }),
            source_label: "test-seedance-body".to_string(),
        })
    }

    async fn submit_reference_media_video(
        &self,
        input: VideoProviderRequestInput,
        progress: &mut dyn VideoGenerationProgressPort,
    ) -> Result<VideoGenerationResult, AppError> {
        self.submit_calls.fetch_add(1, Ordering::SeqCst);
        *self.last_request.lock().expect("last_request lock") = Some(input.request.clone());
        if self.fail_submit {
            return Err(AppError::InvalidState {
                current: "provider".to_string(),
                operation: "测试参考媒体 provider 失败".to_string(),
            });
        }
        if self.cancel_during_submit {
            progress.cancel();
        }
        let bytes = if self.return_empty_bytes {
            Vec::new()
        } else {
            vec![1, 2, 3, 4, 5, 6, 7, 8]
        };
        Ok(VideoGenerationResult {
            bytes,
            format: "mp4".to_string(),
            provider_job_id: if self.return_empty_bytes {
                Some("job-async-1".to_string())
            } else {
                None
            },
            metadata: json!({"backend": SEEDANCE2_VIDEO_BACKEND}),
        })
    }

    fn runtime_config_snapshot(&self) -> Option<Value> {
        Some(json!({"profile": "reference-media-application-test"}))
    }

    fn video_provider_host_port(&self) -> &dyn VideoProviderHostPort {
        &self.provider
    }
}

#[async_trait]
impl ReferenceMediaVideoPreviewHostPort for TestReferenceMediaHost {
    async fn resolve_video_config_for_preview(
        &self,
        config_id: Option<i64>,
    ) -> Result<VideoProviderConfig, AppError> {
        <Self as ReferenceMediaVideoGenerationHostPort>::resolve_video_config(self, config_id).await
    }

    async fn reference_media_provider_request_body_for_preview(
        &self,
        input: VideoProviderRequestInput,
    ) -> Result<VideoProviderRequestBody, AppError> {
        <Self as ReferenceMediaVideoGenerationHostPort>::reference_media_provider_request_body(
            self, input,
        )
        .await
    }

    async fn upload_reference_media_input_for_preview(
        &self,
        owner_user_id: i64,
        project_id: i64,
        source_media_id: i64,
        source_media_kind: &str,
        file_name: &str,
        mime_type: &str,
        bytes: &[u8],
    ) -> Result<(String, a4n_service_protocol::GenerationArtifactRef), AppError> {
        <Self as ReferenceMediaVideoGenerationHostPort>::upload_reference_media_input(
            self,
            owner_user_id,
            project_id,
            source_media_id,
            source_media_kind,
            file_name,
            mime_type,
            bytes,
        )
        .await
    }
}

/// 章节路径只用于校验 gate；不进入真实 provider 请求渲染。
#[derive(Default)]
struct RejectingChapterHost {
    provider: TestVideoProviderHost,
}

#[async_trait]
impl ChapterVideoGenerationHostPort for RejectingChapterHost {
    async fn resolve_video_config(
        &self,
        config_id: Option<i64>,
    ) -> Result<VideoProviderConfig, AppError> {
        Ok(seedance_config(config_id.unwrap_or(7)))
    }

    async fn resolve_chapter_video_image_inputs(
        &self,
        _input: ChapterVideoImageInput,
    ) -> Result<ResolvedChapterVideoImageInputs, AppError> {
        Ok(ResolvedChapterVideoImageInputs::default())
    }

    async fn chapter_video_provider_request_body(
        &self,
        _input: VideoProviderRequestInput,
    ) -> Result<VideoProviderRequestBody, AppError> {
        unreachable!("gate 测试不应到达 request body")
    }

    async fn submit_chapter_video(
        &self,
        _input: VideoProviderRequestInput,
        _progress: &mut dyn VideoGenerationProgressPort,
    ) -> Result<VideoGenerationResult, AppError> {
        unreachable!("gate 测试不应到达 submit")
    }

    fn runtime_config_snapshot(&self) -> Option<Value> {
        None
    }

    fn video_provider_host_port(&self) -> &dyn VideoProviderHostPort {
        &self.provider
    }
}

#[async_trait]
impl ChapterVideoPreviewHostPort for RejectingChapterHost {
    async fn resolve_video_config_for_preview(
        &self,
        config_id: Option<i64>,
    ) -> Result<VideoProviderConfig, AppError> {
        <Self as ChapterVideoGenerationHostPort>::resolve_video_config(self, config_id).await
    }

    fn resolve_image_transport_for_preview(
        &self,
        _backend: &str,
    ) -> Result<ProviderImageInputTransport, AppError> {
        Ok(ProviderImageInputTransport::InlineDataUrl)
    }

    async fn resolve_chapter_video_image_inputs_for_preview(
        &self,
        input: ChapterVideoImageInput,
    ) -> Result<ResolvedChapterVideoImageInputs, AppError> {
        <Self as ChapterVideoGenerationHostPort>::resolve_chapter_video_image_inputs(self, input)
            .await
    }

    async fn chapter_video_provider_request_body_for_preview(
        &self,
        input: VideoProviderRequestInput,
    ) -> Result<VideoProviderRequestBody, AppError> {
        <Self as ChapterVideoGenerationHostPort>::chapter_video_provider_request_body(self, input)
            .await
    }
}

fn seedance_config(id: i64) -> VideoProviderConfig {
    VideoProviderConfig {
        id,
        name: "Seedance 测试配置".to_string(),
        backend: SEEDANCE2_VIDEO_BACKEND.to_string(),
        base_url: Some("http://127.0.0.1:47645".to_string()),
        api_key: None,
        model: "seedance-2-0-pro".to_string(),
        max_concurrent: 4,
        default_width: 1280,
        default_height: 720,
        default_duration_secs: 8,
        min_duration_secs: 4,
        max_duration_secs: 15,
        duration_mode: "range".to_string(),
        supported_durations_json: None,
        default_fps: 24,
        is_default: true,
        pricing: Default::default(),
    }
}

fn long_reference_prompt() -> String {
    format!(
        "{} @image_file_1 用作人物外观与服装一致性参考，保持动作、光影与空间连续。",
        "镜头保持人物、场景、动作、光影、构图和节奏连续，".repeat(8)
    )
}

fn reference_media_segment() -> a4n_video_contract::VideoSegmentPlanV2 {
    serde_json::from_value(json!({
        "schema_version": "video_segment_plan_v2",
        "segment_number": 1,
        "source_shot_id": "shot-1",
        "segment_index_in_shot": 1,
        "shot_segment_count": 1,
        "covered_beat_ids": ["shot-1-beat-1"],
        "title": "参考媒体片段",
        "scene_summary": "参考媒体场景",
        "blocking_summary": "人物保持在画面中央",
        "spatial_anchors": [],
        "emotion_action_goals": [],
        "video_intent": {"hook":"人物入画","audience_effect":"建立人物","rhythm":"平稳","visual_priority":"人物"},
        "emotional_arc": {"start":"平静","turn":"观察","end":"确认","function":"建立场景"},
        "performance_plan": {"movement_path":"原地","body_performance":"克制","gaze_and_hands":"看向镜头外","pause_or_reaction":"短暂停顿","camera_relationship":"中景","time_blocks":[],"action_script":[]},
        "material_coverage": {"required_entities":[],"missing_or_weak_references":[],"risk_note":"无"},
        "transition_to_next": null,
        "source_span_ids": ["span-1"],
        "duration_secs": 8,
        "aspect_ratio": "16:9",
        "reference_image_ids": []
    }))
    .expect("参考媒体测试片段应符合当前合同")
}

async fn insert_reference_media_segment_plan(
    db: &a4n_db::DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
) -> chapter_video_prompt::Model {
    let now = fixed_time(3);
    let source_content = chapter::Entity::find_by_id(chapter_id)
        .one(db)
        .await
        .expect("查询参考媒体测试章节")
        .expect("参考媒体测试章节应存在")
        .content;
    let source_content_sha256 = a4n_domain::source_text_hash(&source_content);
    let segment = reference_media_segment();
    let segment_sha256 = a4n_video_contract::video_segment_content_fingerprint(&segment)
        .expect("参考媒体测试片段应可计算指纹");
    let recipe = VideoSegmentExecutionRecipeV1 {
        schema_version: VIDEO_SEGMENT_EXECUTION_RECIPE_V1.to_string(),
        segment_number: 1,
        workflow_mode: VideoSegmentWorkflowModeV1::ReferenceMedia,
        style: "电影感写实".to_string(),
        prompt_output_language: "zh_cn".to_string(),
        duration_secs: 8,
        duration_secs_options: vec![8],
        aspect_ratio: "16:9".to_string(),
        reference_image_ids: Vec::new(),
        video_library_image_ids: Vec::new(),
        guidance_mode: VideoSegmentGuidanceModeV1::None,
        guidance_reference_image_ids: Vec::new(),
        continuity_source_video_id: None,
        reference_media: Some(VideoReferenceMediaRecipeV1 {
            prompt: long_reference_prompt(),
            image_rows: vec![VideoReferenceMediaBindingV1 {
                url: "https://example.test/ref.png".to_string(),
                usage: "人物外观与服装一致性参考".to_string(),
                upload_sha256: None,
                upload_file_name: None,
                upload_mime_type: None,
            }],
            video_rows: Vec::new(),
            audio_rows: Vec::new(),
        }),
        adopted_video_id: None,
    };
    chapter_video_prompt::ActiveModel {
        project_id: Set(project_id),
        chapter_id: Set(chapter_id),
        style: Set(Some("电影感写实".to_string())),
        prompt_type: Set("frame_segment_plan".to_string()),
        workflow_mode: Set("shared_segment_plan".to_string()),
        shot_number: Set(0),
        title: Set(Some("共享片段规划".to_string())),
        scene_summary: Set(None),
        visual_prompt: Set("{}".to_string()),
        motion_prompt: Set(None),
        negative_prompt: Set(None),
        duration_secs: Set(8),
        aspect_ratio: Set(Some("16:9".to_string())),
        reference_image_ids_json: Set(None),
        status: Set("completed".to_string()),
        generated_video_id: Set(None),
        metadata_json: Set(Some(json!({
            "video_execution_identity": {
                "contract_epoch": a4n_video_contract::VIDEO_EXECUTION_CONTRACT_EPOCH,
                "shooting_script_identity": {},
                "source_content_sha256": source_content_sha256,
                "style_sha256": "style",
                "duration_secs": 8,
                "aspect_ratio": "16:9",
                "segment_identities": [{
                    "source_shot_id": "shot-1",
                    "segment_index_in_shot": 1,
                    "segment_sha256": segment_sha256
                }]
            },
            "segments": [segment],
            "segment_recipes": [recipe]
        }))),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("测试共享片段规划应创建成功")
}

fn reference_media_input(segment_plan_prompt_id: i64) -> ReferenceMediaVideoInput {
    ReferenceMediaVideoInput {
        segment_plan_prompt_id,
        target_segment: a4n_api_contract::video::VideoSegmentTarget {
            source_shot_id: "shot-1".to_string(),
            segment_index_in_shot: 1,
        },
        image_uploads: Vec::new(),
        video_uploads: Vec::new(),
        audio_uploads: Vec::new(),
        config_id: Some(42),
    }
}

async fn ensure_owner_user(db: &a4n_db::DatabaseConnection) -> i64 {
    let now = fixed_time(3);
    user::ActiveModel {
        username: Set(format!(
            "video-provider-owner-{}",
            now.and_utc().timestamp_millis()
        )),
        email: Set(format!(
            "video-provider-owner-{}@example.test",
            now.and_utc().timestamp_millis()
        )),
        password_hash: Set("test-password-hash".to_string()),
        role: Set("user".to_string()),
        oauth_provider: Set(None),
        oauth_subject: Set(None),
        avatar_url: Set(None),
        is_active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        last_login_at: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("测试用户应创建成功")
    .id
}

fn ready_segment_recipe() -> VideoSegmentExecutionRecipeV1 {
    VideoSegmentExecutionRecipeV1 {
        schema_version: VIDEO_SEGMENT_EXECUTION_RECIPE_V1.to_string(),
        segment_number: 1,
        workflow_mode: VideoSegmentWorkflowModeV1::BoundaryFrames,
        style: "电影感写实".to_string(),
        prompt_output_language: "zh_cn".to_string(),
        duration_secs: 6,
        duration_secs_options: vec![6],
        aspect_ratio: "16:9".to_string(),
        reference_image_ids: Vec::new(),
        video_library_image_ids: Vec::new(),
        guidance_mode: VideoSegmentGuidanceModeV1::None,
        guidance_reference_image_ids: Vec::new(),
        continuity_source_video_id: None,
        reference_media: None,
        adopted_video_id: None,
    }
}

async fn insert_ready_segment_plan(
    db: &a4n_db::DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
) -> chapter_video_prompt::Model {
    let now = fixed_time(3);
    let source_content = chapter::Entity::find_by_id(chapter_id)
        .one(db)
        .await
        .expect("查询可用视频测试章节")
        .expect("可用视频测试章节应存在")
        .content;
    let source_content_sha256 = source_text_hash(&source_content);
    let segment = reference_media_segment();
    let mut segment = segment;
    segment.duration_secs = 6;
    let recipe = ready_segment_recipe();
    chapter_video_prompt::ActiveModel {
        project_id: Set(project_id),
        chapter_id: Set(chapter_id),
        style: Set(Some(recipe.style.clone())),
        prompt_type: Set(chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN.to_string()),
        workflow_mode: Set(chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN.to_string()),
        shot_number: Set(0),
        title: Set(Some("可用片段规划".to_string())),
        scene_summary: Set(None),
        visual_prompt: Set("{}".to_string()),
        motion_prompt: Set(None),
        negative_prompt: Set(None),
        duration_secs: Set(6),
        aspect_ratio: Set(Some(recipe.aspect_ratio.clone())),
        reference_image_ids_json: Set(None),
        status: Set(chapter_video_prompt::STATUS_COMPLETED.to_string()),
        metadata_json: Set(Some(json!({
            "contract_version": FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
            "video_execution_identity": {
                "contract_epoch": a4n_video_contract::VIDEO_EXECUTION_CONTRACT_EPOCH,
                "shooting_script_identity": {},
                "frame_set_sha256": "frame-set",
                "source_content_sha256": source_content_sha256,
                "style_sha256": "style",
                "duration_secs": 6,
                "aspect_ratio": "16:9",
                "prompt_output_language": "zh_cn",
                "segment_identities": [{
                    "source_shot_id": "shot-1",
                    "segment_index_in_shot": 1,
                    "segment_sha256": "segment-hash"
                }],
                "generation_asset_fingerprint": {}
            },
            "segments": [segment],
            "segment_recipes": [recipe]
        }))),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("可用视频测试片段规划应创建成功")
}

async fn insert_ready_video_prompt(
    db: &a4n_db::DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
    segment_plan_prompt_id: i64,
    shot_number: i32,
    generated_video_id: Option<i64>,
) -> chapter_video_prompt::Model {
    let now = fixed_time(3);
    let recipe = ready_segment_recipe();
    let plan_identity = chapter_video_prompt::Entity::find_by_id(segment_plan_prompt_id)
        .one(db)
        .await
        .expect("查询可用视频片段规划")
        .expect("可用视频片段规划应存在")
        .metadata_json
        .and_then(|metadata| metadata.get("video_execution_identity").cloned())
        .expect("可用视频片段规划应包含执行身份");
    let execution_identity = crate::video::prompts::generation::video_segment_execution_identity(
        plan_identity,
        Some(segment_plan_prompt_id),
        Some(&recipe),
        ("shot-1", 1),
    )
    .expect("可用视频提示词执行身份应可构造");
    chapter_video_prompt::ActiveModel {
        project_id: Set(project_id),
        chapter_id: Set(chapter_id),
        style: Set(Some("电影感写实".to_string())),
        prompt_type: Set("video_prompt".to_string()),
        workflow_mode: Set("boundary_frames".to_string()),
        shot_number: Set(shot_number),
        title: Set(Some(format!("镜头 {shot_number}"))),
        scene_summary: Set(Some("雨夜对峙".to_string())),
        visual_prompt: Set("雨夜街巷，人物对峙".to_string()),
        motion_prompt: Set(Some("慢推镜头".to_string())),
        negative_prompt: Set(None),
        duration_secs: Set(6),
        aspect_ratio: Set(Some("16:9".to_string())),
        reference_image_ids_json: Set(None),
        status: Set("completed".to_string()),
        generated_video_id: Set(generated_video_id),
        metadata_json: Set(Some(json!({
            "source_prompt_type": chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN,
            "prompt_output_language": "zh_cn",
            "video_execution_identity": execution_identity,
        }))),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("测试视频提示词应创建成功")
}

async fn setup_project_chapter() -> (
    Arc<a4n_db::DatabaseConnection>,
    crate::OrchestratorContext,
    i64,
    i64,
    i64,
) {
    let database = Arc::new(setup_db().await);
    let owner_id = ensure_owner_user(database.as_ref()).await;
    let context = TestOrchestratorBuilder::new(Arc::clone(&database))
        .build()
        .await;
    let mut project = insert_video_project(database.as_ref(), "provider generation 定向").await;
    project = novel_project::ActiveModel {
        id: Set(project.id),
        user_id: Set(owner_id),
        ..Default::default()
    }
    .update(database.as_ref())
    .await
    .expect("测试项目应归属真实用户");
    let chapter = insert_video_chapter(database.as_ref(), project.id, 1).await;
    (database, context, owner_id, project.id, chapter.id)
}

fn empty_progress() -> VideoGenerationProgress<'static> {
    VideoGenerationProgress::without_checkpoint(None, CancellationToken::new())
}

#[tokio::test]
async fn reference_media_preview_returns_request_body_without_submit() {
    let (database, context, owner_id, project_id, chapter_id) = setup_project_chapter().await;
    let plan = insert_reference_media_segment_plan(database.as_ref(), project_id, chapter_id).await;
    let host = TestReferenceMediaHost::default();
    let preview = context
        .preview_reference_media_video_request_validated(
            &host,
            GenerateReferenceMediaVideoInput {
                user_id: owner_id,
                project_id,
                chapter_id,
                input: reference_media_input(plan.id),
            },
        )
        .await
        .expect("参考媒体预览应成功");
    assert_eq!(host.submit_calls.load(Ordering::SeqCst), 0);
    assert_eq!(host.body_calls.load(Ordering::SeqCst), 1);
    assert!(preview.request.get("prompt").is_some());
    assert!(preview.note.contains("当前表单") || !preview.note.is_empty());
}

#[tokio::test]
async fn reference_media_preview_rejects_generation_invalid_recipe() {
    let (database, context, owner_id, project_id, chapter_id) = setup_project_chapter().await;
    let plan = insert_reference_media_segment_plan(database.as_ref(), project_id, chapter_id).await;
    let mut metadata = plan.metadata_json.expect("参考媒体规划应有 metadata");
    metadata["segment_recipes"][0]["reference_media"]["prompt"] = json!("短");
    chapter_video_prompt::ActiveModel {
        id: Set(plan.id),
        metadata_json: Set(Some(metadata)),
        ..Default::default()
    }
    .update(database.as_ref())
    .await
    .expect("应可写入无效参考媒体草稿用于边界测试");
    let host = TestReferenceMediaHost::default();

    let error = context
        .preview_reference_media_video_request_validated(
            &host,
            GenerateReferenceMediaVideoInput {
                user_id: owner_id,
                project_id,
                chapter_id,
                input: reference_media_input(plan.id),
            },
        )
        .await
        .expect_err("预览不得软化生成无效的 prompt");

    assert!(error.to_string().contains("prompt 过短"));
    assert_eq!(host.body_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn reference_media_preview_redacts_validated_local_upload_without_staging() {
    let (database, context, owner_id, project_id, chapter_id) = setup_project_chapter().await;
    let plan = insert_reference_media_segment_plan(database.as_ref(), project_id, chapter_id).await;
    let bytes = b"validated local preview image";
    let sha256 = format!("{:x}", Sha256::digest(bytes));
    let data_base64 = base64::engine::general_purpose::STANDARD.encode(bytes);
    let mut metadata = plan.metadata_json.expect("参考媒体规划应有 metadata");
    metadata["segment_recipes"][0]["reference_media"]["image_rows"][0] = json!({
        "url": "",
        "usage": "人物外观与服装一致性参考",
        "upload_sha256": sha256.clone(),
        "upload_file_name": "reference.png",
        "upload_mime_type": "image/png"
    });
    chapter_video_prompt::ActiveModel {
        id: Set(plan.id),
        metadata_json: Set(Some(metadata)),
        ..Default::default()
    }
    .update(database.as_ref())
    .await
    .expect("应可写入本地参考媒体测试方案");
    let host = TestReferenceMediaHost::default();
    let mut input = reference_media_input(plan.id);
    input.image_uploads = vec![a4n_api_contract::video::ReferenceMediaUploadInput {
        file_name: "reference.png".to_string(),
        mime_type: "image/png".to_string(),
        data_base64: data_base64.clone(),
    }];

    let preview = context
        .preview_reference_media_video_request_validated(
            &host,
            GenerateReferenceMediaVideoInput {
                user_id: owner_id,
                project_id,
                chapter_id,
                input,
            },
        )
        .await
        .expect("合法本地参考媒体应可脱敏预览");

    let serialized = preview.request.to_string();
    assert!(serialized.contains(&sha256));
    assert!(!serialized.contains(&data_base64));
    assert!(preview.note.contains("脱敏 URL"));
    assert_eq!(host.submit_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn reference_media_generate_persists_completed_video() {
    let (database, context, owner_id, project_id, chapter_id) = setup_project_chapter().await;
    let plan = insert_reference_media_segment_plan(database.as_ref(), project_id, chapter_id).await;
    let host = TestReferenceMediaHost::default();
    let mut progress = empty_progress();
    let video = context
        .run_reference_media_video_generation_validated(
            &host,
            GenerateReferenceMediaVideoInput {
                user_id: owner_id,
                project_id,
                chapter_id,
                input: reference_media_input(plan.id),
            },
            &mut progress,
        )
        .await
        .expect("参考媒体生成应完成");

    assert_eq!(host.submit_calls.load(Ordering::SeqCst), 1);
    assert_eq!(host.provider.persist_calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        video.status,
        a4n_domain::video_workflow::VIDEO_STATUS_COMPLETED
    );
    assert_eq!(video.project_id, project_id);
    assert_eq!(video.chapter_id, Some(chapter_id));
    assert_eq!(video.duration_secs, 1.0);
    assert_eq!((video.width, video.height, video.fps), (16, 16, 24));
    assert_eq!(
        video.video_type,
        a4n_domain::video_workflow::VIDEO_TYPE_REFERENCE_MEDIA
    );
    assert!(
        video
            .file_path
            .as_deref()
            .is_some_and(|path| path.contains(&format!("generated/videos/{}.mp4", video.id)))
    );
    let stored = generated_video::Entity::find_by_id(video.id)
        .one(database.as_ref())
        .await
        .expect("查询正式视频")
        .expect("正式视频应落库");
    assert_eq!(
        stored.status,
        a4n_domain::video_workflow::VIDEO_STATUS_COMPLETED
    );
}

#[tokio::test]
async fn video_provider_terminal_status_updates_existing_workflow() {
    let (database, context, _owner_id, project_id, chapter_id) = setup_project_chapter().await;
    let video = context
        .create_video_generation_record_validated(VideoGenerationRecordInput {
            project_id,
            chapter_id: Some(chapter_id),
            video_type: a4n_domain::video_workflow::VIDEO_TYPE_REFERENCE_MEDIA.to_string(),
            workflow_mode: Some("reference_media".to_string()),
            prompt: long_reference_prompt(),
            negative_prompt: None,
            model: "seedance-2-0-pro".to_string(),
            width: 1280,
            height: 720,
            duration_secs: 8.0,
            fps: 24,
            source_video_id: None,
            source_image_id: None,
            metadata_json: Some(json!({"config_id": 42})),
            provider_request_snapshot: json!({"provider": "seedance2-video"}),
            provider_backend: SEEDANCE2_VIDEO_BACKEND.to_string(),
            step_key: "generate_reference_media_video".to_string(),
            title: "参考媒体视频 provider 生成".to_string(),
            prompt_template_name: Some("reference_media_video".to_string()),
            prompt_id: None,
        })
        .await
        .expect("创建 provider 视频记录");
    let video = context
        .mark_generated_video_job_and_prompt_validated(
            video.id,
            "job-async-1",
            Some(json!({"status": "submitted"})),
            None,
        )
        .await
        .expect("绑定 provider job");
    context
        .mark_video_provider_workflow_validated(
            &video,
            crate::workflow::WORKFLOW_STATUS_RUNNING,
            true,
            None,
        )
        .await
        .expect("开始 provider workflow");
    context
        .mark_video_provider_workflow_validated(
            &video,
            crate::workflow::WORKFLOW_STATUS_COMPLETED,
            false,
            None,
        )
        .await
        .expect("完成 provider workflow");

    let workflow = a4n_db::workflow::latest_workflow_run_by_business_key(
        database.as_ref(),
        &format!("video_provider_job:{}", video.id),
    )
    .await
    .expect("查询 provider workflow")
    .expect("provider workflow 应存在");
    assert_eq!(workflow.status, crate::workflow::WORKFLOW_STATUS_COMPLETED);
    assert!(!workflow.can_resume);
}

#[tokio::test]
async fn reference_media_generate_provider_failure_marks_failed() {
    let (database, context, owner_id, project_id, chapter_id) = setup_project_chapter().await;
    let plan = insert_reference_media_segment_plan(database.as_ref(), project_id, chapter_id).await;
    let host = TestReferenceMediaHost {
        fail_submit: true,
        ..Default::default()
    };
    let mut progress = empty_progress();
    let video = context
        .run_reference_media_video_generation_validated(
            &host,
            GenerateReferenceMediaVideoInput {
                user_id: owner_id,
                project_id,
                chapter_id,
                input: reference_media_input(plan.id),
            },
            &mut progress,
        )
        .await
        .expect("provider 失败应返回失败状态视频而不是上层 panic");

    assert_eq!(host.submit_calls.load(Ordering::SeqCst), 1);
    assert_eq!(host.provider.persist_calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        video.status,
        a4n_domain::video_workflow::VIDEO_STATUS_FAILED
    );
    assert!(
        video
            .error_message
            .as_deref()
            .is_some_and(|message| message.contains("测试参考媒体 provider 失败"))
    );
    let stored = generated_video::Entity::find_by_id(video.id)
        .one(database.as_ref())
        .await
        .expect("查询失败视频")
        .expect("失败视频应落库");
    assert_eq!(
        stored.status,
        a4n_domain::video_workflow::VIDEO_STATUS_FAILED
    );
}

#[tokio::test]
async fn reference_media_cancel_before_provider_marks_records_cancelled() {
    let (database, context, owner_id, project_id, chapter_id) = setup_project_chapter().await;
    let plan = insert_reference_media_segment_plan(database.as_ref(), project_id, chapter_id).await;
    let host = TestReferenceMediaHost::default();
    let cancel_token = CancellationToken::new();
    let reporter_token = cancel_token.clone();
    let mut reporter = move |current, _total, _message: &str| {
        if current == 3 {
            reporter_token.cancel();
        }
        Ok(())
    };
    let mut progress =
        VideoGenerationProgress::without_checkpoint(Some(&mut reporter), cancel_token);

    let error = context
        .run_reference_media_video_generation_validated(
            &host,
            GenerateReferenceMediaVideoInput {
                user_id: owner_id,
                project_id,
                chapter_id,
                input: reference_media_input(plan.id),
            },
            &mut progress,
        )
        .await
        .expect_err("provider 调用前取消必须返回 cancelled");

    assert!(matches!(error, AppError::Cancelled));
    assert_eq!(host.submit_calls.load(Ordering::SeqCst), 0);
    let video = generated_video::Entity::find()
        .filter(generated_video::Column::ProjectId.eq(project_id))
        .one(database.as_ref())
        .await
        .expect("查询取消视频")
        .expect("取消视频记录应保留");
    assert_eq!(
        video.status,
        a4n_domain::video_workflow::VIDEO_STATUS_CANCELLED
    );
    let run = generation_run::Entity::find()
        .filter(generation_run::Column::ProjectId.eq(project_id))
        .one(database.as_ref())
        .await
        .expect("查询 generation run")
        .expect("取消视频应有 generation run");
    assert_eq!(run.status, crate::workflow::WORKFLOW_STATUS_CANCELLED);
}

#[tokio::test]
async fn reference_media_cancel_during_provider_discards_late_result() {
    let (database, context, owner_id, project_id, chapter_id) = setup_project_chapter().await;
    let plan = insert_reference_media_segment_plan(database.as_ref(), project_id, chapter_id).await;
    let host = TestReferenceMediaHost {
        cancel_during_submit: true,
        ..Default::default()
    };
    let mut progress = empty_progress();

    let error = context
        .run_reference_media_video_generation_validated(
            &host,
            GenerateReferenceMediaVideoInput {
                user_id: owner_id,
                project_id,
                chapter_id,
                input: reference_media_input(plan.id),
            },
            &mut progress,
        )
        .await
        .expect_err("取消后的迟到 provider 结果不能写成完成");

    assert!(matches!(error, AppError::Cancelled));
    assert_eq!(host.submit_calls.load(Ordering::SeqCst), 1);
    assert_eq!(host.provider.persist_calls.load(Ordering::SeqCst), 0);
    let video = generated_video::Entity::find()
        .filter(generated_video::Column::ProjectId.eq(project_id))
        .one(database.as_ref())
        .await
        .expect("查询取消视频")
        .expect("取消视频记录应保留");
    assert_eq!(
        video.status,
        a4n_domain::video_workflow::VIDEO_STATUS_CANCELLED
    );
}

#[tokio::test]
async fn reference_media_cancel_during_persist_cleans_file_and_stays_cancelled() {
    let (database, context, owner_id, project_id, chapter_id) = setup_project_chapter().await;
    let plan = insert_reference_media_segment_plan(database.as_ref(), project_id, chapter_id).await;
    let cancel_token = CancellationToken::new();
    let host = TestReferenceMediaHost {
        provider: TestVideoProviderHost {
            cancel_on_persist: Some(cancel_token.clone()),
            ..Default::default()
        },
        ..Default::default()
    };
    let mut progress = VideoGenerationProgress::without_checkpoint(None, cancel_token);

    let error = context
        .run_reference_media_video_generation_validated(
            &host,
            GenerateReferenceMediaVideoInput {
                user_id: owner_id,
                project_id,
                chapter_id,
                input: reference_media_input(plan.id),
            },
            &mut progress,
        )
        .await
        .expect_err("文件落盘期间取消不能完成视频");

    assert!(matches!(error, AppError::Cancelled));
    assert_eq!(host.provider.persist_calls.load(Ordering::SeqCst), 1);
    assert_eq!(host.provider.remove_calls.load(Ordering::SeqCst), 1);
    let video = generated_video::Entity::find()
        .filter(generated_video::Column::ProjectId.eq(project_id))
        .one(database.as_ref())
        .await
        .expect("查询取消视频")
        .expect("取消视频记录应保留");
    assert_eq!(
        video.status,
        a4n_domain::video_workflow::VIDEO_STATUS_CANCELLED
    );
}

#[tokio::test]
async fn reference_media_external_interrupt_during_persist_is_not_downgraded_to_failed() {
    let (database, context, owner_id, project_id, chapter_id) = setup_project_chapter().await;
    let plan = insert_reference_media_segment_plan(database.as_ref(), project_id, chapter_id).await;
    let host = TestReferenceMediaHost {
        provider: TestVideoProviderHost {
            interrupt_on_persist: Some(Arc::clone(&database)),
            ..Default::default()
        },
        ..Default::default()
    };
    let mut progress = empty_progress();

    let error = context
        .run_reference_media_video_generation_validated(
            &host,
            GenerateReferenceMediaVideoInput {
                user_id: owner_id,
                project_id,
                chapter_id,
                input: reference_media_input(plan.id),
            },
            &mut progress,
        )
        .await
        .expect_err("外部中断后的迟到文件不能覆盖权威状态");

    assert!(matches!(error, AppError::Cancelled));
    assert_eq!(host.provider.remove_calls.load(Ordering::SeqCst), 1);
    let video = generated_video::Entity::find()
        .filter(generated_video::Column::ProjectId.eq(project_id))
        .one(database.as_ref())
        .await
        .expect("查询中断视频")
        .expect("中断视频记录应保留");
    assert_eq!(
        video.status,
        a4n_domain::video_workflow::VIDEO_STATUS_INTERRUPTED
    );
}

#[tokio::test]
async fn chapter_video_generate_rejects_prompt_missing_execution_identity() {
    let (database, context, owner_id, project_id, chapter_id) = setup_project_chapter().await;
    let plan = insert_ready_segment_plan(database.as_ref(), project_id, chapter_id).await;
    let prompt =
        insert_ready_video_prompt(database.as_ref(), project_id, chapter_id, plan.id, 1, None)
            .await;
    chapter_video_prompt::ActiveModel {
        id: Set(prompt.id),
        metadata_json: Set(Some(json!({
            "source_prompt_type": chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN,
            "prompt_output_language": "zh_cn",
        }))),
        ..Default::default()
    }
    .update(database.as_ref())
    .await
    .expect("应可删除执行身份用于边界测试");
    let host = RejectingChapterHost::default();
    let mut progress = empty_progress();
    let err = context
        .run_chapter_video_generation_validated(
            &host,
            GenerateChapterVideoInput {
                user_id: owner_id,
                project_id,
                chapter_id,
                prompt_id: prompt.id,
                config_id: Some(7),
            },
            &mut progress,
        )
        .await
        .expect_err("缺少 source_prompt_type / execution identity 必须显式失败");
    let message = err.to_string();
    assert!(
        message.contains("source_prompt_type") || message.contains("video_execution_identity"),
        "unexpected error: {message}"
    );
}

#[tokio::test]
async fn chapter_batch_validation_rejects_invalid_inputs_and_accepts_existing_candidates() {
    let (database, context, owner_id, project_id, chapter_id) = setup_project_chapter().await;
    let plan = insert_ready_segment_plan(database.as_ref(), project_id, chapter_id).await;

    let empty = context
        .validate_chapter_video_batch_prompts(&GenerateChapterVideosBatchInput {
            user_id: owner_id,
            project_id,
            chapter_id,
            prompt_ids: Vec::new(),
        })
        .await
        .expect_err("空批量应失败");
    assert!(empty.to_string().contains("至少需要选择一条视频提示词"));

    let p1 = insert_ready_video_prompt(database.as_ref(), project_id, chapter_id, plan.id, 1, None)
        .await;
    let duplicate = context
        .validate_chapter_video_batch_prompts(&GenerateChapterVideosBatchInput {
            user_id: owner_id,
            project_id,
            chapter_id,
            prompt_ids: vec![p1.id, p1.id],
        })
        .await
        .expect_err("重复 id 应失败");
    assert!(duplicate.to_string().contains("不能重复"));

    let mut running =
        insert_ready_video_prompt(database.as_ref(), project_id, chapter_id, plan.id, 2, None)
            .await;
    running = chapter_video_prompt::ActiveModel {
        id: Set(running.id),
        status: Set("running".to_string()),
        ..Default::default()
    }
    .update(database.as_ref())
    .await
    .expect("更新 running 状态");
    let running_err = context
        .validate_chapter_video_batch_prompts(&GenerateChapterVideosBatchInput {
            user_id: owner_id,
            project_id,
            chapter_id,
            prompt_ids: vec![running.id],
        })
        .await
        .expect_err("running 提示词应失败");
    assert!(running_err.to_string().contains("正在运行"));

    let bound_video = a4n_db::video::create_generated_video_record(
        database.as_ref(),
        video_record_input(project_id),
    )
    .await
    .expect("创建绑定视频");
    let bound = insert_ready_video_prompt(
        database.as_ref(),
        project_id,
        chapter_id,
        plan.id,
        3,
        Some(bound_video.id),
    )
    .await;
    let bound_ids = context
        .validate_chapter_video_batch_prompts(&GenerateChapterVideosBatchInput {
            user_id: owner_id,
            project_id,
            chapter_id,
            prompt_ids: vec![bound.id],
        })
        .await
        .expect("已有候选的视频提示词仍可继续生成新候选");
    assert_eq!(bound_ids, vec![bound.id]);
}

#[tokio::test]
async fn chapter_batch_validation_accepts_ready_provider_prompts() {
    let (database, context, owner_id, project_id, chapter_id) = setup_project_chapter().await;
    let plan = insert_ready_segment_plan(database.as_ref(), project_id, chapter_id).await;
    let first =
        insert_ready_video_prompt(database.as_ref(), project_id, chapter_id, plan.id, 1, None)
            .await;
    let second =
        insert_ready_video_prompt(database.as_ref(), project_id, chapter_id, plan.id, 2, None)
            .await;
    let ids = context
        .validate_chapter_video_batch_prompts(&GenerateChapterVideosBatchInput {
            user_id: owner_id,
            project_id,
            chapter_id,
            prompt_ids: vec![first.id, second.id],
        })
        .await
        .expect("boundary_frames completed 提示词应通过批量门禁");
    assert_eq!(ids, vec![first.id, second.id]);
}
