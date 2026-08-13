use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use a4n_api_contract::{
    generation_tasks::GenerationTaskHandle,
    video::{StartVideoGenerationInput, VideoGenerationOperation},
};
use a4n_core::{error::AppError, models::ai::generated_video};
use a4n_service_protocol::{
    BatchCommitPolicy, BatchSummary, GenerationArtifactRef, GenerationError,
    GenerationFinishReason, GenerationResultRef,
};
use futures::future::BoxFuture;
use sha2::{Digest, Sha256};
use tokio::sync::mpsc;

use crate::{
    OrchestratorContext,
    generation::{
        GenerationProgressUpdate,
        artifacts::{GenerationArtifactCoordinator, PreparedArtifactEvent},
        generation_public_error, publish_step_progress,
        terminal::CompletionDescriptor,
    },
};

use super::{
    ChapterVideoGenerationHostPort, GenerateChapterVideoInput, GenerateChapterVideosBatchInput,
    GenerateReferenceMediaVideoInput, ReferenceMediaVideoGenerationHostPort,
    VIDEO_GENERATION_PROGRESS_TOTAL, VideoGenerationCheckpointContext, VideoGenerationProgress,
    VideoProviderHostPort, has_active_provider_job,
};

struct VideoTaskResult {
    summary: String,
    result_ref: Option<GenerationResultRef>,
    artifacts: Vec<PreparedArtifactEvent>,
    batch_summary: Option<BatchSummary>,
    failure: Option<GenerationError>,
}

struct BatchAccumulator {
    total: u32,
    completed_ids: Vec<i64>,
    artifacts: Vec<PreparedArtifactEvent>,
    failed_item_keys: Vec<String>,
    failed: u32,
    cancelled: u32,
    skipped: u32,
    first_error: Option<String>,
    interrupted: bool,
}

impl OrchestratorContext {
    pub async fn run_video_generation_task<H>(
        &self,
        user_id: i64,
        input: StartVideoGenerationInput,
        handle: GenerationTaskHandle,
        host_port: &H,
    ) -> Result<(), String>
    where
        H: ChapterVideoGenerationHostPort + ReferenceMediaVideoGenerationHostPort,
    {
        let Some(execution) = self
            .acquire_generation_task_execution(&handle)
            .await
            .map_err(|error| error.to_string())?
        else {
            return Ok(());
        };
        let session = execution.session.clone();
        let cancel_token = execution.permit.cancellation_token();
        let operation_key = video_operation_key(&input.operation);
        let checkpoint_context = VideoGenerationCheckpointContext::new(
            session.clone(),
            self.database.shared_connection(),
        );
        let publisher = execution.publisher.clone();
        let checkpoint_writer = execution.checkpoint_writer.clone();
        let (progress_sender, progress_receiver) = mpsc::unbounded_channel();
        let progress_publish_failed = Arc::new(AtomicBool::new(false));
        let progress_publish_failed_for_reporter = Arc::clone(&progress_publish_failed);
        let progress_worker = tokio::spawn(publish_step_progress(
            progress_receiver,
            publisher,
            checkpoint_writer,
            session.clone(),
            operation_key,
            operation_key,
            1,
        ));
        let mut reporter = move |current: u32, total: u32, message: &str| {
            progress_sender
                .send(GenerationProgressUpdate {
                    current,
                    total,
                    message: message.to_string(),
                })
                .map_err(|_| {
                    progress_publish_failed_for_reporter.store(true, Ordering::Release);
                    AppError::InvalidState {
                        current: "video_generation_progress".to_string(),
                        operation: "canonical progress publisher 已关闭".to_string(),
                    }
                })
        };
        let application_result = {
            let mut progress = VideoGenerationProgress::with_checkpoint(
                Some(&mut reporter),
                cancel_token,
                checkpoint_context,
            );
            self.execute_video_generation_operation(user_id, input, host_port, &mut progress)
                .await
        };
        drop(reporter);
        let mut progress_result = match progress_worker.await {
            Ok(result) => result,
            Err(error) => Err(generation_public_error(
                "generation_event_invalid",
                operation_key,
                format!("视频进度 publisher 异常退出: {error}"),
            )),
        };
        if progress_result.is_ok() && progress_publish_failed.load(Ordering::Acquire) {
            progress_result = Err(generation_public_error(
                "generation_event_invalid",
                operation_key,
                "视频进度 publisher 已关闭",
            ));
        }
        let task_result = match (application_result, progress_result) {
            (Ok(result), Ok(())) => result,
            (Err(error), Ok(())) => {
                return Err(execution.fail(video_app_error(error, operation_key)).await);
            }
            (Ok(result), Err(error)) => {
                return Err(execution
                    .fail_with_batch_summary(error, result.batch_summary, result.artifacts)
                    .await);
            }
            (Err(error), Err(progress_error)) => {
                return Err(execution
                    .fail(generation_public_error(
                        progress_error.code.as_str(),
                        operation_key,
                        format!("{error}; 进度发布失败: {}", progress_error.message),
                    ))
                    .await);
            }
        };
        if let Some(error) = task_result.failure {
            return Err(execution
                .fail_with_batch_summary(error, task_result.batch_summary, task_result.artifacts)
                .await);
        }
        let result_ref = task_result
            .result_ref
            .ok_or_else(|| "视频 generation 成功结果缺少 result_ref".to_string())?;
        let final_usage = session.snapshot().usage;
        let failure_batch_summary = task_result.batch_summary.clone();
        let failure_artifacts = task_result.artifacts.clone();
        let completion = execution
            .terminal
            .complete_with_artifacts(&session, task_result.artifacts, move |_tx| {
                Box::pin(async move {
                    Ok(CompletionDescriptor {
                        result_ref: Some(result_ref),
                        final_text_hashes: Vec::new(),
                        final_usage,
                        finish_reason: Some(GenerationFinishReason::Stop),
                        summary: Some(task_result.summary),
                        batch_summary: task_result.batch_summary,
                    })
                }) as BoxFuture<'_, Result<CompletionDescriptor, AppError>>
            })
            .await;
        if let Err(error) = completion {
            return Err(execution
                .fail_with_batch_summary(
                    generation_public_error(error.code(), operation_key, error),
                    failure_batch_summary,
                    failure_artifacts,
                )
                .await);
        }
        execution.finish();
        Ok(())
    }

    async fn execute_video_generation_operation<H>(
        &self,
        user_id: i64,
        input: StartVideoGenerationInput,
        host_port: &H,
        progress: &mut VideoGenerationProgress<'_>,
    ) -> Result<VideoTaskResult, AppError>
    where
        H: ChapterVideoGenerationHostPort + ReferenceMediaVideoGenerationHostPort,
    {
        let project_id = input.project_id;
        match input.operation {
            VideoGenerationOperation::Chapter {
                chapter_id,
                prompt_id,
                config_id,
            } => {
                let video = self
                    .run_chapter_video_generation_validated(
                        host_port,
                        GenerateChapterVideoInput {
                            user_id,
                            project_id,
                            chapter_id,
                            prompt_id,
                            config_id,
                        },
                        progress,
                    )
                    .await?;
                single_video_result(
                    ChapterVideoGenerationHostPort::video_provider_host_port(host_port),
                    video,
                    "章节视频生成完成",
                )
            }
            VideoGenerationOperation::ReferenceMedia { chapter_id, input } => {
                let video = self
                    .run_reference_media_video_generation_validated(
                        host_port,
                        GenerateReferenceMediaVideoInput {
                            user_id,
                            project_id,
                            chapter_id,
                            input,
                        },
                        progress,
                    )
                    .await?;
                single_video_result(
                    ReferenceMediaVideoGenerationHostPort::video_provider_host_port(host_port),
                    video,
                    "参考媒体视频生成完成",
                )
            }
            VideoGenerationOperation::ChapterBatch {
                chapter_id,
                prompt_ids,
                config_id,
            } => {
                self.run_canonical_chapter_video_batch(
                    host_port, user_id, project_id, chapter_id, prompt_ids, config_id, progress,
                )
                .await
            }
            VideoGenerationOperation::PromptSegmentPlan { .. }
            | VideoGenerationOperation::PromptInsertedSegment { .. }
            | VideoGenerationOperation::PromptFrames { .. }
            | VideoGenerationOperation::PromptVideos { .. } => Err(AppError::InvalidState {
                current: "video_prompt_generation".to_string(),
                operation: "视频提示词操作必须由 LLM generation worker 执行".to_string(),
            }),
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn run_canonical_chapter_video_batch<H>(
        &self,
        host_port: &H,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
        prompt_ids: Vec<i64>,
        config_id: Option<i64>,
        progress: &mut VideoGenerationProgress<'_>,
    ) -> Result<VideoTaskResult, AppError>
    where
        H: ChapterVideoGenerationHostPort + ReferenceMediaVideoGenerationHostPort,
    {
        let input = GenerateChapterVideosBatchInput {
            user_id,
            project_id,
            chapter_id,
            prompt_ids,
            config_id,
        };
        let prompt_ids = self.validate_chapter_video_batch_prompts(&input).await?;
        let total = u32::try_from(prompt_ids.len())
            .map_err(|_| AppError::InvalidArgument("批量视频数量超出 u32 范围".to_string()))?;
        let progress_total = total
            .checked_mul(VIDEO_GENERATION_PROGRESS_TOTAL)
            .ok_or_else(|| AppError::InvalidArgument("批量视频进度超出 u32 范围".to_string()))?;
        let checkpoint = progress
            .checkpoint_context()
            .ok_or_else(|| AppError::InvalidState {
                current: "video_batch_generation".to_string(),
                operation: "缺少 canonical checkpoint context".to_string(),
            })?;
        let mut batch = BatchAccumulator {
            total,
            completed_ids: Vec::new(),
            artifacts: Vec::new(),
            failed_item_keys: Vec::new(),
            failed: 0,
            cancelled: 0,
            skipped: 0,
            first_error: None,
            interrupted: false,
        };

        for (index, prompt_id) in prompt_ids.iter().copied().enumerate() {
            let item_index = u32::try_from(index).expect("prompt count was converted to u32");
            let item_key = format!("video-prompt-{prompt_id}");
            if progress.check_cancelled("视频批量生成已停止").is_err() {
                batch.cancelled = 1;
                batch.skipped = total.saturating_sub(item_index + 1);
                batch.interrupted = true;
                batch
                    .first_error
                    .get_or_insert_with(|| "视频批量生成已停止".to_string());
                break;
            }
            let base = item_index * VIDEO_GENERATION_PROGRESS_TOTAL;
            let cancel_token = progress.cancel_token();
            let mut mapped_progress = |current: u32, child_total: u32, message: &str| {
                let child_total = child_total.max(1);
                let scaled = (u64::from(current.min(child_total))
                    * u64::from(VIDEO_GENERATION_PROGRESS_TOTAL))
                    / u64::from(child_total);
                let scaled = u32::try_from(scaled)
                    .expect("scaled progress is bounded by VIDEO_GENERATION_PROGRESS_TOTAL");
                progress.progress(
                    (base + scaled).min(progress_total),
                    progress_total.max(1),
                    &format!("第 {}/{} 条视频：{message}", index + 1, prompt_ids.len()),
                )
            };
            let mut child_progress = VideoGenerationProgress::with_checkpoint(
                Some(&mut mapped_progress),
                cancel_token,
                checkpoint.clone(),
            );
            let result = self
                .run_chapter_video_generation_validated(
                    host_port,
                    GenerateChapterVideoInput {
                        user_id,
                        project_id,
                        chapter_id,
                        prompt_id,
                        config_id,
                    },
                    &mut child_progress,
                )
                .await;
            drop(child_progress);
            match result {
                Ok(video) if video.status == super::GENERATED_VIDEO_STATUS_COMPLETED => {
                    let artifact = video_artifact_event(
                        ChapterVideoGenerationHostPort::video_provider_host_port(host_port),
                        &video,
                        Some(item_key),
                        Some(item_index),
                        Some(total),
                    )?;
                    batch.completed_ids.push(video.id);
                    batch.artifacts.push(artifact);
                }
                Ok(video)
                    if has_active_provider_job(video.provider_job_id.as_deref(), &video.status)
                        || video.status == super::GENERATED_VIDEO_ERROR_STATUS_INTERRUPTED =>
                {
                    batch.failed += 1;
                    batch.failed_item_keys.push(item_key);
                    batch.skipped = total.saturating_sub(item_index + 1);
                    batch.interrupted = true;
                    batch.first_error.get_or_insert_with(|| {
                        video
                            .error_message
                            .clone()
                            .unwrap_or_else(|| "视频 provider job 等待已中断".to_string())
                    });
                    break;
                }
                Ok(video) => {
                    batch.failed += 1;
                    batch.failed_item_keys.push(item_key);
                    batch.first_error.get_or_insert_with(|| {
                        video
                            .error_message
                            .clone()
                            .unwrap_or_else(|| format!("视频 #{} 生成失败", video.id))
                    });
                }
                Err(error) => {
                    batch.failed += 1;
                    batch.failed_item_keys.push(item_key);
                    batch.first_error.get_or_insert_with(|| error.to_string());
                }
            }
        }

        let batch_summary = batch.summary();
        let failure = if batch.interrupted {
            Some(generation_public_error(
                "generation_interrupted",
                "video_chapter_batch",
                batch.first_error.as_deref().unwrap_or("批量视频生成已中断"),
            ))
        } else if batch.completed_ids.is_empty() {
            Some(generation_public_error(
                "provider_job_failed",
                "video_chapter_batch",
                batch
                    .first_error
                    .as_deref()
                    .unwrap_or("批量视频全部生成失败"),
            ))
        } else {
            None
        };
        Ok(VideoTaskResult {
            summary: format!(
                "批量视频生成完成，成功 {} 条，失败 {} 条",
                batch.completed_ids.len(),
                batch.failed
            ),
            result_ref: (!batch.completed_ids.is_empty())
                .then(|| batch_video_result_ref(project_id, &batch.completed_ids))
                .transpose()?,
            artifacts: batch.artifacts,
            batch_summary: Some(batch_summary),
            failure,
        })
    }
}

impl BatchAccumulator {
    fn summary(&self) -> BatchSummary {
        let mut failed_item_keys = self.failed_item_keys.clone();
        failed_item_keys.sort_unstable();
        failed_item_keys.dedup();
        let failed_item_keys_truncated = failed_item_keys.len() > 256;
        failed_item_keys.truncate(256);
        BatchSummary {
            policy: BatchCommitPolicy::BestEffort,
            total: self.total,
            succeeded: u32::try_from(self.completed_ids.len())
                .expect("completed video count is bounded by total"),
            failed: self.failed,
            cancelled: self.cancelled,
            skipped: self.skipped,
            failed_item_keys,
            failed_item_keys_truncated,
        }
    }
}

fn single_video_result(
    host_port: &dyn VideoProviderHostPort,
    video: generated_video::Model,
    summary: &str,
) -> Result<VideoTaskResult, AppError> {
    if video.status == super::GENERATED_VIDEO_STATUS_COMPLETED {
        let artifact = video_artifact_event(host_port, &video, None, None, None)?;
        return Ok(VideoTaskResult {
            summary: summary.to_string(),
            result_ref: Some(GenerationResultRef {
                kind: "generated_video".to_string(),
                id: video.id.to_string(),
                revision: artifact.artifact_ref().revision.clone(),
            }),
            artifacts: vec![artifact],
            batch_summary: None,
            failure: None,
        });
    }
    let interrupted = has_active_provider_job(video.provider_job_id.as_deref(), &video.status)
        || video.status == super::GENERATED_VIDEO_ERROR_STATUS_INTERRUPTED;
    Ok(VideoTaskResult {
        summary: video
            .error_message
            .clone()
            .unwrap_or_else(|| "视频生成未完成".to_string()),
        result_ref: None,
        artifacts: Vec::new(),
        batch_summary: None,
        failure: Some(generation_public_error(
            if interrupted {
                "generation_interrupted"
            } else {
                "provider_job_failed"
            },
            "video_generation",
            video
                .error_message
                .as_deref()
                .unwrap_or("视频 provider 未生成正式文件"),
        )),
    })
}

fn video_artifact_event(
    host_port: &dyn VideoProviderHostPort,
    video: &generated_video::Model,
    item_key: Option<String>,
    item_index: Option<u32>,
    item_total: Option<u32>,
) -> Result<PreparedArtifactEvent, AppError> {
    let relative_path = video
        .file_path
        .as_deref()
        .ok_or_else(|| AppError::InvalidState {
            current: format!("generated_video_id={}", video.id),
            operation: "completed 视频缺少 file_path".to_string(),
        })?;
    let file = host_port.inspect_video_file(relative_path, "正式视频文件")?;
    Ok(GenerationArtifactCoordinator::available(
        GenerationArtifactRef {
            artifact_id: format!("generated-video:{}", video.id),
            artifact_kind: "generated_video".to_string(),
            content_type: file.content_type,
            sha256: file.sha256.clone(),
            byte_size: file.byte_size,
            revision: file.sha256,
            temporary: false,
            expires_at_ms: None,
        },
        item_key,
        item_index,
        item_total,
    ))
}

fn batch_video_result_ref(
    project_id: i64,
    completed_ids: &[i64],
) -> Result<GenerationResultRef, AppError> {
    let mut ids = completed_ids.to_vec();
    ids.sort_unstable();
    Ok(GenerationResultRef {
        kind: "generated_video_batch".to_string(),
        id: project_id.to_string(),
        revision: format!("{:x}", Sha256::digest(serde_json::to_vec(&ids)?)),
    })
}

fn video_operation_key(operation: &VideoGenerationOperation) -> &'static str {
    match operation {
        VideoGenerationOperation::Chapter { .. } => "video_chapter",
        VideoGenerationOperation::ChapterBatch { .. } => "video_chapter_batch",
        VideoGenerationOperation::ReferenceMedia { .. } => "video_reference_media",
        VideoGenerationOperation::PromptSegmentPlan { .. } => "video_prompt_segment_plan",
        VideoGenerationOperation::PromptInsertedSegment { .. } => "video_prompt_inserted_segment",
        VideoGenerationOperation::PromptFrames { .. } => "video_prompt_frames",
        VideoGenerationOperation::PromptVideos { .. } => "video_prompt_videos",
    }
}

fn video_app_error(error: AppError, stage_code: &str) -> GenerationError {
    let code = match error {
        AppError::Cancelled => "generation_interrupted",
        AppError::InvalidArgument(_) | AppError::Forbidden(_) | AppError::Unauthorized(_) => {
            "generation_request_invalid"
        }
        AppError::LlmError(_) | AppError::Timeout(_) | AppError::ConfigError(_) => {
            "provider_job_failed"
        }
        _ => "generation_persist_failed",
    };
    generation_public_error(code, stage_code, error)
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use a4n_api_contract::video::VideoGenerationOperation;
    use a4n_core::models::ai::generated_video;
    use a4n_video::VideoJobState;
    use async_trait::async_trait;
    use chrono::Utc;
    use tokio_util::sync::CancellationToken;

    struct InspectOnlyHostPort {
        content_type: &'static str,
        sha256: &'static str,
        byte_size: u64,
    }

    #[async_trait]
    impl VideoProviderHostPort for InspectOnlyHostPort {
        fn inspect_video_file(
            &self,
            _relative_path: &str,
            _label: &str,
        ) -> Result<crate::video::VideoArtifactFile, AppError> {
            Ok(crate::video::VideoArtifactFile {
                content_type: self.content_type.to_string(),
                sha256: self.sha256.to_string(),
                byte_size: self.byte_size,
            })
        }

        async fn poll_video_job(
            &self,
            _project_id: i64,
            _workflow_run_id: i64,
            _config: &a4n_core::models::ai::video_gen_config::Model,
            _job_id: &str,
            _cancel_token: CancellationToken,
        ) -> Result<VideoJobState, AppError> {
            unreachable!("unit test host 不应进入 poll")
        }

        async fn persist_video_result(
            &self,
            _video: &generated_video::Model,
            _prompt_id: Option<i64>,
            _result: a4n_video::VideoGenerationResult,
        ) -> Result<crate::video::PersistedVideoResult, AppError> {
            unreachable!("unit test host 不应进入 persist")
        }

        async fn remove_video_result(
            &self,
            _persisted: &crate::video::PersistedVideoResult,
        ) -> Result<(), AppError> {
            unreachable!("unit test host 不应进入 remove")
        }
    }

    fn completed_video(id: i64, path: &str) -> generated_video::Model {
        let now = Utc::now().naive_utc();
        generated_video::Model {
            id,
            project_id: 1,
            chapter_id: Some(2),
            video_type: "chapter_video".to_string(),
            workflow_mode: Some("boundary_frames".to_string()),
            file_path: Some(path.to_string()),
            thumbnail_path: None,
            prompt: "prompt".to_string(),
            negative_prompt: None,
            model: "m".to_string(),
            width: 1280,
            height: 720,
            duration_secs: 6.0,
            fps: 24,
            file_size: Some(12),
            source_video_id: None,
            source_image_id: None,
            status: crate::video::GENERATED_VIDEO_STATUS_COMPLETED.to_string(),
            provider_job_id: None,
            error_message: None,
            timeline_item_id: None,
            metadata_json: None,
            is_favorite: false,
            created_at: now,
            updated_at: now,
        }
    }

    fn prompt_options() -> a4n_api_contract::video::VideoPromptGenerationOptions {
        a4n_api_contract::video::VideoPromptGenerationOptions {
            workflow_mode: Some("storyboard".to_string()),
            segment_plan_prompt_id: None,
            style: "电影感".to_string(),
            prompt_output_language: Some("zh_cn".to_string()),
            prompt_batch_size: 2,
            target_segment: None,
            target_narrative_unit_index: None,
            duration_secs: 6,
            duration_secs_options: vec![6],
            aspect_ratio: "16:9".to_string(),
            reference_image_ids: Vec::new(),
            video_library_image_ids: Vec::new(),
            regenerate_existing: false,
        }
    }

    #[test]
    fn video_operation_key_covers_all_start_kinds() {
        assert_eq!(
            video_operation_key(&VideoGenerationOperation::Chapter {
                chapter_id: 1,
                prompt_id: 2,
                config_id: None,
            }),
            "video_chapter"
        );
        assert_eq!(
            video_operation_key(&VideoGenerationOperation::ChapterBatch {
                chapter_id: 1,
                prompt_ids: vec![1, 2],
                config_id: None,
            }),
            "video_chapter_batch"
        );
        assert_eq!(
            video_operation_key(&VideoGenerationOperation::ReferenceMedia {
                chapter_id: 1,
                input: a4n_api_contract::video::ReferenceMediaVideoInput {
                    segment_plan_prompt_id: 2,
                    target_segment: a4n_api_contract::video::VideoSegmentTarget {
                        source_shot_id: "shot-1".to_string(),
                        segment_index_in_shot: 1,
                    },
                    image_uploads: Vec::new(),
                    video_uploads: Vec::new(),
                    audio_uploads: Vec::new(),
                    config_id: None,
                },
            }),
            "video_reference_media"
        );
        assert_eq!(
            video_operation_key(&VideoGenerationOperation::PromptSegmentPlan {
                chapter_id: 1,
                options: prompt_options(),
            }),
            "video_prompt_segment_plan"
        );
        assert_eq!(
            video_operation_key(&VideoGenerationOperation::PromptFrames {
                chapter_id: 1,
                options: prompt_options(),
            }),
            "video_prompt_frames"
        );
        assert_eq!(
            video_operation_key(&VideoGenerationOperation::PromptVideos {
                chapter_id: 1,
                options: prompt_options(),
            }),
            "video_prompt_videos"
        );
    }

    #[test]
    fn single_video_result_completed_exposes_artifact_and_result_ref() {
        let host = InspectOnlyHostPort {
            content_type: "video/mp4",
            sha256: "abc123",
            byte_size: 42,
        };
        let video = completed_video(9, "videos/9.mp4");
        let result = single_video_result(&host, video, "章节视频生成完成").expect("completed");
        assert!(result.failure.is_none());
        assert_eq!(result.summary, "章节视频生成完成");
        let result_ref = result.result_ref.expect("result_ref");
        assert_eq!(result_ref.kind, "generated_video");
        assert_eq!(result_ref.id, "9");
        assert_eq!(result_ref.revision, "abc123");
        assert_eq!(result.artifacts.len(), 1);
        assert_eq!(
            result.artifacts[0].artifact_ref().artifact_id,
            "generated-video:9"
        );
        assert_eq!(result.artifacts[0].artifact_ref().sha256, "abc123");
        assert_eq!(result.artifacts[0].artifact_ref().byte_size, 42);
    }

    #[test]
    fn single_video_result_failed_maps_provider_error() {
        let host = InspectOnlyHostPort {
            content_type: "video/mp4",
            sha256: "n/a",
            byte_size: 0,
        };
        let mut video = completed_video(3, "videos/3.mp4");
        video.status = "failed".to_string();
        video.file_path = None;
        video.error_message = Some("上游拒绝".to_string());
        let result = single_video_result(&host, video, "unused").expect("failed path");
        let failure = result.failure.expect("failure");
        assert_eq!(failure.code.as_str(), "provider_job_failed");
        assert!(failure.message.contains("上游拒绝"));
        assert!(result.result_ref.is_none());
        assert!(result.artifacts.is_empty());
    }

    #[test]
    fn batch_result_revision_is_stable_for_sorted_ids() {
        let left = batch_video_result_ref(10, &[3, 1, 2]).expect("left");
        let right = batch_video_result_ref(10, &[2, 3, 1]).expect("right");
        assert_eq!(left.kind, "generated_video_batch");
        assert_eq!(left.id, "10");
        assert_eq!(left.revision, right.revision);
        let different = batch_video_result_ref(10, &[1, 2]).expect("different");
        assert_ne!(left.revision, different.revision);
    }
}
