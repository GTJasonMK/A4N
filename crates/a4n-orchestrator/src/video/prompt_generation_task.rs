use std::sync::Arc;

use a4n_api_contract::{
    generation_tasks::GenerationTaskHandle,
    video::{
        StartVideoGenerationInput, VideoGenerationOperation, VideoInsertedSegmentGenerationOptions,
        VideoPromptGenerationOptions,
    },
};
use a4n_core::{
    error::AppError,
    prompt_language::{PromptOutputLanguage, metadata_matches_prompt_output_language},
    video_execution::{resolve_video_segment_number, video_segment_recipe_from_metadata},
};
use a4n_domain::progress::ProgressPhaseKind;
use a4n_service_protocol::{
    BatchCommitPolicy, BatchSummary, GenerationError, GenerationFinishReason, GenerationResultRef,
    MAX_TEXT_DELTA_BYTES, ProgressEvent, StreamTextHash,
};
use futures::future::BoxFuture;
use sha2::{Digest, Sha256};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::{
    OrchestratorContext,
    generation::{
        checkpoint::{GenerationCheckpointMetadata, GenerationCheckpointWriter},
        events::{GenerationEventPublisher, SessionGenerationEventPublisher},
        generation_public_error,
        terminal::CompletionDescriptor,
    },
};

use super::{
    ChapterVideoPromptSummary, GenerateChapterVideoPromptInput, VIDEO_PROMPT_CHECKPOINT_STAGE_CODE,
    VideoPromptApplicationContext, VideoPromptProgressUpdate,
    prompt_records::{expected_contract_version, video_execution_segment_identity_matches},
    provider_request_rules::required_video_execution_identity,
};

enum PromptTaskEvent {
    Progress(VideoPromptProgressUpdate),
    Text(String),
}

enum PromptTaskOperation {
    SegmentPlan(VideoPromptGenerationOptions),
    Inserted(VideoInsertedSegmentGenerationOptions),
    Frames(VideoPromptGenerationOptions),
    Videos(VideoPromptGenerationOptions),
}

struct PromptTaskInput {
    project_id: i64,
    chapter_id: i64,
    retry_of_task_id: Option<String>,
    operation: PromptTaskOperation,
}

struct PromptTaskResult {
    result_ref: GenerationResultRef,
    summary: &'static str,
}

impl OrchestratorContext {
    pub async fn run_video_prompt_generation_task(
        &self,
        user_id: i64,
        input: StartVideoGenerationInput,
        handle: GenerationTaskHandle,
        context: VideoPromptApplicationContext<'_>,
    ) -> Result<(), String> {
        let task_input = prompt_task_input(input)?;
        let operation_key = task_input.operation.operation_key();
        let batch_total = task_input.operation.batch_total();
        let Some(execution) = self
            .acquire_generation_task_execution(&handle)
            .await
            .map_err(|error| error.to_string())?
        else {
            return Ok(());
        };
        let session = execution.session.clone();
        let cancel_token = execution.permit.cancellation_token();
        let active_task_id = execution.permit.task_id().to_string();
        let root_project_id = session.snapshot().started.root_project_id;
        let publisher = execution.publisher.clone();
        let checkpoint_writer = execution.checkpoint_writer.clone();
        let (sender, receiver) = mpsc::unbounded_channel();
        let mut committed_records = 0_u32;
        let event_worker = tokio::spawn(publish_prompt_task_events(
            receiver,
            publisher,
            checkpoint_writer,
            session.clone(),
            operation_key,
        ));

        let progress_sender = sender.clone();
        let progress_cancel = cancel_token.clone();
        let progress_committed = &mut committed_records;
        let mut progress = move |update: VideoPromptProgressUpdate| {
            let item_committed = is_committed_update(&update);
            if !item_committed && !is_checkpoint_update(&update) && progress_cancel.is_cancelled() {
                return Err(AppError::Cancelled);
            }
            progress_sender
                .send(PromptTaskEvent::Progress(update))
                .map_err(|_| AppError::InvalidState {
                    current: "video_prompt_generation".to_string(),
                    operation: "canonical progress publisher 已关闭".to_string(),
                })?;
            if item_committed {
                *progress_committed += 1;
            }
            Ok(())
        };
        let chunks_cancel = cancel_token.clone();
        let mut chunks = move |content: &str, _index: u32| {
            if !content.is_empty() && !chunks_cancel.is_cancelled() {
                let _ = sender.send(PromptTaskEvent::Text(content.to_string()));
            }
        };

        let application_result = match root_project_id {
            Some(root_project_id) => {
                self.execute_video_prompt_task(
                    user_id,
                    task_input,
                    active_task_id,
                    root_project_id,
                    context,
                    &mut progress,
                    &mut chunks,
                    cancel_token,
                )
                .await
            }
            None => Err(AppError::InvalidState {
                current: "generation_task_without_root_project_id".to_string(),
                operation: "绑定视频提示词 workflow".to_string(),
            }),
        };
        drop(chunks);
        drop(progress);
        let event_result = match event_worker.await {
            Ok(result) => result,
            Err(error) => Err(generation_public_error(
                "generation_event_invalid",
                operation_key,
                format!("视频提示词事件 publisher 异常退出: {error}"),
            )),
        };
        let result = match (application_result, event_result) {
            (Ok(result), Ok(())) => result,
            (Err(error), Ok(())) => {
                return Err(execution
                    .fail_with_batch_summary(
                        video_prompt_app_error(error, operation_key),
                        failed_batch_summary(batch_total, &session),
                        Vec::new(),
                    )
                    .await);
            }
            (Ok(_), Err(error)) => {
                return Err(execution
                    .fail_with_batch_summary(
                        error,
                        failed_batch_summary(batch_total, &session),
                        Vec::new(),
                    )
                    .await);
            }
            (Err(error), Err(event_error)) => {
                return Err(execution
                    .fail_with_batch_summary(
                        generation_public_error(
                            event_error.code.as_str(),
                            operation_key,
                            format!("{error}; 事件发布失败: {}", event_error.message),
                        ),
                        failed_batch_summary(batch_total, &session),
                        Vec::new(),
                    )
                    .await);
            }
        };

        if batch_total.is_some() && committed_records == 0 {
            return Err(execution
                .fail_with_batch_summary(
                    generation_public_error(
                        "generation_validation_failed",
                        operation_key,
                        "本次视频提示词任务没有提交新记录",
                    ),
                    failed_batch_summary(batch_total, &session),
                    Vec::new(),
                )
                .await);
        }

        let snapshot = session.snapshot();
        let final_text_hashes = snapshot
            .streams
            .iter()
            .map(|stream| StreamTextHash {
                stream_id: stream.stream_id.clone(),
                utf8_bytes: stream.next_offset,
                sha256: stream.sha256.clone(),
            })
            .collect::<Vec<_>>();
        let final_usage = snapshot.usage;
        let batch_summary = completed_batch_summary(batch_total);
        let failure_batch_summary = batch_summary.clone();
        let completion = execution
            .terminal
            .complete_with(&session, move |_tx| {
                Box::pin(async move {
                    Ok(CompletionDescriptor {
                        result_ref: Some(result.result_ref),
                        final_text_hashes,
                        final_usage,
                        finish_reason: Some(GenerationFinishReason::Stop),
                        summary: Some(result.summary.to_string()),
                        batch_summary,
                    })
                }) as BoxFuture<'_, Result<CompletionDescriptor, AppError>>
            })
            .await;
        if let Err(error) = completion {
            return Err(execution
                .fail_with_batch_summary(
                    generation_public_error(error.code(), operation_key, error),
                    failure_batch_summary,
                    Vec::new(),
                )
                .await);
        }
        execution.finish();
        Ok(())
    }

    async fn execute_video_prompt_task(
        &self,
        user_id: i64,
        input: PromptTaskInput,
        active_task_id: String,
        root_project_id: i64,
        context: VideoPromptApplicationContext<'_>,
        progress: &mut (dyn FnMut(VideoPromptProgressUpdate) -> Result<(), AppError> + Send),
        chunks: &mut (dyn FnMut(&str, u32) + Send),
        cancel_token: CancellationToken,
    ) -> Result<PromptTaskResult, AppError> {
        let PromptTaskInput {
            project_id,
            chapter_id,
            retry_of_task_id,
            operation,
        } = input;
        match operation {
            PromptTaskOperation::SegmentPlan(options) => {
                let rows = self
                    .generate_chapter_video_segment_plan_validated(
                        context,
                        GenerateChapterVideoPromptInput {
                            user_id,
                            project_id,
                            chapter_id,
                            active_task_id: Some(active_task_id),
                            root_project_id: Some(root_project_id),
                            retry_of_task_id,
                            options,
                        },
                        Some(progress),
                        Some(chunks),
                        cancel_token,
                    )
                    .await?;
                progress(VideoPromptProgressUpdate::committed(
                    100,
                    100,
                    "frame_segment_plan",
                    0,
                    1,
                    1,
                    "视频片段规划已保存",
                ))?;
                Ok(PromptTaskResult {
                    result_ref: prompt_rows_result_ref(project_id, chapter_id, &rows),
                    summary: "视频片段规划已生成",
                })
            }
            PromptTaskOperation::Inserted(options) => {
                let result_ref = super::generate_inserted_segment_draft(
                    self,
                    user_id,
                    project_id,
                    chapter_id,
                    &active_task_id,
                    root_project_id,
                    retry_of_task_id.as_deref(),
                    options,
                    context.prompt_renderer,
                    Some(progress),
                    Some(chunks),
                    &cancel_token,
                )
                .await?;
                Ok(PromptTaskResult {
                    result_ref,
                    summary: "插入片段草稿已生成",
                })
            }
            PromptTaskOperation::Frames(options) => {
                let rows = self
                    .generate_chapter_video_frame_prompts_validated(
                        context,
                        GenerateChapterVideoPromptInput {
                            user_id,
                            project_id,
                            chapter_id,
                            active_task_id: Some(active_task_id),
                            root_project_id: Some(root_project_id),
                            retry_of_task_id,
                            options,
                        },
                        Some(progress),
                        Some(chunks),
                        cancel_token,
                    )
                    .await?;
                Ok(PromptTaskResult {
                    result_ref: prompt_rows_result_ref(project_id, chapter_id, &rows),
                    summary: "帧提示词已生成",
                })
            }
            PromptTaskOperation::Videos(options) => {
                let rows = self
                    .generate_video_prompt_for_target(
                        user_id,
                        project_id,
                        chapter_id,
                        active_task_id,
                        root_project_id,
                        retry_of_task_id,
                        options,
                        context,
                        progress,
                        chunks,
                        cancel_token,
                    )
                    .await?;
                Ok(PromptTaskResult {
                    result_ref: prompt_rows_result_ref(project_id, chapter_id, &rows),
                    summary: "视频提示词已生成",
                })
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn generate_video_prompt_for_target(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
        active_task_id: String,
        root_project_id: i64,
        retry_of_task_id: Option<String>,
        options: VideoPromptGenerationOptions,
        context: VideoPromptApplicationContext<'_>,
        progress: &mut (dyn FnMut(VideoPromptProgressUpdate) -> Result<(), AppError> + Send),
        chunks: &mut (dyn FnMut(&str, u32) + Send),
        cancel_token: CancellationToken,
    ) -> Result<Vec<ChapterVideoPromptSummary>, AppError> {
        let existing = self
            .list_chapter_video_prompts_validated(user_id, project_id, chapter_id)
            .await?;
        let target_identity = current_video_prompt_target_identity(&existing, &options)?;
        validate_video_prompt_task_target(
            &existing,
            &target_identity,
            options.regenerate_existing,
        )?;
        if cancel_token.is_cancelled() {
            return Err(AppError::Cancelled);
        }
        let rows = self
            .generate_chapter_video_prompts_validated(
                context,
                GenerateChapterVideoPromptInput {
                    user_id,
                    project_id,
                    chapter_id,
                    active_task_id: Some(active_task_id),
                    root_project_id: Some(root_project_id),
                    retry_of_task_id,
                    options,
                },
                Some(&mut *progress),
                Some(&mut *chunks),
                cancel_token,
            )
            .await?;
        let shot_number = committed_video_prompt_shot(&existing, &rows, &target_identity)
            .ok_or_else(|| AppError::InvalidState {
                current: "video_prompt_generation".to_string(),
                operation: format!(
                    "模型调用完成但没有提交目标视频提示词 #{}",
                    target_identity.segment_number
                ),
            })?;
        progress(VideoPromptProgressUpdate::committed(
            90,
            100,
            "video_prompt",
            shot_number,
            1,
            1,
            format!("视频提示词 #{shot_number} 已保存"),
        ))?;
        Ok(rows)
    }
}

impl PromptTaskOperation {
    fn operation_key(&self) -> &'static str {
        match self {
            Self::SegmentPlan(_) => "video_prompt_segment_plan",
            Self::Inserted(_) => "video_prompt_inserted_segment",
            Self::Frames(_) => "video_prompt_frames",
            Self::Videos(_) => "video_prompt_videos",
        }
    }

    fn batch_total(&self) -> Option<u32> {
        matches!(self, Self::Frames(_) | Self::Videos(_)).then_some(1)
    }
}

async fn publish_prompt_task_events(
    mut receiver: mpsc::UnboundedReceiver<PromptTaskEvent>,
    publisher: SessionGenerationEventPublisher,
    checkpoint_writer: GenerationCheckpointWriter,
    session: Arc<crate::generation::events::GenerationTaskSession>,
    operation_key: &'static str,
) -> Result<(), GenerationError> {
    let mut pending_event = None;
    loop {
        let event = match pending_event.take() {
            Some(event) => event,
            None => match receiver.recv().await {
                Some(event) => event,
                None => break,
            },
        };
        let event = coalesce_prompt_text_event(event, &mut receiver, &mut pending_event);
        let force_checkpoint = match event {
            PromptTaskEvent::Progress(update) => {
                if is_checkpoint_update(&update) {
                    let checkpoint_key = update.item_key.ok_or_else(|| {
                        generation_public_error(
                            "generation_checkpoint_invalid",
                            operation_key,
                            "视频提示词 checkpoint update 缺少 item_key",
                        )
                    })?;
                    let draft_text = update.draft_text.ok_or_else(|| {
                        generation_public_error(
                            "generation_checkpoint_invalid",
                            operation_key,
                            "视频提示词 checkpoint update 缺少 draft_text",
                        )
                    })?;
                    checkpoint_writer
                        .persist_checkpoint_event(
                            &session,
                            GenerationCheckpointMetadata {
                                step_key: Some(checkpoint_key.clone()),
                                ..Default::default()
                            },
                            checkpoint_key,
                            true,
                            Some(update.message),
                            Some(("segment_plan_draft".to_string(), draft_text)),
                        )
                        .await
                        .map_err(|error| {
                            generation_public_error(error.code(), "video_prompt_checkpoint", error)
                        })?;
                    continue;
                }
                let committed = is_committed_update(&update);
                publisher
                    .progress(ProgressEvent {
                        stage_code: update.stage_code,
                        phase_kind: update.phase_kind,
                        message: update.message,
                        current: Some(update.current.into()),
                        total: Some(update.total.into()),
                        unit_label: update.unit_label,
                        item_key: update.item_key,
                        item_index: update.item_index,
                        item_total: update.item_total,
                        severity: update.severity,
                        estimated: false,
                    })
                    .await
                    .map_err(|error| generation_public_error(error.code(), operation_key, error))?;
                committed
            }
            PromptTaskEvent::Text(delta) => {
                publisher
                    .text_delta("raw_json", &delta)
                    .await
                    .map_err(|error| generation_public_error(error.code(), operation_key, error))?;
                false
            }
        };
        checkpoint_writer
            .persist_if_due(
                &session,
                GenerationCheckpointMetadata {
                    step_key: Some(operation_key.to_string()),
                    ..Default::default()
                },
                force_checkpoint,
            )
            .await
            .map_err(|error| {
                generation_public_error(error.code(), "video_prompt_checkpoint", error)
            })?;
    }
    checkpoint_writer
        .persist_if_due(
            &session,
            GenerationCheckpointMetadata {
                step_key: Some(operation_key.to_string()),
                ..Default::default()
            },
            true,
        )
        .await
        .map(|_| ())
        .map_err(|error| generation_public_error(error.code(), "video_prompt_checkpoint", error))
}

fn coalesce_prompt_text_event(
    event: PromptTaskEvent,
    receiver: &mut mpsc::UnboundedReceiver<PromptTaskEvent>,
    pending_event: &mut Option<PromptTaskEvent>,
) -> PromptTaskEvent {
    let mut text = match event {
        PromptTaskEvent::Text(text) => text,
        event => return event,
    };
    while let Ok(next) = receiver.try_recv() {
        match next {
            PromptTaskEvent::Text(delta)
                if delta.len() <= MAX_TEXT_DELTA_BYTES.saturating_sub(text.len()) =>
            {
                text.push_str(&delta);
            }
            next => {
                *pending_event = Some(next);
                break;
            }
        }
    }
    PromptTaskEvent::Text(text)
}

fn is_committed_update(update: &VideoPromptProgressUpdate) -> bool {
    update.stage_code == "video_prompt_item_committed"
        && update.phase_kind == ProgressPhaseKind::Save
        && update.item_key.is_some()
}

fn is_checkpoint_update(update: &VideoPromptProgressUpdate) -> bool {
    update.stage_code == VIDEO_PROMPT_CHECKPOINT_STAGE_CODE
}

fn prompt_task_input(input: StartVideoGenerationInput) -> Result<PromptTaskInput, String> {
    let StartVideoGenerationInput {
        meta,
        project_id,
        operation,
    } = input;
    let retry_of_task_id = meta.retry_of_task_id;
    let task = match operation {
        VideoGenerationOperation::PromptSegmentPlan {
            chapter_id,
            options,
        } => PromptTaskInput {
            project_id,
            chapter_id,
            retry_of_task_id,
            operation: PromptTaskOperation::SegmentPlan(options),
        },
        VideoGenerationOperation::PromptInsertedSegment {
            chapter_id,
            options,
        } => PromptTaskInput {
            project_id,
            chapter_id,
            retry_of_task_id,
            operation: PromptTaskOperation::Inserted(options),
        },
        VideoGenerationOperation::PromptFrames {
            chapter_id,
            options,
        } => PromptTaskInput {
            project_id,
            chapter_id,
            retry_of_task_id,
            operation: PromptTaskOperation::Frames(options),
        },
        VideoGenerationOperation::PromptVideos {
            chapter_id,
            options,
        } => PromptTaskInput {
            project_id,
            chapter_id,
            retry_of_task_id,
            operation: PromptTaskOperation::Videos(options),
        },
        _ => return Err("非视频提示词操作不能进入 LLM generation worker".to_string()),
    };
    Ok(task)
}

fn validate_video_prompt_task_target(
    rows: &[ChapterVideoPromptSummary],
    identity: &VideoPromptTargetIdentity,
    regenerate_existing: bool,
) -> Result<(), AppError> {
    let exists = rows.iter().any(|row| video_prompt_matches(row, identity));
    if regenerate_existing && !exists {
        return Err(AppError::InvalidArgument(format!(
            "视频提示词 #{} 不存在，不能重新生成",
            identity.segment_number
        )));
    }
    if !regenerate_existing && exists {
        return Err(AppError::InvalidArgument(format!(
            "视频提示词 #{} 已存在；如需覆盖请使用重新生成",
            identity.segment_number
        )));
    }
    Ok(())
}

fn committed_video_prompt_shot(
    before: &[ChapterVideoPromptSummary],
    after: &[ChapterVideoPromptSummary],
    identity: &VideoPromptTargetIdentity,
) -> Option<i32> {
    after
        .iter()
        .filter(|row| video_prompt_matches(row, identity))
        .find(|row| {
            let previous = before.iter().find(|previous| previous.id == row.id);
            previous.is_none_or(|previous| previous.updated_at != row.updated_at)
        })
        .map(|row| row.shot_number)
}

fn video_prompt_matches(
    row: &ChapterVideoPromptSummary,
    identity: &VideoPromptTargetIdentity,
) -> bool {
    let Some(metadata) = row.metadata_json.as_ref() else {
        return false;
    };
    let Some(contract_version) = expected_contract_version(&identity.workflow_mode, "video_prompt")
    else {
        return false;
    };
    row.prompt_type == "video_prompt"
        && row.workflow_mode == identity.workflow_mode
        && metadata
            .get("contract_version")
            .and_then(serde_json::Value::as_str)
            == Some(contract_version)
        && metadata
            .get("source_prompt_type")
            .and_then(serde_json::Value::as_str)
            == Some("frame_segment_plan")
        && metadata_matches_prompt_output_language(Some(metadata), identity.prompt_output_language)
        && required_video_execution_identity(row.id, Some(metadata)).is_ok()
        && video_execution_segment_identity_matches(Some(metadata), &identity.execution_identity)
        && metadata
            .get("video_execution_identity")
            .and_then(|value| value.get("prompt_output_language"))
            .and_then(serde_json::Value::as_str)
            == Some(identity.prompt_output_language.as_str())
}

struct VideoPromptTargetIdentity {
    segment_number: i32,
    workflow_mode: String,
    prompt_output_language: PromptOutputLanguage,
    execution_identity: serde_json::Value,
}

fn current_video_prompt_target_identity(
    rows: &[ChapterVideoPromptSummary],
    options: &VideoPromptGenerationOptions,
) -> Result<VideoPromptTargetIdentity, AppError> {
    let segment_plan_prompt_id = options.segment_plan_prompt_id.ok_or_else(|| {
        AppError::InvalidArgument("生成片段提示词必须提供 segment_plan_prompt_id".to_string())
    })?;
    let plan = rows
        .iter()
        .find(|row| {
            row.id == segment_plan_prompt_id
                && row.prompt_type == "frame_segment_plan"
                && row.workflow_mode == "shared_segment_plan"
                && row.status == "completed"
        })
        .ok_or_else(|| AppError::InvalidArgument("当前共享片段规划不存在或已失效".to_string()))?;
    let plan_metadata = plan.metadata_json.as_ref().ok_or_else(|| {
        AppError::InvalidArgument("当前共享片段规划缺少 metadata_json".to_string())
    })?;
    let target_segment = options.target_segment.as_ref().ok_or_else(|| {
        AppError::InvalidArgument("生成片段提示词必须提供稳定片段目标".to_string())
    })?;
    let segment_number = resolve_video_segment_number(
        Some(plan_metadata),
        &target_segment.source_shot_id,
        target_segment.segment_index_in_shot,
    )
    .map_err(AppError::InvalidArgument)?;
    let expected_plan_contract =
        expected_contract_version("shared_segment_plan", "frame_segment_plan").ok_or_else(
            || AppError::InvalidState {
                current: "missing_shared_segment_plan_contract".to_string(),
                operation: "校验共享片段规划合同".to_string(),
            },
        )?;
    if plan_metadata
        .get("contract_version")
        .and_then(serde_json::Value::as_str)
        != Some(expected_plan_contract)
    {
        return Err(AppError::InvalidArgument(
            "当前共享片段规划合同已失效，请重新生成".to_string(),
        ));
    }
    let recipe = video_segment_recipe_from_metadata(plan.metadata_json.as_ref(), segment_number)
        .map_err(AppError::InvalidArgument)?
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("当前规划缺少片段 {segment_number} 的执行方案"))
        })?;
    if options.workflow_mode.as_deref() != Some(recipe.workflow_mode.as_str()) {
        return Err(AppError::InvalidArgument(
            "片段执行方案已变化，请刷新后重试".to_string(),
        ));
    }
    let prompt_output_language =
        PromptOutputLanguage::from_optional_code(options.prompt_output_language.as_deref())?;
    if prompt_output_language.as_str() != recipe.prompt_output_language {
        return Err(AppError::InvalidArgument(
            "片段提示词语言已变化，请刷新后重试".to_string(),
        ));
    }
    let plan_identity = plan_metadata
        .get("video_execution_identity")
        .cloned()
        .filter(serde_json::Value::is_object)
        .ok_or_else(|| AppError::InvalidArgument("当前共享片段规划缺少有效执行身份".to_string()))?;
    let execution_identity = super::prompt_generation::video_segment_execution_identity(
        plan_identity,
        Some(segment_plan_prompt_id),
        Some(&recipe),
        (
            &target_segment.source_shot_id,
            target_segment.segment_index_in_shot,
        ),
    )?;
    Ok(VideoPromptTargetIdentity {
        segment_number,
        workflow_mode: recipe.workflow_mode.as_str().to_string(),
        prompt_output_language,
        execution_identity,
    })
}

fn prompt_rows_revision(rows: &[ChapterVideoPromptSummary]) -> String {
    let mut digest = Sha256::new();
    let mut identity = rows
        .iter()
        .map(|row| (row.id, row.updated_at.as_str()))
        .collect::<Vec<_>>();
    identity.sort_unstable_by_key(|(id, _)| *id);
    for (id, updated_at) in identity {
        digest.update(id.to_le_bytes());
        digest.update((updated_at.len() as u64).to_le_bytes());
        digest.update(updated_at.as_bytes());
    }
    format!("{:x}", digest.finalize())
}

fn prompt_rows_result_ref(
    project_id: i64,
    chapter_id: i64,
    rows: &[ChapterVideoPromptSummary],
) -> GenerationResultRef {
    GenerationResultRef {
        kind: "chapter_video_prompts".to_string(),
        id: format!("{project_id}:{chapter_id}"),
        revision: prompt_rows_revision(rows),
    }
}

fn completed_batch_summary(total: Option<u32>) -> Option<BatchSummary> {
    total.map(|total| BatchSummary {
        policy: BatchCommitPolicy::BestEffort,
        total,
        succeeded: total,
        failed: 0,
        cancelled: 0,
        skipped: 0,
        failed_item_keys: Vec::new(),
        failed_item_keys_truncated: false,
    })
}

fn failed_batch_summary(
    total: Option<u32>,
    session: &crate::generation::events::GenerationTaskSession,
) -> Option<BatchSummary> {
    total.map(|total| {
        let cancelling =
            session.snapshot().status == a4n_service_protocol::GenerationTaskStatus::Cancelling;
        let remaining = total;
        let failed = u32::from(!cancelling && remaining > 0);
        let cancelled = u32::from(cancelling && remaining > 0);
        BatchSummary {
            policy: BatchCommitPolicy::BestEffort,
            total,
            succeeded: 0,
            failed,
            cancelled,
            skipped: remaining.saturating_sub(failed + cancelled),
            failed_item_keys: if failed > 0 {
                vec!["uncommitted:1".to_string()]
            } else {
                Vec::new()
            },
            failed_item_keys_truncated: false,
        }
    })
}

fn video_prompt_app_error(error: AppError, stage_code: &str) -> GenerationError {
    let code = match error {
        AppError::Cancelled => "generation_interrupted",
        AppError::LlmError(_) | AppError::Timeout(_) | AppError::ConfigError(_) => {
            "provider_job_failed"
        }
        AppError::InvalidArgument(_) | AppError::InvalidState { .. } => {
            "generation_validation_failed"
        }
        AppError::Forbidden(_) | AppError::Unauthorized(_) | AppError::NotFound(_) => {
            "generation_task_forbidden"
        }
        _ => "generation_persist_failed",
    };
    generation_public_error(code, stage_code, error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    #[test]
    fn committed_updates_require_the_persist_contract() {
        assert!(is_committed_update(&VideoPromptProgressUpdate::committed(
            50,
            100,
            "video_prompt",
            1,
            1,
            2,
            "已保存",
        )));
        assert!(!is_committed_update(&VideoPromptProgressUpdate::progress(
            50,
            100,
            "生成中",
        )));
    }

    #[test]
    fn checkpoint_updates_are_durable_without_becoming_formal_commits() {
        let update = VideoPromptProgressUpdate::checkpoint(
            75,
            100,
            "video_segment_plan.shot.shot-2",
            2,
            4,
            "{\"segments\":[]}",
            "镜头草稿已保存",
        );

        assert!(is_checkpoint_update(&update));
        assert!(!is_committed_update(&update));
        assert_eq!(update.draft_text.as_deref(), Some("{\"segments\":[]}"));
    }

    #[test]
    fn prompt_task_input_preserves_retry_source() {
        let retry_of_task_id = "e991ffd3-20ca-4584-882f-485407982dc3";
        let task = prompt_task_input(StartVideoGenerationInput {
            meta: a4n_api_contract::generation_tasks::GenerationStartRequestMeta {
                start_request_id: "request-1".to_string(),
                retry_of_task_id: Some(retry_of_task_id.to_string()),
            },
            project_id: 21,
            operation: VideoGenerationOperation::PromptSegmentPlan {
                chapter_id: 5,
                options: VideoPromptGenerationOptions {
                    workflow_mode: Some("shared_segment_plan".to_string()),
                    segment_plan_prompt_id: None,
                    style: "写实".to_string(),
                    prompt_output_language: Some("zh_cn".to_string()),
                    prompt_batch_size: 1,
                    target_segment: None,
                    target_narrative_unit_index: None,
                    duration_secs: 5,
                    duration_secs_options: vec![5],
                    aspect_ratio: "16:9".to_string(),
                    reference_image_ids: Vec::new(),
                    video_library_image_ids: Vec::new(),
                    regenerate_existing: false,
                },
            },
        })
        .unwrap();

        assert_eq!(task.retry_of_task_id.as_deref(), Some(retry_of_task_id));
    }

    #[test]
    fn text_events_are_coalesced_without_crossing_progress_events() {
        let (sender, mut receiver) = mpsc::unbounded_channel();
        sender.send(PromptTaskEvent::Text("a".to_string())).unwrap();
        sender.send(PromptTaskEvent::Text("b".to_string())).unwrap();
        sender
            .send(PromptTaskEvent::Progress(
                VideoPromptProgressUpdate::progress(50, 100, "生成中"),
            ))
            .unwrap();

        let mut pending = None;
        let event =
            coalesce_prompt_text_event(receiver.try_recv().unwrap(), &mut receiver, &mut pending);

        assert!(matches!(event, PromptTaskEvent::Text(text) if text == "ab"));
        assert!(matches!(pending, Some(PromptTaskEvent::Progress(_))));
    }

    #[test]
    fn completed_batch_summary_counts_targets_not_prompt_records() {
        let summary = completed_batch_summary(Some(1)).unwrap();
        assert_eq!(summary.succeeded, 1);
        assert_eq!(summary.skipped, 0);
    }

    #[test]
    fn video_prompt_targets_reject_invalid_regeneration() {
        let identity = test_identity(1);
        let error = validate_video_prompt_task_target(&[], &identity, true).unwrap_err();
        assert!(error.to_string().contains("不存在"));

        let rows = vec![test_prompt(1, 1, "2026-01-01 00:00:00", "zh_cn")];
        let error = validate_video_prompt_task_target(&rows, &identity, false).unwrap_err();
        assert!(error.to_string().contains("已存在"));

        let mut old_contract = test_prompt(2, 1, "2026-01-01 00:00:00", "zh_cn");
        old_contract.metadata_json.as_mut().unwrap()["contract_version"] = json!("old_contract");
        let mut old_plan = test_prompt(3, 1, "2026-01-01 00:00:00", "zh_cn");
        old_plan.metadata_json.as_mut().unwrap()["video_execution_identity"]["segment_plan_prompt_id"] =
            json!(41);
        let mut other_language = test_prompt(4, 1, "2026-01-01 00:00:00", "en");
        other_language.metadata_json.as_mut().unwrap()["video_execution_identity"]["prompt_output_language"] =
            json!("en");
        let mut incomplete_identity = test_prompt(5, 1, "2026-01-01 00:00:00", "zh_cn");
        incomplete_identity.metadata_json.as_mut().unwrap()["video_execution_identity"]
            .as_object_mut()
            .unwrap()
            .remove("frame_set_sha256");
        for stale in [old_contract, old_plan, other_language, incomplete_identity] {
            assert!(validate_video_prompt_task_target(&[stale], &identity, false).is_ok());
        }

        let mut renumbered = test_prompt(6, 1, "2026-01-01 00:00:00", "zh_cn");
        let metadata = renumbered.metadata_json.as_mut().unwrap();
        metadata["video_execution_identity"]["source_shot_id"] = json!("shot-2");
        metadata["video_execution_identity"]["segment_sha256"] = json!("segment-2");
        metadata["video_execution_identity"]["segment_recipe_fingerprint"] = json!("recipe-2");
        assert!(
            validate_video_prompt_task_target(&[renumbered], &test_identity(2), false).is_err()
        );
    }

    #[test]
    fn video_prompt_commit_requires_a_new_or_updated_row() {
        let before = vec![test_prompt(1, 1, "2026-01-01 00:00:00", "zh_cn")];
        assert_eq!(
            committed_video_prompt_shot(&before, &before, &test_identity(1)),
            None
        );

        let updated = vec![test_prompt(1, 1, "2026-01-01 00:00:01", "zh_cn")];
        assert_eq!(
            committed_video_prompt_shot(&before, &updated, &test_identity(1)),
            Some(1)
        );

        let added = vec![
            test_prompt(1, 1, "2026-01-01 00:00:00", "zh_cn"),
            test_prompt(2, 2, "2026-01-01 00:00:01", "zh_cn"),
        ];
        assert_eq!(
            committed_video_prompt_shot(&before, &added, &test_identity(2)),
            Some(2)
        );
    }

    fn test_identity(segment_number: i32) -> VideoPromptTargetIdentity {
        let mut execution_identity = test_base_execution_identity();
        let object = execution_identity.as_object_mut().unwrap();
        object.insert(
            "source_shot_id".to_string(),
            json!(format!("shot-{segment_number}")),
        );
        object.insert("segment_index_in_shot".to_string(), json!(1));
        object.insert(
            "segment_sha256".to_string(),
            json!(format!("segment-{segment_number}")),
        );
        object.insert("segment_plan_prompt_id".to_string(), json!(42));
        object.insert("segment_number".to_string(), json!(segment_number));
        object.insert(
            "segment_recipe_fingerprint".to_string(),
            json!(format!("recipe-{segment_number}")),
        );
        VideoPromptTargetIdentity {
            segment_number,
            workflow_mode: "chat_video".to_string(),
            prompt_output_language: PromptOutputLanguage::ZhCn,
            execution_identity,
        }
    }

    fn test_base_execution_identity() -> Value {
        json!({
            "contract_epoch": a4n_core::video_execution::VIDEO_EXECUTION_CONTRACT_EPOCH,
            "shooting_script_identity": {},
            "projection_sha256": "projection",
            "segment_plan_sha256": "segment-plan",
            "unit_plan_sha256": "unit-plan",
            "source_content_sha256": "source",
            "style_sha256": "style",
            "duration_secs": 6,
            "aspect_ratio": "16:9",
            "segment_identities": [
                {
                    "source_shot_id": "shot-1",
                    "segment_index_in_shot": 1,
                    "segment_sha256": "segment-1"
                },
                {
                    "source_shot_id": "shot-2",
                    "segment_index_in_shot": 1,
                    "segment_sha256": "segment-2"
                }
            ]
        })
    }

    fn test_prompt(
        id: i64,
        shot_number: i32,
        updated_at: &str,
        language: &str,
    ) -> ChapterVideoPromptSummary {
        let mut execution_identity = test_base_execution_identity();
        let identity = execution_identity.as_object_mut().unwrap();
        identity.insert(
            "source_shot_id".to_string(),
            json!(format!("shot-{shot_number}")),
        );
        identity.insert("segment_index_in_shot".to_string(), json!(1));
        identity.insert(
            "segment_sha256".to_string(),
            json!(format!("segment-{shot_number}")),
        );
        identity.insert("segment_plan_prompt_id".to_string(), json!(42));
        identity.insert("segment_number".to_string(), json!(shot_number));
        identity.insert(
            "segment_recipe_fingerprint".to_string(),
            json!(format!("recipe-{shot_number}")),
        );
        identity.insert("frame_set_sha256".to_string(), json!("frames"));
        identity.insert("prompt_output_language".to_string(), json!(language));
        identity.insert("generation_asset_fingerprint".to_string(), json!({}));
        ChapterVideoPromptSummary {
            id,
            project_id: 1,
            chapter_id: 1,
            style: Some("电影感".to_string()),
            prompt_type: "video_prompt".to_string(),
            workflow_mode: "chat_video".to_string(),
            shot_number,
            title: None,
            scene_summary: None,
            visual_prompt: "画面".to_string(),
            motion_prompt: None,
            negative_prompt: None,
            duration_secs: 6,
            aspect_ratio: Some("16:9".to_string()),
            reference_image_ids_json: Some(json!([])),
            status: "pending".to_string(),
            generated_video_id: None,
            metadata_json: Some(json!({
                "contract_version": crate::video::CHAT_VIDEO_CONTRACT_VERSION,
                "source_prompt_type": "frame_segment_plan",
                "prompt_output_language": language,
                "video_execution_identity": execution_identity,
            })),
            created_at: "2026-01-01 00:00:00".to_string(),
            updated_at: updated_at.to_string(),
        }
    }
}
