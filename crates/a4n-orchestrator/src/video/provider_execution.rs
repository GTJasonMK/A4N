use a4n_core::{
    error::AppError,
    models::ai::{generated_video, video_gen_config},
};
use a4n_db::{SharedDatabaseConnection, generation::canonical_json_sha256};
use a4n_service_protocol::GenerationArtifactRef;
use a4n_video::{VideoGenerationResult, VideoJobState};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

use crate::generation::{
    checkpoint::{GenerationCheckpointMetadata, GenerationCheckpointWriter},
    events::GenerationTaskSession,
};

const VIDEO_JOB_POLL_INTERVAL_SECS: u64 = 8;
const VIDEO_JOB_MAX_WAIT_SECS: u64 = 30 * 60;
const VIDEO_JOB_MAX_CONSECUTIVE_POLL_ERRORS: u32 = 3;
pub const VIDEO_GENERATION_PROGRESS_TOTAL: u32 = 7;

type VideoGenerationProgressCallback<'a> =
    &'a mut (dyn FnMut(u32, u32, &str) -> Result<(), AppError> + Send);

pub struct VideoGenerationProgress<'a> {
    reporter: Option<VideoGenerationProgressCallback<'a>>,
    cancel_token: CancellationToken,
    checkpoint: Option<VideoGenerationCheckpointContext>,
}

impl<'a> VideoGenerationProgress<'a> {
    pub(crate) fn with_checkpoint(
        reporter: Option<VideoGenerationProgressCallback<'a>>,
        cancel_token: CancellationToken,
        checkpoint: VideoGenerationCheckpointContext,
    ) -> Self {
        Self {
            reporter,
            cancel_token,
            checkpoint: Some(checkpoint),
        }
    }

    /// 应用层定向测试使用：不绑定 generation session checkpoint。
    #[cfg(test)]
    pub(crate) fn without_checkpoint(
        reporter: Option<VideoGenerationProgressCallback<'a>>,
        cancel_token: CancellationToken,
    ) -> Self {
        Self {
            reporter,
            cancel_token,
            checkpoint: None,
        }
    }

    pub fn progress(&mut self, current: u32, total: u32, message: &str) -> Result<(), String> {
        if let Some(reporter) = self.reporter.as_deref_mut() {
            reporter(current, total.max(1), message).map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    pub fn check_cancelled(&self, message: &str) -> Result<(), String> {
        if self.cancel_token.is_cancelled() {
            return Err(message.to_string());
        }
        Ok(())
    }

    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel_token.clone()
    }

    pub(crate) async fn checkpoint_provider_request(
        &mut self,
        video_id: i64,
        config: &video_gen_config::Model,
        request: &a4n_video::VideoGenerationRequest,
        staging_artifacts: &[GenerationArtifactRef],
    ) -> Result<Option<String>, AppError> {
        let Some(checkpoint) = self.checkpoint.clone() else {
            return Ok(None);
        };
        checkpoint
            .record_provider_request(video_id, config, request, staging_artifacts)
            .await
            .map(Some)
    }

    pub(crate) async fn checkpoint_provider_job(
        &mut self,
        video_id: i64,
        config: &video_gen_config::Model,
        job_id: &str,
    ) -> Result<(), AppError> {
        let Some(checkpoint) = self.checkpoint.clone() else {
            return Ok(());
        };
        checkpoint
            .record_provider_job(video_id, config, job_id)
            .await
    }

    pub(crate) fn checkpoint_context(&self) -> Option<VideoGenerationCheckpointContext> {
        self.checkpoint.clone()
    }
}

#[derive(Clone)]
pub(crate) struct VideoGenerationCheckpointContext {
    session: Arc<GenerationTaskSession>,
    writer: GenerationCheckpointWriter,
    identity: Arc<tokio::sync::Mutex<Option<ProviderCheckpointIdentity>>>,
}

#[derive(Clone)]
struct ProviderCheckpointIdentity {
    provider_kind: String,
    provider_model: String,
    idempotency_key_sha256: String,
    execution_checkpoint_json: Value,
}

impl VideoGenerationCheckpointContext {
    pub(crate) fn new(
        session: Arc<GenerationTaskSession>,
        database: SharedDatabaseConnection,
    ) -> Self {
        Self {
            session,
            writer: GenerationCheckpointWriter::new(database),
            identity: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }

    async fn record_provider_request(
        &self,
        video_id: i64,
        config: &video_gen_config::Model,
        request: &a4n_video::VideoGenerationRequest,
        staging_artifacts: &[GenerationArtifactRef],
    ) -> Result<String, AppError> {
        let identity_value = json!({
            "task_id": self.session.snapshot().task_id,
            "attempt": self.session.snapshot().attempt,
            "video_id": video_id,
            "provider_kind": config.backend,
            "provider_model": config.model,
            "request": request,
        });
        let idempotency_key_sha256 = canonical_json_sha256(&identity_value)
            .map_err(|error| checkpoint_error(error.to_string()))?;
        let execution_checkpoint_json = json!({
            "version": 1,
            "video_id": video_id,
            "request_fingerprint_sha256": idempotency_key_sha256,
            "staging_artifacts": staging_artifacts,
        });
        let identity = ProviderCheckpointIdentity {
            provider_kind: config.backend.clone(),
            provider_model: config.model.clone(),
            idempotency_key_sha256: idempotency_key_sha256.clone(),
            execution_checkpoint_json: execution_checkpoint_json.clone(),
        };
        self.writer
            .persist_if_due(
                &self.session,
                GenerationCheckpointMetadata {
                    step_key: Some("video_provider_request".to_string()),
                    provider_kind: Some(identity.provider_kind.clone()),
                    provider_model: Some(identity.provider_model.clone()),
                    provider_idempotency_key_sha256: Some(identity.idempotency_key_sha256.clone()),
                    resume_metadata_version: Some(1),
                    execution_checkpoint_json: Some(execution_checkpoint_json),
                    ..Default::default()
                },
                true,
            )
            .await
            .map_err(|error| checkpoint_error(error.to_string()))?;
        *self.identity.lock().await = Some(identity);
        Ok(idempotency_key_sha256)
    }

    async fn record_provider_job(
        &self,
        video_id: i64,
        config: &video_gen_config::Model,
        job_id: &str,
    ) -> Result<(), AppError> {
        let identity = self.identity.lock().await.clone().ok_or_else(|| {
            checkpoint_error("provider job 产生前没有 provider request checkpoint".to_string())
        })?;
        if identity.provider_kind != config.backend || identity.provider_model != config.model {
            return Err(checkpoint_error(
                "provider job checkpoint 与 provider request 不一致".to_string(),
            ));
        }
        let staging_artifacts = identity.execution_checkpoint_json["staging_artifacts"].clone();
        let checkpoint_key = format!("video-provider-job:{video_id}");
        self.writer
            .persist_checkpoint_event(
                &self.session,
                GenerationCheckpointMetadata {
                    step_key: Some("video_provider_job".to_string()),
                    provider_kind: Some(identity.provider_kind),
                    provider_model: Some(identity.provider_model),
                    provider_job_id: Some(job_id.trim().to_string()),
                    provider_idempotency_key_sha256: Some(identity.idempotency_key_sha256),
                    resume_metadata_version: Some(1),
                    execution_checkpoint_json: Some(json!({
                        "version": 1,
                        "video_id": video_id,
                        "provider_job_id": job_id.trim(),
                        "request": identity.execution_checkpoint_json,
                        "staging_artifacts": staging_artifacts,
                    })),
                    ..Default::default()
                },
                checkpoint_key,
                true,
                Some("视频 provider job 已创建，可恢复轮询".to_string()),
            )
            .await
            .map_err(|error| checkpoint_error(error.to_string()))?;
        Ok(())
    }
}

fn checkpoint_error(message: String) -> AppError {
    AppError::InvalidState {
        current: "video_generation_checkpoint".to_string(),
        operation: message,
    }
}

#[derive(Debug, Clone)]
pub struct VideoProviderRequestInput {
    pub project_id: i64,
    pub config: video_gen_config::Model,
    pub request: a4n_video::VideoGenerationRequest,
    pub provider_idempotency_key_sha256: Option<String>,
}

#[derive(Debug, Clone)]
pub struct VideoProviderRequestBody {
    pub request: Value,
    pub source_label: String,
}

#[derive(Debug, Clone)]
pub struct PersistedVideoResult {
    pub video_id: i64,
    pub project_id: i64,
    pub file_path: String,
    pub thumbnail_path: Option<String>,
    pub file_size: i64,
    pub metadata_json: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoArtifactFile {
    pub content_type: String,
    pub sha256: String,
    pub byte_size: u64,
}

#[async_trait]
pub trait VideoProviderHostPort: Send + Sync {
    fn inspect_video_file(
        &self,
        relative_path: &str,
        label: &str,
    ) -> Result<VideoArtifactFile, AppError>;

    async fn poll_video_job(
        &self,
        project_id: i64,
        workflow_run_id: i64,
        config: &video_gen_config::Model,
        job_id: &str,
        cancel_token: CancellationToken,
    ) -> Result<VideoJobState, AppError>;

    async fn persist_video_result(
        &self,
        video: &generated_video::Model,
        prompt_id: Option<i64>,
        result: VideoGenerationResult,
    ) -> Result<PersistedVideoResult, AppError>;

    async fn remove_video_result(&self, persisted: &PersistedVideoResult) -> Result<(), AppError>;
}

pub struct VideoProviderExecutionInput {
    pub video: generated_video::Model,
    pub config: video_gen_config::Model,
    pub prompt_id: Option<i64>,
    pub missing_result_message: String,
    pub cancel_token: CancellationToken,
}

impl crate::OrchestratorContext {
    pub async fn apply_video_provider_result_validated(
        &self,
        input: VideoProviderExecutionInput,
        result: Result<VideoGenerationResult, String>,
        host_port: &dyn VideoProviderHostPort,
    ) -> Result<generated_video::Model, AppError> {
        let VideoProviderExecutionInput {
            video,
            config,
            prompt_id,
            missing_result_message,
            cancel_token,
        } = input;
        match result {
            Ok(result) if !result.bytes.is_empty() => {
                self.complete_video_provider_result(host_port, video, prompt_id, result)
                    .await
            }
            Ok(result) => {
                let job_id = crate::video::required_provider_job_id(
                    result.provider_job_id.as_deref().unwrap_or_default(),
                );
                let job_id = match job_id {
                    Ok(job_id) => job_id.to_string(),
                    Err(_) => {
                        return self
                            .mark_video_provider_failed(video, prompt_id, &missing_result_message)
                            .await;
                    }
                };
                let video = self
                    .mark_generated_video_job_and_prompt_validated(
                        video.id,
                        &job_id,
                        Some(result.metadata),
                        prompt_id,
                    )
                    .await?;
                self.mark_video_provider_workflow_validated(
                    &video,
                    crate::workflow::WORKFLOW_STATUS_RUNNING,
                    true,
                    None,
                )
                .await?;
                self.wait_for_video_provider_job(
                    host_port,
                    video,
                    &config,
                    prompt_id,
                    &job_id,
                    cancel_token,
                )
                .await
            }
            Err(message) => {
                self.mark_video_provider_failed(video, prompt_id, &message)
                    .await
            }
        }
    }

    /// 显式恢复一次 provider job 轮询；状态推进与 Canvas 父 workflow 同步都归这里。
    pub async fn poll_video_provider_once_validated(
        &self,
        video: generated_video::Model,
        config: &video_gen_config::Model,
        prompt_id: Option<i64>,
        cancel_token: CancellationToken,
        host_port: &dyn VideoProviderHostPort,
    ) -> Result<generated_video::Model, AppError> {
        let Some(job_id) =
            crate::video::active_provider_job_id(video.provider_job_id.as_deref(), &video.status)
                .map(str::to_string)
        else {
            return Ok(video);
        };
        let workflow_run_id = self
            .record_video_provider_progress_validated(&video, 1, 2, "正在恢复视频 provider 轮询")
            .await?;
        let state = host_port
            .poll_video_job(
                video.project_id,
                workflow_run_id,
                config,
                &job_id,
                cancel_token.clone(),
            )
            .await?;
        match state {
            VideoJobState::Completed(result) => {
                if cancel_token.is_cancelled() {
                    let updated = self
                        .interrupt_video_provider_job(
                            &video,
                            prompt_id,
                            "视频生成已停止，供应商任务结果不再写入",
                        )
                        .await?;
                    self.sync_canvas_video_workflow(&updated, None).await?;
                    return Ok(updated);
                }
                let updated = self
                    .complete_video_provider_result(host_port, video, prompt_id, result)
                    .await?;
                let (status, error_message) =
                    if updated.status == crate::video::GENERATED_VIDEO_STATUS_COMPLETED {
                        (crate::workflow::WORKFLOW_STATUS_COMPLETED, None)
                    } else {
                        (
                            crate::workflow::WORKFLOW_STATUS_FAILED,
                            updated.error_message.as_deref(),
                        )
                    };
                self.mark_video_provider_workflow_validated(&updated, status, false, error_message)
                    .await?;
                self.sync_canvas_video_workflow(&updated, error_message)
                    .await?;
                Ok(updated)
            }
            VideoJobState::Failed(message) => {
                let updated = self
                    .mark_video_provider_failed(video, prompt_id, &message)
                    .await?;
                self.sync_canvas_video_workflow(&updated, Some(&message))
                    .await?;
                Ok(updated)
            }
            VideoJobState::Pending => {
                let updated = self
                    .update_generated_video_waiting_status_validated(
                        video.id,
                        prompt_id,
                        crate::video::GENERATED_VIDEO_WAITING_STATUS_PENDING,
                    )
                    .await?;
                self.mark_video_provider_workflow_validated(
                    &updated,
                    crate::workflow::WORKFLOW_STATUS_RUNNING,
                    true,
                    None,
                )
                .await?;
                Ok(updated)
            }
            VideoJobState::Running => {
                let updated = self
                    .update_generated_video_waiting_status_validated(
                        video.id,
                        prompt_id,
                        crate::video::GENERATED_VIDEO_WAITING_STATUS_RUNNING,
                    )
                    .await?;
                self.mark_video_provider_workflow_validated(
                    &updated,
                    crate::workflow::WORKFLOW_STATUS_RUNNING,
                    true,
                    None,
                )
                .await?;
                Ok(updated)
            }
        }
    }

    async fn wait_for_video_provider_job(
        &self,
        host_port: &dyn VideoProviderHostPort,
        mut video: generated_video::Model,
        config: &video_gen_config::Model,
        prompt_id: Option<i64>,
        job_id: &str,
        cancel_token: CancellationToken,
    ) -> Result<generated_video::Model, AppError> {
        let workflow = self
            .begin_video_provider_progress_workflow_validated(&video, job_id)
            .await?;
        let started = Instant::now();
        let timeout = Duration::from_secs(VIDEO_JOB_MAX_WAIT_SECS);
        let interval = Duration::from_secs(VIDEO_JOB_POLL_INTERVAL_SECS);
        let mut consecutive_errors = 0_u32;

        loop {
            if cancel_token.is_cancelled() {
                return self
                    .interrupt_video_provider_job(
                        &video,
                        prompt_id,
                        "视频生成已停止，供应商任务仍在处理中，可稍后恢复轮询",
                    )
                    .await;
            }
            if started.elapsed() >= timeout {
                let message = format!(
                    "视频任务 #{} 等待超过 {} 分钟仍未完成，已保留 provider_job_id={}，可稍后恢复轮询",
                    video.id,
                    VIDEO_JOB_MAX_WAIT_SECS / 60,
                    job_id
                );
                return self
                    .interrupt_video_provider_job(&video, prompt_id, &message)
                    .await;
            }

            match host_port
                .poll_video_job(
                    video.project_id,
                    workflow.id,
                    config,
                    job_id,
                    cancel_token.clone(),
                )
                .await
            {
                Ok(VideoJobState::Completed(result)) => {
                    if cancel_token.is_cancelled() {
                        return self
                            .interrupt_video_provider_job(
                                &video,
                                prompt_id,
                                "视频生成已停止，供应商任务结果不再写入",
                            )
                            .await;
                    }
                    let updated = self
                        .complete_video_provider_result(host_port, video, prompt_id, result)
                        .await?;
                    let (status, can_resume, error_message) =
                        if updated.status == crate::video::GENERATED_VIDEO_STATUS_COMPLETED {
                            (crate::workflow::WORKFLOW_STATUS_COMPLETED, false, None)
                        } else {
                            (
                                crate::workflow::WORKFLOW_STATUS_FAILED,
                                false,
                                updated.error_message.as_deref(),
                            )
                        };
                    self.mark_video_provider_workflow_validated(
                        &updated,
                        status,
                        can_resume,
                        error_message,
                    )
                    .await?;
                    return Ok(updated);
                }
                Ok(VideoJobState::Failed(message)) => {
                    return self
                        .mark_video_provider_failed(video, prompt_id, &message)
                        .await;
                }
                Ok(VideoJobState::Pending) => {
                    consecutive_errors = 0;
                    video = self
                        .update_generated_video_waiting_status_validated(
                            video.id,
                            prompt_id,
                            crate::video::GENERATED_VIDEO_WAITING_STATUS_PENDING,
                        )
                        .await?;
                    self.mark_video_provider_workflow_validated(
                        &video,
                        crate::workflow::WORKFLOW_STATUS_RUNNING,
                        true,
                        None,
                    )
                    .await?;
                }
                Ok(VideoJobState::Running) => {
                    consecutive_errors = 0;
                    video = self
                        .update_generated_video_waiting_status_validated(
                            video.id,
                            prompt_id,
                            crate::video::GENERATED_VIDEO_WAITING_STATUS_RUNNING,
                        )
                        .await?;
                    self.mark_video_provider_workflow_validated(
                        &video,
                        crate::workflow::WORKFLOW_STATUS_RUNNING,
                        true,
                        None,
                    )
                    .await?;
                }
                Err(error) => {
                    consecutive_errors = consecutive_errors.checked_add(1).ok_or_else(|| {
                        AppError::InvalidState {
                            current: format!("generated_video_id={}", video.id),
                            operation: "视频 provider 连续轮询错误次数超过 u32 上限".to_string(),
                        }
                    })?;
                    if consecutive_errors >= VIDEO_JOB_MAX_CONSECUTIVE_POLL_ERRORS {
                        let message = format!(
                            "视频任务 #{} 连续 {} 次查询失败，已保留 provider_job_id={}，可稍后恢复轮询。最后一次错误：{}",
                            video.id, consecutive_errors, job_id, error
                        );
                        return self
                            .interrupt_video_provider_job(&video, prompt_id, &message)
                            .await;
                    }
                }
            }

            let sleep_for = timeout.saturating_sub(started.elapsed()).min(interval);
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    return self.interrupt_video_provider_job(
                        &video,
                        prompt_id,
                        "视频生成已停止，供应商任务仍在处理中，可稍后恢复轮询",
                    ).await;
                }
                _ = tokio::time::sleep(sleep_for) => {}
            }
        }
    }

    async fn complete_video_provider_result(
        &self,
        host_port: &dyn VideoProviderHostPort,
        video: generated_video::Model,
        prompt_id: Option<i64>,
        result: VideoGenerationResult,
    ) -> Result<generated_video::Model, AppError> {
        let persisted = match host_port
            .persist_video_result(&video, prompt_id, result)
            .await
        {
            Ok(persisted) => persisted,
            Err(error) => {
                return self
                    .mark_video_provider_failed(video, prompt_id, &error.to_string())
                    .await;
            }
        };
        // 正式文件已写出后的创建窗口：DB 完成写失败必须优先请求 durable cleanup intent，
        // 不能只依赖进程内同步删除；remove_video_result 的 Host 实现负责 outbox。
        let updated = match self
            .mark_generated_video_completed_and_link_prompt(
                video.id,
                &persisted.file_path,
                persisted.thumbnail_path.as_deref(),
                persisted.file_size,
                persisted.metadata_json.clone(),
                prompt_id,
            )
            .await
        {
            Ok(updated) => updated,
            Err(error) => {
                let cleanup = host_port.remove_video_result(&persisted).await.err();
                let message = match cleanup {
                    Some(cleanup) => {
                        format!(
                            "视频完成状态写入失败：{error}；已尝试写入文件 cleanup intent：{cleanup}"
                        )
                    }
                    None => format!(
                        "视频完成状态写入失败：{error}；正式文件 cleanup intent 已写入 outbox"
                    ),
                };
                return self
                    .mark_video_provider_failed(video, prompt_id, &message)
                    .await;
            }
        };
        self.complete_generated_video_provider_runs_validated(&updated)
            .await?;
        Ok(updated)
    }

    async fn mark_video_provider_failed(
        &self,
        video: generated_video::Model,
        prompt_id: Option<i64>,
        message: &str,
    ) -> Result<generated_video::Model, AppError> {
        let updated = self
            .mark_generated_video_failed_validated(video.id, prompt_id, message)
            .await?;
        self.mark_video_provider_workflow_validated(
            &updated,
            crate::workflow::WORKFLOW_STATUS_FAILED,
            false,
            Some(message),
        )
        .await?;
        Ok(updated)
    }

    async fn interrupt_video_provider_job(
        &self,
        video: &generated_video::Model,
        prompt_id: Option<i64>,
        message: &str,
    ) -> Result<generated_video::Model, AppError> {
        let updated = self
            .mark_generated_video_interrupted_validated(video.id, prompt_id, message)
            .await?;
        self.mark_video_provider_workflow_validated(
            &updated,
            crate::workflow::WORKFLOW_STATUS_INTERRUPTED,
            true,
            Some(message),
        )
        .await?;
        Ok(updated)
    }

    async fn sync_canvas_video_workflow(
        &self,
        video: &generated_video::Model,
        error_message: Option<&str>,
    ) -> Result<(), AppError> {
        let Some(canvas) = crate::video::canvas_video_workflow_context(video)
            .map_err(AppError::InvalidArgument)?
        else {
            return Ok(());
        };
        let step_key = format!("canvas_node:{}", canvas.node_id);
        self.upsert_workflow_checkpoint(a4n_db::workflow::UpsertWorkflowCheckpointInput {
            workflow_run_id: canvas.workflow_run_id,
            step_key,
            step_order: 0,
            status: video.status.clone(),
            input_fingerprint_sha256: None,
            input_fingerprint_json: Some(json!({
                "generated_video_id": video.id,
                "provider_job_id": video.provider_job_id,
                "graph_id": canvas.graph_id,
                "node_id": canvas.node_id,
            })),
            checkpoint_json: None,
            artifacts_json: Some(json!({
                "generated_video_id": video.id,
                "status": video.status,
                "file_path": video.file_path,
                "thumbnail_path": video.thumbnail_path,
                "provider_job_id": video.provider_job_id,
            })),
            generation_run_ids_json: None,
            error_message: error_message.map(str::to_string),
            completed: matches!(
                video.status.as_str(),
                crate::video::GENERATED_VIDEO_STATUS_COMPLETED
                    | crate::video::GENERATED_VIDEO_ERROR_STATUS_FAILED
            ),
        })
        .await?;
        self.finalize_canvas_video_workflow_validated(
            canvas.workflow_run_id,
            &video.status,
            error_message,
        )
        .await?;
        Ok(())
    }
}
