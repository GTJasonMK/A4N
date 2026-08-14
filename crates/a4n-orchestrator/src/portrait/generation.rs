use a4n_api_contract::{
    generation_tasks::GenerationTaskHandle,
    portrait::{PortraitGenerationOperation, StartPortraitGenerationInput},
};
use a4n_core::error::AppError;
use a4n_domain::progress::ProgressPhaseKind;
use a4n_service_protocol::{
    BatchCommitPolicy, BatchSummary, GenerationArtifactRef, GenerationError, GenerationResultRef,
};
use sha2::{Digest, Sha256};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::{
    OrchestratorContext,
    collaboration::ProjectAccessRole,
    generation::{
        GenerationProgressUpdate,
        artifacts::{GenerationArtifactCoordinator, PreparedArtifactEvent},
        generation_public_error, publish_step_progress,
    },
    portrait::{
        GenerateAgeVariantInput, GenerateMissingPortraitsInput, GeneratePortraitInput,
        PortraitBatchItem, PortraitBatchOutcome, PortraitGenerationHostPort, PortraitProgress,
        PortraitSummary, RegeneratePortraitInput, validate_reference_image_ids,
    },
};
use a4n_prompt::rendering::PromptRenderer;

struct PortraitGenerationResult {
    summary: String,
    result_ref: GenerationResultRef,
    artifacts: Vec<PreparedArtifactEvent>,
    batch_summary: Option<BatchSummary>,
    failure: Option<GenerationError>,
}

impl OrchestratorContext {
    pub async fn validate_portrait_generation_start(
        &self,
        user_id: i64,
        input: &StartPortraitGenerationInput,
    ) -> Result<(), AppError> {
        if input.project_id <= 0 {
            return Err(AppError::InvalidArgument(
                "project_id 必须大于 0".to_string(),
            ));
        }
        self.verify_project_permission(input.project_id, user_id, ProjectAccessRole::Editor)
            .await?;

        match &input.operation {
            PortraitGenerationOperation::Generate {
                character_name,
                expressions,
                reference_image_ids,
                ..
            } => {
                if character_name.trim().is_empty() {
                    return Err(AppError::InvalidArgument(
                        "character_name 不能为空".to_string(),
                    ));
                }
                validate_reference_image_ids(reference_image_ids)
                    .map_err(AppError::InvalidArgument)?;
                if expressions
                    .as_ref()
                    .is_some_and(|values| values.iter().any(|value| value.trim().is_empty()))
                {
                    return Err(AppError::InvalidArgument(
                        "expressions 不能包含空字符串".to_string(),
                    ));
                }
            }
            PortraitGenerationOperation::Regenerate {
                portrait_id,
                reference_image_ids,
            } => {
                self.ensure_portrait_project(input.project_id, *portrait_id)
                    .await?;
                validate_reference_image_ids(reference_image_ids)
                    .map_err(AppError::InvalidArgument)?;
            }
            PortraitGenerationOperation::GenerateAgeVariant {
                portrait_id,
                age,
                reference_image_ids,
            } => {
                self.ensure_portrait_project(input.project_id, *portrait_id)
                    .await?;
                if age.trim().is_empty() {
                    return Err(AppError::InvalidArgument("age 不能为空".to_string()));
                }
                validate_reference_image_ids(reference_image_ids)
                    .map_err(AppError::InvalidArgument)?;
            }
            PortraitGenerationOperation::GenerateMissing => {
                if self
                    .missing_portrait_profiles(input.project_id)
                    .await?
                    .is_empty()
                {
                    return Err(AppError::InvalidArgument(
                        "当前项目没有缺失立绘，无需生成".to_string(),
                    ));
                }
            }
        }
        Ok(())
    }

    async fn ensure_portrait_project(
        &self,
        project_id: i64,
        portrait_id: i64,
    ) -> Result<(), AppError> {
        if portrait_id <= 0 {
            return Err(AppError::InvalidArgument(
                "portrait_id 必须大于 0".to_string(),
            ));
        }
        let portrait = self
            .character_portrait_by_id(portrait_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("立绘不存在: {portrait_id}")))?;
        if portrait.project_id != project_id {
            return Err(AppError::NotFound(format!(
                "项目 {project_id} 不存在立绘 {portrait_id}"
            )));
        }
        Ok(())
    }

    pub async fn run_portrait_generation_task(
        &self,
        input: StartPortraitGenerationInput,
        handle: GenerationTaskHandle,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn PortraitGenerationHostPort,
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
        let operation_key = portrait_operation_key(&input.operation);
        let publisher = execution.publisher.clone();
        let checkpoint_writer = execution.checkpoint_writer.clone();
        let llm_stream = crate::portrait::llm_stream::PortraitLlmStreamContext {
            publisher: publisher.clone(),
            session: session.clone(),
            checkpoint_writer: checkpoint_writer.clone(),
            cancellation: cancel_token.clone(),
        };
        let (progress_sender, progress_receiver) = mpsc::unbounded_channel();
        let progress_worker = tokio::spawn(publish_step_progress(
            progress_receiver,
            publisher,
            checkpoint_writer,
            session.clone(),
            operation_key,
            operation_key,
            ProgressPhaseKind::AssetGeneration,
            0,
        ));
        let mut reporter = move |current: u32, total: u32, message: &str| {
            progress_sender
                .send(GenerationProgressUpdate {
                    current,
                    total,
                    message: message.to_string(),
                })
                .map_err(|_| AppError::InvalidState {
                    current: "portrait_progress_publisher_closed".to_string(),
                    operation: "发布人物立绘 generation 进度".to_string(),
                })
        };
        let application_result =
            crate::portrait::llm_stream::with_portrait_llm_stream(llm_stream, || async {
                let progress: PortraitProgress<'_> = &mut reporter;
                self.execute_portrait_generation_operation(
                    input,
                    prompt_renderer,
                    host_port,
                    progress,
                    cancel_token,
                )
                .await
            })
            .await;
        drop(reporter);
        let progress_result = match progress_worker.await {
            Ok(result) => result,
            Err(error) => Err(generation_public_error(
                "generation_event_invalid",
                operation_key,
                format!("人物立绘进度 publisher 异常退出: {error}"),
            )),
        };

        let task_result = match (application_result, progress_result) {
            (Ok(result), Ok(())) => result,
            (Err(error), Ok(())) => {
                return Err(execution
                    .fail(portrait_app_error_to_generation_error(error, operation_key))
                    .await);
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
                        format!("{}; 进度发布失败: {}", error, progress_error.message),
                    ))
                    .await);
            }
        };

        let PortraitGenerationResult {
            summary,
            result_ref,
            artifacts,
            batch_summary,
            failure,
        } = task_result;
        if let Some(error) = failure {
            return Err(execution
                .fail_with_batch_summary(error, batch_summary, artifacts)
                .await);
        }

        execution
            .complete_with_result(operation_key, result_ref, summary, artifacts, batch_summary)
            .await
    }

    async fn execute_portrait_generation_operation(
        &self,
        input: StartPortraitGenerationInput,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn PortraitGenerationHostPort,
        progress: PortraitProgress<'_>,
        cancel_token: CancellationToken,
    ) -> Result<PortraitGenerationResult, AppError> {
        let project_id = input.project_id;
        match input.operation {
            PortraitGenerationOperation::Generate {
                character_name,
                style,
                expressions,
                pose,
                reference_image_ids,
            } => {
                let summary = self
                    .generate_portrait_validated(
                        GeneratePortraitInput {
                            project_id,
                            character_name,
                            style,
                            expressions,
                            pose,
                            reference_image_ids,
                        },
                        prompt_renderer,
                        host_port,
                        Some(progress),
                        cancel_token,
                    )
                    .await?;
                let result_ref = portrait_result_ref("portrait", summary.id, &summary)?;
                Ok(PortraitGenerationResult {
                    summary: "人物三视图拆解生成完成".to_string(),
                    result_ref,
                    artifacts: vec![portrait_artifact_event(
                        host_port, &summary, None, None, None,
                    )?],
                    batch_summary: None,
                    failure: None,
                })
            }
            PortraitGenerationOperation::Regenerate {
                portrait_id,
                reference_image_ids,
            } => {
                let summary = self
                    .regenerate_portrait_validated(
                        RegeneratePortraitInput {
                            portrait_id,
                            reference_image_ids,
                        },
                        host_port,
                        Some(progress),
                        cancel_token,
                    )
                    .await?;
                let result_ref = portrait_result_ref("portrait", summary.id, &summary)?;
                Ok(PortraitGenerationResult {
                    summary: "立绘重绘完成".to_string(),
                    result_ref,
                    artifacts: vec![portrait_artifact_event(
                        host_port, &summary, None, None, None,
                    )?],
                    batch_summary: None,
                    failure: None,
                })
            }
            PortraitGenerationOperation::GenerateAgeVariant {
                portrait_id,
                age,
                reference_image_ids,
            } => {
                let summary = self
                    .generate_age_variant_validated(
                        GenerateAgeVariantInput {
                            portrait_id,
                            age,
                            reference_image_ids,
                        },
                        host_port,
                        Some(progress),
                        cancel_token,
                    )
                    .await?;
                let result_ref = portrait_result_ref("portrait", summary.id, &summary)?;
                Ok(PortraitGenerationResult {
                    summary: "年龄变体生成完成".to_string(),
                    result_ref,
                    artifacts: vec![portrait_artifact_event(
                        host_port, &summary, None, None, None,
                    )?],
                    batch_summary: None,
                    failure: None,
                })
            }
            PortraitGenerationOperation::GenerateMissing => {
                let outcome = self
                    .generate_missing_portraits_validated(
                        GenerateMissingPortraitsInput { project_id },
                        host_port,
                        Some(progress),
                        cancel_token,
                    )
                    .await?;
                let batch_summary = portrait_batch_summary(&outcome);
                let artifacts = outcome
                    .items
                    .iter()
                    .map(|item| {
                        portrait_artifact_event(
                            host_port,
                            &item.summary,
                            Some(item.item_key.clone()),
                            Some(item.item_index),
                            Some(outcome.total),
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let result_ref = portrait_batch_result_ref(project_id, &outcome.items)?;
                let failure = if outcome.interrupted {
                    Some(generation_public_error(
                        "generation_interrupted",
                        "portrait_generate_missing",
                        outcome
                            .first_error
                            .as_deref()
                            .unwrap_or("缺失立绘生成已停止"),
                    ))
                } else if outcome.succeeded() == 0 {
                    Some(generation_public_error(
                        "provider_job_failed",
                        "portrait_generate_missing",
                        outcome
                            .first_error
                            .as_deref()
                            .unwrap_or("缺失立绘全部生成失败"),
                    ))
                } else {
                    None
                };
                Ok(PortraitGenerationResult {
                    summary: format!(
                        "缺失立绘生成完成，成功 {} 张，失败 {} 张",
                        outcome.succeeded(),
                        outcome.failed()
                    ),
                    result_ref,
                    artifacts,
                    batch_summary: Some(batch_summary),
                    failure,
                })
            }
        }
    }
}

fn portrait_operation_key(operation: &PortraitGenerationOperation) -> &'static str {
    match operation {
        PortraitGenerationOperation::Generate { .. } => "portrait_generate",
        PortraitGenerationOperation::Regenerate { .. } => "portrait_regenerate",
        PortraitGenerationOperation::GenerateAgeVariant { .. } => "portrait_age_variant",
        PortraitGenerationOperation::GenerateMissing => "portrait_generate_missing",
    }
}

fn portrait_result_ref(
    kind: &str,
    id: i64,
    summary: &PortraitSummary,
) -> Result<GenerationResultRef, AppError> {
    let bytes = serde_json::to_vec(&[
        serde_json::json!(summary.id),
        serde_json::json!(summary.profile_id),
        serde_json::json!(summary.project_id),
        serde_json::json!(&summary.portrait_type),
        serde_json::json!(&summary.image_path),
        serde_json::json!(&summary.prompt),
        serde_json::json!(&summary.expression),
        serde_json::json!(&summary.pose),
        serde_json::json!(summary.is_primary),
        serde_json::json!(&summary.created_at),
    ])?;
    Ok(result_ref_from_bytes(kind, id, bytes))
}

fn portrait_batch_result_ref(
    project_id: i64,
    items: &[PortraitBatchItem],
) -> Result<GenerationResultRef, AppError> {
    let mut ids = items.iter().map(|item| item.summary.id).collect::<Vec<_>>();
    ids.sort_unstable();
    Ok(result_ref_from_bytes(
        "portrait_batch",
        project_id,
        serde_json::to_vec(&ids)?,
    ))
}

fn result_ref_from_bytes(kind: &str, id: i64, bytes: Vec<u8>) -> GenerationResultRef {
    GenerationResultRef {
        kind: kind.to_string(),
        id: id.to_string(),
        revision: format!("{:x}", Sha256::digest(bytes)),
    }
}

fn portrait_batch_summary(outcome: &PortraitBatchOutcome) -> BatchSummary {
    let mut failed_item_keys = outcome.failed_item_keys.clone();
    failed_item_keys.sort_unstable();
    failed_item_keys.dedup();
    let failed = outcome.failed();
    let failed_item_keys_truncated = failed_item_keys.len() > 256;
    failed_item_keys.truncate(256);
    BatchSummary {
        policy: BatchCommitPolicy::BestEffort,
        total: outcome.total,
        succeeded: outcome.succeeded(),
        failed,
        cancelled: outcome.cancelled,
        skipped: outcome.skipped,
        failed_item_keys,
        failed_item_keys_truncated,
    }
}

fn portrait_artifact_event(
    host_port: &dyn PortraitGenerationHostPort,
    summary: &PortraitSummary,
    item_key: Option<String>,
    item_index: Option<u32>,
    item_total: Option<u32>,
) -> Result<PreparedArtifactEvent, AppError> {
    let file = host_port.inspect_portrait_file(&summary.image_path, "正式立绘图片")?;
    Ok(GenerationArtifactCoordinator::available(
        GenerationArtifactRef {
            artifact_id: format!("portrait:{}", summary.id),
            artifact_kind: "portrait_image".to_string(),
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

fn portrait_app_error_to_generation_error(error: AppError, stage_code: &str) -> GenerationError {
    let (code, stage) = match error {
        AppError::Cancelled => ("generation_interrupted", "portrait_cancel"),
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
            ("generation_persist_failed", "portrait_persist")
        }
        _ => ("generation_persist_failed", stage_code),
    };
    generation_public_error(code, stage, error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn portrait_operation_keys_are_stable_and_unique() {
        let operations = [
            PortraitGenerationOperation::Generate {
                character_name: "A".to_string(),
                style: None,
                expressions: None,
                pose: None,
                reference_image_ids: Vec::new(),
            },
            PortraitGenerationOperation::Regenerate {
                portrait_id: 1,
                reference_image_ids: Vec::new(),
            },
            PortraitGenerationOperation::GenerateAgeVariant {
                portrait_id: 1,
                age: "少年".to_string(),
                reference_image_ids: Vec::new(),
            },
            PortraitGenerationOperation::GenerateMissing,
        ];
        let keys = operations
            .iter()
            .map(portrait_operation_key)
            .collect::<HashSet<_>>();
        assert_eq!(keys.len(), operations.len());
        assert!(keys.iter().all(|key| key.starts_with("portrait_")));
    }
}
