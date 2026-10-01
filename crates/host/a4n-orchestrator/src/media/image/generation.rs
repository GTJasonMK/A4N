mod application;
mod contracts;
mod execution;
mod ports;

mod trace;

pub use application::normalize_image_metadata;
pub use contracts::{
    ImageGenerationExecutionInput, ImageGenerationExecutionOutcome,
    ImageGenerationPostCommitAction, ImageGenerationWorkflowBinding,
};
pub use execution::generated_image_record_summary;
pub use trace::image_generation_request_trace_json;

#[cfg(test)]
mod tests;

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use a4n_api_contract::{
    generation_tasks::GenerationTaskHandle,
    image::GeneratedImageSummary,
    image_generation::{
        GenerateBatchImagesInput, GeneratePageImageInput, GenerateSceneImageInput,
        ImageGenerationOperation, ObjectProfileImageInput, SceneProfileImageInput,
        StartImageGenerationInput,
    },
};
use a4n_error::AppError;
use a4n_progress::ProgressPhaseKind;
use a4n_service_protocol::{BatchSummary, GenerationError, GenerationResultRef};
use tokio::sync::mpsc;

use crate::{
    OrchestratorContext,
    generation::{
        GenerationProgressUpdate,
        artifacts::{GenerationArtifactCoordinator, PreparedArtifactEvent},
        best_effort_batch_summary, generation_batch_result_ref, generation_public_error,
        generation_result_ref, publish_step_progress,
    },
    media::{BatchImageGenerationOutcome, ImageGenerationHostPort, ImageGenerationProgress},
};
use a4n_domain::collaboration::ProjectAccessRole;

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
            ImageGenerationOperation::VisualPlanPoint { options } => {
                self.preview_visual_plan_point_generation(user_id, input.project_id, options)
                    .await?;
            }
            ImageGenerationOperation::Scene {
                prompt,
                width,
                height,
                ..
            } => {
                if prompt.trim().is_empty() {
                    return Err(AppError::InvalidArgument("prompt 不能为空".to_string()));
                }
                require_positive_dimension(*width, "width")?;
                require_positive_dimension(*height, "height")?;
            }
            ImageGenerationOperation::Page {
                chapter_number,
                page_number,
                width,
                height,
            } => {
                if *chapter_number <= 0 || *page_number <= 0 {
                    return Err(AppError::InvalidArgument(
                        "chapter_number 和 page_number 必须大于 0".to_string(),
                    ));
                }
                require_positive_dimension(*width, "width")?;
                require_positive_dimension(*height, "height")?;
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
                width,
                height,
            } => {
                require_positive_dimension(*width, "width")?;
                require_positive_dimension(*height, "height")?;
                let input = ObjectProfileImageInput {
                    object_profile_id: *object_profile_id,
                    chapter_id: *chapter_id,
                    style: style.clone(),
                    config_id: *config_id,
                    style_template_id: style_template_id.clone(),
                    style_template_name: style_template_name.clone(),
                    style_template_version: style_template_version.clone(),
                    custom_style: custom_style.clone(),
                    width: Some(*width),
                    height: Some(*height),
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
                width,
                height,
            } => {
                require_positive_dimension(*width, "width")?;
                require_positive_dimension(*height, "height")?;
                let input = SceneProfileImageInput {
                    scene_profile_id: *scene_profile_id,
                    chapter_id: *chapter_id,
                    style: style.clone(),
                    config_id: *config_id,
                    style_template_id: style_template_id.clone(),
                    style_template_name: style_template_name.clone(),
                    style_template_version: style_template_version.clone(),
                    custom_style: custom_style.clone(),
                    width: Some(*width),
                    height: Some(*height),
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
                width,
                height,
            } => {
                let summary = self
                    .generate_scene_image_validated(
                        host_port,
                        user_id,
                        GenerateSceneImageInput {
                            project_id,
                            prompt,
                            config_id,
                            chapter_id,
                            width,
                            height,
                        },
                        progress,
                    )
                    .await?;
                single_image_result(host_port, summary, "场景图片生成完成")
            }
            ImageGenerationOperation::Page {
                chapter_number,
                page_number,
                width,
                height,
            } => {
                let summary = self
                    .generate_page_image_validated(
                        host_port,
                        user_id,
                        GeneratePageImageInput {
                            project_id,
                            chapter_number,
                            page_number,
                            width,
                            height,
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
                        GenerateBatchImagesInput {
                            project_id,
                            requests,
                        },
                        progress,
                    )
                    .await?;
                batch_image_result(host_port, project_id, outcome)
            }
            ImageGenerationOperation::VisualPlanPoint { options } => {
                let item = self
                    .visual_plan_point_batch_item(user_id, project_id, &options)
                    .await?;
                let outcome = self
                    .generate_batch_images_validated(
                        host_port,
                        user_id,
                        GenerateBatchImagesInput {
                            project_id,
                            requests: vec![item],
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
                width,
                height,
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
                            width: Some(width),
                            height: Some(height),
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
                width,
                height,
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
                            width: Some(width),
                            height: Some(height),
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
    let result_ref = image_result_ref(&summary)?;
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
    let batch_summary = best_effort_batch_summary(
        outcome.total,
        outcome.succeeded(),
        outcome.failed(),
        outcome.cancelled,
        outcome.skipped,
        &outcome.failed_item_keys,
    );
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
    let result_ref = generation_batch_result_ref(
        "generated_image_batch",
        project_id,
        outcome.items.iter().map(|item| item.summary.id).collect(),
    )?;
    let failure = if outcome.interrupted {
        let message = outcome
            .first_error
            .as_deref()
            .ok_or_else(|| AppError::InvalidState {
                current: "image_batch_interrupted_without_error".to_string(),
                operation: "批量图片中断必须记录首个错误".to_string(),
            })?;
        Some(generation_public_error(
            "generation_interrupted",
            "image_batch",
            message,
        ))
    } else if outcome.succeeded() == 0 {
        let message = outcome
            .first_error
            .as_deref()
            .ok_or_else(|| AppError::InvalidState {
                current: "image_batch_failed_without_error".to_string(),
                operation: "批量图片全部失败时必须记录首个错误".to_string(),
            })?;
        Some(generation_public_error(
            "provider_job_failed",
            "image_batch",
            message,
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

fn image_result_ref(summary: &GeneratedImageSummary) -> Result<GenerationResultRef, AppError> {
    generation_result_ref(
        "generated_image",
        summary.id,
        &[
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
        ],
    )
}

fn image_artifact_event(
    host_port: &dyn ImageGenerationHostPort,
    summary: &GeneratedImageSummary,
    item_key: Option<String>,
    item_index: Option<u32>,
    item_total: Option<u32>,
) -> Result<PreparedArtifactEvent, AppError> {
    let file = host_port.inspect_generated_image_file(&summary.file_path, "正式图片文件")?;
    Ok(GenerationArtifactCoordinator::generated_media_available(
        format!("generated-image:{}", summary.id),
        "generated_image",
        file,
        item_key,
        item_index,
        item_total,
    ))
}

fn image_operation_key(operation: &ImageGenerationOperation) -> &'static str {
    match operation {
        ImageGenerationOperation::VisualPlanPoint { .. } => "image_visual_plan_point",
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

fn require_positive_dimension(value: i32, label: &str) -> Result<(), AppError> {
    if value <= 0 {
        return Err(AppError::InvalidArgument(format!(
            "图片 {label} 必须大于 0"
        )));
    }
    Ok(())
}
