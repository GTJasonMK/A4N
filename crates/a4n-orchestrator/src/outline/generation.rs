use std::collections::{HashMap, HashSet};

use a4n_api_contract::{
    generation_tasks::{GenerationTaskHandle, GenerationTaskStartError},
    outline::{
        ApplyBeatMappingInput, BatchGeneratePartOutlinesInput, ContinuePartOutlinesInput,
        GenerateChapterOutlinesByCountInput, GeneratePartChapterOutlinesInput, GeneratePartsInput,
        OutlineDeleteTarget, OutlineGenerationOperation, RegenerateChapterOutlineInput,
        RegenerateLastPartInput, RegeneratePartOutlineInput, RegeneratePartsInput,
        StartOutlineGenerationInput,
    },
};
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatRequest;
use a4n_progress::ProgressPhaseKind;
use a4n_service_protocol::{
    GenerationError, GenerationFinishReason, StreamTextHash, TextStreamSnapshot,
};
use futures::future::BoxFuture;
use serde_json::{Map, Value};
use tokio::sync::mpsc;

use a4n_outline_contract::OutlineGenerationApplicationPort;

use crate::{
    OrchestratorContext,
    collaboration::ProjectAccessRole,
    generation::{
        GenerationProgressUpdate, generation_public_error, publish_step_progress,
        terminal::CompletionDescriptor,
    },
    outline::{
        OutlineBatchPartGenerationPreviewApplicationPort, OutlineBeatMappingPreviewApplicationPort,
        OutlineChapterCountGenerationPreviewApplicationPort,
        OutlineChapterRegenerationPreviewApplicationPort,
        OutlinePartChapterGenerationPreviewApplicationPort,
        OutlinePartGenerationPreviewApplicationPort, OutlinePartMutationPreviewApplicationPort,
        OutlineRequestPreviewStep,
    },
};

use super::{OutlineLlmStreamContext, ProgressReporter, with_outline_llm_stream};

impl OrchestratorContext {
    pub(crate) async fn validate_outline_generation_start(
        &self,
        user_id: i64,
        input: &StartOutlineGenerationInput,
    ) -> Result<(), AppError> {
        if input.project_id <= 0 {
            return Err(AppError::InvalidArgument(
                "project_id 必须大于 0".to_string(),
            ));
        }
        self.verify_project_permission(input.project_id, user_id, ProjectAccessRole::Editor)
            .await?;

        match &input.operation {
            OutlineGenerationOperation::GenerateParts {
                part_count,
                chapters_per_part,
                ..
            } => {
                require_optional_positive(*part_count, "part_count")?;
                require_optional_positive(*chapters_per_part, "chapters_per_part")?;
            }
            OutlineGenerationOperation::RegenerateParts {
                expected_parts,
                chapters_per_part,
                ..
            } => {
                validate_expected_part_targets(expected_parts)?;
                require_optional_positive(*chapters_per_part, "chapters_per_part")?;
            }
            OutlineGenerationOperation::RegenerateLastPart { .. } => {}
            OutlineGenerationOperation::ContinuePartOutlines {
                part_number,
                start_chapter,
                end_chapter,
                ..
            } => {
                require_positive(*part_number, "part_number")?;
                require_positive(*start_chapter, "start_chapter")?;
                if end_chapter < start_chapter {
                    return Err(AppError::InvalidArgument(
                        "end_chapter 不能小于 start_chapter".to_string(),
                    ));
                }
            }
            OutlineGenerationOperation::RegeneratePartOutline { part_id, .. }
            | OutlineGenerationOperation::GeneratePartChapterOutlines { part_id }
            | OutlineGenerationOperation::ApplyBeatMapping { part_id, .. } => {
                self.ensure_outline_part_project(input.project_id, *part_id)
                    .await?;
            }
            OutlineGenerationOperation::BatchGeneratePartOutlines { part_ids } => {
                if part_ids.is_empty() || part_ids.len() > 1024 {
                    return Err(AppError::InvalidArgument(
                        "part_ids 数量必须在 1..=1024".to_string(),
                    ));
                }
                let mut parts_by_id = self
                    .get_parts(part_ids)
                    .await?
                    .into_iter()
                    .map(|part| (part.id, part))
                    .collect::<HashMap<_, _>>();
                let mut seen = HashSet::with_capacity(part_ids.len());
                for part_id in part_ids {
                    if !seen.insert(*part_id) {
                        return Err(AppError::InvalidArgument(format!(
                            "part_ids 包含重复分部: {part_id}"
                        )));
                    }
                    require_positive_i64(*part_id, "part_id")?;
                    let part = parts_by_id
                        .remove(part_id)
                        .ok_or_else(|| AppError::NotFound(format!("分部大纲不存在: {part_id}")))?;
                    if part.project_id != input.project_id {
                        return Err(AppError::NotFound(format!(
                            "项目 {} 不存在分部大纲 {}",
                            input.project_id, part_id
                        )));
                    }
                }
            }
            OutlineGenerationOperation::GenerateChapterOutlinesByCount {
                start_chapter,
                count,
            } => {
                require_positive(*start_chapter, "start_chapter")?;
                require_positive(*count, "count")?;
            }
            OutlineGenerationOperation::RegenerateChapterOutline { outline_id, .. } => {
                require_positive_i64(*outline_id, "outline_id")?;
                let belongs_to_project = self
                    .chapter_outlines_for_project(input.project_id)
                    .await?
                    .into_iter()
                    .any(|outline| outline.id == *outline_id);
                if !belongs_to_project {
                    return Err(AppError::NotFound(format!(
                        "项目 {} 不存在章节大纲 {}",
                        input.project_id, outline_id
                    )));
                }
            }
        }

        if let OutlineGenerationOperation::ApplyBeatMapping { beat_sheet, .. } = &input.operation
            && beat_sheet.trim().is_empty()
        {
            return Err(AppError::InvalidArgument("beat_sheet 不能为空".to_string()));
        }
        Ok(())
    }

    pub async fn run_outline_generation_task(
        &self,
        input: StartOutlineGenerationInput,
        handle: GenerationTaskHandle,
    ) -> Result<(), String> {
        let Some(execution) = self
            .acquire_generation_task_execution(&handle)
            .await
            .map_err(|error| error.to_string())?
        else {
            return Ok(());
        };
        let session = execution.session.clone();
        let cancel_token = execution.permit.cancellation_token();
        let operation_key = outline_operation_key(&input.operation);
        let publisher = execution.publisher.clone();
        let checkpoint_writer = execution.checkpoint_writer.clone();
        let llm_stream = OutlineLlmStreamContext {
            publisher: publisher.clone(),
            session: session.clone(),
            checkpoint_writer: checkpoint_writer.clone(),
            cancellation: cancel_token.clone(),
        };
        let (progress_sender, progress_receiver) = mpsc::unbounded_channel();
        let progress_session = session.clone();
        let progress_worker = tokio::spawn(publish_step_progress(
            progress_receiver,
            publisher,
            checkpoint_writer,
            progress_session,
            operation_key,
            "outline_checkpoint",
            ProgressPhaseKind::ModelWait,
            0,
        ));
        let reporter_cancel = cancel_token.clone();
        let reporter = move |current: u32, total: u32, message: &str| {
            if reporter_cancel.is_cancelled() {
                return Err(AppError::Cancelled);
            }
            progress_sender
                .send(GenerationProgressUpdate {
                    current,
                    total,
                    message: message.to_string(),
                })
                .map_err(|_| AppError::InvalidState {
                    current: "outline_progress_publisher_closed".to_string(),
                    operation: "发布大纲 generation 进度".to_string(),
                })
        };
        let progress: ProgressReporter<'_> = Some(&reporter);
        let application_result = with_outline_llm_stream(llm_stream, || async {
            self.execute_outline_generation_operation(input, &handle.task_id, progress)
                .await
        })
        .await;
        drop(reporter);
        let progress_result = match progress_worker.await {
            Ok(result) => result,
            Err(error) => Err(generation_public_error(
                "generation_event_invalid",
                operation_key,
                format!("大纲进度 publisher 异常退出: {error}"),
            )),
        };

        let summary = match (application_result, progress_result) {
            (Ok(summary), Ok(())) => summary,
            (Err(error), Ok(())) => {
                return Err(execution
                    .fail(outline_app_error_to_generation_error(error, operation_key))
                    .await);
            }
            (Ok(_), Err(error)) => {
                return Err(execution.fail(error).await);
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

        let snapshot = session.snapshot();
        let final_text_hashes = snapshot_text_hashes(&snapshot.streams);
        let final_usage = snapshot.usage;
        let completion = execution
            .terminal
            .complete_with(&session, move |_tx| {
                Box::pin(async move {
                    Ok(CompletionDescriptor {
                        result_ref: None,
                        final_text_hashes,
                        final_usage,
                        finish_reason: Some(GenerationFinishReason::Stop),
                        summary: Some(summary.to_string()),
                        batch_summary: None,
                    })
                }) as BoxFuture<'_, Result<CompletionDescriptor, AppError>>
            })
            .await;
        if let Err(error) = completion {
            return Err(execution
                .fail(generation_public_error(
                    error.code(),
                    "outline_terminal",
                    error,
                ))
                .await);
        }
        execution.finish();
        Ok(())
    }

    async fn ensure_outline_part_project(
        &self,
        project_id: i64,
        part_id: i64,
    ) -> Result<(), AppError> {
        require_positive_i64(part_id, "part_id")?;
        let part = self.get_part(part_id).await?;
        if part.project_id == project_id {
            Ok(())
        } else {
            Err(AppError::NotFound(format!(
                "项目 {project_id} 不存在分部大纲 {part_id}"
            )))
        }
    }

    async fn execute_outline_generation_operation(
        &self,
        input: StartOutlineGenerationInput,
        task_id: &str,
        progress: ProgressReporter<'_>,
    ) -> Result<&'static str, AppError> {
        let project_id = input.project_id;
        let task_id = Some(task_id.to_string());
        match input.operation {
            OutlineGenerationOperation::GenerateParts {
                part_count,
                chapters_per_part,
                variables,
            } => {
                self.generate_parts_validated(
                    GeneratePartsInput {
                        project_id,
                        part_count,
                        chapters_per_part,
                        task_id,
                        variables: variables_value(variables),
                    },
                    progress,
                )
                .await?;
                Ok("分部大纲已生成")
            }
            OutlineGenerationOperation::RegenerateParts {
                expected_parts,
                chapters_per_part,
                variables,
            } => {
                self.regenerate_parts_validated(
                    RegeneratePartsInput {
                        project_id,
                        expected_parts,
                        chapters_per_part,
                        task_id,
                        variables: variables_value(variables),
                    },
                    progress,
                )
                .await?;
                Ok("所有分部大纲已重新生成")
            }
            OutlineGenerationOperation::RegenerateLastPart { variables } => {
                self.regenerate_last_part_validated(
                    RegenerateLastPartInput {
                        project_id,
                        task_id,
                        variables: variables_value(variables),
                    },
                    progress,
                )
                .await?;
                Ok("最后一部大纲已重新生成")
            }
            OutlineGenerationOperation::ContinuePartOutlines {
                part_number,
                start_chapter,
                end_chapter,
                variables,
            } => {
                self.continue_part_outlines_validated(
                    ContinuePartOutlinesInput {
                        project_id,
                        part_number,
                        start_chapter,
                        end_chapter,
                        task_id,
                        variables: variables_value(variables),
                    },
                    progress,
                )
                .await?;
                Ok("分部续作完成")
            }
            OutlineGenerationOperation::RegeneratePartOutline { part_id, variables } => {
                self.regenerate_part_outline_validated(
                    RegeneratePartOutlineInput {
                        project_id,
                        part_id,
                        task_id,
                        variables: variables_value(variables),
                    },
                    progress,
                )
                .await?;
                Ok("指定分部大纲已重新生成")
            }
            OutlineGenerationOperation::GeneratePartChapterOutlines { part_id } => {
                self.generate_part_chapter_outlines_validated(
                    GeneratePartChapterOutlinesInput {
                        project_id,
                        part_id,
                        task_id,
                    },
                    progress,
                )
                .await?;
                Ok("本部章节大纲已生成")
            }
            OutlineGenerationOperation::BatchGeneratePartOutlines { part_ids } => {
                self.batch_generate_part_outlines_validated(
                    BatchGeneratePartOutlinesInput {
                        project_id,
                        part_ids,
                        task_id,
                    },
                    progress,
                )
                .await?;
                Ok("批量章节大纲已生成")
            }
            OutlineGenerationOperation::GenerateChapterOutlinesByCount {
                start_chapter,
                count,
            } => {
                self.generate_chapter_outlines_by_count_validated(
                    GenerateChapterOutlinesByCountInput {
                        project_id,
                        start_chapter,
                        count,
                        task_id,
                    },
                    progress,
                )
                .await?;
                Ok("章节大纲已生成")
            }
            OutlineGenerationOperation::RegenerateChapterOutline {
                outline_id,
                variables,
            } => {
                self.regenerate_chapter_outline_validated(
                    RegenerateChapterOutlineInput {
                        project_id,
                        outline_id,
                        task_id,
                        variables: variables_value(variables),
                    },
                    progress,
                )
                .await?;
                Ok("章节大纲已重新生成")
            }
            OutlineGenerationOperation::ApplyBeatMapping {
                part_id,
                beat_sheet,
            } => {
                self.apply_beat_mapping_validated(
                    ApplyBeatMappingInput {
                        project_id,
                        part_id,
                        beat_sheet,
                        task_id,
                    },
                    progress,
                )
                .await?;
                Ok("节拍映射已应用")
            }
        }
    }
}

fn outline_app_error_to_generation_error(error: AppError, stage_code: &str) -> GenerationError {
    let (code, stage) = match error {
        AppError::Cancelled => ("generation_interrupted", "outline_cancel"),
        AppError::LlmError(_) | AppError::Timeout(_) | AppError::ConfigError(_) => {
            ("provider_job_failed", stage_code)
        }
        AppError::Forbidden(_) | AppError::Unauthorized(_) | AppError::NotFound(_) => {
            ("generation_task_forbidden", stage_code)
        }
        AppError::InvalidArgument(_) | AppError::InvalidState { .. } => {
            ("generation_request_invalid", stage_code)
        }
        AppError::Database(_) | AppError::Serialization(_) => {
            ("generation_persist_failed", "outline_persist")
        }
        _ => ("generation_persist_failed", stage_code),
    };
    generation_public_error(code, stage, error)
}

#[async_trait::async_trait]
impl OutlineGenerationApplicationPort for OrchestratorContext {
    async fn start_task(
        &self,
        user_id: i64,
        input: &StartOutlineGenerationInput,
    ) -> Result<a4n_api_contract::generation_tasks::GenerationTaskStart, GenerationTaskStartError>
    {
        OrchestratorContext::start_outline_generation_task(self, user_id, input)
            .await
            .map_err(crate::GenerationTaskAccessError::into_start_error)
    }

    async fn run_task(
        &self,
        input: StartOutlineGenerationInput,
        handle: GenerationTaskHandle,
    ) -> Result<(), String> {
        OrchestratorContext::run_outline_generation_task(self, input, handle).await
    }
}

#[async_trait::async_trait]
impl OutlinePartGenerationPreviewApplicationPort for OrchestratorContext {
    async fn preview_generate_parts(
        &self,
        input: GeneratePartsInput,
    ) -> Result<Vec<OutlineRequestPreviewStep>, AppError> {
        self.preview_generate_parts_validated(input).await
    }

    async fn preview_regenerate_parts(
        &self,
        input: RegeneratePartsInput,
    ) -> Result<Vec<OutlineRequestPreviewStep>, AppError> {
        self.preview_regenerate_parts_validated(input).await
    }
}

#[async_trait::async_trait]
impl OutlinePartMutationPreviewApplicationPort for OrchestratorContext {
    async fn preview_regenerate_last_part(
        &self,
        input: RegenerateLastPartInput,
    ) -> Result<ChatRequest, AppError> {
        self.preview_regenerate_last_part_validated(input).await
    }

    async fn preview_continue_part_outlines(
        &self,
        input: ContinuePartOutlinesInput,
    ) -> Result<ChatRequest, AppError> {
        self.preview_continue_part_outlines_validated(input).await
    }

    async fn preview_regenerate_part_outline(
        &self,
        input: RegeneratePartOutlineInput,
    ) -> Result<ChatRequest, AppError> {
        self.preview_regenerate_part_outline_validated(input).await
    }
}

#[async_trait::async_trait]
impl OutlinePartChapterGenerationPreviewApplicationPort for OrchestratorContext {
    async fn preview_generate_part_chapter_outlines(
        &self,
        input: GeneratePartChapterOutlinesInput,
    ) -> Result<Vec<OutlineRequestPreviewStep>, AppError> {
        self.preview_generate_part_chapter_outlines_validated(input)
            .await
    }
}

#[async_trait::async_trait]
impl OutlineBatchPartGenerationPreviewApplicationPort for OrchestratorContext {
    async fn preview_batch_generate_part_outlines(
        &self,
        input: BatchGeneratePartOutlinesInput,
    ) -> Result<Vec<OutlineRequestPreviewStep>, AppError> {
        self.preview_batch_generate_part_outlines_validated(input)
            .await
    }
}

#[async_trait::async_trait]
impl OutlineChapterCountGenerationPreviewApplicationPort for OrchestratorContext {
    async fn preview_generate_chapter_outlines_by_count(
        &self,
        input: GenerateChapterOutlinesByCountInput,
    ) -> Result<Vec<OutlineRequestPreviewStep>, AppError> {
        self.preview_generate_chapter_outlines_by_count_validated(input)
            .await
    }
}

#[async_trait::async_trait]
impl OutlineChapterRegenerationPreviewApplicationPort for OrchestratorContext {
    async fn preview_regenerate_chapter_outline(
        &self,
        input: RegenerateChapterOutlineInput,
    ) -> Result<ChatRequest, AppError> {
        self.preview_regenerate_chapter_outline_validated(input)
            .await
    }
}

#[async_trait::async_trait]
impl OutlineBeatMappingPreviewApplicationPort for OrchestratorContext {
    async fn preview_apply_beat_mapping(
        &self,
        input: ApplyBeatMappingInput,
    ) -> Result<ChatRequest, AppError> {
        self.preview_apply_beat_mapping_validated(input).await
    }
}

fn outline_operation_key(operation: &OutlineGenerationOperation) -> &'static str {
    match operation {
        OutlineGenerationOperation::GenerateParts { .. } => "outline_generate_parts",
        OutlineGenerationOperation::RegenerateParts { .. } => "outline_regenerate_parts",
        OutlineGenerationOperation::RegenerateLastPart { .. } => "outline_regenerate_last_part",
        OutlineGenerationOperation::ContinuePartOutlines { .. } => "outline_continue_part",
        OutlineGenerationOperation::RegeneratePartOutline { .. } => "outline_regenerate_part",
        OutlineGenerationOperation::GeneratePartChapterOutlines { .. } => {
            "outline_generate_part_chapters"
        }
        OutlineGenerationOperation::BatchGeneratePartOutlines { .. } => {
            "outline_generate_batch_part_chapters"
        }
        OutlineGenerationOperation::GenerateChapterOutlinesByCount { .. } => {
            "outline_generate_chapters_by_count"
        }
        OutlineGenerationOperation::RegenerateChapterOutline { .. } => "outline_regenerate_chapter",
        OutlineGenerationOperation::ApplyBeatMapping { .. } => "outline_apply_beat_mapping",
    }
}

fn variables_value(variables: Option<std::collections::BTreeMap<String, Value>>) -> Option<Value> {
    variables.map(|variables| Value::Object(variables.into_iter().collect::<Map<_, _>>()))
}

fn snapshot_text_hashes(streams: &[TextStreamSnapshot]) -> Vec<StreamTextHash> {
    streams
        .iter()
        .map(|stream| StreamTextHash {
            stream_id: stream.stream_id.clone(),
            utf8_bytes: stream.next_offset,
            sha256: stream.sha256.clone(),
        })
        .collect()
}

fn require_optional_positive(value: Option<i32>, field: &str) -> Result<(), AppError> {
    if value.is_some_and(|value| value <= 0) {
        return Err(AppError::InvalidArgument(format!("{field} 必须大于 0")));
    }
    Ok(())
}

fn validate_expected_part_targets(targets: &[OutlineDeleteTarget]) -> Result<(), AppError> {
    if targets.is_empty() || targets.len() > 1024 {
        return Err(AppError::InvalidArgument(
            "expected_parts 数量必须在 1..=1024".to_string(),
        ));
    }
    let mut ids = HashSet::with_capacity(targets.len());
    for target in targets {
        if target.id <= 0 {
            return Err(AppError::InvalidArgument(
                "expected_parts.id 必须大于 0".to_string(),
            ));
        }
        let expected_updated_at = target.expected_updated_at.trim();
        if expected_updated_at.is_empty() || expected_updated_at.len() > 64 {
            return Err(AppError::InvalidArgument(
                "expected_parts.expected_updated_at 无效".to_string(),
            ));
        }
        if !ids.insert(target.id) {
            return Err(AppError::InvalidArgument(format!(
                "expected_parts 包含重复分部: {}",
                target.id
            )));
        }
    }
    Ok(())
}

fn require_positive(value: i32, field: &str) -> Result<(), AppError> {
    if value <= 0 {
        return Err(AppError::InvalidArgument(format!("{field} 必须大于 0")));
    }
    Ok(())
}

fn require_positive_i64(value: i64, field: &str) -> Result<(), AppError> {
    if value <= 0 {
        return Err(AppError::InvalidArgument(format!("{field} 必须大于 0")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outline_start_errors_use_stable_codes_and_retry_policy() {
        let invalid = crate::GenerationTaskAccessError::InvalidArgument("请求无效".to_string())
            .into_start_error();
        assert_eq!(invalid.code(), "generation_request_invalid");
        assert!(!invalid.retryable);

        let forbidden =
            crate::GenerationTaskAccessError::Forbidden("无权限".into()).into_start_error();
        assert_eq!(forbidden.code(), "generation_task_forbidden");
        assert!(!forbidden.retryable);
    }

    #[test]
    fn snapshot_text_hashes_cover_every_stream() {
        let streams = [
            TextStreamSnapshot {
                stream_id: "raw_json".to_string(),
                text: "{}".to_string(),
                next_offset: 2,
                sha256: "a".repeat(64),
            },
            TextStreamSnapshot {
                stream_id: "notes".to_string(),
                text: "ok".to_string(),
                next_offset: 2,
                sha256: "b".repeat(64),
            },
        ];

        let hashes = snapshot_text_hashes(&streams);

        assert_eq!(hashes.len(), streams.len());
        for (hash, stream) in hashes.iter().zip(&streams) {
            assert_eq!(hash.stream_id, stream.stream_id);
            assert_eq!(hash.utf8_bytes, stream.next_offset);
            assert_eq!(hash.sha256, stream.sha256);
        }
    }

    #[test]
    fn outline_generation_operations_have_stable_unique_stage_keys() {
        let operations = [
            OutlineGenerationOperation::GenerateParts {
                part_count: Some(1),
                chapters_per_part: Some(10),
                variables: None,
            },
            OutlineGenerationOperation::RegenerateParts {
                expected_parts: vec![OutlineDeleteTarget {
                    id: 1,
                    expected_updated_at: "2026-08-12 00:00:00".to_string(),
                }],
                chapters_per_part: None,
                variables: None,
            },
            OutlineGenerationOperation::RegenerateLastPart { variables: None },
            OutlineGenerationOperation::ContinuePartOutlines {
                part_number: 2,
                start_chapter: 11,
                end_chapter: 20,
                variables: None,
            },
            OutlineGenerationOperation::RegeneratePartOutline {
                part_id: 1,
                variables: None,
            },
            OutlineGenerationOperation::GeneratePartChapterOutlines { part_id: 1 },
            OutlineGenerationOperation::BatchGeneratePartOutlines { part_ids: vec![1] },
            OutlineGenerationOperation::GenerateChapterOutlinesByCount {
                start_chapter: 1,
                count: 10,
            },
            OutlineGenerationOperation::RegenerateChapterOutline {
                outline_id: 1,
                variables: None,
            },
            OutlineGenerationOperation::ApplyBeatMapping {
                part_id: 1,
                beat_sheet: "{}".to_string(),
            },
        ];
        let keys = operations
            .iter()
            .map(outline_operation_key)
            .collect::<HashSet<_>>();
        assert_eq!(keys.len(), operations.len());
        assert!(keys.iter().all(|key| key.starts_with("outline_")));
    }
}
