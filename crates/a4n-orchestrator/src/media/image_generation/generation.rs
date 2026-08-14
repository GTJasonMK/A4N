use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use a4n_api_contract::{
    generation_tasks::GenerationTaskHandle,
    image_generation::{ImageGenerationOperation, StartImageGenerationInput},
};
use a4n_core::error::AppError;
use a4n_domain::progress::ProgressPhaseKind;
use a4n_service_protocol::{
    BatchCommitPolicy, BatchSummary, GenerationArtifactRef, GenerationError, GenerationResultRef,
};
use sha2::{Digest, Sha256};
use tokio::sync::mpsc;

use crate::{
    OrchestratorContext,
    collaboration::ProjectAccessRole,
    generation::{
        GenerationProgressUpdate,
        artifacts::{GenerationArtifactCoordinator, PreparedArtifactEvent},
        generation_public_error, publish_step_progress,
    },
    media::{
        BatchImageGenerationInput, BatchImageGenerationItem, BatchImageGenerationOutcome,
        GeneratedImageSummary, ImageGenerationHostPort, ImageGenerationProgress,
        ObjectProfileImageInput, PageImageGenerationInput, SceneImageGenerationInput,
        SceneProfileImageInput,
    },
};

struct ImageGenerationResult {
    summary: String,
    result_ref: GenerationResultRef,
    artifacts: Vec<PreparedArtifactEvent>,
    batch_summary: Option<BatchSummary>,
    failure: Option<GenerationError>,
}

impl OrchestratorContext {
    pub async fn validate_image_generation_start(
        &self,
        user_id: i64,
        input: &StartImageGenerationInput,
    ) -> Result<(), AppError> {
        if input.project_id <= 0 {
            return Err(AppError::InvalidArgument(
                "project_id 必须大于 0".to_string(),
            ));
        }
        self.verify_project_permission(input.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        match &input.operation {
            ImageGenerationOperation::Scene { prompt, .. } => {
                if prompt.trim().is_empty() {
                    return Err(AppError::InvalidArgument("prompt 不能为空".to_string()));
                }
            }
            ImageGenerationOperation::Page {
                chapter_number,
                page_number,
            } => {
                if *chapter_number <= 0 || *page_number <= 0 {
                    return Err(AppError::InvalidArgument(
                        "chapter_number 和 page_number 必须大于 0".to_string(),
                    ));
                }
            }
            ImageGenerationOperation::Batch { requests } => {
                if requests.is_empty() {
                    return Err(AppError::InvalidArgument(
                        "批量图片请求不能为空".to_string(),
                    ));
                }
                self.verify_project_permission(
                    input.project_id,
                    user_id,
                    ProjectAccessRole::Editor,
                )
                .await?;
            }
            ImageGenerationOperation::ObjectProfile {
                object_profile_id,
                chapter_id,
                style,
                config_id,
                style_template_id,
                style_template_name,
                style_template_version,
                custom_style,
            } => {
                let input = ObjectProfileImageInput {
                    object_profile_id: *object_profile_id,
                    chapter_id: *chapter_id,
                    style: style.clone(),
                    config_id: *config_id,
                    style_template_id: style_template_id.clone(),
                    style_template_name: style_template_name.clone(),
                    style_template_version: style_template_version.clone(),
                    custom_style: custom_style.clone(),
                };
                self.generate_object_profile_image_prompt_validated(user_id, &input)
                    .await?;
            }
            ImageGenerationOperation::SceneProfile {
                scene_profile_id,
                chapter_id,
                style,
                config_id,
                style_template_id,
                style_template_name,
                style_template_version,
                custom_style,
            } => {
                let input = SceneProfileImageInput {
                    scene_profile_id: *scene_profile_id,
                    chapter_id: *chapter_id,
                    style: style.clone(),
                    config_id: *config_id,
                    style_template_id: style_template_id.clone(),
                    style_template_name: style_template_name.clone(),
                    style_template_version: style_template_version.clone(),
                    custom_style: custom_style.clone(),
                };
                self.generate_scene_profile_image_prompt_validated(user_id, &input)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn run_image_generation_task(
        &self,
        user_id: i64,
        input: StartImageGenerationInput,
        handle: GenerationTaskHandle,
        host_port: &dyn ImageGenerationHostPort,
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
        let operation_key = image_operation_key(&input.operation);
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
            ProgressPhaseKind::AssetGeneration,
            0,
        ));
        let reporter = move |current: u32, total: u32, message: &str| {
            if progress_sender
                .send(GenerationProgressUpdate {
                    current,
                    total,
                    message: message.to_string(),
                })
                .is_err()
            {
                progress_publish_failed_for_reporter.store(true, Ordering::Release);
            }
        };
        let application_result = {
            let progress = ImageGenerationProgress::new(Some(&reporter), Some(&cancel_token));
            self.execute_image_generation_operation(user_id, input, host_port, &progress)
                .await
        };
        drop(reporter);
        let mut progress_result = match progress_worker.await {
            Ok(result) => result,
            Err(error) => Err(generation_public_error(
                "generation_event_invalid",
                operation_key,
                format!("图片进度 publisher 异常退出: {error}"),
            )),
        };
        if progress_result.is_ok() && progress_publish_failed.load(Ordering::Acquire) {
            progress_result = Err(generation_public_error(
                "generation_event_invalid",
                operation_key,
                "图片进度 publisher 已关闭".to_string(),
            ));
        }
        let task_result = match (application_result, progress_result) {
            (Ok(result), Ok(())) => result,
            (Err(error), Ok(())) => {
                return Err(execution
                    .fail(image_app_error_to_generation_error(error, operation_key))
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
        let ImageGenerationResult {
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

    async fn execute_image_generation_operation(
        &self,
        user_id: i64,
        input: StartImageGenerationInput,
        host_port: &dyn ImageGenerationHostPort,
        progress: &ImageGenerationProgress<'_>,
    ) -> Result<ImageGenerationResult, AppError> {
        let project_id = input.project_id;
        match input.operation {
            ImageGenerationOperation::Scene {
                prompt,
                config_id,
                chapter_id,
            } => {
                let summary = self
                    .generate_scene_image_validated(
                        host_port,
                        user_id,
                        SceneImageGenerationInput {
                            project_id,
                            prompt,
                            config_id,
                            chapter_id,
                        },
                        progress,
                    )
                    .await?;
                single_image_result(host_port, summary, "场景图片生成完成")
            }
            ImageGenerationOperation::Page {
                chapter_number,
                page_number,
            } => {
                let summary = self
                    .generate_page_image_validated(
                        host_port,
                        user_id,
                        PageImageGenerationInput {
                            project_id,
                            chapter_number,
                            page_number,
                        },
                        progress,
                    )
                    .await?;
                single_image_result(host_port, summary, "整页漫画图片生成完成")
            }
            ImageGenerationOperation::Batch { requests } => {
                let outcome = self
                    .generate_batch_images_validated(
                        host_port,
                        user_id,
                        BatchImageGenerationInput {
                            project_id,
                            requests_json: serde_json::Value::Array(requests),
                        },
                        progress,
                    )
                    .await?;
                batch_image_result(host_port, project_id, outcome)
            }
            ImageGenerationOperation::ObjectProfile {
                object_profile_id,
                chapter_id,
                style,
                config_id,
                style_template_id,
                style_template_name,
                style_template_version,
                custom_style,
            } => {
                let summary = self
                    .generate_object_profile_image_validated(
                        host_port,
                        user_id,
                        ObjectProfileImageInput {
                            object_profile_id,
                            chapter_id,
                            style,
                            config_id,
                            style_template_id,
                            style_template_name,
                            style_template_version,
                            custom_style,
                        },
                        progress,
                    )
                    .await?;
                single_image_result(host_port, summary, "物体素材图生成完成")
            }
            ImageGenerationOperation::SceneProfile {
                scene_profile_id,
                chapter_id,
                style,
                config_id,
                style_template_id,
                style_template_name,
                style_template_version,
                custom_style,
            } => {
                let summary = self
                    .generate_scene_profile_image_validated(
                        host_port,
                        user_id,
                        SceneProfileImageInput {
                            scene_profile_id,
                            chapter_id,
                            style,
                            config_id,
                            style_template_id,
                            style_template_name,
                            style_template_version,
                            custom_style,
                        },
                        progress,
                    )
                    .await?;
                single_image_result(host_port, summary, "场景素材图生成完成")
            }
        }
    }
}

fn single_image_result(
    host_port: &dyn ImageGenerationHostPort,
    summary: GeneratedImageSummary,
    message: &str,
) -> Result<ImageGenerationResult, AppError> {
    let result_ref = image_result_ref("generated_image", summary.id, &summary)?;
    Ok(ImageGenerationResult {
        summary: message.to_string(),
        result_ref,
        artifacts: vec![image_artifact_event(host_port, &summary, None, None, None)?],
        batch_summary: None,
        failure: None,
    })
}

fn batch_image_result(
    host_port: &dyn ImageGenerationHostPort,
    project_id: i64,
    outcome: BatchImageGenerationOutcome,
) -> Result<ImageGenerationResult, AppError> {
    let batch_summary = image_batch_summary(&outcome);
    let artifacts = outcome
        .items
        .iter()
        .map(|item| {
            image_artifact_event(
                host_port,
                &item.summary,
                Some(item.item_key.clone()),
                Some(item.item_index),
                Some(outcome.total),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let result_ref = batch_result_ref(project_id, &outcome.items)?;
    let failure = if outcome.interrupted {
        Some(generation_public_error(
            "generation_interrupted",
            "image_batch",
            outcome
                .first_error
                .as_deref()
                .unwrap_or("批量图片生成已停止"),
        ))
    } else if outcome.succeeded() == 0 {
        Some(generation_public_error(
            "provider_job_failed",
            "image_batch",
            outcome
                .first_error
                .as_deref()
                .unwrap_or("批量图片全部生成失败"),
        ))
    } else {
        None
    };
    Ok(ImageGenerationResult {
        summary: format!(
            "批量图片生成完成，成功 {} 张，失败 {} 张",
            outcome.succeeded(),
            outcome.failed()
        ),
        result_ref,
        artifacts,
        batch_summary: Some(batch_summary),
        failure,
    })
}

fn image_result_ref(
    kind: &str,
    id: i64,
    summary: &GeneratedImageSummary,
) -> Result<GenerationResultRef, AppError> {
    Ok(GenerationResultRef {
        kind: kind.to_string(),
        id: id.to_string(),
        revision: format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&[
                serde_json::json!(summary.id),
                serde_json::json!(summary.project_id),
                serde_json::json!(&summary.image_type),
                serde_json::json!(&summary.file_path),
                serde_json::json!(&summary.prompt),
                serde_json::json!(&summary.model),
                serde_json::json!(summary.width),
                serde_json::json!(summary.height),
                serde_json::json!(summary.file_size),
                serde_json::json!(summary.chapter_id),
                serde_json::json!(&summary.panel_id),
                serde_json::json!(&summary.created_at),
            ])?)
        ),
    })
}

fn batch_result_ref(
    project_id: i64,
    items: &[BatchImageGenerationItem],
) -> Result<GenerationResultRef, AppError> {
    let mut ids = items.iter().map(|item| item.summary.id).collect::<Vec<_>>();
    ids.sort_unstable();
    Ok(GenerationResultRef {
        kind: "generated_image_batch".to_string(),
        id: project_id.to_string(),
        revision: format!("{:x}", Sha256::digest(serde_json::to_vec(&ids)?)),
    })
}

fn image_batch_summary(outcome: &BatchImageGenerationOutcome) -> BatchSummary {
    let mut failed_item_keys = outcome.failed_item_keys.clone();
    failed_item_keys.sort_unstable();
    failed_item_keys.dedup();
    let failed_item_keys_truncated = failed_item_keys.len() > 256;
    failed_item_keys.truncate(256);
    BatchSummary {
        policy: BatchCommitPolicy::BestEffort,
        total: outcome.total,
        succeeded: outcome.succeeded(),
        failed: outcome.failed(),
        cancelled: outcome.cancelled,
        skipped: outcome.skipped,
        failed_item_keys,
        failed_item_keys_truncated,
    }
}

fn image_artifact_event(
    host_port: &dyn ImageGenerationHostPort,
    summary: &GeneratedImageSummary,
    item_key: Option<String>,
    item_index: Option<u32>,
    item_total: Option<u32>,
) -> Result<PreparedArtifactEvent, AppError> {
    let file = host_port.inspect_generated_image_file(&summary.file_path, "正式图片文件")?;
    Ok(GenerationArtifactCoordinator::available(
        GenerationArtifactRef {
            artifact_id: format!("generated-image:{}", summary.id),
            artifact_kind: "generated_image".to_string(),
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

fn image_operation_key(operation: &ImageGenerationOperation) -> &'static str {
    match operation {
        ImageGenerationOperation::Scene { .. } => "image_scene",
        ImageGenerationOperation::Page { .. } => "image_page",
        ImageGenerationOperation::Batch { .. } => "image_batch",
        ImageGenerationOperation::ObjectProfile { .. } => "image_object_profile",
        ImageGenerationOperation::SceneProfile { .. } => "image_scene_profile",
    }
}

fn image_app_error_to_generation_error(error: AppError, stage_code: &str) -> GenerationError {
    let message = error.to_string();
    let (code, stage) = match error {
        AppError::Cancelled => ("generation_interrupted", "image_cancel"),
        AppError::LlmError(_) | AppError::Timeout(_) | AppError::ConfigError(_) => {
            ("provider_job_failed", stage_code)
        }
        AppError::Forbidden(_) | AppError::Unauthorized(_) | AppError::NotFound(_) => {
            ("generation_task_forbidden", stage_code)
        }
        AppError::Database(_) | AppError::Serialization(_) => {
            ("generation_persist_failed", "image_persist")
        }
        AppError::InvalidArgument(_) | AppError::InvalidState { .. } => {
            if message.contains("已停止") || message.contains("取消") {
                ("generation_interrupted", "image_cancel")
            } else {
                ("generation_request_invalid", stage_code)
            }
        }
        _ => ("generation_persist_failed", stage_code),
    };
    generation_public_error(code, stage, message)
}

#[cfg(test)]
mod tests {
    use super::image_operation_key;
    use a4n_api_contract::image_generation::ImageGenerationOperation;

    #[test]
    fn image_operation_keys_are_stable_and_unique() {
        let operations = [
            ImageGenerationOperation::Scene {
                prompt: "draw".to_string(),
                config_id: None,
                chapter_id: None,
            },
            ImageGenerationOperation::Page {
                chapter_number: 1,
                page_number: 1,
            },
            ImageGenerationOperation::Batch { requests: vec![] },
            ImageGenerationOperation::ObjectProfile {
                object_profile_id: 1,
                chapter_id: None,
                style: None,
                config_id: None,
                style_template_id: None,
                style_template_name: None,
                style_template_version: None,
                custom_style: None,
            },
            ImageGenerationOperation::SceneProfile {
                scene_profile_id: 1,
                chapter_id: None,
                style: None,
                config_id: None,
                style_template_id: None,
                style_template_name: None,
                style_template_version: None,
                custom_style: None,
            },
        ];
        let keys = operations
            .iter()
            .map(image_operation_key)
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(keys.len(), operations.len());
    }
}
