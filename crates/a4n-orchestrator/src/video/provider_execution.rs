use a4n_db::{SharedDatabaseConnection, generation::canonical_json_sha256};
use a4n_db_models::models::ai::generated_video;
use a4n_error::AppError;
use a4n_provider_contract::VideoProviderConfig;
use a4n_service_protocol::GenerationArtifactRef;
use a4n_video::{VideoGenerationResult, VideoJobState};
use a4n_video_contract::{
    VideoCancellationHandle, VideoGenerationProgressPort, VideoProviderHostPort,
    VideoProviderResultTarget,
};
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
pub use a4n_domain::video_workflow::VIDEO_GENERATION_PROGRESS_TOTAL;

fn video_cancellation_handle(token: &CancellationToken) -> VideoCancellationHandle {
    let cancelled = token.clone();
    VideoCancellationHandle::new(move || {
        let token = cancelled.clone();
        Box::pin(async move { token.cancelled().await })
    })
}

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

    pub fn progress(&mut self, current: u32, total: u32, message: &str) -> Result<(), AppError> {
        if let Some(reporter) = self.reporter.as_deref_mut()
            && let Err(error) = reporter(current, total.max(1), message)
        {
            if matches!(error, AppError::Cancelled) {
                self.cancel_token.cancel();
            } else {
                return Err(error);
            }
        }
        Ok(())
    }

    pub fn check_cancelled(&self, _message: &str) -> Result<(), AppError> {
        if self.cancel_token.is_cancelled() {
            return Err(AppError::Cancelled);
        }
        Ok(())
    }

    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel_token.clone()
    }

    pub(crate) fn cancellation_handle(&self) -> VideoCancellationHandle {
        video_cancellation_handle(&self.cancel_token)
    }

    pub(crate) async fn checkpoint_provider_request(
        &mut self,
        video_id: i64,
        config: &VideoProviderConfig,
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
        config: &VideoProviderConfig,
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

impl VideoGenerationProgressPort for VideoGenerationProgress<'_> {
    fn progress(&mut self, current: u32, total: u32, message: &str) -> Result<(), AppError> {
        VideoGenerationProgress::progress(self, current, total, message)
    }

    fn cancellation_handle(&self) -> VideoCancellationHandle {
        self.cancellation_handle()
    }

    fn cancel(&self) {
        self.cancel_token.cancel();
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
        config: &VideoProviderConfig,
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
        config: &VideoProviderConfig,
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
                None,
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

fn video_provider_result_target(video: &generated_video::Model) -> VideoProviderResultTarget {
    VideoProviderResultTarget {
        video_id: video.id,
        project_id: video.project_id,
        chapter_id: video.chapter_id,
        video_type: video.video_type.clone(),
        workflow_mode: video.workflow_mode.clone(),
    }
}

pub struct VideoProviderExecutionInput {
    pub video: generated_video::Model,
    pub config: VideoProviderConfig,
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
        let has_resumable_job = result.as_ref().ok().is_some_and(|result| {
            result.bytes.is_empty()
                && crate::video::normalized_provider_job_id(result.provider_job_id.as_deref())
                    .is_some()
        });
        if cancel_token.is_cancelled() && !has_resumable_job {
            self.mark_generated_video_cancelled_validated(video.id, prompt_id, "视频生成已停止")
                .await?;
            return Err(AppError::Cancelled);
        }
        match result {
            Ok(result) if !result.bytes.is_empty() => {
                self.complete_video_provider_result(
                    host_port,
                    video,
                    prompt_id,
                    result,
                    &cancel_token,
                )
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
        config: &VideoProviderConfig,
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
        let business_key = crate::video::video_provider_workflow_business_key(video.id)
            .map_err(AppError::InvalidArgument)?;
        if let Some(run) = self
            .latest_workflow_run_status_record_by_business_key(&business_key)
            .await?
            .filter(|run| run.status == crate::workflow::WORKFLOW_STATUS_INTERRUPTED)
        {
            self.mark_workflow_running_validated(run.id).await?;
        }
        let workflow_run_id = self
            .record_video_provider_progress_validated(&video, 1, 2, "正在恢复视频 provider 轮询")
            .await?;
        let cancellation = video_cancellation_handle(&cancel_token);
        let state = host_port
            .poll_video_job(
                video.project_id,
                workflow_run_id,
                config,
                &job_id,
                &cancellation,
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
                let video = self
                    .update_generated_video_waiting_status_validated(
                        video.id,
                        prompt_id,
                        a4n_domain::video_workflow::VIDEO_STATUS_RUNNING,
                    )
                    .await?;
                let updated = self
                    .complete_video_provider_result(
                        host_port,
                        video,
                        prompt_id,
                        result,
                        &cancel_token,
                    )
                    .await?;
                let (status, error_message) =
                    if updated.status == a4n_domain::video_workflow::VIDEO_STATUS_COMPLETED {
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
                        a4n_domain::video_workflow::VIDEO_STATUS_PENDING,
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
                        a4n_domain::video_workflow::VIDEO_STATUS_RUNNING,
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
        config: &VideoProviderConfig,
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
        let cancellation = video_cancellation_handle(&cancel_token);

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
                .poll_video_job(video.project_id, workflow.id, config, job_id, &cancellation)
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
                        .complete_video_provider_result(
                            host_port,
                            video,
                            prompt_id,
                            result,
                            &cancel_token,
                        )
                        .await?;
                    let (status, can_resume, error_message) =
                        if updated.status == a4n_domain::video_workflow::VIDEO_STATUS_COMPLETED {
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
                            a4n_domain::video_workflow::VIDEO_STATUS_PENDING,
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
                            a4n_domain::video_workflow::VIDEO_STATUS_RUNNING,
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
        cancel_token: &CancellationToken,
    ) -> Result<generated_video::Model, AppError> {
        let persisted = match host_port
            .persist_video_result(&video_provider_result_target(&video), prompt_id, result)
            .await
        {
            Ok(persisted) => persisted,
            Err(error) => {
                return self
                    .mark_video_provider_failed(video, prompt_id, &error.to_string())
                    .await;
            }
        };
        if cancel_token.is_cancelled() {
            let cancelled =
                if crate::video::normalized_provider_job_id(video.provider_job_id.as_deref())
                    .is_some()
                {
                    self.interrupt_video_provider_job(
                        &video,
                        prompt_id,
                        "视频生成已停止，已丢弃迟到的供应商结果",
                    )
                    .await
                } else {
                    self.mark_generated_video_cancelled_validated(
                        video.id,
                        prompt_id,
                        "视频生成已停止，已丢弃迟到的供应商结果",
                    )
                    .await
                };
            if let Err(error) = cancelled {
                let authoritative = self.generated_video_by_id(video.id).await?;
                if authoritative.as_ref().is_some_and(|video| {
                    video.status == a4n_domain::video_workflow::VIDEO_STATUS_COMPLETED
                }) {
                    return Err(error);
                }
                if let Err(cleanup_error) = host_port.remove_video_result(&persisted).await {
                    tracing::error!(
                        video_id = video.id,
                        error = %cleanup_error,
                        "取消视频时写入迟到文件 cleanup intent 失败"
                    );
                }
                if authoritative.as_ref().is_some_and(|video| {
                    matches!(
                        video.status.as_str(),
                        a4n_domain::video_workflow::VIDEO_STATUS_CANCELLED
                            | a4n_domain::video_workflow::VIDEO_STATUS_INTERRUPTED
                    )
                }) {
                    return Err(AppError::Cancelled);
                }
                return Err(error);
            }
            if let Err(cleanup_error) = host_port.remove_video_result(&persisted).await {
                tracing::error!(
                    video_id = video.id,
                    error = %cleanup_error,
                    "取消视频时写入迟到文件 cleanup intent 失败"
                );
            }
            return Err(AppError::Cancelled);
        }
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
                let can_adopt = crate::video::can_adopt_authoritative_video_completion(&error);
                let authoritative = self.generated_video_by_id(video.id).await?;
                if let Some(authoritative) = authoritative {
                    if authoritative.status == a4n_domain::video_workflow::VIDEO_STATUS_COMPLETED {
                        if authoritative.file_path.as_deref() != Some(persisted.file_path.as_str())
                            && let Err(cleanup_error) =
                                host_port.remove_video_result(&persisted).await
                        {
                            tracing::error!(
                                video_id = video.id,
                                error = %cleanup_error,
                                "清理重复供应商视频文件失败"
                            );
                        }
                        if !can_adopt {
                            return Err(error);
                        }
                        authoritative
                    } else if matches!(
                        authoritative.status.as_str(),
                        a4n_domain::video_workflow::VIDEO_STATUS_CANCELLED
                            | a4n_domain::video_workflow::VIDEO_STATUS_INTERRUPTED
                    ) {
                        if let Err(cleanup_error) = host_port.remove_video_result(&persisted).await
                        {
                            tracing::error!(
                                video_id = video.id,
                                error = %cleanup_error,
                                "清理迟到视频文件失败"
                            );
                        }
                        return Err(AppError::Cancelled);
                    } else if authoritative.status
                        == a4n_domain::video_workflow::VIDEO_STATUS_FAILED
                    {
                        if let Err(cleanup_error) = host_port.remove_video_result(&persisted).await
                        {
                            tracing::error!(
                                video_id = video.id,
                                error = %cleanup_error,
                                "清理失败视频的迟到文件失败"
                            );
                        }
                        return Err(error);
                    } else {
                        let cleanup = host_port.remove_video_result(&persisted).await.err();
                        let message = match cleanup {
                            Some(cleanup) => format!(
                                "视频完成状态写入失败：{error}；已尝试写入文件 cleanup intent：{cleanup}"
                            ),
                            None => format!(
                                "视频完成状态写入失败：{error}；正式文件 cleanup intent 已写入 outbox"
                            ),
                        };
                        return self
                            .mark_video_provider_failed(authoritative, prompt_id, &message)
                            .await;
                    }
                } else {
                    let cleanup = host_port.remove_video_result(&persisted).await.err();
                    if let Some(cleanup_error) = cleanup.as_ref() {
                        tracing::error!(
                            video_id = video.id,
                            error = %cleanup_error,
                            "视频记录消失后清理供应商文件失败"
                        );
                    }
                    return Err(error);
                }
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
        let checkpoint_completed = match video.status.as_str() {
            a4n_domain::video_workflow::VIDEO_STATUS_COMPLETED
            | a4n_domain::video_workflow::VIDEO_STATUS_FAILED => true,
            a4n_domain::video_workflow::VIDEO_STATUS_INTERRUPTED => false,
            other => {
                return Err(AppError::InvalidState {
                    current: format!("generated_video_status={other}"),
                    operation: "同步画布视频 workflow".to_string(),
                });
            }
        };
        let step_key = crate::canvas::canvas_node_step_key(canvas.node_id)
            .map_err(AppError::InvalidArgument)?;
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
            completed: checkpoint_completed,
        })
        .await?;
        if video.status == a4n_domain::video_workflow::VIDEO_STATUS_COMPLETED {
            self.complete_workflow_run_validated(canvas.workflow_run_id)
                .await?;
        } else if video.status == a4n_domain::video_workflow::VIDEO_STATUS_FAILED {
            let error_message = error_message.ok_or_else(|| AppError::InvalidState {
                current: "failed_video_without_error".to_string(),
                operation: "同步画布视频失败终态".to_string(),
            })?;
            self.fail_workflow_run_validated(canvas.workflow_run_id, error_message)
                .await?;
        } else {
            let error_message = error_message.ok_or_else(|| AppError::InvalidState {
                current: "interrupted_video_without_error".to_string(),
                operation: "同步画布视频中断终态".to_string(),
            })?;
            self.mark_workflow_status(
                canvas.workflow_run_id,
                crate::workflow::WORKFLOW_STATUS_INTERRUPTED,
                true,
                Some(error_message),
            )
            .await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use a4n_db::workflow::UpsertWorkflowRunInput;
    use a4n_db_models::models::ai::generated_video;
    use a4n_domain::workflow::{
        WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED, WORKFLOW_STATUS_INTERRUPTED,
        WORKFLOW_STATUS_RUNNING,
    };
    use serde_json::json;

    use super::AppError;
    use crate::test_support::TestOrchestratorBuilder;

    #[tokio::test]
    async fn canvas_video_sync_only_accepts_produced_terminal_or_interrupted_statuses() {
        let database = Arc::new(a4n_migration::test_support::connect_test_database().await);
        let context = TestOrchestratorBuilder::new(Arc::clone(&database))
            .build()
            .await;

        for (status, error_message, expected_can_resume) in [
            (WORKFLOW_STATUS_COMPLETED, None, false),
            (WORKFLOW_STATUS_FAILED, Some("provider failed"), true),
            (
                WORKFLOW_STATUS_INTERRUPTED,
                Some("provider interrupted"),
                true,
            ),
        ] {
            let run = context
                .begin_workflow_run_validated(canvas_workflow_input(status))
                .await
                .expect("应可创建画布 workflow");
            context
                .sync_canvas_video_workflow(
                    &canvas_video(run.id, status, error_message),
                    error_message,
                )
                .await
                .expect("生产路径状态应可同步");
            let stored = context
                .workflow_run_model_by_id(run.id)
                .await
                .expect("应可读取同步后的 workflow");
            assert_eq!(stored.status, status);
            assert_eq!(stored.can_resume, expected_can_resume);
        }

        let run = context
            .begin_workflow_run_validated(canvas_workflow_input("running-rejected"))
            .await
            .expect("应可创建待拒绝状态的画布 workflow");
        let error = context
            .sync_canvas_video_workflow(&canvas_video(run.id, WORKFLOW_STATUS_RUNNING, None), None)
            .await
            .expect_err("provider 同步入口不会产出 running，必须显式拒绝");
        assert!(matches!(error, AppError::InvalidState { .. }));
        assert!(
            context
                .workflow_checkpoint_models(run.id)
                .await
                .expect("应可读取 workflow checkpoints")
                .is_empty()
        );
    }

    fn canvas_workflow_input(key: &str) -> UpsertWorkflowRunInput {
        UpsertWorkflowRunInput {
            business_key: format!("canvas-video-sync-test:{key}"),
            workflow_kind: "video_canvas_graph".to_string(),
            project_id: None,
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: None,
            scope_id: None,
            chapter_id: None,
            mode: Some("single_node".to_string()),
            input_fingerprint_json: None,
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: None,
            active_task_id: None,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        }
    }

    fn canvas_video(
        workflow_run_id: i64,
        status: &str,
        error_message: Option<&str>,
    ) -> generated_video::Model {
        let now = chrono::Utc::now().naive_utc();
        generated_video::Model {
            id: workflow_run_id,
            project_id: 1,
            chapter_id: None,
            video_type: a4n_domain::video_workflow::VIDEO_TYPE_CANVAS.to_string(),
            workflow_mode: Some(a4n_domain::video_workflow::WORKFLOW_MODE_CANVAS_VIDEO.to_string()),
            file_path: None,
            thumbnail_path: None,
            prompt: "test prompt".to_string(),
            negative_prompt: None,
            model: "test-model".to_string(),
            width: 1280,
            height: 720,
            duration_secs: 5.0,
            fps: 24,
            file_size: None,
            source_video_id: None,
            source_image_id: None,
            status: status.to_string(),
            provider_job_id: Some("provider-job".to_string()),
            error_message: error_message.map(str::to_string),
            timeline_item_id: None,
            metadata_json: Some(json!({
                "config_id": 1,
                "canvas": {
                    "workflow_run_id": workflow_run_id,
                    "graph_id": 1,
                    "node_id": 1,
                }
            })),
            is_favorite: false,
            created_at: now,
            updated_at: now,
        }
    }
}
