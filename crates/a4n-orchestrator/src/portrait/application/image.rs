use a4n_api_contract::portrait::{
    GeneratePortraitInput, PortraitSummary, PortraitType, UploadPortraitInput,
};
use a4n_db::generation::{
    GenerationRunCheckpointContext, GenerationRunTraceTarget, RecordCompletedGenerationRunInput,
    RecordCompletedGenerationRunWithCheckpointInput, generation_checkpoint_fingerprint_sha256,
};
use a4n_db::video::{
    CreateGeneratedImageRecordDraftInput, CreateImagePublicationIntentInput,
    ImagePublicationFileFacts,
};
use a4n_db_models::models::{
    character::{character_portrait, protagonist_profile},
    media::image_publication,
};
use a4n_domain::media::{IMAGE_PUBLICATION_STATUS_PENDING, IMAGE_PUBLICATION_STATUS_PUBLISHED};
use a4n_domain::{character::CharacterPortraitCreateInput, image::validate_reference_image_ids};
use a4n_error::AppError;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    OrchestratorContext,
    collaboration::ProjectAccessRole,
    media::{PersistGeneratedImageFileInput, PrepareGeneratedImageFileInput},
};

use super::{
    contracts::{
        GeneratedPortraitImage, PortraitGenerationHostPort, PortraitImageProviderInput,
        PortraitReferenceImageSource,
    },
    workflow::InitialPortraitPrompt,
};

pub(super) struct PersistGeneratedPortraitInput<'a> {
    pub(super) workflow_run_id: Option<i64>,
    pub(super) requested_by_user_id: i64,
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

struct PortraitPublicationInput<'a> {
    workflow_run_id: Option<i64>,
    requested_by_user_id: i64,
    project_id: i64,
    profile_id: i64,
    character_name: &'a str,
    portrait_type: &'a str,
    prompt: &'a str,
    expression: Option<&'a str>,
    pose: Option<&'a str>,
    variant_kind: Option<&'a str>,
    auto_generated: bool,
    reference_image_ids: &'a [i64],
    bytes: &'a [u8],
    format: &'a str,
    model: &'a str,
    width: i32,
    height: i32,
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
    width: i32,
    height: i32,
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
                width,
                height,
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
    publish_portrait_image(
        context,
        host_port,
        PortraitPublicationInput {
            workflow_run_id: input.workflow_run_id,
            requested_by_user_id: input.requested_by_user_id,
            project_id: input.project_id,
            profile_id: input.profile_id,
            character_name: input.character_name,
            portrait_type: input.portrait_type.as_str(),
            prompt: input.prompt,
            expression: input.expression,
            pose: input.pose,
            variant_kind: input.variant_kind,
            auto_generated: input.auto_generated,
            reference_image_ids: input.reference_image_ids,
            bytes: &generated.bytes,
            format: &generated.format,
            model: &generated.model,
            width: generated.width,
            height: generated.height,
            provider_trace: Some(PortraitProviderTrace {
                provider: &generated.provider,
                request_json: &generated.request_json,
                output_format: &generated.format,
            }),
        },
        input.path_context,
    )
    .await
}

pub(super) async fn upload_portrait(
    context: &OrchestratorContext,
    host_port: &dyn PortraitGenerationHostPort,
    requested_by_user_id: i64,
    input: UploadPortraitInput,
) -> Result<PortraitSummary, AppError> {
    let profile = context
        .portrait_prompt_profile_rows(input.project_id, input.profile_id)
        .await?
        .profile;
    let prompt = required_text(&input.prompt, "立绘描述")?;
    let decoded =
        host_port.decode_uploaded_image(&input.file_name, &input.mime_type, &input.data_base64)?;
    let portrait = publish_portrait_image(
        context,
        host_port,
        PortraitPublicationInput {
            workflow_run_id: None,
            requested_by_user_id,
            project_id: input.project_id,
            profile_id: input.profile_id,
            character_name: &profile.name,
            portrait_type: input.portrait_type.as_str(),
            prompt: &prompt,
            expression: input.expression.as_deref(),
            pose: input.pose.as_deref(),
            variant_kind: Some("manual_upload"),
            auto_generated: false,
            reference_image_ids: &[],
            bytes: &decoded.bytes,
            format: &decoded.format,
            model: &decoded.model,
            width: decoded.width,
            height: decoded.height,
            provider_trace: None,
        },
        "上传立绘图片",
    )
    .await?;
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

async fn publish_portrait_image(
    context: &OrchestratorContext,
    host_port: &dyn PortraitGenerationHostPort,
    input: PortraitPublicationInput<'_>,
    file_context: &str,
) -> Result<character_portrait::Model, AppError> {
    let file_size = i64::try_from(input.bytes.len())
        .map_err(|_| AppError::InvalidArgument("立绘图片过大，无法记录文件大小".to_string()))?;
    let expected_sha256 = format!("{:x}", Sha256::digest(input.bytes));
    let publication_id = portrait_publication_id(&input);
    let file_extra = format!("profile_{}", input.profile_id);
    let prepared = host_port.prepare_portrait_file(PrepareGeneratedImageFileInput {
        project_id: input.project_id,
        chapter_id: None,
        image_type: "portrait",
        file_extra: Some(&file_extra),
        format: input.format,
        publication_id: &publication_id,
        file_context,
    })?;
    let (publication, _publication_guard) = context
        .create_image_publication_intent_for_user_validated(
            input.requested_by_user_id,
            CreateImagePublicationIntentInput {
                publication_id: publication_id.clone(),
                project_id: input.project_id,
                requested_by_user_id: input.requested_by_user_id,
                staging_path: prepared.staging_path.clone(),
                final_path: prepared.final_path.clone(),
                image: CreateGeneratedImageRecordDraftInput {
                    project_id: input.project_id,
                    image_type: "portrait".to_string(),
                    file_path: prepared.final_path.clone(),
                    prompt: input.prompt.to_string(),
                    model: input.model.to_string(),
                    width: input.width,
                    height: input.height,
                    file_size: Some(file_size),
                    chapter_id: None,
                    panel_id: None,
                    metadata_json: Some(portrait_publication_metadata(&input)),
                    created_at: Some(chrono::Utc::now().naive_utc()),
                },
                expected_file_size: file_size,
                expected_sha256: expected_sha256.clone(),
            },
        )
        .await?;

    if publication.status == IMAGE_PUBLICATION_STATUS_PUBLISHED {
        host_port.validate_portrait_file(&publication.final_path, "已发布立绘图片")?;
        return context
            .ensure_portrait_publication_identity_for_user_validated(
                input.requested_by_user_id,
                &publication,
            )
            .await;
    }
    if publication.status != IMAGE_PUBLICATION_STATUS_PENDING {
        return Err(AppError::InvalidState {
            current: publication.status,
            operation: "写入立绘图片".to_string(),
        });
    }

    let persisted = match host_port.persist_portrait_file(
        PersistGeneratedImageFileInput {
            project_id: input.project_id,
            chapter_id: None,
            image_type: "portrait",
            file_extra: Some(&file_extra),
            bytes: input.bytes,
            format: input.format,
            file_context,
        },
        &prepared,
    ) {
        Ok(persisted) => persisted,
        Err(error) => {
            return Err(cleanup_staging_after_error(
                host_port,
                &prepared.staging_path,
                file_context,
                error,
            )
            .await);
        }
    };
    let portrait = context
        .ensure_portrait_publication_identity_for_user_validated(
            input.requested_by_user_id,
            &publication,
        )
        .await?;
    let facts = ImagePublicationFileFacts {
        file_size: persisted.file_size,
        sha256: expected_sha256,
    };
    let generation_run = portrait_generation_run(
        &input,
        &portrait,
        &publication.final_path,
        persisted.file_size,
    )?;
    let publish_result = match generation_run {
        Some(generation_run) => {
            context
                .publish_image_publication_for_user_validated(
                    input.requested_by_user_id,
                    input.project_id,
                    &publication_id,
                    facts,
                    Some(generation_run),
                )
                .await
        }
        None => {
            context
                .publish_image_publication_for_user_validated(
                    input.requested_by_user_id,
                    input.project_id,
                    &publication_id,
                    facts,
                    None,
                )
                .await
        }
    };
    if let Err(error) = publish_result {
        return Err(cleanup_staging_after_error(
            host_port,
            &prepared.staging_path,
            file_context,
            error,
        )
        .await);
    }
    if let Err(error) = host_port
        .remove_portrait_file(&prepared.staging_path, "立绘暂存文件")
        .await
    {
        tracing::warn!(
            error = %error,
            publication_id = %publication_id,
            "立绘发布成功但暂存文件清理意图登记失败"
        );
    }
    Ok(portrait)
}

impl OrchestratorContext {
    pub async fn ensure_portrait_publication_identity_for_user_validated(
        &self,
        user_id: i64,
        publication: &image_publication::Model,
    ) -> Result<character_portrait::Model, AppError> {
        self.verify_project_permission(publication.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        if publication.image_type != "portrait" {
            return Err(AppError::InvalidState {
                current: publication.image_type.clone(),
                operation: "恢复立绘 publication".to_string(),
            });
        }
        let metadata = publication
            .metadata_json
            .as_ref()
            .and_then(Value::as_object)
            .ok_or_else(|| AppError::InvalidState {
                current: format!("publication_id={}", publication.publication_id),
                operation: "读取立绘 publication 元数据".to_string(),
            })?;
        let profile_id = metadata
            .get("profile_id")
            .and_then(Value::as_i64)
            .filter(|id| *id > 0)
            .ok_or_else(|| AppError::InvalidState {
                current: format!("publication_id={}", publication.publication_id),
                operation: "读取立绘 publication profile_id".to_string(),
            })?;
        let portrait_type = metadata
            .get("portrait_type")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::InvalidState {
                current: format!("publication_id={}", publication.publication_id),
                operation: "读取立绘 publication portrait_type".to_string(),
            })?;
        PortraitType::try_from(portrait_type).map_err(|error| AppError::InvalidState {
            current: portrait_type.to_string(),
            operation: format!("读取立绘 publication portrait_type：{error}"),
        })?;
        let expression = metadata
            .get("expression")
            .and_then(Value::as_str)
            .map(ToString::to_string);
        let pose = metadata
            .get("pose")
            .and_then(Value::as_str)
            .map(ToString::to_string);
        let portraits = self
            .character_portrait_records_by_project(publication.project_id)
            .await?;
        let portrait_id = metadata.get("portrait_id").and_then(Value::as_i64);
        let existing = match portrait_id {
            Some(portrait_id) => portraits
                .into_iter()
                .find(|portrait| portrait.id == portrait_id),
            None => portraits.into_iter().find(|portrait| {
                portrait.profile_id == profile_id && portrait.image_path == publication.final_path
            }),
        };
        let portrait = match existing {
            Some(portrait) => portrait,
            None => {
                self.create_character_portrait_for_profile(CharacterPortraitCreateInput {
                    profile_id,
                    project_id: publication.project_id,
                    portrait_type: portrait_type.to_string(),
                    image_path: publication.final_path.clone(),
                    prompt: publication.prompt.clone(),
                    expression,
                    pose,
                })
                .await?
            }
        };
        if portrait.project_id != publication.project_id
            || portrait.profile_id != profile_id
            || portrait.image_path != publication.final_path
            || portrait.portrait_type != portrait_type
        {
            return Err(AppError::InvalidState {
                current: format!("portrait_id={}", portrait.id),
                operation: "立绘 publication 身份与主记录不一致".to_string(),
            });
        }
        if publication.status == IMAGE_PUBLICATION_STATUS_PENDING
            && portrait_id != Some(portrait.id)
        {
            let mut next_metadata = Value::Object(metadata.clone());
            next_metadata["portrait_id"] = json!(portrait.id);
            self.update_image_publication_metadata(
                &publication.publication_id,
                publication.version,
                next_metadata,
            )
            .await?;
        }
        Ok(portrait)
    }
}

pub(super) async fn record_initial_prompt_generation(
    context: &OrchestratorContext,
    portrait: &character_portrait::Model,
    input: &GeneratePortraitInput,
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
    context
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
        .await?;
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

fn portrait_publication_id(input: &PortraitPublicationInput<'_>) -> String {
    match input.workflow_run_id {
        Some(workflow_run_id) => {
            format!(
                "portrait-publication-{workflow_run_id}-{}",
                input.profile_id
            )
        }
        None => format!("portrait-publication-upload-{}", Uuid::new_v4().simple()),
    }
}

fn portrait_publication_metadata(input: &PortraitPublicationInput<'_>) -> Value {
    let provider = input.provider_trace.as_ref().map(|trace| trace.provider);
    json!({
        "metadata_schema_version": 2,
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
            "command": "portrait_image_publication",
            "image_type": "portrait",
            "provider": provider,
            "provider_model": input.model,
            "width": input.width,
            "height": input.height,
            "prompt": input.prompt,
            "reference_image_ids": input.reference_image_ids,
        },
    })
}

fn portrait_generation_run(
    input: &PortraitPublicationInput<'_>,
    portrait: &character_portrait::Model,
    image_path: &str,
    file_size: i64,
) -> Result<Option<RecordCompletedGenerationRunWithCheckpointInput>, AppError> {
    let Some(trace) = input.provider_trace.as_ref() else {
        return Ok(None);
    };
    let workflow_run_id = input
        .workflow_run_id
        .ok_or_else(|| AppError::InvalidState {
            current: "portrait_generation_run_without_workflow".to_string(),
            operation: "记录立绘图片生成追踪".to_string(),
        })?;
    let media_refs_json = (!input.reference_image_ids.is_empty())
        .then(|| json!({"reference_image_ids": input.reference_image_ids}));
    let input_variables_json = json!({
        "profile_id": input.profile_id,
        "character_portrait_id": portrait.id,
        "portrait_type": input.portrait_type,
        "expression": input.expression,
        "pose": input.pose,
        "variant_kind": input.variant_kind,
        "auto_generated": input.auto_generated,
    });
    let source_refs_json = json!({
        "character_portrait_id": portrait.id,
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
    Ok(Some(RecordCompletedGenerationRunWithCheckpointInput {
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
                "character_portrait_id": portrait.id,
                "format": trace.output_format,
                "file_size": file_size,
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
                "file_size": file_size,
            })),
            parsed_output_json: Some(json!({
                "character_portrait_id": portrait.id,
                "profile_id": input.profile_id,
                "portrait_type": input.portrait_type,
                "image_path": image_path,
                "width": input.width,
                "height": input.height,
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
    }))
}

async fn cleanup_staging_after_error(
    host_port: &dyn PortraitGenerationHostPort,
    staging_path: &str,
    label: &str,
    original_error: AppError,
) -> AppError {
    match host_port.remove_portrait_file(staging_path, label).await {
        Ok(()) => original_error,
        Err(cleanup_error) => crate::errors::app_error_with_secondary(
            original_error,
            format!("失败后的立绘暂存文件清理也失败: {cleanup_error}"),
        ),
    }
}

fn required_text(value: &str, field_name: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::InvalidArgument(format!("{field_name}不能为空")));
    }
    Ok(value.to_string())
}
