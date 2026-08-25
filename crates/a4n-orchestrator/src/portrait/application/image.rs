use a4n_core::{
    error::AppError,
    models::character::{character_portrait, protagonist_profile},
};
use a4n_db::generation::{
    GenerationRunCheckpointContext, GenerationRunTraceTarget, RecordCompletedGenerationRunInput,
    RecordCompletedGenerationRunWithCheckpointInput, generation_checkpoint_fingerprint_sha256,
};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use crate::{
    OrchestratorContext, character::CharacterPortraitCreateInput,
    portrait::validate_reference_image_ids, video::CreateGeneratedImageRecordDraftInput,
};

use super::{
    contracts::{
        GeneratedPortraitImage, PersistPortraitFileInput, PortraitGenerationHostPort,
        PortraitImageProviderInput, PortraitReferenceImageSource, PortraitSummary, PortraitType,
        UploadPortraitInput,
    },
    workflow::InitialPortraitPrompt,
};

pub(super) struct PersistGeneratedPortraitInput<'a> {
    pub(super) workflow_run_id: Option<i64>,
    pub(super) profile_id: i64,
    pub(super) project_id: i64,
    pub(super) portrait_type: PortraitType,
    pub(super) prompt: &'a str,
    pub(super) expression: Option<&'a str>,
    pub(super) pose: Option<&'a str>,
    pub(super) character_name: &'a str,
    pub(super) variant_kind: Option<&'a str>,
    pub(super) auto_generated: bool,
    pub(super) path_context: &'a str,
    pub(super) reference_image_ids: &'a [i64],
}

struct PortraitMirrorInput<'a> {
    workflow_run_id: Option<i64>,
    project_id: i64,
    profile_id: i64,
    portrait_id: i64,
    character_name: &'a str,
    portrait_type: &'a str,
    image_path: &'a str,
    prompt: &'a str,
    model: &'a str,
    width: i32,
    height: i32,
    file_size: i64,
    expression: Option<&'a str>,
    pose: Option<&'a str>,
    variant_kind: Option<&'a str>,
    auto_generated: bool,
    reference_image_ids: &'a [i64],
    provider_trace: Option<PortraitProviderTrace<'a>>,
}

struct PortraitProviderTrace<'a> {
    provider: &'a str,
    request_json: &'a Value,
    output_format: &'a str,
}

pub(super) async fn generate_portrait_image(
    context: &OrchestratorContext,
    host_port: &dyn PortraitGenerationHostPort,
    project_id: i64,
    prompt: &str,
    reference_image_ids: &[i64],
    workflow_run_id: Option<i64>,
    cancel_token: CancellationToken,
) -> Result<GeneratedPortraitImage, AppError> {
    let image_ids =
        validate_reference_image_ids(reference_image_ids).map_err(AppError::InvalidArgument)?;
    let records = context
        .portrait_reference_images_by_ids(project_id, &image_ids)
        .await?;
    let reference_images = records
        .into_iter()
        .enumerate()
        .map(|(index, image)| PortraitReferenceImageSource {
            label: format!("参考图 {}", index + 1),
            file_path: image.file_path,
        })
        .collect();
    host_port
        .generate_image(
            PortraitImageProviderInput {
                project_id,
                prompt: prompt.to_string(),
                reference_images,
                workflow_run_id,
            },
            cancel_token,
        )
        .await
}

pub(super) async fn persist_generated_portrait(
    context: &OrchestratorContext,
    host_port: &dyn PortraitGenerationHostPort,
    generated: &GeneratedPortraitImage,
    input: PersistGeneratedPortraitInput<'_>,
) -> Result<character_portrait::Model, AppError> {
    let persisted = host_port.persist_portrait_file(PersistPortraitFileInput {
        project_id: input.project_id,
        profile_id: input.profile_id,
        bytes: &generated.bytes,
        format: &generated.format,
        path_context: input.path_context,
    })?;
    let portrait = match context
        .create_character_portrait_for_profile(CharacterPortraitCreateInput {
            profile_id: input.profile_id,
            project_id: input.project_id,
            portrait_type: input.portrait_type.to_string(),
            image_path: persisted.relative_path.clone(),
            prompt: input.prompt.to_string(),
            expression: input.expression.map(str::to_string),
            pose: input.pose.map(str::to_string),
        })
        .await
    {
        Ok(portrait) => portrait,
        Err(error) => {
            return Err(cleanup_created_file_after_error(
                host_port,
                &persisted.relative_path,
                "立绘图片",
                error,
            )
            .await);
        }
    };
    let mirror_result = mirror_portrait(
        context,
        PortraitMirrorInput {
            workflow_run_id: input.workflow_run_id,
            project_id: input.project_id,
            profile_id: input.profile_id,
            portrait_id: portrait.id,
            character_name: input.character_name,
            portrait_type: input.portrait_type.as_str(),
            image_path: &persisted.relative_path,
            prompt: input.prompt,
            model: &generated.model,
            width: generated.width,
            height: generated.height,
            file_size: persisted.file_size,
            expression: input.expression,
            pose: input.pose,
            variant_kind: input.variant_kind,
            auto_generated: input.auto_generated,
            reference_image_ids: input.reference_image_ids,
            provider_trace: Some(PortraitProviderTrace {
                provider: &generated.provider,
                request_json: &generated.request_json,
                output_format: &generated.format,
            }),
        },
    )
    .await;
    if let Err(error) = mirror_result {
        return Err(rollback_portrait_creation(
            context,
            host_port,
            portrait.id,
            &persisted.relative_path,
            error,
        )
        .await);
    }
    Ok(portrait)
}

pub(super) async fn upload_portrait(
    context: &OrchestratorContext,
    host_port: &dyn PortraitGenerationHostPort,
    input: UploadPortraitInput,
) -> Result<PortraitSummary, AppError> {
    let profile = context
        .portrait_prompt_profile_rows(input.project_id, input.profile_id)
        .await?
        .profile;
    let prompt = required_text(&input.prompt, "立绘描述")?;
    let decoded =
        host_port.decode_uploaded_image(&input.file_name, &input.mime_type, &input.data_base64)?;
    let persisted = host_port.persist_portrait_file(PersistPortraitFileInput {
        project_id: input.project_id,
        profile_id: input.profile_id,
        bytes: &decoded.bytes,
        format: &decoded.format,
        path_context: "上传立绘图片",
    })?;
    let portrait = match context
        .create_character_portrait_for_profile(CharacterPortraitCreateInput {
            profile_id: input.profile_id,
            project_id: input.project_id,
            portrait_type: input.portrait_type.to_string(),
            image_path: persisted.relative_path.clone(),
            prompt: prompt.clone(),
            expression: input.expression,
            pose: input.pose,
        })
        .await
    {
        Ok(portrait) => portrait,
        Err(error) => {
            return Err(cleanup_created_file_after_error(
                host_port,
                &persisted.relative_path,
                "上传立绘图片",
                error,
            )
            .await);
        }
    };
    if let Err(error) = mirror_portrait(
        context,
        PortraitMirrorInput {
            workflow_run_id: None,
            project_id: input.project_id,
            profile_id: input.profile_id,
            portrait_id: portrait.id,
            character_name: &profile.name,
            portrait_type: &portrait.portrait_type,
            image_path: &persisted.relative_path,
            prompt: &prompt,
            model: &decoded.model,
            width: decoded.width,
            height: decoded.height,
            file_size: persisted.file_size,
            expression: portrait.expression.as_deref(),
            pose: portrait.pose.as_deref(),
            variant_kind: Some("manual_upload"),
            auto_generated: false,
            reference_image_ids: &[],
            provider_trace: None,
        },
    )
    .await
    {
        return Err(rollback_portrait_creation(
            context,
            host_port,
            portrait.id,
            &persisted.relative_path,
            error,
        )
        .await);
    }
    if input.set_primary && !portrait.is_primary {
        context
            .set_primary_character_portrait_record(portrait.id)
            .await
            .map_err(|error| AppError::InvalidState {
                current: "portrait_uploaded".to_string(),
                operation: format!("设置主图失败: {error}"),
            })?;
        let updated = required_portrait(context, portrait.id).await?;
        return portrait_summary(&updated);
    }
    portrait_summary(&portrait)
}

pub(super) async fn record_initial_prompt_generation(
    context: &OrchestratorContext,
    host_port: &dyn PortraitGenerationHostPort,
    portrait: &character_portrait::Model,
    input: &super::contracts::GeneratePortraitInput,
    profile: &protagonist_profile::Model,
    expression: &str,
    workflow_run_id: i64,
    workflow_fingerprint: Value,
    prompt: InitialPortraitPrompt,
) -> Result<(), AppError> {
    let request_json = serde_json::to_value(&prompt.request)?;
    let messages_json = serde_json::to_value(&prompt.request.messages)?;
    let input_variables_json = json!({
        "profile_id": profile.id,
        "character_name": profile.name,
        "portrait_type": "full_body",
        "expression": expression,
        "pose": input.pose,
        "style": input.style,
        "reference_image_ids": input.reference_image_ids,
    });
    let source_refs_json = json!({
        "profile_id": profile.id,
        "appearance": profile.appearance,
        "gender": profile.gender,
        "age": profile.age,
        "personality": profile.personality,
        "background": profile.background,
    });
    let media_refs_json = json!({"reference_image_ids": input.reference_image_ids});
    let input_fingerprint_json = json!({
        "prompt_name": "character_portrait_generation",
        "project_id": input.project_id,
        "request": request_json,
        "input_variables": input_variables_json,
        "source_refs": source_refs_json,
        "media_refs": media_refs_json,
    });
    let record_result = context
        .record_completed_generation_run_with_checkpoint(
            RecordCompletedGenerationRunWithCheckpointInput {
                run: RecordCompletedGenerationRunInput {
                    project_id: input.project_id,
                    workflow: "portrait_generation".to_string(),
                    step_key: "generate_portrait_prompt".to_string(),
                    title: "人物立绘提示词生成".to_string(),
                    model_kind: "llm".to_string(),
                    provider: None,
                    model: None,
                    prompt_template_names_json: Some(json!(["character_portrait_generation"])),
                    prompt_template_hashes_json: None,
                    rendered_messages_json: Some(messages_json),
                    rendered_prompts_json: None,
                    input_variables_json: Some(input_variables_json),
                    source_refs_json: Some(source_refs_json),
                    media_refs_json: Some(media_refs_json),
                    request_json: Some(request_json),
                    response_summary_json: Some(json!({
                        "character_portrait_id": portrait.id,
                        "prompt_chars": prompt.image_prompt.chars().count(),
                        "image_path": portrait.image_path,
                    })),
                    targets: vec![GenerationRunTraceTarget {
                        target_kind: "character_portrait".to_string(),
                        target_table: "character_portrait".to_string(),
                        target_id: portrait.id,
                    }],
                },
                checkpoint: GenerationRunCheckpointContext {
                    workflow_run_id: Some(workflow_run_id),
                    call_key: Some(format!(
                        "character_portrait_generation:profile:{}:full_body",
                        profile.id
                    )),
                    input_fingerprint_sha256: Some(
                        generation_checkpoint_fingerprint_sha256(&input_fingerprint_json)
                            .map_err(|error| AppError::InvalidArgument(error.to_string()))?,
                    ),
                    input_fingerprint_json: Some(input_fingerprint_json),
                    raw_response_json: Some(prompt.raw_response_json),
                    parsed_output_json: Some(json!({
                        "character_portrait_id": portrait.id,
                        "profile_id": profile.id,
                        "portrait_type": "full_body",
                        "image_prompt": prompt.image_prompt,
                        "image_path": portrait.image_path,
                    })),
                    validation_json: None,
                    artifact_refs_json: Some(json!({
                        "targets": [{
                            "target_kind": "character_portrait",
                            "target_table": "character_portrait",
                            "target_id": portrait.id,
                        }],
                    })),
                    attempt: None,
                },
            },
        )
        .await;
    if let Err(error) = record_result {
        return Err(rollback_portrait_creation(
            context,
            host_port,
            portrait.id,
            &portrait.image_path,
            error,
        )
        .await);
    }
    let summary = portrait_summary(portrait)?;
    super::workflow::save_completed_portrait_checkpoint(
        context,
        workflow_run_id,
        "portrait_generation.final".to_string(),
        2,
        workflow_fingerprint,
        &summary,
    )
    .await
}

pub(super) async fn required_portrait(
    context: &OrchestratorContext,
    portrait_id: i64,
) -> Result<character_portrait::Model, AppError> {
    context
        .character_portrait_by_id(portrait_id)
        .await?
        .ok_or_else(|| AppError::NotFound("立绘不存在".to_string()))
}

pub(super) async fn required_portrait_profile(
    context: &OrchestratorContext,
    profile_id: i64,
    portrait_id: i64,
) -> Result<protagonist_profile::Model, AppError> {
    context.profile_by_id(profile_id).await?.ok_or_else(|| {
        AppError::NotFound(format!("立绘 #{portrait_id} 缺少角色档案 #{profile_id}"))
    })
}

pub fn portrait_summary(portrait: &character_portrait::Model) -> Result<PortraitSummary, AppError> {
    Ok(PortraitSummary {
        id: portrait.id,
        profile_id: portrait.profile_id,
        project_id: portrait.project_id,
        portrait_type: portrait_type_from_record(portrait)?,
        image_path: portrait.image_path.clone(),
        prompt: portrait.prompt.clone(),
        expression: portrait.expression.clone(),
        pose: portrait.pose.clone(),
        is_primary: portrait.is_primary,
        created_at: portrait.created_at.to_string(),
    })
}

pub(super) fn portrait_type_from_record(
    portrait: &character_portrait::Model,
) -> Result<PortraitType, AppError> {
    PortraitType::try_from(portrait.portrait_type.as_str()).map_err(|error| {
        AppError::InvalidState {
            current: portrait.portrait_type.clone(),
            operation: format!("读取立绘 #{}：{error}", portrait.id),
        }
    })
}

async fn mirror_portrait(
    context: &OrchestratorContext,
    input: PortraitMirrorInput<'_>,
) -> Result<(), AppError> {
    let now = chrono::Utc::now().naive_utc();
    let metadata_json = Some(json!({
        "metadata_schema_version": 1,
        "portrait_id": input.portrait_id,
        "profile_id": input.profile_id,
        "character_name": input.character_name,
        "portrait_type": input.portrait_type,
        "expression": input.expression,
        "pose": input.pose,
        "variant_kind": input.variant_kind,
        "auto_generated": input.auto_generated,
        "reference_image_ids": input.reference_image_ids,
        "image_type": "portrait",
        "generation": {
            "command": "mirror_portrait_to_generated_image",
            "image_type": "portrait",
            "provider_model": input.model,
            "width": input.width,
            "height": input.height,
            "prompt": input.prompt,
            "reference_image_ids": input.reference_image_ids,
            "generated_at": now.to_string(),
        },
    }));
    let image_record = CreateGeneratedImageRecordDraftInput {
        project_id: input.project_id,
        image_type: "portrait".to_string(),
        file_path: input.image_path.to_string(),
        prompt: input.prompt.to_string(),
        model: input.model.to_string(),
        width: input.width,
        height: input.height,
        file_size: Some(input.file_size),
        chapter_id: None,
        panel_id: None,
        metadata_json,
        created_at: Some(now),
    };
    let Some(trace) = input.provider_trace else {
        context.create_generated_image_record(image_record).await?;
        return Ok(());
    };
    let media_refs_json = (!input.reference_image_ids.is_empty())
        .then(|| json!({"reference_image_ids": input.reference_image_ids}));
    let input_variables_json = json!({
        "profile_id": input.profile_id,
        "character_portrait_id": input.portrait_id,
        "portrait_type": input.portrait_type,
        "expression": input.expression,
        "pose": input.pose,
        "variant_kind": input.variant_kind,
        "auto_generated": input.auto_generated,
    });
    let source_refs_json = json!({
        "character_portrait_id": input.portrait_id,
        "profile_id": input.profile_id,
    });
    let input_fingerprint_json = json!({
        "command": "generate_portrait_image",
        "project_id": input.project_id,
        "profile_id": input.profile_id,
        "portrait_type": input.portrait_type,
        "expression": input.expression,
        "pose": input.pose,
        "variant_kind": input.variant_kind,
        "auto_generated": input.auto_generated,
        "prompt": input.prompt,
        "width": input.width,
        "height": input.height,
        "provider": trace.provider,
        "model": input.model,
        "request": trace.request_json,
        "media_refs": media_refs_json,
    });
    context
        .create_generated_image_record_with_generation_run_checkpoint(
            image_record,
            RecordCompletedGenerationRunWithCheckpointInput {
                run: RecordCompletedGenerationRunInput {
                    project_id: input.project_id,
                    workflow: "portrait_generation".to_string(),
                    step_key: "generate_portrait_image".to_string(),
                    title: "人物立绘图片生成".to_string(),
                    model_kind: "image".to_string(),
                    provider: Some(trace.provider.to_string()),
                    model: Some(input.model.to_string()),
                    prompt_template_names_json: None,
                    prompt_template_hashes_json: None,
                    rendered_messages_json: None,
                    rendered_prompts_json: Some(json!({"prompt": input.prompt})),
                    input_variables_json: Some(input_variables_json),
                    source_refs_json: Some(source_refs_json),
                    media_refs_json,
                    request_json: Some(trace.request_json.clone()),
                    response_summary_json: Some(json!({
                        "character_portrait_id": input.portrait_id,
                        "format": trace.output_format,
                        "file_size": input.file_size,
                    })),
                    targets: vec![GenerationRunTraceTarget {
                        target_kind: "character_portrait".to_string(),
                        target_table: "character_portrait".to_string(),
                        target_id: input.portrait_id,
                    }],
                },
                checkpoint: GenerationRunCheckpointContext {
                    workflow_run_id: input.workflow_run_id,
                    call_key: Some(format!(
                        "generate_portrait_image:profile:{}:{}:{}:{}",
                        input.profile_id,
                        input.portrait_type,
                        input.expression.unwrap_or("none"),
                        input.pose.unwrap_or("none")
                    )),
                    input_fingerprint_sha256: Some(
                        generation_checkpoint_fingerprint_sha256(&input_fingerprint_json)
                            .map_err(|error| AppError::InvalidArgument(error.to_string()))?,
                    ),
                    input_fingerprint_json: Some(input_fingerprint_json),
                    raw_response_json: Some(json!({
                        "format": trace.output_format,
                        "model": input.model,
                        "file_size": input.file_size,
                    })),
                    parsed_output_json: Some(json!({
                        "character_portrait_id": input.portrait_id,
                        "profile_id": input.profile_id,
                        "portrait_type": input.portrait_type,
                        "image_path": input.image_path,
                        "width": input.width,
                        "height": input.height,
                    })),
                    validation_json: None,
                    artifact_refs_json: Some(json!({
                        "targets": [{
                            "target_kind": "character_portrait",
                            "target_table": "character_portrait",
                            "target_id": input.portrait_id,
                        }],
                    })),
                    attempt: None,
                },
            },
        )
        .await?;
    Ok(())
}

async fn rollback_portrait_creation(
    context: &OrchestratorContext,
    host_port: &dyn PortraitGenerationHostPort,
    portrait_id: i64,
    image_path: &str,
    original_error: AppError,
) -> AppError {
    let rollback = context
        .rollback_failed_portrait_creation_records(portrait_id, image_path)
        .await;
    match rollback {
        Ok(rollback) => {
            if rollback.delete_file
                && let Err(cleanup_error) =
                    host_port.remove_portrait_file(image_path, "立绘图片").await
            {
                return combined_error(original_error, cleanup_error);
            }
            original_error
        }
        Err(cleanup_error) => combined_error(original_error, cleanup_error),
    }
}

async fn cleanup_created_file_after_error(
    host_port: &dyn PortraitGenerationHostPort,
    image_path: &str,
    label: &str,
    original_error: AppError,
) -> AppError {
    match host_port.remove_portrait_file(image_path, label).await {
        Ok(()) => original_error,
        Err(cleanup_error) => combined_error(original_error, cleanup_error),
    }
}

fn combined_error(original: AppError, cleanup: AppError) -> AppError {
    crate::errors::app_error_with_secondary(
        original,
        format!("失败后的回滚/文件清理也失败: {cleanup}"),
    )
}

fn required_text(value: &str, field_name: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::InvalidArgument(format!("{field_name}不能为空")));
    }
    Ok(value.to_string())
}
