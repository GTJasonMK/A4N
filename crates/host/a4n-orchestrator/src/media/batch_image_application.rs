use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

use a4n_api_contract::image::GeneratedImageSummary;
use a4n_error::AppError;
use a4n_image_contract::ImageGenerationRequest;
use a4n_provider_contract::ImageProviderConfig;
use futures::stream::{self, StreamExt};
use serde_json::{Value, json};

use crate::OrchestratorContext;

use super::{
    ImageGenerationExecutionInput, ImageGenerationHostPort, ImageGenerationPostCommitAction,
    ImageGenerationProgress, ImageGenerationWorkflowBinding, batch_image_item_step_key,
    batch_image_workflow_run_input, image_generation_request_trace_json,
};

#[derive(Debug)]
pub struct BatchImageExecutionItem {
    pub index: usize,
    pub request: ImageGenerationRequest,
    pub image_type: String,
    pub chapter_id: Option<i64>,
    pub panel_id: Option<String>,
    pub metadata_json: Option<Value>,
    pub config: ImageProviderConfig,
}

#[derive(Debug)]
pub struct BatchImageGenerationApplicationInput {
    pub requested_by_user_id: i64,
    pub project_id: i64,
    pub requests: Vec<Value>,
    pub items: Vec<BatchImageExecutionItem>,
    pub runtime_config_snapshot: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct BatchImageGenerationItem {
    pub item_key: String,
    pub item_index: u32,
    pub summary: GeneratedImageSummary,
}

#[derive(Debug, Clone)]
pub struct BatchImageGenerationOutcome {
    pub total: u32,
    pub items: Vec<BatchImageGenerationItem>,
    pub failed_item_keys: Vec<String>,
    pub cancelled: u32,
    pub skipped: u32,
    pub interrupted: bool,
    pub first_error: Option<String>,
}

impl BatchImageGenerationOutcome {
    pub fn succeeded(&self) -> u32 {
        self.items.len() as u32
    }

    pub fn failed(&self) -> u32 {
        self.failed_item_keys.len() as u32
    }
}

enum BatchItemResult {
    Succeeded(BatchImageGenerationItem),
    Failed { item_key: String, error: String },
    Cancelled { error: String },
    Skipped { error: String },
}

impl OrchestratorContext {
    pub async fn run_batch_image_generation_validated(
        &self,
        host_port: &dyn ImageGenerationHostPort,
        input: BatchImageGenerationApplicationInput,
        progress: &ImageGenerationProgress<'_>,
    ) -> Result<BatchImageGenerationOutcome, AppError> {
        validate_batch_input(&input)?;
        let workflow_run = self
            .begin_workflow_run_validated(
                batch_image_workflow_run_input(input.project_id, &input.requests)
                    .map_err(AppError::InvalidArgument)?,
            )
            .await?;
        let workflow_run_id = workflow_run.id;
        let result = self
            .execute_batch_image_generation(host_port, input, workflow_run_id, progress)
            .await;
        let finalize_result = match &result {
            Ok(outcome) if outcome.interrupted => {
                let error =
                    outcome
                        .first_error
                        .as_deref()
                        .ok_or_else(|| AppError::InvalidState {
                            current: "batch_image_interrupted_without_error".to_string(),
                            operation: "写入批量图片取消终态".to_string(),
                        })?;
                self.cancel_workflow_run_status_validated(workflow_run_id, error)
                    .await
                    .map(|_| ())
            }
            Ok(outcome) if outcome.succeeded() == 0 => {
                let error =
                    outcome
                        .first_error
                        .as_deref()
                        .ok_or_else(|| AppError::InvalidState {
                            current: "batch_image_failed_without_error".to_string(),
                            operation: "写入批量图片失败终态".to_string(),
                        })?;
                self.fail_workflow_run_validated(workflow_run_id, error)
                    .await
                    .map(|_| ())
            }
            Ok(_) => self
                .complete_workflow_run_validated(workflow_run_id)
                .await
                .map(|_| ()),
            Err(error @ AppError::Cancelled) => {
                let message = error.to_string();
                self.cancel_workflow_run_status_validated(workflow_run_id, &message)
                    .await
                    .map(|_| ())
            }
            Err(error) => {
                let message = error.to_string();
                self.fail_workflow_run_validated(workflow_run_id, &message)
                    .await
                    .map(|_| ())
            }
        };

        if let Err(finalize_error) = finalize_result {
            if matches!(&finalize_error, AppError::Cancelled) {
                return Err(AppError::Cancelled);
            }
            return match result {
                Ok(_) => Err(AppError::InvalidState {
                    current: "batch_images_generated".to_string(),
                    operation: format!("批量图片生成完成，但工作流状态写入失败: {finalize_error}"),
                }),
                Err(error) => Err(crate::errors::app_error_with_secondary(
                    error,
                    format!("工作流失败状态写入失败: {finalize_error}"),
                )),
            };
        }
        result
    }

    async fn execute_batch_image_generation(
        &self,
        host_port: &dyn ImageGenerationHostPort,
        input: BatchImageGenerationApplicationInput,
        workflow_run_id: i64,
        progress: &ImageGenerationProgress<'_>,
    ) -> Result<BatchImageGenerationOutcome, AppError> {
        let BatchImageGenerationApplicationInput {
            requested_by_user_id,
            project_id,
            requests: _,
            items,
            runtime_config_snapshot,
        } = input;
        let total = u32::try_from(items.len()).map_err(|_| {
            AppError::InvalidArgument("批量图片请求数量超出 progress 计数范围".to_string())
        })?;
        let concurrency = items
            .iter()
            .map(|item| item.config.max_concurrent.max(1) as usize)
            .min()
            .unwrap_or(1)
            .clamp(1, items.len().max(1));
        let item_count = items.len();
        progress.progress(0, total, &format!("开始批量生成图片，并发 {concurrency}"));

        let completed = Arc::new(AtomicU32::new(0));
        let results = stream::iter(items)
            .map(|item| {
                let completed = Arc::clone(&completed);
                let runtime_config_snapshot = runtime_config_snapshot.clone();
                async move {
                    let item_index = item.index;
                    let display_index = item_index + 1;
                    let item_key = format!("image-request-{display_index}");
                    if let Err(error) = progress.check_cancelled() {
                        return (item_index, BatchItemResult::Skipped { error });
                    }
                    progress.progress(
                        completed.load(Ordering::Relaxed),
                        total,
                        &format!("正在生成图片 {display_index}/{item_count}"),
                    );
                    let request_json =
                        image_generation_request_trace_json(&item.config, &item.request);
                    let fingerprint_json =
                        batch_image_item_fingerprint(project_id, &item, &request_json);
                    let step_order = i32::try_from(display_index)
                        .expect("batch item count validated against i32");
                    let file_extra = item.chapter_id.map(|chapter| format!("ch{chapter:02}"));
                    let item_cancel_token = progress.cancel_token();
                    let item_progress =
                        ImageGenerationProgress::new(None, Some(&item_cancel_token));
                    let result = self
                        .run_image_generation_validated(
                            host_port,
                            ImageGenerationExecutionInput {
                                requested_by_user_id,
                                project_id,
                                chapter_id: item.chapter_id,
                                image_type: item.image_type,
                                command: "image_batch".to_string(),
                                file_extra,
                                file_context: "批量图片".to_string(),
                                width: item.request.width,
                                height: item.request.height,
                                config: item.config,
                                request: item.request,
                                panel_id: item.panel_id,
                                metadata_json: item.metadata_json,
                                runtime_config_snapshot,
                                workflow: ImageGenerationWorkflowBinding::ExistingItem {
                                    workflow_run_id,
                                    step_key: batch_image_item_step_key(item_index),
                                    step_order,
                                    fingerprint_json,
                                },
                                post_commit: ImageGenerationPostCommitAction::None,
                            },
                            &item_progress,
                        )
                        .await;
                    let next_current = completed.fetch_add(1, Ordering::SeqCst) + 1;
                    let item_result = match result {
                        Ok(outcome) => {
                            progress.progress(
                                next_current,
                                total,
                                &format!(
                                    "{}图片 {display_index}/{item_count}",
                                    if outcome.restored {
                                        "已从断点恢复"
                                    } else {
                                        "已生成"
                                    }
                                ),
                            );
                            BatchItemResult::Succeeded(BatchImageGenerationItem {
                                item_key,
                                item_index: u32::try_from(item_index)
                                    .expect("batch item count validated against u32"),
                                summary: outcome.summary,
                            })
                        }
                        Err(error) if image_generation_cancelled(progress, &error) => {
                            let error = match progress.check_cancelled() {
                                Err(error) => error,
                                Ok(()) => error.to_string(),
                            };
                            progress.progress(
                                next_current,
                                total,
                                &format!("图片 {display_index}/{item_count} 已取消"),
                            );
                            BatchItemResult::Cancelled { error }
                        }
                        Err(error) => {
                            progress.progress(
                                next_current,
                                total,
                                &format!("图片 {display_index}/{item_count} 失败：{error}"),
                            );
                            BatchItemResult::Failed {
                                item_key,
                                error: error.to_string(),
                            }
                        }
                    };
                    (item_index, item_result)
                }
            })
            .buffer_unordered(concurrency)
            .collect::<Vec<(usize, BatchItemResult)>>()
            .await;

        let mut ordered = results;
        ordered.sort_by_key(|(index, _)| *index);
        let mut outcome = BatchImageGenerationOutcome {
            total,
            items: Vec::new(),
            failed_item_keys: Vec::new(),
            cancelled: 0,
            skipped: 0,
            interrupted: false,
            first_error: None,
        };
        for (_, item) in ordered {
            match item {
                BatchItemResult::Succeeded(item) => outcome.items.push(item),
                BatchItemResult::Failed { item_key, error } => {
                    outcome.failed_item_keys.push(item_key);
                    outcome.first_error.get_or_insert(error);
                }
                BatchItemResult::Cancelled { error } => {
                    outcome.cancelled += 1;
                    outcome.first_error.get_or_insert(error);
                }
                BatchItemResult::Skipped { error } => {
                    outcome.skipped += 1;
                    outcome.first_error.get_or_insert(error);
                }
            }
        }
        outcome.interrupted = outcome.cancelled > 0 || outcome.skipped > 0;
        Ok(outcome)
    }
}

fn image_generation_cancelled(progress: &ImageGenerationProgress<'_>, error: &AppError) -> bool {
    matches!(error, AppError::Cancelled)
        || progress.check_cancelled().is_err()
        || error.to_string().contains("图片生成已停止")
}

fn validate_batch_input(input: &BatchImageGenerationApplicationInput) -> Result<(), AppError> {
    if input.project_id <= 0 {
        return Err(AppError::InvalidArgument("项目 ID 必须大于 0".to_string()));
    }
    if input.items.is_empty() {
        return Err(AppError::InvalidArgument(
            "批量图片请求不能为空".to_string(),
        ));
    }
    if input.requests.len() != input.items.len() {
        return Err(AppError::InvalidArgument(format!(
            "批量图片请求与执行项数量不一致：{} != {}",
            input.requests.len(),
            input.items.len()
        )));
    }
    i32::try_from(input.items.len()).map_err(|_| {
        AppError::InvalidArgument("批量图片请求数量超出 checkpoint 序号范围".to_string())
    })?;
    for (expected_index, item) in input.items.iter().enumerate() {
        if item.index != expected_index {
            return Err(AppError::InvalidArgument(format!(
                "批量图片执行项序号不连续：期望 {expected_index}，实际 {}",
                item.index
            )));
        }
    }
    Ok(())
}

fn batch_image_item_fingerprint(
    project_id: i64,
    item: &BatchImageExecutionItem,
    request_json: &Value,
) -> Value {
    json!({
        "workflow": "image_batch",
        "project_id": project_id,
        "index": item.index,
        "image_type": item.image_type,
        "chapter_id": item.chapter_id,
        "panel_id": item.panel_id,
        "metadata_json": item.metadata_json,
        "request": request_json,
    })
}
