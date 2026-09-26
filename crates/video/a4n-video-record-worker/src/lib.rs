//! PostgreSQL-backed generated-video record read worker.

use std::{collections::HashSet, env, net::SocketAddr, sync::Arc};

use a4n_api_contract::image::GeneratedImageSummary;
use a4n_api_contract::video::{ChapterVideoPromptSummary, GeneratedVideoSummary};
use a4n_db::DbHandle;
use a4n_domain::collaboration::ProjectAccessRole;
use a4n_video_record_worker_contract::{
    ChapterVideosInput, LoadVideoSegmentPlanDraftInput, ReferenceImagesInput,
    VIDEO_PROMPT_CAPABILITY, VIDEO_PROMPT_DRAFT_CAPABILITY, VIDEO_PROMPT_DRAFT_LOAD_OPERATION,
    VIDEO_PROMPT_DRAFT_UPDATE_OPERATION, VIDEO_PROMPT_LIST_OPERATION,
    VIDEO_PROMPT_VALIDATE_SOURCE_OPERATION, VIDEO_RECORD_CAPABILITY, VIDEO_RECORD_LIST_OPERATION,
    VIDEO_RECORD_MUTATION_CAPABILITY, VIDEO_RECORD_PLAYBACK_OPERATION,
    VIDEO_RECORD_REFERENCE_IMAGES_OPERATION, VIDEO_RECORD_UPDATE_OPERATION,
    VIDEO_RECORD_WORKER_CONTRACT_VERSION, VIDEO_RECORD_WORKER_ID, VideoIdInput,
    VideoSegmentPlanDraftSummary, VideoSourceInput, VideoUpdateInput,
};
use a4n_worker_client::WORKER_AUTH_HEADER;
use a4n_worker_contract::{
    WORKER_CAPABILITIES_PATH, WORKER_HEALTH_PATH, WORKER_SCHEMA_VERSION, WorkerCapabilities,
    WorkerCapability, WorkerEnvelope, WorkerError, WorkerErrorCode, WorkerHealth,
    WorkerHealthStatus, WorkerOperationRef, WorkerOperationStatus, WorkerPrincipalKind,
    WorkerResponse,
};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseConnection, Statement};
use serde_json::Value;
use subtle::ConstantTimeEq;

const WORKER_ADDR_ENV: &str = "A4N_VIDEO_RECORD_WORKER_ADDR";
const DATABASE_URL_ENV: &str = "A4N_DATABASE_URL";
const WORKER_TOKEN_ENV: &str = "A4N_VIDEO_RECORD_WORKER_TOKEN";
const DEFAULT_ADDR: &str = "127.0.0.1:47675";

#[derive(Clone)]
pub struct VideoRecordWorker {
    state: WorkerState,
}

#[derive(Clone)]
struct WorkerState {
    database: DbHandle,
    worker_id: Arc<str>,
    auth_token: Arc<str>,
}

#[derive(Debug)]
struct WorkerFailure {
    code: WorkerErrorCode,
    message: String,
    retryable: bool,
}

impl VideoRecordWorker {
    pub fn new(
        database: DatabaseConnection,
        worker_id: impl Into<String>,
        auth_token: impl Into<String>,
    ) -> Self {
        Self {
            state: WorkerState {
                database: DbHandle::new(Arc::new(database)),
                worker_id: Arc::<str>::from(worker_id.into()),
                auth_token: Arc::<str>::from(auth_token.into()),
            },
        }
    }

    pub fn router(&self) -> Router {
        Router::new()
            .route(WORKER_HEALTH_PATH, get(health))
            .route(WORKER_CAPABILITIES_PATH, get(capabilities))
            .route("/v1/commands/video.record.playback", post(playback_source))
            .route(
                "/v1/commands/video.record.list_by_chapter",
                post(list_by_chapter),
            )
            .route(
                "/v1/commands/video.record.reference_images",
                post(validated_reference_images),
            )
            .route("/v1/commands/video.record.update", post(update_record))
            .route(
                "/v1/commands/video.prompt.draft.load",
                post(load_video_segment_plan_draft),
            )
            .route(
                "/v1/commands/video.prompt.draft.update",
                post(update_video_segment_plan_draft_recipe),
            )
            .route(
                "/v1/commands/video.prompt.list_by_chapter",
                post(list_prompts),
            )
            .route(
                "/v1/commands/video.prompt.validate_source",
                post(validate_source),
            )
            .with_state(self.state.clone())
    }

    pub async fn serve(self, address: SocketAddr) -> anyhow::Result<()> {
        let listener = tokio::net::TcpListener::bind(address).await?;
        axum::serve(listener, self.router())
            .with_graceful_shutdown(shutdown_signal())
            .await
            .map_err(anyhow::Error::from)
    }
}

pub fn run() -> anyhow::Result<()> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async_main())
}

async fn async_main() -> anyhow::Result<()> {
    init_tracing();
    let database_url = a4n_config::required_non_empty_env_string(DATABASE_URL_ENV)?;
    a4n_config::validate_postgres_database_url(DATABASE_URL_ENV, &database_url)?;
    let auth_token = a4n_config::required_non_empty_env_string(WORKER_TOKEN_ENV)?;
    let address = worker_addr()?;
    let mut options = ConnectOptions::new(database_url);
    options
        .max_connections(8)
        .min_connections(1)
        .connect_timeout(std::time::Duration::from_secs(5));
    let database = Database::connect(options).await?;
    VideoRecordWorker::new(database, VIDEO_RECORD_WORKER_ID, auth_token)
        .serve(address)
        .await
}

fn worker_addr() -> anyhow::Result<SocketAddr> {
    let value = env::var(WORKER_ADDR_ENV).unwrap_or_else(|_| DEFAULT_ADDR.into());
    let address: SocketAddr = value
        .parse()
        .map_err(|error| anyhow::anyhow!("{WORKER_ADDR_ENV} 无效: {error}"))?;
    if !address.ip().is_loopback() || address.port() == 0 {
        anyhow::bail!("{WORKER_ADDR_ENV} 必须是 loopback 且端口大于 0")
    }
    Ok(address)
}

async fn health(State(state): State<WorkerState>, headers: HeaderMap) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    let status = match state
        .database
        .connection()
        .execute(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT 1",
        ))
        .await
    {
        Ok(_) => WorkerHealthStatus::Healthy,
        Err(_) => WorkerHealthStatus::Degraded,
    };
    Json(WorkerHealth {
        schema_version: WORKER_SCHEMA_VERSION,
        worker_id: state.worker_id.to_string(),
        status,
        version: env!("CARGO_PKG_VERSION").into(),
    })
    .into_response()
}

async fn capabilities(State(state): State<WorkerState>, headers: HeaderMap) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    Json(WorkerCapabilities {
        schema_version: WORKER_SCHEMA_VERSION,
        worker_id: state.worker_id.to_string(),
        capabilities: vec![
            WorkerCapability {
                name: VIDEO_RECORD_CAPABILITY.into(),
                version: VIDEO_RECORD_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
            WorkerCapability {
                name: VIDEO_PROMPT_CAPABILITY.into(),
                version: VIDEO_RECORD_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
            WorkerCapability {
                name: VIDEO_RECORD_MUTATION_CAPABILITY.into(),
                version: VIDEO_RECORD_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
            WorkerCapability {
                name: VIDEO_PROMPT_DRAFT_CAPABILITY.into(),
                version: VIDEO_RECORD_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
        ],
    })
    .into_response()
}

async fn playback_source(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<VideoIdInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.playback_source(request)
    })
    .await
}

async fn list_by_chapter(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ChapterVideosInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.list_by_chapter(request)
    })
    .await
}

async fn validated_reference_images(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ReferenceImagesInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.validated_reference_images(request)
    })
    .await
}

async fn update_record(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<VideoUpdateInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.update_record(request)
    })
    .await
}

async fn load_video_segment_plan_draft(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<LoadVideoSegmentPlanDraftInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.load_video_segment_plan_draft(request)
    })
    .await
}

async fn update_video_segment_plan_draft_recipe(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<
        WorkerEnvelope<a4n_video_record_worker_contract::UpdateVideoSegmentPlanDraftRecipeInput>,
    >,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.update_video_segment_plan_draft_recipe(request)
    })
    .await
}

async fn list_prompts(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ChapterVideosInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.list_prompts(request)
    })
    .await
}

async fn validate_source(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<VideoSourceInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.validate_source(request)
    })
    .await
}

async fn command_response<T, O, F, Fut>(
    state: &WorkerState,
    headers: &HeaderMap,
    request: WorkerEnvelope<T>,
    command: F,
) -> Response
where
    T: serde::Serialize + Clone,
    O: serde::Serialize,
    F: FnOnce(WorkerEnvelope<T>) -> Fut,
    Fut: std::future::Future<Output = Result<O, WorkerFailure>>,
{
    if !state.authorized(headers) {
        return unauthorized();
    }
    let response = match command(request.clone()).await {
        Ok(payload) => completed(&request, payload),
        Err(error) => failed(&request, error),
    };
    Json(response).into_response()
}

impl WorkerState {
    async fn load_video_segment_plan_draft(
        &self,
        request: WorkerEnvelope<LoadVideoSegmentPlanDraftInput>,
    ) -> Result<VideoSegmentPlanDraftSummary, WorkerFailure> {
        validate_request(&request, VIDEO_PROMPT_DRAFT_LOAD_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        positive(input.project_id, "project_id")?;
        positive(input.chapter_id, "chapter_id")?;
        positive(input.workflow_run_id, "workflow_run_id")?;
        verify_project_permission(
            &self.database,
            input.project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        let run = a4n_db::workflow::workflow_run_summary_by_id(
            self.database.connection(),
            input.workflow_run_id,
        )
        .await
        .map_err(workflow_error)?;
        if run.workflow_kind != "video_segment_plan"
            || run.project_id != Some(input.project_id)
            || run.chapter_id != Some(input.chapter_id)
            || run.mode.as_deref()
                != Some(a4n_db_models::models::chapter::chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN)
        {
            return Err(WorkerFailure::invalid_request(format!(
                "片段规划 workflow {} 与当前项目或章节不一致",
                input.workflow_run_id
            )));
        }
        if !a4n_db::workflow::workflow_run_is_running_or_resumable(&run.status, run.can_resume) {
            return Err(WorkerFailure::conflict(format!(
                "{}:加载片段规划草稿执行方案",
                run.status
            )));
        }
        let fingerprint = run
            .input_fingerprint_json
            .as_ref()
            .and_then(Value::as_object)
            .ok_or_else(|| WorkerFailure::conflict("缺少片段规划 workflow 输入指纹"))?;
        if fingerprint.get("schema_version").and_then(Value::as_i64) != Some(1)
            || fingerprint.get("workflow_kind").and_then(Value::as_str)
                != Some("video_segment_plan")
            || fingerprint.get("project_id").and_then(Value::as_i64) != Some(input.project_id)
            || fingerprint.get("chapter_id").and_then(Value::as_i64) != Some(input.chapter_id)
            || fingerprint.get("workflow_mode").and_then(Value::as_str)
                != Some(
                    a4n_db_models::models::chapter::chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN,
                )
        {
            return Err(WorkerFailure::conflict("片段规划 workflow 输入指纹无效"));
        }
        let style = fingerprint
            .get("options")
            .and_then(Value::as_object)
            .and_then(|options| options.get("style"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|style| !style.is_empty())
            .ok_or_else(|| WorkerFailure::conflict("片段规划 workflow 缺少 style"))?;
        let checkpoints = a4n_db::video::video_segment_plan_draft_checkpoints(
            self.database.connection(),
            run.id,
            "video_segment_plan.shot.",
        )
        .await
        .map_err(video_error)?
        .into_iter()
        .map(
            |checkpoint| a4n_video_contract::VideoSegmentPlanDraftCheckpoint {
                step_key: checkpoint.step_key,
                extra_json: checkpoint.extra_json,
                parsed_output_json: checkpoint.parsed_output_json,
                artifacts_json: checkpoint.artifacts_json,
            },
        )
        .collect::<Vec<_>>();
        let (segments, recipes, completed_shot_count, total_shot_count) = if checkpoints.is_empty()
        {
            (Vec::new(), Vec::new(), 0, 0)
        } else {
            a4n_video_contract::load_video_segment_plan_draft_recipe_state(&checkpoints, style)
                .map_err(worker_app_error)?
        };
        Ok(VideoSegmentPlanDraftSummary {
            run: a4n_workflow_core::workflow_run_summary_to_api(run),
            segments,
            recipes,
            completed_shot_count,
            total_shot_count,
        })
    }

    async fn update_video_segment_plan_draft_recipe(
        &self,
        request: WorkerEnvelope<
            a4n_video_record_worker_contract::UpdateVideoSegmentPlanDraftRecipeInput,
        >,
    ) -> Result<(), WorkerFailure> {
        validate_request(&request, VIDEO_PROMPT_DRAFT_UPDATE_OPERATION)?;
        let input = request.payload.clone();
        let load_request = WorkerEnvelope {
            schema_version: request.schema_version,
            request_id: request.request_id.clone(),
            trace_id: request.trace_id.clone(),
            principal: request.principal.clone(),
            operation_id: VIDEO_PROMPT_DRAFT_LOAD_OPERATION.to_string(),
            idempotency_key: request.idempotency_key.clone(),
            project_id: request.project_id,
            root_project_id: request.root_project_id,
            target_revision: request.target_revision,
            runtime_config_snapshot_ref: request.runtime_config_snapshot_ref.clone(),
            lease: request.lease.clone(),
            payload: LoadVideoSegmentPlanDraftInput {
                project_id: input.project_id,
                chapter_id: input.chapter_id,
                workflow_run_id: input.workflow_run_id,
            },
        };
        let summary = self.load_video_segment_plan_draft(load_request).await?;
        let style = summary
            .run
            .input_fingerprint_json
            .as_ref()
            .and_then(Value::as_object)
            .and_then(|fingerprint| fingerprint.get("options"))
            .and_then(Value::as_object)
            .and_then(|options| options.get("style"))
            .and_then(Value::as_str)
            .filter(|style| !style.trim().is_empty())
            .ok_or_else(|| WorkerFailure::conflict("片段规划 workflow 缺少 style"))?;
        a4n_workflow_core::update_video_segment_plan_draft_recipe_checkpoint(
            self.database.connection(),
            input.workflow_run_id,
            &input.source_shot_id,
            input.segment_index_in_shot,
            style,
            input.recipe,
        )
        .await
        .map_err(worker_app_error)
    }

    async fn update_record(
        &self,
        request: WorkerEnvelope<VideoUpdateInput>,
    ) -> Result<GeneratedVideoSummary, WorkerFailure> {
        validate_request(&request, VIDEO_RECORD_UPDATE_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        let video_id = positive(input.video_id, "video_id")?;
        if input.input.is_empty() {
            return Err(WorkerFailure::invalid_request("视频更新内容不能为空"));
        }
        let video = a4n_db::video::generated_video_by_id(self.database.connection(), video_id)
            .await
            .map_err(database_error)?
            .ok_or_else(|| WorkerFailure::not_found("视频不存在"))?;
        verify_project_permission(
            &self.database,
            video.project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        let modifies_pushed_source =
            input.input.prompt.is_some() || input.input.negative_prompt.is_some();
        if modifies_pushed_source && self.video_record_is_pushed_source(&video).await? {
            return Err(WorkerFailure::invalid_request(
                "该视频片段已推送到成片后处理，不能修改",
            ));
        }
        let (negative_prompt, clear_negative_prompt) = match input.input.negative_prompt {
            Some(Some(value)) => (Some(value), false),
            Some(None) => (None, true),
            None => (None, false),
        };
        let updated = a4n_db::video::update_generated_video_record(
            self.database.connection(),
            video_id,
            a4n_domain::media::UpdateGeneratedVideoRecordInput {
                prompt: input.input.prompt,
                negative_prompt,
                clear_negative_prompt,
                metadata_json: None,
                clear_metadata_json: false,
                is_favorite: input.input.is_favorite,
            },
        )
        .await
        .map_err(video_error)?;
        Ok(video_summary(&updated))
    }

    async fn video_record_is_pushed_source(
        &self,
        video: &a4n_db_models::models::ai::generated_video::Model,
    ) -> Result<bool, WorkerFailure> {
        if !matches!(
            video.video_type.as_str(),
            a4n_domain::video_workflow::VIDEO_TYPE_CHAPTER
                | a4n_domain::video_workflow::VIDEO_TYPE_REFERENCE_MEDIA
                | a4n_domain::video_workflow::VIDEO_TYPE_ASSEMBLY_INSERT
        ) {
            return Ok(false);
        }
        let (Some(chapter_id), Some(workflow_mode)) = (
            video.chapter_id,
            video.workflow_mode.as_deref().map(str::trim),
        ) else {
            return Ok(false);
        };
        let rows = a4n_db::video::generated_videos_for_chapter(
            self.database.connection(),
            video.project_id,
            chapter_id,
        )
        .await
        .map_err(database_error)?;
        for row in rows.into_iter().filter(|row| {
            row.video_type == a4n_domain::video_workflow::VIDEO_TYPE_ASSEMBLY_PUSH
                && row.workflow_mode.as_deref() == Some(workflow_mode)
                && row.status == a4n_domain::video_workflow::VIDEO_STATUS_COMPLETED
        }) {
            let Some(metadata) = row
                .metadata_json
                .as_ref()
                .and_then(|value| value.get("assembly_push"))
                .and_then(serde_json::Value::as_object)
            else {
                return Err(WorkerFailure::invalid_request(format!(
                    "推送批次 #{} 缺少 assembly_push metadata",
                    row.id
                )));
            };
            let status = metadata
                .get("status")
                .and_then(serde_json::Value::as_str)
                .filter(|status| {
                    matches!(
                        *status,
                        a4n_domain::video_workflow::ASSEMBLY_PUSH_STATUS_PENDING
                            | a4n_domain::video_workflow::ASSEMBLY_PUSH_STATUS_ACCEPTED
                            | a4n_domain::video_workflow::ASSEMBLY_PUSH_STATUS_REJECTED
                    )
                })
                .ok_or_else(|| {
                    WorkerFailure::invalid_request(format!(
                        "推送批次 #{} 的 assembly_push.status 无效",
                        row.id
                    ))
                })?;
            if status == a4n_domain::video_workflow::ASSEMBLY_PUSH_STATUS_REJECTED {
                continue;
            }
            let source_ids = metadata
                .get("source_video_ids")
                .and_then(serde_json::Value::as_array)
                .ok_or_else(|| {
                    WorkerFailure::invalid_request(format!(
                        "推送批次 #{} 缺少 source_video_ids",
                        row.id
                    ))
                })?;
            if source_ids.iter().any(|source_id| {
                source_id
                    .as_i64()
                    .is_some_and(|source_id| source_id == video.id)
            }) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    async fn playback_source(
        &self,
        request: WorkerEnvelope<VideoIdInput>,
    ) -> Result<GeneratedVideoSummary, WorkerFailure> {
        validate_request(&request, VIDEO_RECORD_PLAYBACK_OPERATION)?;
        let video_id = positive(request.payload.video_id, "video_id")?;
        let video = a4n_db::video::generated_video_by_id(self.database.connection(), video_id)
            .await
            .map_err(database_error)?
            .ok_or_else(|| WorkerFailure::not_found("视频不存在"))?;
        verify_project_permission(
            &self.database,
            video.project_id,
            user_id(&request)?,
            ProjectAccessRole::Editor,
        )
        .await?;
        Ok(video_summary(&video))
    }

    async fn list_by_chapter(
        &self,
        request: WorkerEnvelope<ChapterVideosInput>,
    ) -> Result<Vec<GeneratedVideoSummary>, WorkerFailure> {
        validate_request(&request, VIDEO_RECORD_LIST_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        positive(input.project_id, "project_id")?;
        positive(input.chapter_id, "chapter_id")?;
        verify_project_permission(
            &self.database,
            input.project_id,
            user_id,
            ProjectAccessRole::Viewer,
        )
        .await?;
        a4n_db::video::ensure_video_chapter_belongs_to_project_record(
            self.database.connection(),
            input.project_id,
            input.chapter_id,
        )
        .await
        .map_err(video_error)?;
        a4n_db::video::generated_videos_for_chapter(
            self.database.connection(),
            input.project_id,
            input.chapter_id,
        )
        .await
        .map_err(database_error)
        .map(|videos| videos.iter().map(video_summary).collect())
    }

    async fn validated_reference_images(
        &self,
        request: WorkerEnvelope<ReferenceImagesInput>,
    ) -> Result<Vec<GeneratedImageSummary>, WorkerFailure> {
        validate_request(&request, VIDEO_RECORD_REFERENCE_IMAGES_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        positive(input.project_id, "project_id")?;
        positive(input.chapter_id, "chapter_id")?;
        verify_project_permission(
            &self.database,
            input.project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        let images = a4n_db::video::validated_video_reference_image_records(
            self.database.connection(),
            input.project_id,
            input.chapter_id,
            input.reference_image_ids_json.as_ref(),
        )
        .await
        .map_err(video_error)?;
        let project_ids = images
            .iter()
            .map(|image| image.project_id)
            .collect::<HashSet<_>>();
        for project_id in project_ids {
            verify_project_permission(
                &self.database,
                project_id,
                user_id,
                ProjectAccessRole::Editor,
            )
            .await?;
        }
        Ok(images.iter().map(image_summary).collect())
    }

    async fn list_prompts(
        &self,
        request: WorkerEnvelope<ChapterVideosInput>,
    ) -> Result<Vec<ChapterVideoPromptSummary>, WorkerFailure> {
        validate_request(&request, VIDEO_PROMPT_LIST_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        positive(input.project_id, "project_id")?;
        positive(input.chapter_id, "chapter_id")?;
        verify_project_permission(
            &self.database,
            input.project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        validate_video_source(
            &self.database,
            input.project_id,
            input.chapter_id,
            "查看视频提示词",
            false,
        )
        .await?;
        let prompts = a4n_db::video::chapter_video_prompts_for_chapter(
            self.database.connection(),
            input.project_id,
            input.chapter_id,
        )
        .await
        .map_err(database_error)?;
        let mut result = Vec::with_capacity(prompts.len());
        for prompt in prompts {
            let prompt =
                a4n_db::video::with_video_plan_source_sync(self.database.connection(), prompt)
                    .await
                    .map_err(video_error)?;
            result.push(a4n_video::chapter_video_prompt_summary(&prompt));
        }
        Ok(result)
    }

    async fn validate_source(
        &self,
        request: WorkerEnvelope<VideoSourceInput>,
    ) -> Result<(), WorkerFailure> {
        validate_request(&request, VIDEO_PROMPT_VALIDATE_SOURCE_OPERATION)?;
        let input = request.payload;
        positive(input.project_id, "project_id")?;
        positive(input.chapter_id, "chapter_id")?;
        validate_video_source(
            &self.database,
            input.project_id,
            input.chapter_id,
            &input.operation,
            input.writable,
        )
        .await
    }

    fn authorized(&self, headers: &HeaderMap) -> bool {
        headers
            .get(WORKER_AUTH_HEADER)
            .is_some_and(|value| bool::from(value.as_bytes().ct_eq(self.auth_token.as_bytes())))
    }
}

fn video_summary(
    video: &a4n_db_models::models::ai::generated_video::Model,
) -> GeneratedVideoSummary {
    GeneratedVideoSummary {
        id: video.id,
        project_id: video.project_id,
        chapter_id: video.chapter_id,
        video_type: video.video_type.clone(),
        workflow_mode: video.workflow_mode.clone(),
        file_path: video.file_path.clone(),
        thumbnail_path: video.thumbnail_path.clone(),
        prompt: video.prompt.clone(),
        negative_prompt: video.negative_prompt.clone(),
        model: video.model.clone(),
        width: video.width,
        height: video.height,
        duration_secs: video.duration_secs,
        fps: video.fps,
        file_size: video.file_size,
        source_video_id: video.source_video_id,
        source_image_id: video.source_image_id,
        status: video.status.clone(),
        provider_job_id: video.provider_job_id.clone(),
        error_message: video.error_message.clone(),
        metadata_json: video.metadata_json.clone(),
        timeline_item_id: video.timeline_item_id,
        is_favorite: video.is_favorite,
        created_at: video.created_at.and_utc().to_rfc3339(),
        updated_at: video.updated_at.and_utc().to_rfc3339(),
    }
}

fn image_summary(
    image: &a4n_db_models::models::ai::generated_image::Model,
) -> GeneratedImageSummary {
    GeneratedImageSummary {
        id: image.id,
        project_id: image.project_id,
        image_type: image.image_type.clone(),
        file_path: image.file_path.clone(),
        prompt: image.prompt.clone(),
        model: image.model.clone(),
        width: image.width,
        height: image.height,
        file_size: image.file_size,
        ai_score: image.ai_score,
        is_favorite: image.is_favorite,
        is_system_managed: a4n_domain::image::is_system_managed_image_type(&image.image_type),
        chapter_id: image.chapter_id,
        panel_id: image.panel_id.clone(),
        metadata_json: image.metadata_json.clone(),
        created_at: image.created_at.and_utc().to_rfc3339(),
    }
}

async fn verify_project_permission(
    database: &DbHandle,
    project_id: i64,
    user_id: i64,
    role: ProjectAccessRole,
) -> Result<(), WorkerFailure> {
    a4n_db::collaboration::verify_project_permission(
        database.connection(),
        project_id,
        user_id,
        role,
    )
    .await
    .map(|_| ())
    .map_err(collaboration_error)
}

fn validate_request<T: serde::Serialize>(
    request: &WorkerEnvelope<T>,
    operation: &str,
) -> Result<(), WorkerFailure> {
    request
        .validate()
        .map_err(|error| WorkerFailure::invalid_request(error.to_string()))?;
    if request.operation_id != operation {
        return Err(WorkerFailure::invalid_request(format!(
            "operation_id 不匹配: expected={operation}, actual={}",
            request.operation_id
        )));
    }
    if request.lease.worker_id != VIDEO_RECORD_WORKER_ID {
        return Err(WorkerFailure::invalid_request(
            "lease.worker_id 不属于 video record worker",
        ));
    }
    Ok(())
}

fn user_id<T: serde::Serialize>(request: &WorkerEnvelope<T>) -> Result<i64, WorkerFailure> {
    if request.principal.kind != WorkerPrincipalKind::User {
        return Err(WorkerFailure::unauthorized("仅允许用户 principal"));
    }
    request
        .principal
        .subject
        .parse::<i64>()
        .ok()
        .filter(|id| *id > 0)
        .ok_or_else(|| WorkerFailure::unauthorized("用户 principal subject 无效"))
}

fn positive(value: i64, field: &str) -> Result<i64, WorkerFailure> {
    if value > 0 {
        Ok(value)
    } else {
        Err(WorkerFailure::invalid_request(format!(
            "{field} 必须大于 0"
        )))
    }
}

async fn validate_video_source(
    database: &DbHandle,
    project_id: i64,
    chapter_id: i64,
    operation: &str,
    writable: bool,
) -> Result<(), WorkerFailure> {
    let (project, chapter) = a4n_db::video::project_and_chapter_for_video_source(
        database.connection(),
        project_id,
        chapter_id,
    )
    .await
    .map_err(video_error)?;
    match project.project_kind.as_str() {
        "text" => {
            if chapter.project_id != project.id {
                return Err(WorkerFailure::invalid_request(format!(
                    "{operation}失败：章节不属于文本项目 {project_id}"
                )));
            }
            if chapter.status == "deleted" || (writable && chapter.status == "generating") {
                return Err(WorkerFailure::conflict(format!(
                    "{}: {operation}",
                    chapter.status
                )));
            }
        }
        "video" => {
            if project.root_project_id != Some(chapter.project_id) {
                return Err(WorkerFailure::invalid_request(format!(
                    "{operation}失败：章节不属于视频项目 {project_id} 绑定的根文本项目"
                )));
            }
            let snapshot_id = project.source_snapshot_id.ok_or_else(|| {
                WorkerFailure::conflict(format!("missing_source_snapshot: {operation}"))
            })?;
            a4n_db::video::source_snapshot_chapter_for_video_source(
                database.connection(),
                snapshot_id,
                chapter_id,
            )
            .await
            .map_err(video_error)?
            .ok_or_else(|| {
                WorkerFailure::not_found(format!(
                    "{operation}失败：视频项目 {project_id} 的源快照不包含章节 {chapter_id}"
                ))
            })?;
        }
        other => {
            return Err(WorkerFailure::invalid_request(format!(
                "{operation}失败：视频链路不支持项目类型 {other}: project_id={project_id}"
            )));
        }
    }
    Ok(())
}

fn database_error(error: sea_orm::DbErr) -> WorkerFailure {
    WorkerFailure::unavailable(error.to_string())
}

fn worker_app_error(error: a4n_error::AppError) -> WorkerFailure {
    match error {
        a4n_error::AppError::InvalidArgument(message) => WorkerFailure::invalid_request(message),
        a4n_error::AppError::InvalidState { current, operation } => {
            WorkerFailure::conflict(format!("{current}: {operation}"))
        }
        a4n_error::AppError::NotFound(message) => WorkerFailure::not_found(message),
        a4n_error::AppError::Forbidden(message) => WorkerFailure::unauthorized(message),
        other => WorkerFailure::unavailable(other.to_string()),
    }
}

fn workflow_error(error: a4n_db::workflow::WorkflowReadError) -> WorkerFailure {
    match error {
        a4n_db::workflow::WorkflowReadError::InvalidArgument(message)
        | a4n_db::workflow::WorkflowReadError::InvalidData(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::workflow::WorkflowReadError::NotFound(message) => WorkerFailure::not_found(message),
        a4n_db::workflow::WorkflowReadError::Database(error) => database_error(error),
    }
}

fn video_error(error: a4n_db::video::VideoDbError) -> WorkerFailure {
    match error {
        a4n_db::video::VideoDbError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::video::VideoDbError::NotFound(message) => WorkerFailure::not_found(message),
        a4n_db::video::VideoDbError::Forbidden(message) => WorkerFailure::unauthorized(message),
        a4n_db::video::VideoDbError::InvalidState { current, operation } => {
            WorkerFailure::conflict(format!("{current}: {operation}"))
        }
        a4n_db::video::VideoDbError::Database(error) => database_error(error),
    }
}

fn collaboration_error(error: a4n_db::collaboration::CollaborationDbError) -> WorkerFailure {
    match error {
        a4n_db::collaboration::CollaborationDbError::InvalidArgument(message) => {
            WorkerFailure::invalid_request(message)
        }
        a4n_db::collaboration::CollaborationDbError::NotFound(message) => {
            WorkerFailure::not_found(message)
        }
        a4n_db::collaboration::CollaborationDbError::Forbidden(message) => {
            WorkerFailure::unauthorized(message)
        }
        a4n_db::collaboration::CollaborationDbError::InvalidState(message)
        | a4n_db::collaboration::CollaborationDbError::Internal(message) => {
            WorkerFailure::conflict(message)
        }
        a4n_db::collaboration::CollaborationDbError::Database(error) => database_error(error),
    }
}

fn completed<T>(request: &WorkerEnvelope<impl serde::Serialize>, payload: T) -> WorkerResponse<T> {
    WorkerResponse {
        schema_version: WORKER_SCHEMA_VERSION,
        request_id: request.request_id.clone(),
        trace_id: request.trace_id.clone(),
        operation: WorkerOperationRef {
            operation_id: request.operation_id.clone(),
            task_id: None,
        },
        status: WorkerOperationStatus::Completed,
        payload: Some(payload),
        resource_refs: Vec::new(),
        committed_revision: None,
        provider_job_id: None,
        result_summary: None,
        pagination: None,
        error: None,
    }
}

fn failed<T>(
    request: &WorkerEnvelope<impl serde::Serialize>,
    failure: WorkerFailure,
) -> WorkerResponse<T> {
    WorkerResponse {
        schema_version: WORKER_SCHEMA_VERSION,
        request_id: request.request_id.clone(),
        trace_id: request.trace_id.clone(),
        operation: WorkerOperationRef {
            operation_id: request.operation_id.clone(),
            task_id: None,
        },
        status: WorkerOperationStatus::Failed,
        payload: None,
        resource_refs: Vec::new(),
        committed_revision: None,
        provider_job_id: None,
        result_summary: None,
        pagination: None,
        error: Some(WorkerError {
            code: failure.code,
            message: failure.message,
            retryable: failure.retryable,
        }),
    }
}

fn unauthorized() -> Response {
    StatusCode::UNAUTHORIZED.into_response()
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .try_init();
}

impl WorkerFailure {
    fn invalid_request(message: impl Into<String>) -> Self {
        Self {
            code: WorkerErrorCode::InvalidRequest,
            message: message.into(),
            retryable: false,
        }
    }

    fn unauthorized(message: impl Into<String>) -> Self {
        Self {
            code: WorkerErrorCode::Unauthorized,
            message: message.into(),
            retryable: false,
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            code: WorkerErrorCode::NotFound,
            message: message.into(),
            retryable: false,
        }
    }

    fn conflict(message: impl Into<String>) -> Self {
        Self {
            code: WorkerErrorCode::Conflict,
            message: message.into(),
            retryable: false,
        }
    }

    fn unavailable(message: impl Into<String>) -> Self {
        Self {
            code: WorkerErrorCode::Unavailable,
            message: message.into(),
            retryable: true,
        }
    }
}
