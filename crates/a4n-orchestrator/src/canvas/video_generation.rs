use a4n_api_contract::canvas::CanvasRunSummary;
use a4n_canvas_contract::GenerateCanvasVideoInput;
pub(crate) use a4n_canvas_host_contract::{
    CanvasVideoFileInput, CanvasVideoGenerationHostPort, CanvasVideoProviderInput,
};
use a4n_db::{
    canvas::{CanvasGraphRecordDocument, FreeformCanvasGenerationSpecUpsertInput},
    generation::{
        GENERATION_SPEC_TARGET_ENTITY_KIND_CANVAS_NODE, GENERATION_SPEC_TARGET_KIND_CANVAS_VIDEO,
    },
};
use a4n_db_models::models::{
    ai::generated_video,
    canvas::canvas_node,
    generation::{generation_spec, workflow_checkpoint},
};
use a4n_domain::CanvasNodeRefKind;
use a4n_domain::video_workflow::WORKFLOW_MODE_CANVAS_VIDEO;
use a4n_error::AppError;
use a4n_provider_contract::VideoProviderConfig;
use a4n_video::{VideoGenerationRequest, VideoGenerationResult, VideoJobState};
use a4n_video_host_contract::{
    CanvasVideoImageInput, CanvasVideoPreviewHostPort, CanvasVideoRequestPreview,
    VideoProviderValidationMode,
};
use serde_json::{Value, json};
use tokio::time::{Duration, Instant};

use crate::{
    OrchestratorContext,
    canvas::CanvasNodeRunHandle,
    project::ProjectSourceMode,
    workflow::{
        WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED, WORKFLOW_STATUS_INTERRUPTED,
        WORKFLOW_STATUS_RUNNING,
    },
};

const CANVAS_VIDEO_JOB_POLL_INTERVAL_SECS: u64 = 8;
const CANVAS_VIDEO_JOB_MAX_WAIT_SECS: u64 = 30 * 60;
const CANVAS_VIDEO_JOB_MAX_CONSECUTIVE_POLL_ERRORS: u32 = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasVideoNodeInput {
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub config_id: Option<i64>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub duration_secs: Option<i32>,
    pub fps: Option<i32>,
    pub first_frame_image_id: Option<i64>,
    pub last_frame_image_id: Option<i64>,
    pub reference_image_ids: Vec<i64>,
    pub source_node_ids: Vec<i64>,
    pub canvas_text_asset_id: Option<i64>,
}

pub struct CanvasVideoNodeExecutionInput<'a> {
    pub user_id: i64,
    pub project_id: i64,
    pub graph_id: i64,
    pub node: &'a canvas_node::Model,
    pub document: &'a CanvasGraphRecordDocument,
    pub shared_run: Option<CanvasNodeRunHandle>,
    pub mode: &'a str,
}

impl OrchestratorContext {
    pub async fn preview_canvas_video_request_validated(
        &self,
        input: GenerateCanvasVideoInput,
        host_port: &dyn CanvasVideoPreviewHostPort,
    ) -> Result<CanvasVideoRequestPreview, AppError> {
        self.verify_project_permission(
            input.project_id,
            input.user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        self.require_freeform_video_project(input.project_id)
            .await?;
        let video_input = CanvasVideoNodeInput {
            prompt: input.prompt,
            negative_prompt: input.negative_prompt,
            config_id: input.config_id,
            width: input.width,
            height: input.height,
            duration_secs: input.duration_secs,
            fps: input.fps,
            first_frame_image_id: input.first_frame_image_id,
            last_frame_image_id: input.last_frame_image_id,
            reference_image_ids: input.reference_image_ids,
            source_node_ids: input.source_node_ids,
            canvas_text_asset_id: None,
        };
        let prepared = prepare_canvas_video_request(
            host_port,
            input.user_id,
            input.project_id,
            &video_input,
            VideoProviderValidationMode::Preview,
        )
        .await?;
        Ok(CanvasVideoRequestPreview {
            config_id: prepared.config.id,
            provider_backend: prepared.config.backend,
            model: prepared.config.model,
            graph_id: input.graph_id,
            node_id: input.node_id,
            request: prepared.request,
        })
    }

    pub async fn run_canvas_video_node_validated(
        &self,
        input: CanvasVideoNodeExecutionInput<'_>,
        host_port: &dyn CanvasVideoGenerationHostPort,
    ) -> Result<CanvasRunSummary, AppError> {
        self.verify_project_permission(
            input.project_id,
            input.user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        self.require_freeform_video_project(input.project_id)
            .await?;
        let checkpoints = match input.shared_run {
            Some(run) => Some(self.workflow_checkpoint_models(run.workflow_run_id).await?),
            None => None,
        };
        let video_input =
            build_canvas_video_node_input(input.node, input.document, checkpoints.as_deref())
                .map_err(AppError::InvalidArgument)?;
        let PreparedCanvasVideoRequest {
            config,
            prompt,
            request,
            first_frame_image_id,
            last_frame_image_id,
            reference_image_ids,
        } = prepare_canvas_video_request(
            host_port,
            input.user_id,
            input.project_id,
            &video_input,
            VideoProviderValidationMode::Generate,
        )
        .await?;
        let step_key = crate::canvas::canvas_node_step_key(input.node.id)
            .map_err(AppError::InvalidArgument)?;
        let input_json = json!({
            "user_id": input.user_id,
            "project_id": input.project_id,
            "graph_id": input.graph_id,
            "node_id": input.node.id,
            "workflow_run_id": Value::Null,
            "prompt": prompt,
            "negative_prompt": request.negative_prompt,
            "config_id": video_input.config_id,
            "width": request.width,
            "height": request.height,
            "duration_secs": request.duration_secs,
            "fps": request.fps,
            "first_frame_image_id": first_frame_image_id,
            "last_frame_image_id": last_frame_image_id,
            "reference_image_ids": reference_image_ids,
            "source_node_ids": video_input.source_node_ids,
        });
        let mut lifecycle = self
            .start_canvas_node_execution(
                input.project_id,
                input.graph_id,
                input.mode,
                step_key,
                input_json,
                input.shared_run,
            )
            .await?;
        let result = async {
            let spec = self
                .upsert_canvas_video_generation_spec(&input, &video_input)
                .await?;
            lifecycle.generation_spec_id = Some(spec.id);
            self.ensure_video_capacity(config.id, config.max_concurrent)
                .await?;
            let provider_request_snapshot = json!({
                "config_id": config.id,
                "provider_backend": config.backend,
                "model": config.model,
                "request": request,
            });
            let mut video = self
                .create_video_generation_record_validated(
                    crate::video::VideoGenerationRecordInput {
                        project_id: input.project_id,
                        chapter_id: None,
                        video_type: a4n_domain::video_workflow::VIDEO_TYPE_CANVAS.to_string(),
                        workflow_mode: Some(WORKFLOW_MODE_CANVAS_VIDEO.to_string()),
                        prompt: prompt.clone(),
                        negative_prompt: request.negative_prompt.clone(),
                        model: config.model.clone(),
                        width: request.width,
                        height: request.height,
                        duration_secs: request.duration_secs as f32,
                        fps: request.fps,
                        source_video_id: None,
                        source_image_id: first_frame_image_id,
                        metadata_json: Some(json!({
                            "canvas": {
                                "graph_id": input.graph_id,
                                "node_id": input.node.id,
                                "workflow_run_id": lifecycle.run.workflow_run_id,
                                "source_node_ids": video_input.source_node_ids,
                            },
                            "config_id": config.id,
                            "first_frame_image_id": first_frame_image_id,
                            "last_frame_image_id": last_frame_image_id,
                            "reference_image_ids": reference_image_ids,
                            "provider_request_snapshot": provider_request_snapshot,
                            "runtime_config_snapshot": host_port.runtime_config_snapshot(),
                        })),
                        provider_request_snapshot,
                        provider_backend: config.backend.clone(),
                        step_key: "generate_canvas_video".to_string(),
                        title: "画布视频 provider 生成".to_string(),
                        prompt_template_name: Some("canvas_video_prompt".to_string()),
                        prompt_id: None,
                    },
                )
                .await?;
            if let Err(error) = self
                .ensure_created_video_capacity(config.id, config.max_concurrent, video.id)
                .await
            {
                let failed = self
                    .mark_generated_video_failed_validated(video.id, None, &error.to_string())
                    .await?;
                return self
                    .finish_canvas_video_generation(&lifecycle, spec.id, failed)
                    .await;
            }
            if self
                .canvas_workflow_cancellation_reason(lifecycle.run.workflow_run_id)
                .await?
                .is_some()
            {
                self.cancel_canvas_video_record(&video, "画布运行已取消，视频未提交 provider")
                    .await?;
                return Err(AppError::Cancelled);
            }
            let result = host_port
                .submit_canvas_video(CanvasVideoProviderInput {
                    user_id: input.user_id,
                    project_id: input.project_id,
                    workflow_run_id: lifecycle.run.workflow_run_id,
                    config: config.clone(),
                    request,
                })
                .await;
            video = self
                .apply_canvas_video_provider_result(host_port, &config, video, result)
                .await?;
            self.finish_canvas_video_generation(&lifecycle, spec.id, video)
                .await
        }
        .await;

        match result {
            Ok(summary) => Ok(summary),
            Err(primary) => self.finish_canvas_node_error(lifecycle, primary).await,
        }
    }

    async fn apply_canvas_video_provider_result(
        &self,
        host_port: &dyn CanvasVideoGenerationHostPort,
        config: &VideoProviderConfig,
        video: generated_video::Model,
        result: Result<VideoGenerationResult, AppError>,
    ) -> Result<generated_video::Model, AppError> {
        match result {
            Ok(result) if !result.bytes.is_empty() => {
                self.complete_canvas_video_result(host_port, video, result)
                    .await
            }
            Ok(result) => {
                let Some(job_id) = result
                    .provider_job_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                else {
                    return self
                        .mark_generated_video_failed_validated(
                            video.id,
                            None,
                            "画布视频 provider 未返回文件或任务 ID",
                        )
                        .await;
                };
                let video = self
                    .mark_generated_video_job_and_prompt_validated(
                        video.id,
                        job_id,
                        Some(result.metadata),
                        None,
                    )
                    .await?;
                self.mark_video_provider_workflow_validated(
                    &video,
                    WORKFLOW_STATUS_RUNNING,
                    true,
                    None,
                )
                .await?;
                self.wait_for_canvas_video_job_completion(host_port, config, video, job_id)
                    .await
            }
            Err(AppError::Cancelled) => {
                self.cancel_canvas_video_record(&video, "画布视频生成已取消")
                    .await?;
                Err(AppError::Cancelled)
            }
            Err(error) => {
                self.mark_generated_video_failed_validated(video.id, None, &error.to_string())
                    .await
            }
        }
    }

    async fn complete_canvas_video_result(
        &self,
        host_port: &dyn CanvasVideoGenerationHostPort,
        video: generated_video::Model,
        result: VideoGenerationResult,
    ) -> Result<generated_video::Model, AppError> {
        let workflow_run_id = canvas_video_workflow_run_id(&video)?;
        if self
            .canvas_workflow_cancellation_reason(workflow_run_id)
            .await?
            .is_some()
        {
            self.cancel_canvas_video_record(&video, "画布运行已取消，视频结果不再写入")
                .await?;
            return Err(AppError::Cancelled);
        }
        let persisted = match host_port.persist_canvas_video_file(CanvasVideoFileInput {
            project_id: video.project_id,
            video_id: video.id,
            bytes: &result.bytes,
            format: &result.format,
        }) {
            Ok(persisted) => persisted,
            Err(error) => {
                return self
                    .mark_generated_video_failed_validated(video.id, None, &error.to_string())
                    .await;
            }
        };
        let updated = match self
            .mark_generated_video_completed_and_link_prompt(
                video.id,
                &persisted.relative_path,
                None,
                persisted.file_size,
                Some(json!({
                    "provider": result.metadata,
                    "canvas_video": true,
                })),
                None,
            )
            .await
        {
            Ok(updated) => updated,
            Err(error) => {
                let can_adopt = crate::video::can_adopt_authoritative_video_completion(&error);
                let authoritative = self.generated_video_by_id(video.id).await?;
                if let Some(authoritative) = authoritative {
                    if authoritative.status == a4n_domain::video_workflow::VIDEO_STATUS_COMPLETED {
                        if authoritative.file_path.as_deref()
                            != Some(persisted.relative_path.as_str())
                            && let Err(cleanup_error) = host_port
                                .remove_canvas_video_file(&persisted.relative_path)
                                .await
                        {
                            tracing::error!(
                                video_id = video.id,
                                error = %cleanup_error,
                                "清理重复画布视频文件失败"
                            );
                        }
                        if !can_adopt {
                            return Err(error);
                        }
                        authoritative
                    } else {
                        if let Err(cleanup_error) = host_port
                            .remove_canvas_video_file(&persisted.relative_path)
                            .await
                        {
                            tracing::error!(
                                video_id = video.id,
                                error = %cleanup_error,
                                "清理未采用的画布视频文件失败"
                            );
                        }
                        if matches!(
                            authoritative.status.as_str(),
                            a4n_domain::video_workflow::VIDEO_STATUS_CANCELLED
                                | a4n_domain::video_workflow::VIDEO_STATUS_INTERRUPTED
                        ) {
                            return Err(AppError::Cancelled);
                        }
                        return Err(error);
                    }
                } else {
                    if let Err(cleanup_error) = host_port
                        .remove_canvas_video_file(&persisted.relative_path)
                        .await
                    {
                        tracing::error!(
                            video_id = video.id,
                            error = %cleanup_error,
                            "视频记录消失后清理画布视频文件失败"
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

    async fn wait_for_canvas_video_job_completion(
        &self,
        host_port: &dyn CanvasVideoGenerationHostPort,
        config: &VideoProviderConfig,
        mut video: generated_video::Model,
        job_id: &str,
    ) -> Result<generated_video::Model, AppError> {
        let workflow_run_id = canvas_video_workflow_run_id(&video)?;
        let started = Instant::now();
        let timeout = Duration::from_secs(CANVAS_VIDEO_JOB_MAX_WAIT_SECS);
        let interval = Duration::from_secs(CANVAS_VIDEO_JOB_POLL_INTERVAL_SECS);
        let mut consecutive_errors = 0_u32;

        loop {
            if self
                .canvas_workflow_cancellation_reason(workflow_run_id)
                .await?
                .is_some()
            {
                self.interrupt_canvas_video_polling(
                    &video,
                    "画布运行已取消，视频 provider 轮询已停止",
                )
                .await?;
                return Err(AppError::Cancelled);
            }
            if started.elapsed() >= timeout {
                let message = format!(
                    "视频任务 #{} 等待超过 {} 分钟仍未完成，已保留 provider_job_id={}，可稍后恢复轮询",
                    video.id,
                    CANVAS_VIDEO_JOB_MAX_WAIT_SECS / 60,
                    job_id
                );
                self.interrupt_canvas_video_polling(&video, &message)
                    .await?;
                return Err(AppError::InvalidState {
                    current: format!("generated_video_id={}", video.id),
                    operation: message,
                });
            }

            match host_port
                .poll_canvas_video(video.project_id, workflow_run_id, config, job_id)
                .await
            {
                Ok(VideoJobState::Completed(result)) => {
                    if self
                        .canvas_workflow_cancellation_reason(workflow_run_id)
                        .await?
                        .is_some()
                    {
                        self.interrupt_canvas_video_polling(
                            &video,
                            "画布运行已取消，视频 provider 结果不再写入画布运行",
                        )
                        .await?;
                        return Err(AppError::Cancelled);
                    }
                    let updated = self
                        .complete_canvas_video_result(host_port, video, result)
                        .await?;
                    let (status, can_resume, error_message) =
                        if updated.status == WORKFLOW_STATUS_COMPLETED {
                            (WORKFLOW_STATUS_COMPLETED, false, None)
                        } else {
                            (
                                WORKFLOW_STATUS_FAILED,
                                true,
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
                    let updated = self
                        .mark_generated_video_failed_validated(video.id, None, &message)
                        .await?;
                    self.mark_video_provider_workflow_validated(
                        &updated,
                        WORKFLOW_STATUS_FAILED,
                        true,
                        Some(&message),
                    )
                    .await?;
                    return Ok(updated);
                }
                Ok(VideoJobState::Pending) => {
                    consecutive_errors = 0;
                    video = self
                        .update_generated_video_waiting_status_validated(
                            video.id,
                            None,
                            a4n_domain::video_workflow::VIDEO_STATUS_PENDING,
                        )
                        .await?;
                    self.mark_video_provider_workflow_validated(
                        &video,
                        WORKFLOW_STATUS_RUNNING,
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
                            None,
                            a4n_domain::video_workflow::VIDEO_STATUS_RUNNING,
                        )
                        .await?;
                    self.mark_video_provider_workflow_validated(
                        &video,
                        WORKFLOW_STATUS_RUNNING,
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
                    if consecutive_errors >= CANVAS_VIDEO_JOB_MAX_CONSECUTIVE_POLL_ERRORS {
                        let message = format!(
                            "视频任务 #{} 连续 {} 次查询失败，已保留 provider_job_id={}，可稍后恢复轮询。最后一次错误：{}",
                            video.id, consecutive_errors, job_id, error
                        );
                        self.interrupt_canvas_video_polling(&video, &message)
                            .await?;
                        return Err(AppError::InvalidState {
                            current: format!("generated_video_id={}", video.id),
                            operation: message,
                        });
                    }
                }
            }

            tokio::time::sleep(timeout.saturating_sub(started.elapsed()).min(interval)).await;
        }
    }

    async fn interrupt_canvas_video_polling(
        &self,
        video: &generated_video::Model,
        message: &str,
    ) -> Result<(), AppError> {
        let updated = self
            .mark_generated_video_interrupted_validated(video.id, None, message)
            .await?;
        self.mark_video_provider_workflow_validated(
            &updated,
            WORKFLOW_STATUS_INTERRUPTED,
            true,
            Some(message),
        )
        .await
    }

    async fn cancel_canvas_video_record(
        &self,
        video: &generated_video::Model,
        message: &str,
    ) -> Result<(), AppError> {
        if crate::video::normalized_provider_job_id(video.provider_job_id.as_deref()).is_some() {
            self.interrupt_canvas_video_polling(video, message).await
        } else {
            self.mark_generated_video_cancelled_validated(video.id, None, message)
                .await
                .map(|_| ())
        }
    }

    async fn finish_canvas_video_generation(
        &self,
        lifecycle: &crate::canvas::CanvasNodeLifecycle,
        generation_spec_id: i64,
        video: generated_video::Model,
    ) -> Result<CanvasRunSummary, AppError> {
        match video.status.as_str() {
            WORKFLOW_STATUS_FAILED => {
                let error = video
                    .error_message
                    .as_deref()
                    .filter(|message| !message.trim().is_empty())
                    .ok_or_else(|| AppError::InvalidState {
                        current: format!("failed_generated_video_without_error:{}", video.id),
                        operation: "完成画布视频生成".to_string(),
                    })?
                    .to_string();
                self.mark_canvas_generation_spec_status_record(
                    generation_spec_id,
                    WORKFLOW_STATUS_FAILED,
                )
                .await?;
                self.record_canvas_node_checkpoint(
                    lifecycle.run.workflow_run_id,
                    lifecycle.step_key.clone(),
                    WORKFLOW_STATUS_FAILED,
                    lifecycle.input_json.clone(),
                    Some(json!({
                        "generated_video_id": video.id,
                        "status": video.status,
                        "generation_spec_id": generation_spec_id,
                    })),
                    Some(error.clone()),
                    true,
                )
                .await?;
                let failed = self
                    .fail_workflow_run_validated(lifecycle.run.workflow_run_id, &error)
                    .await?;
                Ok(CanvasRunSummary {
                    workflow_run_id: failed.id,
                    project_id: lifecycle.project_id,
                    graph_id: lifecycle.graph_id,
                    mode: lifecycle.mode.clone(),
                    status: failed.status,
                    error_message: Some(error),
                })
            }
            WORKFLOW_STATUS_RUNNING => {
                self.record_canvas_node_checkpoint(
                    lifecycle.run.workflow_run_id,
                    lifecycle.step_key.clone(),
                    WORKFLOW_STATUS_RUNNING,
                    lifecycle.input_json.clone(),
                    Some(json!({
                        "generated_video_id": video.id,
                        "status": video.status,
                        "provider_job_id": video.provider_job_id,
                        "generation_spec_id": generation_spec_id,
                    })),
                    None,
                    false,
                )
                .await?;
                Ok(CanvasRunSummary {
                    workflow_run_id: lifecycle.run.workflow_run_id,
                    project_id: lifecycle.project_id,
                    graph_id: lifecycle.graph_id,
                    mode: lifecycle.mode.clone(),
                    status: WORKFLOW_STATUS_RUNNING.to_string(),
                    error_message: None,
                })
            }
            WORKFLOW_STATUS_COMPLETED => {
                self.mark_canvas_generation_spec_status_record(
                    generation_spec_id,
                    WORKFLOW_STATUS_COMPLETED,
                )
                .await?;
                let asset = self
                    .ensure_canvas_video_media_asset_record(
                        crate::canvas::CanvasVideoMediaAssetEnsureInput {
                            project_id: lifecycle.project_id,
                            video: video.clone(),
                            generation_spec_id: Some(generation_spec_id),
                            origin: "generated_canvas_video".to_string(),
                        },
                    )
                    .await?;
                self.finish_canvas_node_with_artifacts(
                    lifecycle,
                    json!({
                        "data_type": "video",
                        "generated_video_id": video.id,
                        "status": video.status,
                        "file_path": video.file_path,
                        "thumbnail_path": video.thumbnail_path,
                        "generation_spec_id": generation_spec_id,
                        "media_asset_id": asset.id,
                    }),
                )
                .await
            }
            other => Err(AppError::InvalidState {
                current: format!("generated_video_status={other}"),
                operation: format!("画布视频生成返回未知状态: {}", video.id),
            }),
        }
    }

    async fn upsert_canvas_video_generation_spec(
        &self,
        input: &CanvasVideoNodeExecutionInput<'_>,
        video_input: &CanvasVideoNodeInput,
    ) -> Result<generation_spec::Model, AppError> {
        let source_material_json = json!({
            "prompt": video_input.prompt,
            "first_frame_image_id": video_input.first_frame_image_id,
            "last_frame_image_id": video_input.last_frame_image_id,
            "reference_image_ids": video_input.reference_image_ids,
            "source_node_ids": video_input.source_node_ids,
        });
        self.upsert_freeform_canvas_generation_spec_record(
            FreeformCanvasGenerationSpecUpsertInput {
                project_id: input.project_id,
                target_entity_id: input.node.id,
                canvas_text_asset_id: video_input.canvas_text_asset_id,
                target_kind: GENERATION_SPEC_TARGET_KIND_CANVAS_VIDEO.to_string(),
                target_entity_kind: GENERATION_SPEC_TARGET_ENTITY_KIND_CANVAS_NODE.to_string(),
                status: WORKFLOW_STATUS_RUNNING.to_string(),
                params_json: json!({
                    "canvas_graph_id": input.graph_id,
                    "canvas_node_id": input.node.id,
                    "source_material": source_material_json,
                    "params": {
                        "workflow": "generate_video",
                        "config_id": video_input.config_id,
                        "width": video_input.width,
                        "height": video_input.height,
                        "duration_secs": video_input.duration_secs,
                        "fps": video_input.fps,
                        "node_config": input.node.config_json,
                    },
                }),
            },
        )
        .await
    }

    async fn require_freeform_video_project(&self, project_id: i64) -> Result<(), AppError> {
        let project = self
            .project_by_id(project_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
        if project
            .source_mode()
            .map_err(|error| AppError::InvalidArgument(format!("项目 {project_id}: {error}")))?
            != ProjectSourceMode::Freeform
        {
            return Err(AppError::InvalidArgument(
                "GenerateVideoNode 当前只支持自由视频项目".to_string(),
            ));
        }
        Ok(())
    }
}

fn canvas_video_workflow_run_id(video: &generated_video::Model) -> Result<i64, AppError> {
    video
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("canvas"))
        .and_then(|canvas| canvas.get("workflow_run_id"))
        .and_then(Value::as_i64)
        .ok_or_else(|| AppError::InvalidState {
            current: format!("generated_video_id={}", video.id),
            operation: "画布视频记录缺少 workflow_run_id".to_string(),
        })
}

pub fn build_canvas_video_node_input_for_preview(
    node: &canvas_node::Model,
    document: &CanvasGraphRecordDocument,
) -> Result<CanvasVideoNodeInput, String> {
    build_canvas_video_node_input(node, document, None)
}

fn build_canvas_video_node_input(
    node: &canvas_node::Model,
    document: &CanvasGraphRecordDocument,
    checkpoints: Option<&[workflow_checkpoint::Model]>,
) -> Result<CanvasVideoNodeInput, String> {
    let mut prompt_parts = Vec::new();
    let mut first_frame_image_id = None;
    let mut last_frame_image_id = None;
    let mut reference_image_ids = Vec::new();
    let mut source_node_ids = Vec::new();
    for edge in crate::canvas::incoming_edges(document, node.id) {
        let source = crate::canvas::source_node(document, edge.source_node_id)?;
        source_node_ids.push(source.id);
        if let Some(prompt) =
            crate::canvas::source_text_value(source, document, checkpoints, "GenerateVideoNode")?
        {
            prompt_parts.push(prompt);
        }
        if let Some(image_id) = crate::canvas::source_node_output_id(
            checkpoints,
            source,
            "GenerateVideoNode",
            "generated_image_id",
            CanvasNodeRefKind::GeneratedImage,
        )? {
            match edge.target_port.as_str() {
                "first_frame" => set_single_image_input(
                    &mut first_frame_image_id,
                    image_id,
                    "GenerateVideoNode 首帧输入",
                )?,
                "end_frame" => set_single_image_input(
                    &mut last_frame_image_id,
                    image_id,
                    "GenerateVideoNode 尾帧输入",
                )?,
                "reference_image" if !reference_image_ids.contains(&image_id) => {
                    reference_image_ids.push(image_id);
                }
                "text" => {}
                other => {
                    return Err(format!(
                        "GenerateVideoNode 图片输入端口无效: target_port={other}, role={}",
                        edge.role
                    ));
                }
            }
        }
    }
    let input_mode = generate_video_input_mode(node.config_json.get("input_mode"))?;
    validate_generate_video_image_inputs(
        input_mode,
        first_frame_image_id,
        last_frame_image_id,
        &reference_image_ids,
    )?;
    let prompt = prompt_parts
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    if prompt.trim().is_empty() {
        return Err("GenerateVideoNode 缺少 prompt 输入".to_string());
    }
    Ok(CanvasVideoNodeInput {
        prompt,
        negative_prompt: crate::canvas::canvas_config_string(&node.config_json, "negative_prompt"),
        config_id: crate::canvas::canvas_config_i64(&node.config_json, "video_config_id"),
        width: crate::canvas::canvas_config_i64(&node.config_json, "width")
            .and_then(crate::canvas::i64_to_i32),
        height: crate::canvas::canvas_config_i64(&node.config_json, "height")
            .and_then(crate::canvas::i64_to_i32),
        duration_secs: crate::canvas::canvas_config_i64(&node.config_json, "duration_secs")
            .and_then(crate::canvas::i64_to_i32),
        fps: crate::canvas::canvas_config_i64(&node.config_json, "fps")
            .and_then(crate::canvas::i64_to_i32),
        first_frame_image_id,
        last_frame_image_id,
        reference_image_ids,
        source_node_ids,
        canvas_text_asset_id: crate::canvas::first_canvas_text_asset_id(node, document),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GenerateVideoInputMode {
    TextOnly,
    FirstFrame,
    FirstEndFrame,
    ReferenceImages,
}

fn generate_video_input_mode(value: Option<&Value>) -> Result<GenerateVideoInputMode, String> {
    let value = value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "GenerateVideoNode 缺少 input_mode".to_string())?;
    match value {
        "text_only" => Ok(GenerateVideoInputMode::TextOnly),
        "first_frame" => Ok(GenerateVideoInputMode::FirstFrame),
        "first_end_frame" => Ok(GenerateVideoInputMode::FirstEndFrame),
        "reference_images" => Ok(GenerateVideoInputMode::ReferenceImages),
        other => Err(format!("GenerateVideoNode input_mode 无效: {other}")),
    }
}

fn validate_generate_video_image_inputs(
    mode: GenerateVideoInputMode,
    first_frame_image_id: Option<i64>,
    last_frame_image_id: Option<i64>,
    reference_image_ids: &[i64],
) -> Result<(), String> {
    match mode {
        GenerateVideoInputMode::TextOnly => {
            if first_frame_image_id.is_some()
                || last_frame_image_id.is_some()
                || !reference_image_ids.is_empty()
            {
                return Err("GenerateVideoNode 当前为仅提示词模式，不能连接图片输入".to_string());
            }
        }
        GenerateVideoInputMode::FirstFrame => {
            if first_frame_image_id.is_none() {
                return Err("GenerateVideoNode 首帧模式缺少 first_frame 输入".to_string());
            }
            if last_frame_image_id.is_some() || !reference_image_ids.is_empty() {
                return Err("GenerateVideoNode 首帧模式只能连接 first_frame 输入".to_string());
            }
        }
        GenerateVideoInputMode::FirstEndFrame => {
            if first_frame_image_id.is_none() || last_frame_image_id.is_none() {
                return Err(
                    "GenerateVideoNode 首尾帧模式必须同时连接 first_frame 和 end_frame 输入"
                        .to_string(),
                );
            }
            if !reference_image_ids.is_empty() {
                return Err("GenerateVideoNode 首尾帧模式不能连接 reference_image 输入".to_string());
            }
        }
        GenerateVideoInputMode::ReferenceImages => {
            if reference_image_ids.is_empty() {
                return Err("GenerateVideoNode 参考图模式缺少 reference_image 输入".to_string());
            }
            if first_frame_image_id.is_some() || last_frame_image_id.is_some() {
                return Err("GenerateVideoNode 参考图模式只能连接 reference_image 输入".to_string());
            }
        }
    }
    Ok(())
}

fn set_single_image_input(
    slot: &mut Option<i64>,
    image_id: i64,
    label: &str,
) -> Result<(), String> {
    if let Some(existing) = *slot
        && existing != image_id
    {
        return Err(format!("{label}只允许一个来源"));
    }
    *slot = Some(image_id);
    Ok(())
}

struct PreparedCanvasVideoRequest {
    config: VideoProviderConfig,
    prompt: String,
    request: VideoGenerationRequest,
    first_frame_image_id: Option<i64>,
    last_frame_image_id: Option<i64>,
    reference_image_ids: Vec<i64>,
}

async fn prepare_canvas_video_request(
    host_port: &dyn CanvasVideoPreviewHostPort,
    user_id: i64,
    project_id: i64,
    input: &CanvasVideoNodeInput,
    mode: VideoProviderValidationMode,
) -> Result<PreparedCanvasVideoRequest, AppError> {
    let config = host_port
        .resolve_video_config_for_preview(input.config_id)
        .await?;
    let duration_secs = input.duration_secs.unwrap_or(config.default_duration_secs);
    crate::video::validate_video_duration_for_provider_config_secs(&config, duration_secs)
        .map_err(AppError::InvalidArgument)?;
    let width = input.width.unwrap_or(config.default_width);
    let height = input.height.unwrap_or(config.default_height);
    if width <= 0 || height <= 0 {
        return Err(AppError::InvalidArgument(
            "画布视频宽高必须为正整数".to_string(),
        ));
    }
    let fps = input.fps.unwrap_or(config.default_fps);
    if fps <= 0 {
        return Err(AppError::InvalidArgument(
            "画布视频 fps 必须为正整数".to_string(),
        ));
    }
    let prompt = input.prompt.trim().to_string();
    if prompt.is_empty() {
        return Err(AppError::InvalidArgument(
            "GenerateVideoNode 缺少有效 prompt 输入".to_string(),
        ));
    }
    let first_frame_image_id = input.first_frame_image_id;
    let last_frame_image_id = input.last_frame_image_id;
    let reference_image_ids = input.reference_image_ids.clone();
    let images = host_port
        .resolve_canvas_video_images_for_preview(CanvasVideoImageInput {
            user_id,
            project_id,
            mode,
            first_frame_image_id,
            last_frame_image_id,
            reference_image_ids: reference_image_ids.clone(),
        })
        .await?;
    let request = VideoGenerationRequest {
        prompt: prompt.clone(),
        negative_prompt: input
            .negative_prompt
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        width,
        height,
        duration_secs,
        fps,
        model: config.model.clone(),
        media: a4n_video::VideoMediaInput::from_images(
            images.first_frame_image_url,
            images.last_frame_image_url,
            images.reference_image_urls,
        )
        .map_err(AppError::InvalidArgument)?,
    };
    Ok(PreparedCanvasVideoRequest {
        config,
        prompt,
        request,
        first_frame_image_id,
        last_frame_image_id,
        reference_image_ids,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        GenerateVideoInputMode, generate_video_input_mode, validate_generate_video_image_inputs,
    };
    use serde_json::json;

    #[test]
    fn video_input_mode_requires_canonical_value() {
        assert!(
            generate_video_input_mode(None)
                .expect_err("缺少 input_mode 必须失败")
                .contains("缺少 input_mode")
        );
    }

    #[test]
    fn text_only_mode_rejects_image_sources() {
        assert!(
            validate_generate_video_image_inputs(
                GenerateVideoInputMode::TextOnly,
                Some(1),
                None,
                &[],
            )
            .is_err()
        );
    }

    #[test]
    fn unknown_mode_is_rejected() {
        assert!(generate_video_input_mode(Some(&json!("unknown"))).is_err());
    }
}
