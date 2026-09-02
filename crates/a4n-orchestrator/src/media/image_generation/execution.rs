use a4n_core::{
    config::{ensure_image_size_supported, image_sizes_from_json},
    error::AppError,
    models::ai::generated_image,
};
use a4n_db::generation::{
    GenerationRunCheckpointContext, RecordCompletedGenerationRunInput,
    RecordCompletedGenerationRunWithCheckpointInput, generation_checkpoint_fingerprint_sha256,
};
use serde_json::{Value, json};

use crate::{
    OrchestratorContext,
    video::CreateGeneratedImageRecordDraftInput,
    workflow::{
        CompletedWorkflowCheckpointInput, required_checkpoint_field, required_checkpoint_json,
    },
};

use super::{
    contracts::{
        GeneratedImageSummary, ImageGenerationExecutionInput, ImageGenerationExecutionOutcome,
        ImageGenerationHostPort, ImageGenerationPostCommitAction, ImageGenerationProgress,
        ImageGenerationProviderInput, ImageGenerationWorkflowBinding,
        PersistGeneratedImageFileInput,
    },
    trace::image_generation_request_trace_json,
};

struct ImageGenerationWorkflowScope {
    workflow_run_id: i64,
    step_key: String,
    step_order: i32,
    fingerprint_json: Value,
    owned: bool,
}

impl OrchestratorContext {
    pub async fn run_image_generation_validated(
        &self,
        host_port: &dyn ImageGenerationHostPort,
        input: ImageGenerationExecutionInput,
        progress: &ImageGenerationProgress<'_>,
    ) -> Result<ImageGenerationExecutionOutcome, AppError> {
        validate_execution_input(&input)?;
        let request_json = image_generation_request_trace_json(&input.config, &input.request);
        let workflow = self
            .resolve_image_generation_workflow(&input, &request_json)
            .await?;
        let result = self
            .execute_image_generation_item(host_port, input, request_json, &workflow, progress)
            .await;

        if workflow.owned {
            return self
                .finalize_workflow_result(workflow.workflow_run_id, result, "图片生成")
                .await;
        }
        result
    }

    async fn resolve_image_generation_workflow(
        &self,
        input: &ImageGenerationExecutionInput,
        request_json: &Value,
    ) -> Result<ImageGenerationWorkflowScope, AppError> {
        match &input.workflow {
            ImageGenerationWorkflowBinding::Owned => {
                let started = self
                    .begin_image_generation_workflow_validated(
                        input.project_id,
                        input.chapter_id,
                        &input.command,
                        &input.image_type,
                        input.panel_id.as_deref(),
                        request_json,
                        input.metadata_json.as_ref(),
                    )
                    .await?;
                Ok(ImageGenerationWorkflowScope {
                    workflow_run_id: started.workflow_run.id,
                    step_key: started.step_key,
                    step_order: 1,
                    fingerprint_json: started.fingerprint_json,
                    owned: true,
                })
            }
            ImageGenerationWorkflowBinding::Attached { workflow_run_id } => {
                require_positive(*workflow_run_id, "图片生成 workflow_run_id")?;
                Ok(ImageGenerationWorkflowScope {
                    workflow_run_id: *workflow_run_id,
                    step_key: super::super::image_generation_step_key(&input.command),
                    step_order: 1,
                    fingerprint_json: super::super::image_generation_workflow_fingerprint(
                        input.project_id,
                        input.chapter_id,
                        &input.command,
                        &input.image_type,
                        input.panel_id.as_deref(),
                        request_json,
                        input.metadata_json.as_ref(),
                    ),
                    owned: false,
                })
            }
            ImageGenerationWorkflowBinding::ExistingItem {
                workflow_run_id,
                step_key,
                step_order,
                fingerprint_json,
            } => {
                require_positive(*workflow_run_id, "图片生成 workflow_run_id")?;
                require_positive(i64::from(*step_order), "图片 checkpoint step_order")?;
                let step_key = required_text(step_key, "图片 checkpoint step_key")?;
                Ok(ImageGenerationWorkflowScope {
                    workflow_run_id: *workflow_run_id,
                    step_key: step_key.to_string(),
                    step_order: *step_order,
                    fingerprint_json: fingerprint_json.clone(),
                    owned: false,
                })
            }
        }
    }

    async fn execute_image_generation_item(
        &self,
        host_port: &dyn ImageGenerationHostPort,
        input: ImageGenerationExecutionInput,
        request_json: Value,
        workflow: &ImageGenerationWorkflowScope,
        progress: &ImageGenerationProgress<'_>,
    ) -> Result<ImageGenerationExecutionOutcome, AppError> {
        if let Some(image) = self
            .load_image_generation_checkpoint(host_port, workflow, input.project_id)
            .await?
        {
            let outcome = ImageGenerationExecutionOutcome {
                summary: generated_image_summary(&image),
                restored: true,
                created_at: image.created_at,
            };
            self.complete_image_generation_post_commit(host_port, &input, &outcome)
                .await?;
            return Ok(outcome);
        }

        progress
            .check_cancelled()
            .map_err(|_| AppError::Cancelled)?;
        let provider_output = host_port
            .generate_image(
                ImageGenerationProviderInput {
                    project_id: input.project_id,
                    workflow_run_id: workflow.workflow_run_id,
                    config: input.config.clone(),
                    request: input.request.clone(),
                },
                progress,
            )
            .await?;
        let (actual_width, actual_height) =
            crate::media::image_dimensions_from_bytes(&provider_output.bytes)?;
        progress
            .check_cancelled()
            .map_err(|_| AppError::Cancelled)?;

        let created_at = chrono::Utc::now().naive_utc();
        let metadata_json = attach_image_generation_trace(
            input.metadata_json.clone(),
            ImageGenerationTrace {
                command: &input.command,
                config: &input.config,
                image_type: &input.image_type,
                prompt: &input.request.prompt,
                provider_model: &provider_output.model,
                chapter_id: input.chapter_id,
                panel_id: input.panel_id.as_deref(),
                width: actual_width,
                height: actual_height,
                generated_at: created_at,
                runtime_config_snapshot: input.runtime_config_snapshot.clone(),
            },
        )?;
        let media_refs_json = metadata_json.as_ref().and_then(image_generation_media_refs);
        let persisted = host_port.persist_generated_image_file(PersistGeneratedImageFileInput {
            project_id: input.project_id,
            chapter_id: input.chapter_id,
            image_type: &input.image_type,
            file_extra: input.file_extra.as_deref(),
            bytes: &provider_output.bytes,
            format: &provider_output.format,
            file_context: &input.file_context,
        })?;

        let image = match self
            .create_generated_image_record_with_generation_run_checkpoint(
                CreateGeneratedImageRecordDraftInput {
                    project_id: input.project_id,
                    image_type: input.image_type.clone(),
                    file_path: persisted.relative_path.clone(),
                    prompt: input.request.prompt.clone(),
                    model: provider_output.model.clone(),
                    width: actual_width,
                    height: actual_height,
                    file_size: Some(persisted.file_size),
                    chapter_id: input.chapter_id,
                    panel_id: input.panel_id.clone(),
                    metadata_json,
                    created_at: Some(created_at),
                },
                generation_trace_input(
                    &input,
                    &provider_output,
                    request_json,
                    media_refs_json,
                    workflow.workflow_run_id,
                    persisted.file_size,
                )?,
            )
            .await
        {
            Ok(image) => image,
            Err(error) => {
                return Err(cleanup_created_file_after_error(
                    host_port,
                    &persisted.relative_path,
                    "已写出的图片文件",
                    error,
                )
                .await);
            }
        };
        let summary = generated_image_summary(&image);
        self.save_completed_workflow_checkpoint(CompletedWorkflowCheckpointInput {
            workflow_run_id: Some(workflow.workflow_run_id),
            step_key: workflow.step_key.clone(),
            step_order: workflow.step_order,
            input_fingerprint_json: workflow.fingerprint_json.clone(),
            checkpoint_json: Some(json!({
                "generated_image_id": image.id,
                "file_path": image.file_path,
                "summary": summary,
            })),
        })
        .await?;
        let outcome = ImageGenerationExecutionOutcome {
            summary,
            restored: false,
            created_at,
        };
        self.complete_image_generation_post_commit(host_port, &input, &outcome)
            .await?;
        Ok(outcome)
    }

    async fn load_image_generation_checkpoint(
        &self,
        host_port: &dyn ImageGenerationHostPort,
        workflow: &ImageGenerationWorkflowScope,
        project_id: i64,
    ) -> Result<Option<generated_image::Model>, AppError> {
        let Some(checkpoint) = self
            .load_completed_workflow_checkpoint(
                Some(workflow.workflow_run_id),
                &workflow.step_key,
                &workflow.fingerprint_json,
            )
            .await?
        else {
            return Ok(None);
        };
        let checkpoint_json = required_checkpoint_json(&checkpoint, &workflow.step_key)
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        let image_id =
            required_checkpoint_field(&checkpoint_json, &workflow.step_key, "generated_image_id")
                .map_err(|error| AppError::InvalidArgument(error.to_string()))?
                .as_i64()
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "{}.generated_image_id 必须是整数",
                        workflow.step_key
                    ))
                })?;
        let checkpoint_file_path =
            required_checkpoint_field(&checkpoint_json, &workflow.step_key, "file_path")
                .map_err(|error| AppError::InvalidArgument(error.to_string()))?
                .as_str()
                .map(ToString::to_string)
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "{}.file_path 必须是字符串",
                        workflow.step_key
                    ))
                })?;
        let image = self.generated_image_by_id(image_id).await?.ok_or_else(|| {
            AppError::NotFound(format!(
                "{} checkpoint 指向的图片记录不存在: {image_id}",
                workflow.step_key
            ))
        })?;
        if image.project_id != project_id {
            return Err(AppError::InvalidArgument(format!(
                "{} checkpoint 指向其他项目图片: image_project_id={}",
                workflow.step_key, image.project_id
            )));
        }
        if image.file_path != checkpoint_file_path {
            return Err(AppError::InvalidArgument(format!(
                "{} checkpoint 图片路径不一致: checkpoint={}, db={}",
                workflow.step_key, checkpoint_file_path, image.file_path
            )));
        }
        host_port.validate_generated_image_file(&image.file_path, "断点图片文件")?;
        Ok(Some(image))
    }

    async fn complete_image_generation_post_commit(
        &self,
        host_port: &dyn ImageGenerationHostPort,
        input: &ImageGenerationExecutionInput,
        outcome: &ImageGenerationExecutionOutcome,
    ) -> Result<(), AppError> {
        let ImageGenerationPostCommitAction::MangaPage {
            page_prompt_id,
            page_number,
        } = &input.post_commit
        else {
            return Ok(());
        };
        let chapter_id = input.chapter_id.ok_or_else(|| {
            AppError::InvalidArgument("漫画页图片完成缺少 chapter_id".to_string())
        })?;
        self.mark_manga_page_prompt_image_completed_with_image(
            *page_prompt_id,
            outcome.summary.id,
            outcome.created_at,
        )
        .await?;
        let old_paths = self
            .delete_manga_page_image_records(
                input.project_id,
                chapter_id,
                *page_number,
                Some(outcome.summary.id),
            )
            .await?;
        for path in old_paths {
            host_port
                .remove_generated_image_file(&path, "旧漫画页图片")
                .await?;
        }
        Ok(())
    }
}

struct ImageGenerationTrace<'a> {
    command: &'a str,
    config: &'a a4n_core::models::ai::image_gen_config::Model,
    image_type: &'a str,
    prompt: &'a str,
    provider_model: &'a str,
    chapter_id: Option<i64>,
    panel_id: Option<&'a str>,
    width: i32,
    height: i32,
    generated_at: chrono::NaiveDateTime,
    runtime_config_snapshot: Option<Value>,
}

fn attach_image_generation_trace(
    metadata_json: Option<Value>,
    trace: ImageGenerationTrace<'_>,
) -> Result<Option<Value>, AppError> {
    let generation = json!({
        "command": trace.command,
        "config_id": trace.config.id,
        "config_name": trace.config.name,
        "backend": trace.config.backend,
        "base_url": trace.config.base_url,
        "configured_model": trace.config.model,
        "provider_model": trace.provider_model,
        "image_type": trace.image_type,
        "chapter_id": trace.chapter_id,
        "panel_id": trace.panel_id,
        "width": trace.width,
        "height": trace.height,
        "prompt": trace.prompt,
        "generated_at": trace.generated_at.to_string(),
        "runtime_config_snapshot": trace.runtime_config_snapshot,
    });
    let mut metadata = match metadata_json {
        Some(Value::Object(map)) => map,
        Some(_) => {
            return Err(AppError::InvalidArgument(
                "图片生成 metadata_json 必须是对象".to_string(),
            ));
        }
        None => serde_json::Map::new(),
    };
    metadata.insert("generation".to_string(), generation);
    metadata
        .entry("metadata_schema_version".to_string())
        .or_insert_with(|| json!(1));
    Ok(Some(Value::Object(metadata)))
}

fn image_generation_media_refs(metadata_json: &Value) -> Option<Value> {
    let references = metadata_json
        .get("generation_context")
        .and_then(|context| context.get("reference_image_ids"))
        .or_else(|| metadata_json.get("reference_image_ids"))
        .cloned();
    let bindings = metadata_json
        .get("reference_bindings")
        .cloned()
        .or_else(|| {
            metadata_json
                .get("generation_context")
                .and_then(|context| context.get("reference_bindings").cloned())
        });
    match (references, bindings) {
        (Some(references), Some(bindings)) => Some(json!({
            "reference_image_ids": references,
            "reference_bindings": bindings,
        })),
        (Some(references), None) => Some(json!({"reference_image_ids": references})),
        (None, Some(bindings)) => Some(json!({"reference_bindings": bindings})),
        (None, None) => None,
    }
}

fn generation_trace_input(
    input: &ImageGenerationExecutionInput,
    output: &super::contracts::ImageGenerationProviderOutput,
    request_json: Value,
    media_refs_json: Option<Value>,
    workflow_run_id: i64,
    file_size: i64,
) -> Result<RecordCompletedGenerationRunWithCheckpointInput, AppError> {
    let call_key = format!(
        "{}:{}:chapter:{}:panel:{}",
        input.command,
        input.image_type,
        input
            .chapter_id
            .map(|value| value.to_string())
            .unwrap_or_else(|| "none".to_string()),
        input.panel_id.as_deref().unwrap_or("none")
    );
    let input_fingerprint_json = json!({
        "command": input.command,
        "project_id": input.project_id,
        "chapter_id": input.chapter_id,
        "panel_id": input.panel_id,
        "image_type": input.image_type,
        "prompt": input.request.prompt,
        "width": input.width,
        "height": input.height,
        "provider": input.config.backend,
        "model": output.model,
        "request": request_json,
        "media_refs": media_refs_json,
    });
    let input_fingerprint_sha256 =
        generation_checkpoint_fingerprint_sha256(&input_fingerprint_json)
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
    Ok(RecordCompletedGenerationRunWithCheckpointInput {
        run: RecordCompletedGenerationRunInput {
            project_id: input.project_id,
            workflow: "image_generation".to_string(),
            step_key: input.command.clone(),
            title: format!("{} 图片 provider 生成", input.image_type),
            model_kind: "image".to_string(),
            provider: Some(input.config.backend.clone()),
            model: Some(output.model.clone()),
            prompt_template_names_json: None,
            prompt_template_hashes_json: None,
            rendered_messages_json: None,
            rendered_prompts_json: Some(json!({"prompt": input.request.prompt})),
            input_variables_json: None,
            source_refs_json: None,
            media_refs_json,
            request_json: input_fingerprint_json.get("request").cloned(),
            response_summary_json: Some(json!({
                "format": output.format,
                "file_size": file_size,
            })),
            targets: Vec::new(),
        },
        checkpoint: GenerationRunCheckpointContext {
            workflow_run_id: Some(workflow_run_id),
            call_key: Some(call_key),
            input_fingerprint_sha256: Some(input_fingerprint_sha256),
            input_fingerprint_json: Some(input_fingerprint_json),
            raw_response_json: Some(json!({
                "format": output.format,
                "model": output.model,
                "file_size": file_size,
            })),
            parsed_output_json: None,
            validation_json: None,
            artifact_refs_json: None,
            attempt: None,
        },
    })
}

pub fn generated_image_summary(image: &generated_image::Model) -> GeneratedImageSummary {
    GeneratedImageSummary {
        id: image.id,
        project_id: image.project_id,
        image_type: image.image_type.clone(),
        file_path: image.file_path.clone(),
        prompt: image.prompt.clone(),
        model: image.model.clone(),
        width: image.width,
        height: image.height,
        file_size: image.file_size,
        ai_score: image.ai_score,
        is_favorite: image.is_favorite,
        is_system_managed: a4n_domain::image::is_system_managed_image_type(&image.image_type),
        chapter_id: image.chapter_id,
        panel_id: image.panel_id.clone(),
        metadata_json: image.metadata_json.clone(),
        created_at: image.created_at.to_string(),
    }
}

async fn cleanup_created_file_after_error(
    host_port: &dyn ImageGenerationHostPort,
    relative_path: &str,
    label: &str,
    error: AppError,
) -> AppError {
    match host_port
        .remove_generated_image_file(relative_path, label)
        .await
    {
        Ok(()) => error,
        Err(cleanup_error) => crate::errors::app_error_with_secondary(
            error,
            format!("清理{label}失败: {cleanup_error}"),
        ),
    }
}

fn validate_execution_input(input: &ImageGenerationExecutionInput) -> Result<(), AppError> {
    require_positive(input.project_id, "project_id")?;
    if let Some(chapter_id) = input.chapter_id {
        require_positive(chapter_id, "chapter_id")?;
    }
    required_text(&input.command, "图片生成 command")?;
    required_text(&input.image_type, "图片类型")?;
    required_text(&input.file_context, "图片文件上下文")?;
    required_text(&input.request.prompt, "图片提示词")?;
    require_positive(i64::from(input.width), "图片宽度")?;
    require_positive(i64::from(input.height), "图片高度")?;
    if input.request.width != input.width || input.request.height != input.height {
        return Err(AppError::InvalidArgument(
            "图片生成请求尺寸与执行尺寸不一致".to_string(),
        ));
    }
    let supported_sizes = image_sizes_from_json(&input.config.supported_sizes_json)?;
    ensure_image_size_supported(&supported_sizes, input.width, input.height)?;
    if let ImageGenerationPostCommitAction::MangaPage {
        page_prompt_id,
        page_number,
    } = &input.post_commit
    {
        require_positive(*page_prompt_id, "page_prompt_id")?;
        require_positive(i64::from(*page_number), "page_number")?;
        if input.chapter_id.is_none() {
            return Err(AppError::InvalidArgument(
                "漫画页图片完成缺少 chapter_id".to_string(),
            ));
        }
    }
    Ok(())
}

fn require_positive(value: i64, label: &str) -> Result<(), AppError> {
    if value <= 0 {
        return Err(AppError::InvalidArgument(format!("{label}必须是正整数")));
    }
    Ok(())
}

fn required_text<'a>(value: &'a str, label: &str) -> Result<&'a str, AppError> {
    let value = value.trim();
    if value.is_empty() {
        Err(AppError::InvalidArgument(format!("{label}不能为空")))
    } else {
        Ok(value)
    }
}
