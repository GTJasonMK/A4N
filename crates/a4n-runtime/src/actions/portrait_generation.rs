use crate::AppState;
use crate::actions::access;
use crate::actions::image_generation::{normalize_image_metadata, resolve_image_config};
use crate::actions::image_records::{IMPORTED_IMAGE_MODEL, read_uploaded_image_data};
use crate::actions::portrait_records::portrait_project_id_for_user;
use crate::actions::request_preview::ModelRequestPreview;
use crate::actions::template_variables::parse_template_variables;
use crate::actions::workflow_checkpoint::save_completed_workflow_checkpoint;
use crate::file_utils::{
    canonicalize_data_file_path, data_dir_relative_file_path, remove_data_file_in_data_dir,
};
use a4n_core::error::AppError;
use a4n_core::files::paths::ProjectPaths;
use a4n_core::models::ai::generated_image;
use a4n_core::models::character::{character_portrait, protagonist_profile};
use a4n_core::models::project::novel_blueprint;
use a4n_engine::image::{ImageGenerationRequest, ImageReference};
use a4n_engine::llm::provider::{ChatRequest, is_truncated_finish_reason};
use a4n_engine::queue::priority::{TaskPriority, TaskType as QueueTaskType};
use a4n_services::generation::{
    GenerationRunCheckpointContext, GenerationRunTraceTarget, RecordCompletedGenerationRunInput,
    RecordCompletedGenerationRunWithCheckpointInput, context_aware_chat_request_trace_json,
    delete_generation_run_targets_for, generation_checkpoint_fingerprint_sha256,
};
use a4n_services::image::{
    CreateImageRecord, configured_image_provider, image_backend_supports_reference_images,
};
use a4n_services::llm_checkpoint::{
    CompletedLlmCallCheckpointInput, llm_call_checkpoint_fingerprint_sha256,
    load_completed_llm_call_checkpoint, required_llm_call_checkpoint_json,
    save_completed_llm_call_checkpoint,
};
use a4n_services::portrait::{
    CreatePortraitInput, PortraitPromptGenerationInput, PortraitPromptRequestInput,
    PortraitPromptSpec, design_brief::character_sheet_seed_instruction,
    normalize_portrait_prompt_response,
};
use a4n_services::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING};
use base64::Engine as _;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QuerySelect, Set,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use tokio_util::sync::CancellationToken;

mod preview;
mod workflow;

use preview::{blocked_preview_step, chat_preview_step, portrait_preview, provider_preview_step};
use workflow::{
    finalize_portrait_workflow, load_completed_portrait_final_checkpoint,
    missing_portrait_item_fingerprint, portrait_checkpoint_string, portrait_generate_business_key,
    portrait_generate_workflow_fingerprint, portrait_image_business_key,
    portrait_image_workflow_fingerprint, portrait_prompt_checkpoint_fingerprint,
};

const DEFAULT_PORTRAIT_EXPRESSION: &str = "neutral";

pub type PortraitProgress<'a> = &'a mut (dyn FnMut(u32, u32, &str) -> Result<(), AppError> + Send);

#[derive(Debug, Clone, Serialize)]
pub struct PortraitSummary {
    pub id: i64,
    pub profile_id: i64,
    pub project_id: i64,
    pub portrait_type: String,
    pub image_path: String,
    pub prompt: String,
    pub expression: Option<String>,
    pub pose: Option<String>,
    pub is_primary: bool,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratePortraitPromptInput {
    pub project_id: i64,
    pub profile_id: i64,
    pub portrait_type: String,
    pub variables: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GeneratePortraitInput {
    pub project_id: i64,
    pub character_name: String,
    pub style: Option<String>,
    pub expressions: Option<Vec<String>>,
    #[serde(default)]
    pub pose: Option<String>,
    #[serde(default)]
    pub reference_image_ids: Vec<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RegeneratePortraitInput {
    pub portrait_id: i64,
    #[serde(default)]
    pub reference_image_ids: Vec<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GenerateAgeVariantInput {
    pub portrait_id: i64,
    pub age: String,
    #[serde(default)]
    pub reference_image_ids: Vec<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GenerateMissingPortraitsInput {
    pub project_id: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UploadPortraitInput {
    pub project_id: i64,
    pub profile_id: i64,
    pub file_name: String,
    pub mime_type: String,
    pub data_base64: String,
    pub portrait_type: String,
    pub prompt: String,
    #[serde(default)]
    pub expression: Option<String>,
    #[serde(default)]
    pub pose: Option<String>,
    #[serde(default)]
    pub set_primary: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortraitTaskScope {
    pub project_id: i64,
    pub total: u32,
}

struct GeneratePortraitPromptRequestContext {
    profile: protagonist_profile::Model,
    existing_count: u64,
    expression: String,
    request: ChatRequest,
}

struct PortraitImageMirror<'a> {
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
    provider_trace: Option<PortraitImageProviderTrace<'a>>,
}

struct PortraitImageProviderTrace<'a> {
    provider: &'a str,
    request_json: &'a Value,
    output_format: &'a str,
}

struct PersistGeneratedPortraitInput<'a> {
    workflow_run_id: Option<i64>,
    project_id: i64,
    profile_id: i64,
    character_name: &'a str,
    portrait_type: &'a str,
    prompt: &'a str,
    expression: Option<&'a str>,
    pose: Option<&'a str>,
    is_primary: bool,
    variant_kind: Option<&'a str>,
    auto_generated: bool,
    path_context: &'a str,
    reference_image_ids: &'a [i64],
}

pub async fn generate_portrait_task_scope(
    state: &AppState,
    user_id: i64,
    input: &GeneratePortraitInput,
) -> Result<PortraitTaskScope, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    Ok(PortraitTaskScope {
        project_id: input.project_id,
        total: 6,
    })
}

pub async fn regenerate_portrait_task_scope(
    state: &AppState,
    user_id: i64,
    input: &RegeneratePortraitInput,
) -> Result<PortraitTaskScope, String> {
    let project_id = portrait_project_id_for_user(state, user_id, input.portrait_id).await?;
    Ok(PortraitTaskScope {
        project_id,
        total: 4,
    })
}

pub async fn generate_age_variant_task_scope(
    state: &AppState,
    user_id: i64,
    input: &GenerateAgeVariantInput,
) -> Result<PortraitTaskScope, String> {
    let project_id = portrait_project_id_for_user(state, user_id, input.portrait_id).await?;
    Ok(PortraitTaskScope {
        project_id,
        total: 4,
    })
}

pub async fn generate_missing_portraits_task_scope(
    state: &AppState,
    user_id: i64,
    input: &GenerateMissingPortraitsInput,
) -> Result<PortraitTaskScope, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    Ok(PortraitTaskScope {
        project_id: input.project_id,
        total: 1,
    })
}

pub async fn generate_portrait_prompt(
    state: &AppState,
    user_id: i64,
    input: GeneratePortraitPromptInput,
) -> Result<String, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let variables = parse_template_variables(input.variables.as_ref())?;
    let profile = portrait_prompt_profile(state, input.project_id, input.profile_id).await?;
    let profile_context = portrait_profile_context(&profile);
    let project_context = portrait_project_context(state.db.as_ref(), input.project_id).await?;
    let spec = PortraitPromptSpec {
        profile_name: &profile.name,
        appearance: profile.appearance.as_deref(),
        gender: profile.gender.as_deref(),
        age: profile.age.as_deref(),
        personality: profile.personality.as_deref(),
        background: profile.background.as_deref(),
        profile_context: Some(&profile_context),
        project_context: project_context.as_ref(),
        portrait_type: &input.portrait_type,
        art_style: None,
        expression: None,
        existing_portrait_count: None,
    };
    let request = state
        .portrait
        .build_generate_prompt_request(PortraitPromptRequestInput {
            project_id: input.project_id,
            spec,
            prompt_engine: state.engine.as_ref(),
            variables: &variables,
        })
        .map_err(|e| e.to_string())?;
    let workflow_fingerprint = portrait_prompt_workflow_fingerprint(
        &input,
        &profile,
        &profile_context,
        project_context.as_ref(),
        &request,
    )?;
    let workflow_run = state
        .workflow_run
        .upsert_run(UpsertWorkflowRunInput {
            business_key: portrait_prompt_business_key(&workflow_fingerprint)?,
            workflow_kind: "portrait_prompt_generation".to_string(),
            project_id: Some(input.project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("protagonist_profile".to_string()),
            scope_id: Some(input.profile_id),
            chapter_id: None,
            mode: Some(input.portrait_type.clone()),
            input_fingerprint_json: Some(workflow_fingerprint),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some("portrait_prompt.generate".to_string()),
            active_task_id: None,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map_err(|err| err.to_string())?;
    let workflow_run_id = workflow_run.id;

    let result = state
        .portrait
        .generate_prompt_with_checkpoint(PortraitPromptGenerationInput {
            project_id: input.project_id,
            spec,
            llm: state.llm.as_ref(),
            prompt_engine: state.engine.as_ref(),
            variables: &variables,
            workflow_run_id: Some(workflow_run_id),
        })
        .await
        .map(|prompt| prompt.trim().to_string())
        .map_err(|e| e.to_string());

    finalize_portrait_workflow(state, workflow_run_id, &result)
        .await
        .map_err(|status_err| match result.as_ref() {
            Ok(_) => format!("人物三视图提示词生成成功，但工作流状态写入失败: {status_err}"),
            Err(err) => format!("{err}；工作流失败状态写入失败: {status_err}"),
        })?;
    result
}

fn portrait_prompt_workflow_fingerprint(
    input: &GeneratePortraitPromptInput,
    profile: &protagonist_profile::Model,
    profile_context: &Value,
    project_context: Option<&Value>,
    request: &ChatRequest,
) -> Result<Value, String> {
    Ok(json!({
        "workflow": "portrait_prompt_generation",
        "schema_version": "portrait_prompt_workflow_v1",
        "project_id": input.project_id,
        "profile": {
            "id": profile.id,
            "name": profile.name,
            "appearance": profile.appearance,
            "gender": profile.gender,
            "age": profile.age,
            "personality": profile.personality,
            "background": profile.background,
            "base_attributes": profile.base_attributes,
            "current_state": profile.current_state,
            "character_arc": profile.character_arc,
            "voice_fingerprint": profile.voice_fingerprint,
        },
        "portrait_type": input.portrait_type,
        "variables": input.variables.clone(),
        "profile_context": profile_context,
        "project_context": project_context.cloned().unwrap_or_else(|| json!({})),
        "request": context_aware_chat_request_trace_json(request)
            .map_err(|err| format!("人物三视图提示词请求序列化失败: {err}"))?,
    }))
}

fn portrait_prompt_business_key(fingerprint: &Value) -> Result<String, String> {
    Ok(format!(
        "portrait_prompt_generation:{}",
        generation_checkpoint_fingerprint_sha256(fingerprint).map_err(|err| err.to_string())?
    ))
}

pub async fn preview_generate_portrait_prompt_request(
    state: &AppState,
    user_id: i64,
    input: GeneratePortraitPromptInput,
) -> Result<ModelRequestPreview, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let variables = parse_template_variables(input.variables.as_ref())?;
    let profile = portrait_prompt_profile(state, input.project_id, input.profile_id).await?;
    let profile_context = portrait_profile_context(&profile);
    let project_context = portrait_project_context(state.db.as_ref(), input.project_id).await?;
    let spec = PortraitPromptSpec {
        profile_name: &profile.name,
        appearance: profile.appearance.as_deref(),
        gender: profile.gender.as_deref(),
        age: profile.age.as_deref(),
        personality: profile.personality.as_deref(),
        background: profile.background.as_deref(),
        profile_context: Some(&profile_context),
        project_context: project_context.as_ref(),
        portrait_type: &input.portrait_type,
        art_style: None,
        expression: None,
        existing_portrait_count: None,
    };
    let request = state
        .portrait
        .build_generate_prompt_request(PortraitPromptRequestInput {
            project_id: input.project_id,
            spec,
            prompt_engine: state.engine.as_ref(),
            variables: &variables,
        })
        .map_err(|e| e.to_string())?;
    portrait_preview(
        "portrait_prompt_generation",
        "人物三视图拆解提示词 LLM 请求预览",
        vec![
            chat_preview_step(
                state,
                "portrait_prompt",
                "人物三视图拆解提示词 LLM 请求",
                Some("character_portrait_generation"),
                request,
            )
            .await?,
        ],
    )
}

pub async fn generate_portrait(
    state: &AppState,
    user_id: i64,
    input: GeneratePortraitInput,
    mut progress: Option<PortraitProgress<'_>>,
    cancel_token: CancellationToken,
) -> Result<PortraitSummary, String> {
    generate_portrait_task_scope(state, user_id, &input).await?;
    report_progress(&mut progress, 0, 6, "正在读取角色档案")?;
    check_cancelled(&cancel_token)?;
    let GeneratePortraitPromptRequestContext {
        profile,
        existing_count,
        expression,
        request,
    } = build_generate_portrait_prompt_request(
        state,
        input.project_id,
        &input.character_name,
        input.style.as_deref(),
        input.expressions.as_deref(),
        input.pose.as_deref(),
    )
    .await?;
    let workflow_fingerprint =
        portrait_generate_workflow_fingerprint(&input, &profile, &expression, &request)?;
    let workflow_run = state
        .workflow_run
        .upsert_run(UpsertWorkflowRunInput {
            business_key: portrait_generate_business_key(&workflow_fingerprint)?,
            workflow_kind: "portrait_generation".to_string(),
            project_id: Some(input.project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("protagonist_profile".to_string()),
            scope_id: Some(profile.id),
            chapter_id: None,
            mode: Some("full_body".to_string()),
            input_fingerprint_json: Some(workflow_fingerprint.clone()),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some("portrait_generation.prompt".to_string()),
            active_task_id: None,
            can_resume: false,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map_err(|err| err.to_string())?;
    let workflow_run_id = workflow_run.id;

    let result = async {
        report_progress(&mut progress, 1, 6, "正在渲染人物三视图拆解提示词模板")?;
        check_cancelled(&cancel_token)?;
        report_progress(&mut progress, 2, 6, "正在调用模型生成图片提示词")?;
        let mut llm_request = request.clone();
        let call_key = "portrait_generation.prompt".to_string();
        let prompt_fingerprint =
            portrait_prompt_checkpoint_fingerprint(&workflow_fingerprint, &llm_request)?;
        let input_fingerprint_sha256 =
            llm_call_checkpoint_fingerprint_sha256(&prompt_fingerprint)
                .map_err(|err| err.to_string())?;
        if let Some(trace_context) = llm_request.trace_context.as_mut() {
            trace_context.workflow_run_id = Some(workflow_run_id);
            trace_context.call_key = Some(call_key.clone());
            trace_context.input_fingerprint_sha256 = Some(input_fingerprint_sha256);
        }
        state
            .workflow_run
            .mark_current_step(workflow_run_id, &call_key)
            .await
            .map_err(|err| err.to_string())?;
        let (_raw_prompt_response, image_prompt, raw_response_json) = if let Some(checkpoint) =
            load_completed_llm_call_checkpoint(
                state.db.clone(),
                workflow_run_id,
                &call_key,
                &prompt_fingerprint,
            )
            .await
            .map_err(|err| err.to_string())?
        {
            report_progress(&mut progress, 2, 6, "图片提示词已从断点恢复")?;
            let checkpoint_json =
                required_llm_call_checkpoint_json(checkpoint, &call_key)
                    .map_err(|err| err.to_string())?;
            let raw_prompt_response =
                portrait_checkpoint_string(&checkpoint_json, &call_key, "response_content")?;
            let image_prompt =
                portrait_checkpoint_string(&checkpoint_json, &call_key, "parsed_output")?;
            let reparsed = normalize_portrait_prompt_response(&raw_prompt_response)
                .map_err(|err| err.to_string())?;
            if reparsed != image_prompt {
                return Err(format!(
                    "{call_key} checkpoint 的 parsed_output 与 response_content 重新解析结果不一致"
                ));
            }
            let usage = checkpoint_json
                .get("usage")
                .cloned()
                .ok_or_else(|| format!("{call_key}.usage 缺失"))?;
            let finish_reason = checkpoint_json
                .get("finish_reason")
                .cloned()
                .ok_or_else(|| format!("{call_key}.finish_reason 缺失"))?;
            let raw_response_json = json!({
                "text": raw_prompt_response.clone(),
                "usage": usage,
                "finish_reason": finish_reason,
            });
            (raw_prompt_response, image_prompt, raw_response_json)
        } else {
            let llm_response = state
                .llm
                .chat(llm_request.clone())
                .await
                .map_err(|e| e.to_string())?;
            if is_truncated_finish_reason(llm_response.finish_reason.as_deref()) {
                return Err("人物三视图提示词输出达到当前 max_tokens 上限，提示词可能被截断；请在设置页调大“媒体提示词”输出档位，或减少角色上下文后重试".to_string());
            }
            let raw_prompt_response = llm_response.message.content.clone();
            let image_prompt = normalize_portrait_prompt_response(&raw_prompt_response)
                .map_err(|e| e.to_string())?;
            let finish_reason = llm_response.finish_reason.clone();
            let response_usage_json = llm_response
                .usage
                .as_ref()
                .map(serde_json::to_value)
                .transpose()
                .map_err(|err| format!("人物三视图提示词 LLM 用量序列化失败: {err}"))?;
            let raw_response_json = json!({
                "text": raw_prompt_response.clone(),
                "usage": response_usage_json.clone(),
                "finish_reason": finish_reason,
            });
            save_completed_llm_call_checkpoint(
                state.db.clone(),
                CompletedLlmCallCheckpointInput {
                    workflow_run_id,
                    call_key,
                    call_order: 1,
                    input_fingerprint_json: prompt_fingerprint,
                    checkpoint_json: json!({
                        "output": image_prompt.clone(),
                        "parsed_output": image_prompt.clone(),
                        "response_content": raw_prompt_response.clone(),
                        "usage": response_usage_json,
                        "finish_reason": finish_reason,
                    }),
                    artifacts_json: None,
                },
            )
            .await
            .map_err(|err| err.to_string())?;
            (raw_prompt_response, image_prompt, raw_response_json)
        };

        report_progress(&mut progress, 3, 6, "正在调用图片模型生成人物三视图拆解")?;
        check_cancelled(&cancel_token)?;
        if let Some(summary) = load_completed_portrait_final_checkpoint(
            state,
            workflow_run_id,
            "portrait_generation.final",
            &workflow_fingerprint,
            input.project_id,
        )
        .await?
        {
            report_progress(&mut progress, 6, 6, "人物三视图拆解已从断点恢复")?;
            return Ok(summary);
        }
        let generated = run_portrait_image_generation(
            state,
            input.project_id,
            &image_prompt,
            &input.reference_image_ids,
            Some(workflow_run_id),
            cancel_token.clone(),
        )
        .await?;
        check_cancelled(&cancel_token)?;
        report_progress(&mut progress, 4, 6, "正在保存人物三视图拆解图片")?;
        report_progress(&mut progress, 5, 6, "正在写入人物三视图拆解记录")?;
        let portrait = persist_generated_portrait(
            state,
            &generated,
        PersistGeneratedPortraitInput {
            workflow_run_id: Some(workflow_run_id),
            profile_id: profile.id,
                project_id: input.project_id,
                portrait_type: "full_body",
                prompt: &image_prompt,
                expression: Some(&expression),
                pose: input.pose.as_deref(),
                is_primary: existing_count == 0,
                character_name: &profile.name,
                variant_kind: None,
                auto_generated: false,
                path_context: "人物三视图拆解图片",
                reference_image_ids: &input.reference_image_ids,
            },
        )
        .await?;
        let request_json = serde_json::to_value(&llm_request)
            .map_err(|err| format!("LLM 请求序列化失败: {err}"))?;
        let messages_json = serde_json::to_value(&llm_request.messages)
            .map_err(|err| format!("LLM 消息序列化失败: {err}"))?;
        let input_variables_json = json!({
            "profile_id": profile.id,
            "character_name": &profile.name,
            "portrait_type": "full_body",
            "expression": &expression,
            "pose": &input.pose,
            "style": &input.style,
            "reference_image_ids": &input.reference_image_ids,
        });
        let source_refs_json = json!({
            "profile_id": profile.id,
            "appearance": &profile.appearance,
            "gender": &profile.gender,
            "age": &profile.age,
            "personality": &profile.personality,
            "background": &profile.background,
        });
        let media_refs_json = json!({
            "reference_image_ids": &input.reference_image_ids,
        });
        let input_fingerprint_json = json!({
            "prompt_name": "character_portrait_generation",
            "project_id": input.project_id,
            "request": request_json.clone(),
            "input_variables": input_variables_json.clone(),
            "source_refs": source_refs_json.clone(),
            "media_refs": media_refs_json.clone(),
        });
        if let Err(err) = state
            .generation_run
            .record_completed_run_with_checkpoint(RecordCompletedGenerationRunWithCheckpointInput {
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
                        "prompt_chars": image_prompt.chars().count(),
                        "image_path": &portrait.image_path,
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
                            .map_err(|err| err.to_string())?,
                    ),
                    input_fingerprint_json: Some(input_fingerprint_json),
                    raw_response_json: Some(raw_response_json),
                    parsed_output_json: Some(json!({
                        "character_portrait_id": portrait.id,
                        "profile_id": profile.id,
                        "portrait_type": "full_body",
                        "image_prompt": image_prompt.clone(),
                        "image_path": &portrait.image_path,
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
            })
            .await
        {
            return match rollback_failed_portrait_creation(state, portrait.id, &portrait.image_path)
                .await
            {
                Ok(()) => Err(err.to_string()),
                Err(cleanup_err) => Err(format!("{err}；{cleanup_err}")),
            };
        }
        save_completed_workflow_checkpoint(
            state,
            Some(workflow_run_id),
            "portrait_generation.final",
            2,
            workflow_fingerprint,
            json!({
                "portrait_id": portrait.id,
                "image_path": portrait.image_path,
                "summary": to_summary(&portrait),
            }),
        )
        .await?;
        report_progress(&mut progress, 6, 6, "人物三视图拆解生成完成")?;
        Ok(to_summary(&portrait))
    }
    .await;

    if let Err(status_err) = finalize_portrait_workflow(state, workflow_run_id, &result).await {
        return match result {
            Ok(_) => Err(format!(
                "人物三视图拆解生成完成，但工作流状态写入失败: {status_err}"
            )),
            Err(err) => Err(format!("{err}；工作流失败状态写入失败: {status_err}")),
        };
    }
    result
}

pub async fn regenerate_portrait(
    state: &AppState,
    user_id: i64,
    input: RegeneratePortraitInput,
    mut progress: Option<PortraitProgress<'_>>,
    cancel_token: CancellationToken,
) -> Result<PortraitSummary, String> {
    regenerate_portrait_task_scope(state, user_id, &input).await?;
    let existing = character_portrait_by_id(state, input.portrait_id).await?;
    report_progress(&mut progress, 0, 4, "正在读取立绘和角色档案")?;
    check_cancelled(&cancel_token)?;
    let profile = portrait_profile_by_id(state, existing.profile_id, existing.id).await?;
    let workflow_fingerprint = portrait_image_workflow_fingerprint(
        "portrait_regeneration",
        existing.project_id,
        existing.id,
        &existing.prompt,
        &input.reference_image_ids,
        json!({ "variant_kind": "regenerated" }),
    );
    let workflow_run = state
        .workflow_run
        .upsert_run(UpsertWorkflowRunInput {
            business_key: portrait_image_business_key(
                "portrait_regeneration",
                &workflow_fingerprint,
            )?,
            workflow_kind: "portrait_regeneration".to_string(),
            project_id: Some(existing.project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("character_portrait".to_string()),
            scope_id: Some(existing.id),
            chapter_id: None,
            mode: Some("regenerated".to_string()),
            input_fingerprint_json: Some(workflow_fingerprint.clone()),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some("portrait_regeneration.final".to_string()),
            active_task_id: None,
            can_resume: false,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map_err(|err| err.to_string())?;
    let workflow_run_id = workflow_run.id;

    let result = async {
        report_progress(&mut progress, 1, 4, "正在调用图片模型重绘立绘")?;
        if let Some(summary) = load_completed_portrait_final_checkpoint(
            state,
            workflow_run_id,
            "portrait_regeneration.final",
            &workflow_fingerprint,
            existing.project_id,
        )
        .await?
        {
            report_progress(&mut progress, 4, 4, "立绘重绘已从断点恢复")?;
            return Ok(summary);
        }
        let generated = run_portrait_image_generation(
            state,
            existing.project_id,
            &existing.prompt,
            &input.reference_image_ids,
            Some(workflow_run_id),
            cancel_token.clone(),
        )
        .await?;
        check_cancelled(&cancel_token)?;
        report_progress(&mut progress, 2, 4, "正在保存重绘图片")?;
        report_progress(&mut progress, 3, 4, "正在写入重绘记录")?;
        let new_portrait = persist_generated_portrait(
            state,
            &generated,
            PersistGeneratedPortraitInput {
                workflow_run_id: Some(workflow_run_id),
                profile_id: existing.profile_id,
                project_id: existing.project_id,
                portrait_type: &existing.portrait_type,
                prompt: &existing.prompt,
                expression: existing.expression.as_deref(),
                pose: existing.pose.as_deref(),
                is_primary: false,
                character_name: &profile.name,
                variant_kind: Some("regenerated"),
                auto_generated: false,
                path_context: "重绘立绘",
                reference_image_ids: &input.reference_image_ids,
            },
        )
        .await?;
        save_completed_workflow_checkpoint(
            state,
            Some(workflow_run_id),
            "portrait_regeneration.final",
            1,
            workflow_fingerprint,
            json!({
                "portrait_id": new_portrait.id,
                "image_path": new_portrait.image_path,
                "summary": to_summary(&new_portrait),
            }),
        )
        .await?;
        report_progress(&mut progress, 4, 4, "立绘重绘完成")?;
        Ok(to_summary(&new_portrait))
    }
    .await;

    if let Err(status_err) = finalize_portrait_workflow(state, workflow_run_id, &result).await {
        return match result {
            Ok(_) => Err(format!("立绘重绘完成，但工作流状态写入失败: {status_err}")),
            Err(err) => Err(format!("{err}；工作流失败状态写入失败: {status_err}")),
        };
    }
    result
}

pub async fn generate_age_variant(
    state: &AppState,
    user_id: i64,
    input: GenerateAgeVariantInput,
    mut progress: Option<PortraitProgress<'_>>,
    cancel_token: CancellationToken,
) -> Result<PortraitSummary, String> {
    generate_age_variant_task_scope(state, user_id, &input).await?;
    let existing = character_portrait_by_id(state, input.portrait_id).await?;
    report_progress(&mut progress, 0, 4, "正在读取立绘和角色档案")?;
    check_cancelled(&cancel_token)?;
    let profile = portrait_profile_by_id(state, existing.profile_id, existing.id).await?;
    let new_prompt = age_variant_prompt(&existing.prompt, &input.age);
    let workflow_fingerprint = portrait_image_workflow_fingerprint(
        "portrait_age_variant_generation",
        existing.project_id,
        existing.id,
        &new_prompt,
        &input.reference_image_ids,
        json!({
            "variant_kind": "age_variant",
            "age": input.age,
        }),
    );
    let workflow_run = state
        .workflow_run
        .upsert_run(UpsertWorkflowRunInput {
            business_key: portrait_image_business_key(
                "portrait_age_variant_generation",
                &workflow_fingerprint,
            )?,
            workflow_kind: "portrait_age_variant_generation".to_string(),
            project_id: Some(existing.project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("character_portrait".to_string()),
            scope_id: Some(existing.id),
            chapter_id: None,
            mode: Some("age_variant".to_string()),
            input_fingerprint_json: Some(workflow_fingerprint.clone()),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some("portrait_age_variant_generation.final".to_string()),
            active_task_id: None,
            can_resume: false,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map_err(|err| err.to_string())?;
    let workflow_run_id = workflow_run.id;

    let result = async {
        report_progress(&mut progress, 1, 4, "正在调用图片模型生成年龄变体")?;
        if let Some(summary) = load_completed_portrait_final_checkpoint(
            state,
            workflow_run_id,
            "portrait_age_variant_generation.final",
            &workflow_fingerprint,
            existing.project_id,
        )
        .await?
        {
            report_progress(&mut progress, 4, 4, "年龄变体已从断点恢复")?;
            return Ok(summary);
        }
        let generated = run_portrait_image_generation(
            state,
            existing.project_id,
            &new_prompt,
            &input.reference_image_ids,
            Some(workflow_run_id),
            cancel_token.clone(),
        )
        .await?;
        check_cancelled(&cancel_token)?;
        report_progress(&mut progress, 2, 4, "正在保存年龄变体图片")?;
        report_progress(&mut progress, 3, 4, "正在写入年龄变体记录")?;
        let new_portrait = persist_generated_portrait(
            state,
            &generated,
            PersistGeneratedPortraitInput {
                workflow_run_id: Some(workflow_run_id),
                profile_id: existing.profile_id,
                project_id: existing.project_id,
                portrait_type: &existing.portrait_type,
                prompt: &new_prompt,
                expression: existing.expression.as_deref(),
                pose: existing.pose.as_deref(),
                is_primary: false,
                character_name: &profile.name,
                variant_kind: Some("age_variant"),
                auto_generated: false,
                path_context: "年龄变体立绘",
                reference_image_ids: &input.reference_image_ids,
            },
        )
        .await?;
        save_completed_workflow_checkpoint(
            state,
            Some(workflow_run_id),
            "portrait_age_variant_generation.final",
            1,
            workflow_fingerprint,
            json!({
                "portrait_id": new_portrait.id,
                "image_path": new_portrait.image_path,
                "summary": to_summary(&new_portrait),
            }),
        )
        .await?;
        report_progress(&mut progress, 4, 4, "年龄变体生成完成")?;
        Ok(to_summary(&new_portrait))
    }
    .await;

    if let Err(status_err) = finalize_portrait_workflow(state, workflow_run_id, &result).await {
        return match result {
            Ok(_) => Err(format!(
                "年龄变体生成完成，但工作流状态写入失败: {status_err}"
            )),
            Err(err) => Err(format!("{err}；工作流失败状态写入失败: {status_err}")),
        };
    }
    result
}

pub async fn generate_missing_portraits(
    state: &AppState,
    user_id: i64,
    input: GenerateMissingPortraitsInput,
    mut progress: Option<PortraitProgress<'_>>,
    cancel_token: CancellationToken,
) -> Result<Vec<PortraitSummary>, String> {
    generate_missing_portraits_task_scope(state, user_id, &input).await?;
    let missing_profiles = missing_portrait_profiles(state, input.project_id).await?;
    let workflow_fingerprint = json!({
        "workflow": "missing_portrait_generation",
        "project_id": input.project_id,
    });
    let workflow_run = state
        .workflow_run
        .upsert_run(UpsertWorkflowRunInput {
            business_key: format!("missing_portrait_generation:{}", input.project_id),
            workflow_kind: "missing_portrait_generation".to_string(),
            project_id: Some(input.project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("novel_project".to_string()),
            scope_id: Some(input.project_id),
            chapter_id: None,
            mode: Some("missing".to_string()),
            input_fingerprint_json: Some(workflow_fingerprint),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some("missing_portrait_generation.items".to_string()),
            active_task_id: None,
            can_resume: false,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map_err(|err| err.to_string())?;
    let workflow_run_id = workflow_run.id;

    let result = async {
        let total = u32::try_from(missing_profiles.len())
            .unwrap_or(u32::MAX)
            .max(1);
        let mut results = Vec::new();
        for (index, profile) in missing_profiles.iter().enumerate() {
            let current = u32::try_from(index).unwrap_or(u32::MAX).min(total);
            let display_index = index + 1;
            check_cancelled(&cancel_token)?;
            report_progress(
                &mut progress,
                current,
                total,
                &format!(
                    "正在生成人物三视图拆解 {display_index}/{}：{}",
                    missing_profiles.len(),
                    profile.name
                ),
            )?;
            let prompt = profile_portrait_seed_prompt(profile)?;
            let step_key = format!("missing_portrait_generation.profile.{}", profile.id);
            let item_fingerprint =
                missing_portrait_item_fingerprint(input.project_id, profile, &prompt);
            if let Some(summary) = load_completed_portrait_final_checkpoint(
                state,
                workflow_run_id,
                &step_key,
                &item_fingerprint,
                input.project_id,
            )
            .await?
            {
                results.push(summary);
                let next_current = u32::try_from(display_index).unwrap_or(u32::MAX).min(total);
                report_progress(
                    &mut progress,
                    next_current,
                    total,
                    &format!(
                        "已从断点恢复人物三视图拆解 {display_index}/{}：{}",
                        missing_profiles.len(),
                        profile.name
                    ),
                )?;
                continue;
            }
            let generated = run_portrait_image_generation(
                state,
                input.project_id,
                &prompt,
                &[],
                Some(workflow_run_id),
                cancel_token.clone(),
            )
            .await?;
            check_cancelled(&cancel_token)?;
            let portrait = persist_generated_portrait(
                state,
                &generated,
                PersistGeneratedPortraitInput {
                    workflow_run_id: Some(workflow_run_id),
                    profile_id: profile.id,
                    project_id: input.project_id,
                    portrait_type: "half_body",
                    prompt: &prompt,
                    expression: None,
                    pose: None,
                    is_primary: true,
                    character_name: &profile.name,
                    variant_kind: None,
                    auto_generated: true,
                    path_context: "缺失人物三视图拆解",
                    reference_image_ids: &[],
                },
            )
            .await?;
            save_completed_workflow_checkpoint(
                state,
                Some(workflow_run_id),
                step_key,
                i32::try_from(display_index).unwrap_or(i32::MAX),
                item_fingerprint,
                json!({
                    "portrait_id": portrait.id,
                    "image_path": portrait.image_path,
                    "summary": to_summary(&portrait),
                }),
            )
            .await?;
            results.push(to_summary(&portrait));
            let next_current = u32::try_from(display_index).unwrap_or(u32::MAX).min(total);
            report_progress(
                &mut progress,
                next_current,
                total,
                &format!(
                    "已生成人物三视图拆解 {display_index}/{}：{}",
                    missing_profiles.len(),
                    profile.name
                ),
            )?;
        }
        Ok(results)
    }
    .await;

    if let Err(status_err) = finalize_portrait_workflow(state, workflow_run_id, &result).await {
        return match result {
            Ok(_) => Err(format!(
                "缺失立绘生成完成，但工作流状态写入失败: {status_err}"
            )),
            Err(err) => Err(format!("{err}；工作流失败状态写入失败: {status_err}")),
        };
    }
    result
}

pub async fn upload_portrait(
    state: &AppState,
    user_id: i64,
    input: UploadPortraitInput,
) -> Result<PortraitSummary, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let profile = portrait_prompt_profile(state, input.project_id, input.profile_id).await?;
    let prompt = required_upload_text(&input.prompt, "立绘描述")?;
    let imported =
        read_uploaded_image_data(&input.file_name, &input.mime_type, &input.data_base64)?;
    let path = save_portrait_bytes(
        state,
        input.project_id,
        input.profile_id,
        &imported.bytes,
        imported.extension,
    )?;
    let path_str = data_dir_relative_file_path(&state.config.data_dir, &path, "上传立绘图片")?;
    let portrait = match state
        .portrait
        .create(CreatePortraitInput {
            profile_id: input.profile_id,
            project_id: input.project_id,
            portrait_type: &input.portrait_type,
            image_path: &path_str,
            prompt: &prompt,
            expression: input.expression.as_deref(),
            pose: input.pose.as_deref(),
        })
        .await
    {
        Ok(portrait) => portrait,
        Err(err) => {
            let cleanup =
                remove_data_file_in_data_dir(&state.config.data_dir, &path_str, "上传立绘图片")
                    .err()
                    .filter(|value| !value.trim().is_empty());
            return match cleanup {
                Some(cleanup) => Err(format!(
                    "上传立绘记录创建失败：{err}；后续清理也失败：{cleanup}"
                )),
                None => Err(format!("上传立绘记录创建失败：{err}")),
            };
        }
    };

    mirror_or_rollback(
        state,
        &portrait,
        &path_str,
        PortraitImageMirror {
            workflow_run_id: None,
            project_id: input.project_id,
            profile_id: input.profile_id,
            portrait_id: portrait.id,
            character_name: &profile.name,
            portrait_type: &portrait.portrait_type,
            image_path: &path_str,
            prompt: &prompt,
            model: IMPORTED_IMAGE_MODEL,
            width: imported.width,
            height: imported.height,
            file_size: imported.file_size,
            expression: portrait.expression.as_deref(),
            pose: portrait.pose.as_deref(),
            variant_kind: Some("manual_upload"),
            auto_generated: false,
            reference_image_ids: &[],
            provider_trace: None,
        },
    )
    .await?;

    if input.set_primary && !portrait.is_primary {
        state
            .portrait
            .set_primary(portrait.id)
            .await
            .map_err(|err| format!("立绘上传成功，但设置主图失败：{err}"))?;
        let updated = character_portrait_by_id(state, portrait.id).await?;
        return Ok(to_summary(&updated));
    }

    Ok(to_summary(&portrait))
}

pub async fn preview_generate_portrait_request(
    state: &AppState,
    user_id: i64,
    input: GeneratePortraitInput,
) -> Result<ModelRequestPreview, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let context = build_generate_portrait_prompt_request(
        state,
        input.project_id,
        &input.character_name,
        input.style.as_deref(),
        input.expressions.as_deref(),
        input.pose.as_deref(),
    )
    .await?;
    let reference_count = validate_reference_image_ids(&input.reference_image_ids)?.len();
    let image_step_message = if reference_count > 0 {
        format!(
            "该图片请求依赖上一步模型返回的 image prompt，生成前无法伪造完整 provider request；真实生成会附带 {} 张参考图。",
            reference_count
        )
    } else {
        "该图片请求依赖上一步模型返回的 image prompt，生成前无法伪造完整 provider request"
            .to_string()
    };
    portrait_preview(
        "portrait_generation",
        "人物三视图拆解生成请求预览",
        vec![
            chat_preview_step(
                state,
                "portrait_prompt",
                "生成图片提示词的 LLM 请求",
                Some("character_portrait_generation"),
                context.request,
            )
            .await?,
            blocked_preview_step(
                "portrait_image",
                "人物三视图拆解图片 provider 请求",
                "image_generation",
                &image_step_message,
            ),
        ],
    )
}

pub async fn preview_regenerate_portrait_request(
    state: &AppState,
    user_id: i64,
    input: RegeneratePortraitInput,
) -> Result<ModelRequestPreview, String> {
    portrait_project_id_for_user(state, user_id, input.portrait_id).await?;
    let existing = character_portrait_by_id(state, input.portrait_id).await?;
    let context = build_portrait_image_request(
        state,
        existing.project_id,
        &existing.prompt,
        &input.reference_image_ids,
    )
    .await?;
    portrait_preview(
        "portrait_regeneration",
        "立绘重绘请求预览",
        vec![provider_preview_step(
            "portrait_image",
            "立绘图片 provider 请求",
            &context.config.backend,
            context.request,
        )?],
    )
}

pub async fn preview_generate_age_variant_request(
    state: &AppState,
    user_id: i64,
    input: GenerateAgeVariantInput,
) -> Result<ModelRequestPreview, String> {
    portrait_project_id_for_user(state, user_id, input.portrait_id).await?;
    let existing = character_portrait_by_id(state, input.portrait_id).await?;
    let context = build_portrait_image_request(
        state,
        existing.project_id,
        &age_variant_prompt(&existing.prompt, &input.age),
        &input.reference_image_ids,
    )
    .await?;
    portrait_preview(
        "portrait_age_variant_generation",
        "立绘年龄变体请求预览",
        vec![provider_preview_step(
            "portrait_age_variant_image",
            "年龄变体图片 provider 请求",
            &context.config.backend,
            context.request,
        )?],
    )
}

pub async fn preview_generate_missing_portraits_request(
    state: &AppState,
    user_id: i64,
    input: GenerateMissingPortraitsInput,
) -> Result<ModelRequestPreview, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let missing_profiles = missing_portrait_profiles(state, input.project_id).await?;
    let mut steps = Vec::with_capacity(missing_profiles.len());
    for (index, profile) in missing_profiles.iter().enumerate() {
        let prompt = profile_portrait_seed_prompt(profile)?;
        let context = build_portrait_image_request(state, input.project_id, &prompt, &[]).await?;
        steps.push(provider_preview_step(
            &format!("missing_portrait_{}", index + 1),
            &format!("补齐角色「{}」人物三视图拆解 provider 请求", profile.name),
            &context.config.backend,
            context.request,
        )?);
    }
    portrait_preview(
        "missing_portrait_generation",
        "补齐人物三视图拆解请求预览",
        steps,
    )
}

async fn portrait_prompt_profile(
    state: &AppState,
    project_id: i64,
    profile_id: i64,
) -> Result<protagonist_profile::Model, String> {
    let profile = protagonist_profile::Entity::find_by_id(profile_id)
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("角色档案不存在: {profile_id}"))?;
    if profile.project_id != project_id {
        return Err(format!("角色档案 {profile_id} 不属于项目 {project_id}"));
    }
    Ok(profile)
}

async fn build_generate_portrait_prompt_request(
    state: &AppState,
    project_id: i64,
    character_name: &str,
    style: Option<&str>,
    expressions: Option<&[String]>,
    pose: Option<&str>,
) -> Result<GeneratePortraitPromptRequestContext, String> {
    let profile = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .filter(protagonist_profile::Column::Name.eq(character_name))
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("项目 {project_id} 中未找到角色「{character_name}」"))?;

    let existing_count = character_portrait::Entity::find()
        .filter(character_portrait::Column::ProfileId.eq(profile.id))
        .count(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    let profile_context = portrait_profile_context(&profile);
    let project_context = portrait_project_context(state.db.as_ref(), project_id).await?;
    let expression = first_expression_or_default(expressions);
    let style_with_pose = portrait_style_with_pose(style, pose);
    let variables = HashMap::new();
    let spec = PortraitPromptSpec {
        profile_name: &profile.name,
        appearance: profile.appearance.as_deref(),
        gender: profile.gender.as_deref(),
        age: profile.age.as_deref(),
        personality: profile.personality.as_deref(),
        background: profile.background.as_deref(),
        profile_context: Some(&profile_context),
        project_context: project_context.as_ref(),
        portrait_type: "full_body",
        art_style: style_with_pose.as_deref(),
        expression: Some(&expression),
        existing_portrait_count: Some(existing_count),
    };
    let request = state
        .portrait
        .build_generate_prompt_request(PortraitPromptRequestInput {
            project_id,
            spec,
            prompt_engine: state.engine.as_ref(),
            variables: &variables,
        })
        .map_err(|err| err.to_string())?;
    Ok(GeneratePortraitPromptRequestContext {
        profile,
        existing_count,
        expression,
        request,
    })
}

fn portrait_style_with_pose(style: Option<&str>, pose: Option<&str>) -> Option<String> {
    let style = style.map(str::trim).filter(|value| !value.is_empty());
    let pose = pose.map(str::trim).filter(|value| !value.is_empty());
    match (style, pose) {
        (Some(style), Some(pose)) => Some(format!("{style}\n姿势要求：{pose}")),
        (Some(style), None) => Some(style.to_string()),
        (None, Some(pose)) => Some(format!("姿势要求：{pose}")),
        (None, None) => None,
    }
}

fn profile_portrait_seed_prompt(profile: &protagonist_profile::Model) -> Result<String, String> {
    let mut parts = vec![
        character_sheet_seed_instruction(),
        format!("角色名：{}", profile.name),
    ];
    for (label, value) in [
        ("性别", profile.gender.as_deref()),
        ("年龄", profile.age.as_deref()),
        ("外观", profile.appearance.as_deref()),
        ("性格", profile.personality.as_deref()),
        ("背景", profile.background.as_deref()),
    ] {
        if let Some(text) = optional_profile_text(value) {
            parts.push(format!("{label}：{text}"));
        }
    }
    let profile_context = portrait_profile_context(profile);
    if matches!(profile_context.as_object(), Some(fields) if !fields.is_empty()) {
        let context = serde_json::to_string(&profile_context).map_err(|err| err.to_string())?;
        parts.push(format!("角色档案上下文：{context}"));
    }
    Ok(parts.join("，"))
}

fn optional_profile_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn required_upload_text(value: &str, field_name: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{field_name}不能为空"));
    }
    Ok(value.to_string())
}

fn first_expression_or_default(expressions: Option<&[String]>) -> String {
    expressions
        .unwrap_or_default()
        .iter()
        .map(|value| value.trim())
        .find(|value| !value.is_empty())
        .unwrap_or(DEFAULT_PORTRAIT_EXPRESSION)
        .to_string()
}

fn portrait_profile_context(profile: &protagonist_profile::Model) -> Value {
    let mut object = Map::new();
    insert_meaningful_json(&mut object, "base_attributes", &profile.base_attributes);
    insert_meaningful_json(&mut object, "current_state", &profile.current_state);
    if let Some(arc) = profile.character_arc.as_ref() {
        insert_meaningful_json(&mut object, "character_arc", arc);
    }
    if let Some(voice) = profile.voice_fingerprint.as_ref() {
        insert_meaningful_json(&mut object, "voice_fingerprint", voice);
    }
    Value::Object(object)
}

async fn portrait_project_context(
    db: &impl ConnectionTrait,
    project_id: i64,
) -> Result<Option<Value>, String> {
    let Some(blueprint) = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project_id))
        .one(db)
        .await
        .map_err(|e| e.to_string())?
    else {
        return Ok(None);
    };
    let mut object = Map::new();
    insert_optional_text(&mut object, "title", Some(blueprint.title.as_str()));
    insert_optional_text(&mut object, "genre", Some(blueprint.genre.as_str()));
    insert_optional_text(&mut object, "style", blueprint.style.as_deref());
    insert_optional_text(&mut object, "tone", blueprint.tone.as_deref());
    insert_optional_text(
        &mut object,
        "target_audience",
        blueprint.target_audience.as_deref(),
    );
    insert_optional_text(
        &mut object,
        "one_sentence_summary",
        blueprint.one_sentence_summary.as_deref(),
    );
    if let Some(world_setting) = blueprint.world_setting.as_ref() {
        insert_meaningful_json(&mut object, "world_setting", world_setting);
    }
    if object.is_empty() {
        Ok(None)
    } else {
        Ok(Some(Value::Object(object)))
    }
}

fn insert_optional_text(object: &mut Map<String, Value>, key: &str, value: Option<&str>) {
    if let Some(text) = value.map(str::trim).filter(|text| !text.is_empty()) {
        object.insert(key.to_string(), Value::String(text.to_string()));
    }
}

fn insert_meaningful_json(object: &mut Map<String, Value>, key: &str, value: &Value) {
    if is_meaningful_json(value) {
        object.insert(key.to_string(), value.clone());
    }
}

fn is_meaningful_json(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::String(text) => !text.trim().is_empty(),
        Value::Array(items) => !items.is_empty(),
        Value::Object(fields) => !fields.is_empty(),
        Value::Bool(_) | Value::Number(_) => true,
    }
}

async fn missing_portrait_profiles(
    state: &AppState,
    project_id: i64,
) -> Result<Vec<protagonist_profile::Model>, String> {
    let profiles = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    let mut missing_profiles = Vec::new();
    for profile in profiles {
        let existing_count = character_portrait::Entity::find()
            .filter(character_portrait::Column::ProfileId.eq(profile.id))
            .count(state.db.as_ref())
            .await
            .map_err(|e| e.to_string())?;
        if existing_count == 0 {
            missing_profiles.push(profile);
        }
    }
    Ok(missing_profiles)
}

struct PortraitImageRequestContext {
    request: ImageGenerationRequest,
    config: a4n_core::models::ai::image_gen_config::Model,
}

async fn build_portrait_image_request(
    state: &AppState,
    project_id: i64,
    prompt: &str,
    reference_image_ids: &[i64],
) -> Result<PortraitImageRequestContext, String> {
    let config = resolve_image_config(state, None).await?;
    let reference_images =
        collect_portrait_reference_images(state, project_id, reference_image_ids).await?;
    if !reference_images.is_empty() && !image_backend_supports_reference_images(&config.backend) {
        return Err(format!(
            "当前图片后端 {} 不支持真实参考图输入，请切换到 openai-compatible-image-edit、openai-chat-image 或 cliproxy-image，或移除参考图",
            config.backend
        ));
    }
    Ok(PortraitImageRequestContext {
        request: ImageGenerationRequest {
            prompt: prompt.to_string(),
            width: config.default_width,
            height: config.default_height,
            model: config.model.clone(),
            reference_images,
        },
        config,
    })
}

async fn collect_portrait_reference_images(
    state: &AppState,
    project_id: i64,
    image_ids: &[i64],
) -> Result<Vec<ImageReference>, String> {
    let image_ids = validate_reference_image_ids(image_ids)?;
    if image_ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut out = Vec::with_capacity(image_ids.len());
    for (index, image_id) in image_ids.iter().copied().enumerate() {
        let image = generated_image::Entity::find_by_id(image_id)
            .one(state.db.as_ref())
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("参考图不存在：#{image_id}"))?;
        if image.project_id != project_id {
            return Err(format!("参考图 #{} 不属于当前项目", image_id));
        }
        out.push(read_portrait_reference_image(
            state,
            &format!("参考图 {}", index + 1),
            &image.file_path,
        )?);
    }
    Ok(out)
}

fn validate_reference_image_ids(image_ids: &[i64]) -> Result<Vec<i64>, String> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::with_capacity(image_ids.len());
    for image_id in image_ids.iter().copied() {
        if image_id <= 0 {
            return Err(format!("参考图 ID 必须为正整数：{image_id}"));
        }
        if !seen.insert(image_id) {
            return Err(format!("参考图包含重复 ID：{image_id}"));
        }
        normalized.push(image_id);
    }
    Ok(normalized)
}

fn read_portrait_reference_image(
    state: &AppState,
    label: &str,
    path: &str,
) -> Result<ImageReference, String> {
    let path = canonicalize_data_file_path(&state.config.data_dir, path, label)?;
    let bytes = std::fs::read(&path).map_err(|e| format!("读取{label}失败：{e}"))?;
    let mime = portrait_reference_image_mime(&path)?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(ImageReference {
        label: label.to_string(),
        data_uri: format!("data:{mime};base64,{encoded}"),
    })
}

fn portrait_reference_image_mime(path: &Path) -> Result<&'static str, String> {
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    match ext.as_str() {
        "png" => Ok("image/png"),
        "jpg" | "jpeg" => Ok("image/jpeg"),
        "webp" => Ok("image/webp"),
        other => Err(format!("参考图格式不支持: {other}，仅支持 png/jpg/webp")),
    }
}

struct GeneratedPortraitImage {
    bytes: Vec<u8>,
    format: String,
    model: String,
    provider: String,
    request_json: Value,
    width: i32,
    height: i32,
    file_size: i64,
}

async fn run_portrait_image_generation(
    state: &AppState,
    project_id: i64,
    prompt: &str,
    reference_image_ids: &[i64],
    workflow_run_id: Option<i64>,
    cancel_token: CancellationToken,
) -> Result<GeneratedPortraitImage, String> {
    let context =
        build_portrait_image_request(state, project_id, prompt, reference_image_ids).await?;
    let provider_backend = context.config.backend.clone();
    let request_json = serde_json::to_value(&context.request)
        .map_err(|err| format!("人物立绘图片请求序列化失败: {err}"))?;
    let provider = configured_image_provider(&context.config).map_err(|e| e.to_string())?;
    let result = super::image_generation::run_queued_request_with_cancel_and_workflow(
        state,
        QueueTaskType::ImageGeneration,
        TaskPriority::Medium,
        Some(project_id),
        workflow_run_id,
        cancel_token,
        || async move {
            provider
                .generate(context.request)
                .await
                .map_err(|e| e.to_string())
        },
    )
    .await?;
    let file_size = result.bytes.len() as i64;
    Ok(GeneratedPortraitImage {
        bytes: result.bytes,
        format: result.format,
        model: result.model,
        provider: provider_backend,
        request_json,
        width: context.config.default_width,
        height: context.config.default_height,
        file_size,
    })
}

fn save_portrait_bytes(
    state: &AppState,
    project_id: i64,
    profile_id: i64,
    bytes: &[u8],
    format: &str,
) -> Result<std::path::PathBuf, String> {
    let dir = ProjectPaths::new(&state.config.data_dir, project_id).portraits_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("无法创建立绘目录：{e}"))?;
    let ts = chrono::Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_else(|| chrono::Utc::now().timestamp_micros() * 1000);
    for attempt in 0..20 {
        let suffix = if attempt == 0 {
            String::new()
        } else {
            format!("_{attempt}")
        };
        let path = dir.join(format!("portrait_{profile_id}_{ts}{suffix}.{format}"));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => {
                use std::io::Write;
                file.write_all(bytes)
                    .map_err(|e| format!("无法写入立绘：{e}"))?;
                return Ok(path);
            }
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(format!("无法创建立绘文件：{err}")),
        }
    }
    Err("无法创建立绘文件：文件名冲突过多".to_string())
}

async fn persist_generated_portrait(
    state: &AppState,
    generated: &GeneratedPortraitImage,
    input: PersistGeneratedPortraitInput<'_>,
) -> Result<character_portrait::Model, String> {
    let path = save_portrait_bytes(
        state,
        input.project_id,
        input.profile_id,
        &generated.bytes,
        &generated.format,
    )?;
    let path_str = data_dir_relative_file_path(&state.config.data_dir, &path, input.path_context)?;
    let portrait = insert_portrait(
        state,
        InsertPortraitInput {
            profile_id: input.profile_id,
            project_id: input.project_id,
            portrait_type: input.portrait_type,
            image_path: &path_str,
            prompt: input.prompt,
            expression: input.expression,
            pose: input.pose,
            is_primary: input.is_primary,
        },
    )
    .await?;
    mirror_or_rollback(
        state,
        &portrait,
        &path_str,
        PortraitImageMirror {
            workflow_run_id: input.workflow_run_id,
            project_id: input.project_id,
            profile_id: input.profile_id,
            portrait_id: portrait.id,
            character_name: input.character_name,
            portrait_type: input.portrait_type,
            image_path: &path_str,
            prompt: input.prompt,
            model: &generated.model,
            width: generated.width,
            height: generated.height,
            file_size: generated.file_size,
            expression: input.expression,
            pose: input.pose,
            variant_kind: input.variant_kind,
            auto_generated: input.auto_generated,
            reference_image_ids: input.reference_image_ids,
            provider_trace: Some(PortraitImageProviderTrace {
                provider: &generated.provider,
                request_json: &generated.request_json,
                output_format: &generated.format,
            }),
        },
    )
    .await?;
    Ok(portrait)
}

struct InsertPortraitInput<'a> {
    profile_id: i64,
    project_id: i64,
    portrait_type: &'a str,
    image_path: &'a str,
    prompt: &'a str,
    expression: Option<&'a str>,
    pose: Option<&'a str>,
    is_primary: bool,
}

async fn insert_portrait(
    state: &AppState,
    input: InsertPortraitInput<'_>,
) -> Result<character_portrait::Model, String> {
    character_portrait::ActiveModel {
        profile_id: Set(input.profile_id),
        project_id: Set(input.project_id),
        portrait_type: Set(input.portrait_type.to_string()),
        image_path: Set(input.image_path.to_string()),
        prompt: Set(input.prompt.to_string()),
        expression: Set(input.expression.map(str::to_string)),
        pose: Set(input.pose.map(str::to_string)),
        is_primary: Set(input.is_primary),
        created_at: Set(chrono::Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(state.db.as_ref())
    .await
    .map_err(|e| e.to_string())
}

async fn mirror_or_rollback(
    state: &AppState,
    portrait: &character_portrait::Model,
    image_path: &str,
    mirror: PortraitImageMirror<'_>,
) -> Result<(), String> {
    if let Err(err) = mirror_portrait_to_generated_image(state, mirror).await {
        return match rollback_failed_portrait_creation(state, portrait.id, image_path).await {
            Ok(()) => Err(err),
            Err(cleanup_err) => Err(format!("{err}；{cleanup_err}")),
        };
    }
    Ok(())
}

async fn mirror_portrait_to_generated_image(
    state: &AppState,
    input: PortraitImageMirror<'_>,
) -> Result<(), String> {
    let now = chrono::Utc::now().naive_utc();
    let metadata_json = normalize_image_metadata(Some(json!({
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
    })));
    let image_record = CreateImageRecord {
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
    if let Some(trace) = input.provider_trace {
        let media_refs_json = if input.reference_image_ids.is_empty() {
            None
        } else {
            Some(json!({
                "reference_image_ids": input.reference_image_ids,
            }))
        };
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
            "media_refs": media_refs_json.clone(),
        });
        state
            .image
            .create_record_with_generation_run_checkpoint(
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
                        rendered_prompts_json: Some(json!({
                            "prompt": input.prompt,
                        })),
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
                                .map_err(|err| err.to_string())?,
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
            .await
            .map_err(|e| e.to_string())?;
    } else {
        state
            .image
            .create_record(image_record)
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

async fn rollback_failed_portrait_creation(
    state: &AppState,
    portrait_id: i64,
    image_path: &str,
) -> Result<(), String> {
    let image_ids = generated_image::Entity::find()
        .select_only()
        .column(generated_image::Column::Id)
        .filter(generated_image::Column::FilePath.eq(image_path))
        .into_tuple::<i64>()
        .all(state.db.as_ref())
        .await
        .map_err(|e| format!("查询立绘镜像图片记录失败: {e}"))?;
    delete_generation_run_targets_for(state.db.as_ref(), "generated_image", &image_ids)
        .await
        .map_err(|e| format!("回滚立绘镜像图片溯源目标失败: {e}"))?;
    generated_image::Entity::delete_many()
        .filter(generated_image::Column::FilePath.eq(image_path))
        .exec(state.db.as_ref())
        .await
        .map_err(|e| format!("回滚立绘镜像图片记录失败: {e}"))?;
    delete_generation_run_targets_for(state.db.as_ref(), "character_portrait", &[portrait_id])
        .await
        .map_err(|e| format!("回滚立绘溯源目标失败: {e}"))?;
    character_portrait::Entity::delete_by_id(portrait_id)
        .exec(state.db.as_ref())
        .await
        .map_err(|e| format!("回滚立绘记录失败: {e}"))?;
    remove_data_file_in_data_dir(&state.config.data_dir, image_path, "立绘图片")
}

pub(super) async fn character_portrait_by_id(
    state: &AppState,
    portrait_id: i64,
) -> Result<character_portrait::Model, String> {
    character_portrait::Entity::find_by_id(portrait_id)
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "立绘不存在".to_string())
}

async fn portrait_profile_by_id(
    state: &AppState,
    profile_id: i64,
    portrait_id: i64,
) -> Result<protagonist_profile::Model, String> {
    protagonist_profile::Entity::find_by_id(profile_id)
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("立绘 #{portrait_id} 缺少角色档案 #{profile_id}"))
}

fn age_variant_prompt(prompt: &str, age: &str) -> String {
    format!("{prompt}，age: {age}")
}

fn report_progress(
    progress: &mut Option<PortraitProgress<'_>>,
    current: u32,
    total: u32,
    message: &str,
) -> Result<(), String> {
    if let Some(progress) = progress.as_deref_mut() {
        progress(current, total, message).map_err(|err| err.to_string())?;
    }
    Ok(())
}

fn check_cancelled(cancel_token: &CancellationToken) -> Result<(), String> {
    if cancel_token.is_cancelled() {
        return Err(AppError::Cancelled.to_string());
    }
    Ok(())
}

pub(crate) fn to_summary(p: &character_portrait::Model) -> PortraitSummary {
    PortraitSummary {
        id: p.id,
        profile_id: p.profile_id,
        project_id: p.project_id,
        portrait_type: p.portrait_type.clone(),
        image_path: p.image_path.clone(),
        prompt: p.prompt.clone(),
        expression: p.expression.clone(),
        pose: p.pose.clone(),
        is_primary: p.is_primary,
        created_at: Some(p.created_at.to_string()),
    }
}

#[cfg(test)]
mod tests;
