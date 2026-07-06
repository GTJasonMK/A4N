use crate::{
    actions::{
        access,
        llm_request::{
            ContextAwareJsonLlmRequestInput, PACK_KIND_PROFILE_SYNC_SOURCE_CONTEXT,
            chat_request_with_workflow_run_id, context_aware_system_user_json_llm_request,
        },
    },
    state::AppState,
};
use a4n_core::config::app_config::llm_token_profiles;
use a4n_core::error::AppError;
use a4n_core::models::{chapter::chapter, character::protagonist_profile, project::text_anchor};
use a4n_engine::llm::provider::ChatRequest;
use a4n_services::generation::{
    RecordCompletedGenerationRunInput, context_aware_chat_request_trace_json,
};
use a4n_services::llm_checkpoint::{
    CompletedLlmCallCheckpointInput, llm_call_checkpoint_fingerprint_sha256,
    load_completed_llm_call_checkpoint, required_llm_call_checkpoint_json,
    save_completed_llm_call_checkpoint,
};
use a4n_services::source::source_content_sha256;
use a4n_services::structured_llm_stream::{
    ChatJsonStreamInput, StructuredLlmProgress, chat_json_stream_with_progress,
};
use a4n_services::structured_progress::{FieldMilestone, ProgressSpan, StructuredJsonProgressSpec};
use a4n_services::text_anchor::{
    EVIDENCE_ROLE_FIRST_SEEN, PROFILE_KIND_CHARACTER, ProfileEvidenceInput,
};
use a4n_services::workflow::{
    UpsertWorkflowRunInput, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED,
    WORKFLOW_STATUS_RUNNING,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde_json::{Value, json};
use std::collections::{BTreeSet, HashMap};

use super::character_discovery::{
    build_character_discovery_manifest_user_prompt, character_discovery_stable_context,
    discovered_character_base_attributes, discovered_character_keys, existing_character_keys,
    existing_profile_for_prompt, parse_character_discovery_output, profile_character_keys,
    profile_for_result,
};
use super::json_utils::{required_i32, required_string, required_value};
use super::sync_context::character_llm_chapter_context;
use super::{CharacterAiPreviewStep, CharacterAiProgress};

const CHARACTER_DISCOVERY_PROGRESS_TOTAL: u32 = 100;
const CHARACTER_DISCOVERY_LLM_SPAN: ProgressSpan = ProgressSpan::new(20, 75);
const CHARACTER_DISCOVERY_CALL_KEY: &str = "character_discovery.llm";
const CHARACTER_DISCOVERY_FIELDS: &[FieldMilestone] = &[
    FieldMilestone::new("name", "角色名", 1),
    FieldMilestone::new("aliases", "别名", 1),
    FieldMilestone::new("importance", "重要性", 1),
    FieldMilestone::new("role", "角色定位", 1),
    FieldMilestone::new("explicit_attributes", "显性属性", 1),
    FieldMilestone::new("implicit_attributes", "隐性属性", 1),
    FieldMilestone::new("social_attributes", "社会属性", 1),
    FieldMilestone::new("evidence_anchor_id", "证据锚点", 1),
];

struct CharacterDiscoveryRequestContext {
    chapters: Vec<chapter::Model>,
    text_anchors: Vec<text_anchor::Model>,
    existing_keys: BTreeSet<String>,
    existing_key_profiles: HashMap<String, i64>,
    request: ChatRequest,
}

#[derive(Debug, Clone, Copy)]
enum ChapterAnchorMode {
    EnsureCurrent,
    RequireExistingCurrent,
}

pub async fn discover_character_profiles_for_chapters(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_number: Option<i32>,
    progress: Option<&CharacterAiProgress<'_>>,
) -> Result<Value, String> {
    access::verify_project_editor(state, user_id, project_id)
        .await
        .map_err(|err| err.to_string())?;
    report_character_discovery_progress(
        progress,
        0,
        CHARACTER_DISCOVERY_PROGRESS_TOTAL,
        "正在读取可分析章节和已有角色",
        "角色发现已停止",
    )?;
    let request_context = build_character_discovery_request(
        state,
        project_id,
        chapter_number,
        ChapterAnchorMode::EnsureCurrent,
    )
    .await?;
    let workflow_run = state
        .workflow_run
        .upsert_run(UpsertWorkflowRunInput {
            business_key: character_discovery_business_key(project_id, chapter_number),
            workflow_kind: "character_discovery".to_string(),
            project_id: Some(project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("novel_project".to_string()),
            scope_id: Some(project_id),
            chapter_id: None,
            mode: Some(
                chapter_number
                    .map(|value| format!("chapter:{value}"))
                    .unwrap_or_else(|| "all".to_string()),
            ),
            input_fingerprint_json: Some(character_discovery_workflow_fingerprint(
                project_id,
                chapter_number,
                &request_context.chapters,
                &request_context.text_anchors,
                &request_context.request,
            )),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some(CHARACTER_DISCOVERY_CALL_KEY.to_string()),
            active_task_id: None,
            can_resume: false,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map_err(|err| err.to_string())?;
    let workflow_run_id = workflow_run.id;

    let result = async {
        let chapters = request_context.chapters;
        let text_anchors = request_context.text_anchors;
        let mut existing_keys = request_context.existing_keys;
        let mut existing_key_profiles = request_context.existing_key_profiles;

    report_character_discovery_progress(
        progress,
        10,
        CHARACTER_DISCOVERY_PROGRESS_TOTAL,
        "正在构造角色发现提示词",
        "角色发现已停止",
    )?;
    report_character_discovery_progress(
        progress,
        CHARACTER_DISCOVERY_LLM_SPAN.start(),
        CHARACTER_DISCOVERY_PROGRESS_TOTAL,
        "正在调用模型发现角色",
        "角色发现已停止",
    )?;
    let fingerprint_request = request_context.request.clone();
    let mut llm_request =
        chat_request_with_workflow_run_id(fingerprint_request.clone(), workflow_run_id);
    let analyzed_chapters = chapters
        .iter()
        .map(|chapter| chapter.chapter_number)
        .collect::<BTreeSet<_>>();
    let call_fingerprint_json =
        character_discovery_call_fingerprint_json(project_id, chapter_number, &llm_request)?;
    let call_fingerprint_sha256 = llm_call_checkpoint_fingerprint_sha256(&call_fingerprint_json)
        .map_err(|err| err.to_string())?;
    if let Some(trace_context) = llm_request.trace_context.as_mut() {
        trace_context.call_key = Some(CHARACTER_DISCOVERY_CALL_KEY.to_string());
        trace_context.input_fingerprint_sha256 = Some(call_fingerprint_sha256);
    }
    let (discovered, response_content) = if let Some(checkpoint) =
        load_completed_character_discovery_call_checkpoint(
            state,
            workflow_run_id,
            &call_fingerprint_json,
        )
        .await?
    {
        report_character_discovery_progress(
            progress,
            80,
            CHARACTER_DISCOVERY_PROGRESS_TOTAL,
            "角色发现已从断点恢复，正在写入发现结果",
            "角色发现已停止",
        )?;
        let discovered = parse_and_validate_character_discovery_content(
            &checkpoint.response_content,
            &analyzed_chapters,
            &chapters,
            &text_anchors,
        )?;
        let parsed_output = character_discovery_parsed_output_json(&discovered);
        if parsed_output != checkpoint.parsed_output {
            return Err(format!(
                "{} checkpoint 的 parsed_output 与 response_content 重新解析结果不一致",
                CHARACTER_DISCOVERY_CALL_KEY
            ));
        }
        (discovered, checkpoint.response_content)
    } else {
        let response_content = chat_json_stream_with_progress(
            ChatJsonStreamInput {
                llm: state.llm.as_ref(),
                request: llm_request.clone(),
                prompt_name: "character_discovery",
                truncated_message: "角色发现输出达到当前 max_tokens 上限，返回 JSON 被截断；请在设置页调大“角色发现”输出档位，或减少本次章节上下文后重试",
                structured_progress: Some(StructuredLlmProgress {
                    spec: StructuredJsonProgressSpec {
                        root_array_key: "characters",
                        item_label: "角色",
                        expected_items: None,
                        milestones: CHARACTER_DISCOVERY_FIELDS,
                    },
                    span: CHARACTER_DISCOVERY_LLM_SPAN,
                }),
                cancel_token: progress.and_then(|progress| progress.cancel_token),
                cancelled_message: "角色发现已停止",
                },
            |current, message| {
                report_character_discovery_progress(
                    progress,
                    current,
                    CHARACTER_DISCOVERY_PROGRESS_TOTAL,
                    message,
                    "角色发现已停止",
                )
            },
        )
        .await?;

        report_character_discovery_progress(
            progress,
            80,
            CHARACTER_DISCOVERY_PROGRESS_TOTAL,
            "模型已返回，正在解析发现结果",
            "角色发现已停止",
        )?;
        let discovered = parse_and_validate_character_discovery_content(
            &response_content,
            &analyzed_chapters,
            &chapters,
            &text_anchors,
        )?;
        let parsed_output = character_discovery_parsed_output_json(&discovered);
        save_completed_character_discovery_call_checkpoint(
            state,
            workflow_run_id,
            call_fingerprint_json,
            &response_content,
            parsed_output,
        )
        .await?;
        (discovered, response_content)
    };
    let discovered_count = discovered.len();
    let request_json =
        serde_json::to_value(&llm_request).map_err(|err| format!("LLM 请求序列化失败: {err}"))?;
    let messages_json = serde_json::to_value(&llm_request.messages)
        .map_err(|err| format!("LLM 消息序列化失败: {err}"))?;
    let mut created = Vec::new();
    let mut existing = Vec::new();

    let discovered_total = discovered.len().max(1) as u32;
    for (index, character) in discovered.into_iter().enumerate() {
        let message = format!("正在写入发现角色 {}/{}", index + 1, discovered_total);
        report_character_discovery_progress(
            progress,
            90,
            CHARACTER_DISCOVERY_PROGRESS_TOTAL,
            &message,
            "角色发现已停止",
        )?;
        let keys = discovered_character_keys(&character)?;
        let name = required_string(&character, "name")?;
        if keys.iter().any(|key| existing_keys.contains(key)) {
            if let Some(profile_id) = keys
                .iter()
                .find_map(|key| existing_key_profiles.get(key).copied())
            {
                bind_character_profile_evidence(state, project_id, profile_id, &character).await?;
            }
            existing.push(json!({
                "name": name,
                "aliases": required_value(&character, "aliases")?.clone(),
                "reason": "already_exists",
            }));
            for key in keys {
                existing_keys.insert(key);
            }
            continue;
        }

        let base_attributes = discovered_character_base_attributes(&character)?;
        match state
            .character
            .create_profile_with_generation_run(
                project_id,
                &name,
                base_attributes,
                RecordCompletedGenerationRunInput {
                    project_id,
                    workflow: "character_discovery".to_string(),
                    step_key: "discover_character_profiles_for_chapters".to_string(),
                    title: format!("角色发现: {name}"),
                    model_kind: "llm".to_string(),
                    provider: None,
                    model: None,
                    prompt_template_names_json: Some(json!(["character_discovery"])),
                    prompt_template_hashes_json: None,
                    rendered_messages_json: Some(messages_json.clone()),
                    rendered_prompts_json: None,
                    input_variables_json: Some(json!({
                        "chapter_number": chapter_number,
                        "analyzed_chapters": &analyzed_chapters,
                        "character_name": &name,
                        "text_anchor_count": text_anchors.len(),
                    })),
                    source_refs_json: Some(json!({
                        "chapters": chapters.iter().map(|chapter| json!({
                            "id": chapter.id,
                            "chapter_number": chapter.chapter_number,
                            "title": &chapter.title,
                        })).collect::<Vec<_>>(),
                    })),
                    media_refs_json: None,
                    request_json: Some(request_json.clone()),
                    response_summary_json: Some(json!({
                        "character_name": &name,
                        "discovered_count": discovered_count,
                        "response_chars": response_content.chars().count(),
                    })),
                    targets: Vec::new(),
                },
            )
            .await
        {
            Ok(profile) => {
                for key in keys {
                    existing_keys.insert(key.clone());
                    existing_key_profiles.insert(key, profile.id);
                }
                bind_character_profile_evidence(state, project_id, profile.id, &character).await?;
                created.push(profile_for_result(&profile)?);
            }
            Err(AppError::InvalidArgument(message)) if message.starts_with("角色档案已存在:") =>
            {
                let profile = existing_character_profile_by_name(state, project_id, &name).await?;
                bind_character_profile_evidence(state, project_id, profile.id, &character).await?;
                for key in keys {
                    existing_keys.insert(key.clone());
                    existing_key_profiles.insert(key, profile.id);
                }
                existing.push(json!({
                    "name": name,
                    "aliases": required_value(&character, "aliases")?.clone(),
                    "reason": message,
                }));
            }
            Err(err) => return Err(format!("写入发现角色失败：{name}：{err}")),
        }
    }

    report_character_discovery_progress(
        progress,
        CHARACTER_DISCOVERY_PROGRESS_TOTAL,
        CHARACTER_DISCOVERY_PROGRESS_TOTAL,
        "角色发现已完成",
        "角色发现已停止",
    )?;
    Ok(json!({
        "status": "discovered",
        "chapters_analyzed": chapters.len(),
        "chapter_numbers": chapters.iter().map(|chapter| chapter.chapter_number).collect::<Vec<_>>(),
        "discovered_count": discovered_count,
        "created": created,
        "existing": existing,
    }))
    }
    .await;

    if let Err(status_err) =
        finalize_character_discovery_workflow(state, workflow_run_id, &result).await
    {
        return match result {
            Ok(_) => Err(format!("角色发现完成，但工作流状态写入失败: {status_err}")),
            Err(err) => Err(format!("{err}；工作流失败状态写入失败: {status_err}")),
        };
    }
    result
}

fn character_discovery_business_key(project_id: i64, chapter_number: Option<i32>) -> String {
    match chapter_number {
        Some(chapter_number) => {
            format!("character_discovery:{project_id}:chapter:{chapter_number}")
        }
        None => format!("character_discovery:{project_id}:all"),
    }
}

fn character_discovery_workflow_fingerprint(
    project_id: i64,
    chapter_number: Option<i32>,
    chapters: &[chapter::Model],
    text_anchors: &[text_anchor::Model],
    request: &ChatRequest,
) -> Value {
    json!({
        "workflow": "character_discovery",
        "contract_version": 1,
        "project_id": project_id,
        "chapter_number": chapter_number,
        "chapters": chapters.iter().map(|chapter| json!({
            "id": chapter.id,
            "chapter_number": chapter.chapter_number,
            "content_sha256": source_content_sha256(&chapter.content),
            "analysis_data": chapter.analysis_data,
        })).collect::<Vec<_>>(),
        "text_anchors": text_anchors.iter().map(|anchor| json!({
            "id": anchor.id,
            "source_chapter_id": anchor.source_chapter_id,
            "source_id": anchor.source_id,
            "quote_hash": anchor.quote_hash,
            "start_offset": anchor.start_offset,
            "end_offset": anchor.end_offset,
        })).collect::<Vec<_>>(),
        "system_prompt_sha256": request.messages.first()
            .map(|message| source_content_sha256(&message.content)),
        "temperature": request.temperature,
        "max_tokens_profile": request.max_tokens_profile,
        "response_format": request.response_format,
    })
}

fn character_discovery_call_fingerprint_json(
    project_id: i64,
    chapter_number: Option<i32>,
    request: &ChatRequest,
) -> Result<Value, String> {
    Ok(json!({
        "workflow": "character_discovery",
        "call_key": CHARACTER_DISCOVERY_CALL_KEY,
        "project_id": project_id,
        "chapter_number": chapter_number,
        "request": context_aware_chat_request_trace_json(request).map_err(|err| err.to_string())?,
    }))
}

fn character_discovery_parsed_output_json(discovered: &[Value]) -> Value {
    json!({
        "characters": discovered,
    })
}

struct CharacterDiscoveryCallCheckpoint {
    response_content: String,
    parsed_output: Value,
}

async fn load_completed_character_discovery_call_checkpoint(
    state: &AppState,
    workflow_run_id: i64,
    input_fingerprint_json: &Value,
) -> Result<Option<CharacterDiscoveryCallCheckpoint>, String> {
    let Some(checkpoint) = load_completed_llm_call_checkpoint(
        state.db.clone(),
        workflow_run_id,
        CHARACTER_DISCOVERY_CALL_KEY,
        input_fingerprint_json,
    )
    .await
    .map_err(|err| err.to_string())?
    else {
        return Ok(None);
    };
    let checkpoint_json =
        required_llm_call_checkpoint_json(checkpoint, CHARACTER_DISCOVERY_CALL_KEY)
            .map_err(|err| err.to_string())?;
    let response_content = checkpoint_json
        .get("response_content")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| {
            format!(
                "{}.response_content 必须是字符串",
                CHARACTER_DISCOVERY_CALL_KEY
            )
        })?;
    let parsed_output = checkpoint_json
        .get("parsed_output")
        .cloned()
        .ok_or_else(|| format!("{}.parsed_output 缺失", CHARACTER_DISCOVERY_CALL_KEY))?;
    Ok(Some(CharacterDiscoveryCallCheckpoint {
        response_content,
        parsed_output,
    }))
}

async fn save_completed_character_discovery_call_checkpoint(
    state: &AppState,
    workflow_run_id: i64,
    input_fingerprint_json: Value,
    response_content: &str,
    parsed_output: Value,
) -> Result<(), String> {
    save_completed_llm_call_checkpoint(
        state.db.clone(),
        CompletedLlmCallCheckpointInput {
            workflow_run_id,
            call_key: CHARACTER_DISCOVERY_CALL_KEY.to_string(),
            call_order: 1,
            input_fingerprint_json,
            checkpoint_json: json!({
                "output": parsed_output.clone(),
                "parsed_output": parsed_output,
                "response_content": response_content,
                "usage": null,
                "finish_reason": null,
            }),
            artifacts_json: None,
        },
    )
    .await
    .map_err(|err| err.to_string())
}

async fn finalize_character_discovery_workflow(
    state: &AppState,
    workflow_run_id: i64,
    result: &Result<Value, String>,
) -> Result<(), String> {
    let (status, can_resume, error_message) = match result {
        Ok(_) => (WORKFLOW_STATUS_COMPLETED, false, None),
        Err(message) => (WORKFLOW_STATUS_FAILED, true, Some(message.as_str())),
    };
    state
        .workflow_run
        .mark_status(workflow_run_id, status, can_resume, error_message)
        .await
        .map_err(|err| err.to_string())?;
    Ok(())
}

pub(crate) async fn character_discovery_preview_steps(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_number: Option<i32>,
    initial_state: &str,
    initial_note: Option<String>,
) -> Result<Vec<CharacterAiPreviewStep>, String> {
    access::verify_project_editor(state, user_id, project_id)
        .await
        .map_err(|err| err.to_string())?;
    let context = build_character_discovery_request(
        state,
        project_id,
        chapter_number,
        ChapterAnchorMode::RequireExistingCurrent,
    )
    .await?;
    let note = match initial_note {
        Some(note) => format!("{note}；分析章节数：{}", context.chapters.len()),
        None => format!("分析章节数：{}", context.chapters.len()),
    };
    Ok(vec![CharacterAiPreviewStep::llm(
        "character_discovery",
        "角色发现初始 LLM 请求",
        "character_discovery",
        initial_state,
        Some(note),
        context.request,
    )])
}

async fn build_character_discovery_request(
    state: &AppState,
    project_id: i64,
    chapter_number: Option<i32>,
    anchor_mode: ChapterAnchorMode,
) -> Result<CharacterDiscoveryRequestContext, String> {
    let chapters = select_character_discovery_chapters(
        crate::workflow::character_sync_chapters(state, project_id).await?,
        project_id,
        chapter_number,
    )?;

    let existing_profiles = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .order_by_asc(protagonist_profile::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    let existing_keys = existing_character_keys(&existing_profiles);
    let existing_key_profiles = existing_profiles
        .iter()
        .flat_map(|profile| {
            profile_character_keys(profile)
                .into_iter()
                .map(|key| (key, profile.id))
        })
        .collect::<HashMap<_, _>>();
    let existing_prompt_profiles: Vec<Value> = existing_profiles
        .iter()
        .map(existing_profile_for_prompt)
        .collect::<Result<_, _>>()?;
    let mut text_anchors = Vec::new();
    for chapter in &chapters {
        let anchors = match anchor_mode {
            ChapterAnchorMode::EnsureCurrent => {
                state
                    .text_anchor
                    .ensure_chapter_content_anchors(project_id, chapter.id)
                    .await
            }
            ChapterAnchorMode::RequireExistingCurrent => {
                state
                    .text_anchor
                    .read_current_chapter_content_anchors(project_id, chapter.id)
                    .await
            }
        }
        .map_err(|err| err.to_string())?;
        text_anchors.extend(anchors);
    }
    let chapter_data: Vec<Value> = chapters.iter().map(character_llm_chapter_context).collect();
    let system_prompt = state
        .engine
        .render("character_discovery", &HashMap::new(), Some(project_id))
        .map_err(|e| e.to_string())?;
    let stable_context = character_discovery_stable_context(
        &existing_prompt_profiles,
        &chapter_data,
        &chapters,
        &text_anchors,
    );
    let user_prompt = build_character_discovery_manifest_user_prompt()?;
    let chapter_ids = chapters
        .iter()
        .map(|chapter| chapter.id)
        .collect::<Vec<_>>();
    let text_anchor_ids = text_anchors
        .iter()
        .map(|anchor| anchor.id)
        .collect::<Vec<_>>();

    Ok(CharacterDiscoveryRequestContext {
        chapters,
        text_anchors,
        existing_keys,
        existing_key_profiles,
        request: context_aware_system_user_json_llm_request(
            system_prompt,
            ContextAwareJsonLlmRequestInput {
                project_id,
                workflow_run_id: None,
                workflow: "character_discovery".to_string(),
                step_key: "character_discovery.parsed".to_string(),
                scope_kind: "profile_sync".to_string(),
                pack_kind: PACK_KIND_PROFILE_SYNC_SOURCE_CONTEXT.to_string(),
                source_refs_json: Some(json!({
                    "chapter_number": chapter_number,
                    "chapter_ids": chapter_ids,
                    "text_anchor_ids": text_anchor_ids,
                })),
                config_json: json!({
                    "ledger_version": "character_discovery_context_v1",
                    "prompt_name": "character_discovery",
                }),
                stable_context_json: stable_context,
                dynamic_user_prompt: user_prompt,
                temperature: Some(0.2),
                max_tokens_profile: llm_token_profiles::CHARACTER_DISCOVERY.to_string(),
                call_key: Some(character_discovery_business_key(project_id, chapter_number)),
            },
        )?,
    })
}

pub(super) fn select_character_discovery_chapters(
    chapters: Vec<chapter::Model>,
    project_id: i64,
    chapter_number: Option<i32>,
) -> Result<Vec<chapter::Model>, String> {
    if chapter_number.is_some_and(|chapter_number| chapter_number <= 0) {
        return Err("角色发现章节号必须大于 0".to_string());
    }
    if let Some(chapter_number) = chapter_number {
        let selected = chapters
            .into_iter()
            .filter(|chapter| chapter.chapter_number == chapter_number)
            .collect::<Vec<_>>();
        if selected.is_empty() {
            return Err(format!(
                "第 {chapter_number} 章没有可用于角色发现的有效正文，请先生成或导入正文"
            ));
        }
        return Ok(selected);
    }

    if chapters.is_empty() {
        return Err(format!(
            "项目还没有可用于角色发现的章节: project_id={project_id}"
        ));
    }
    Ok(chapters)
}

pub(super) fn validate_discovered_character_chapter_scope(
    characters: &[Value],
    analyzed_chapters: &BTreeSet<i32>,
) -> Result<(), String> {
    if analyzed_chapters.is_empty() {
        return Err("角色发现缺少已分析章节".to_string());
    }

    for (index, character) in characters.iter().enumerate() {
        let first_seen = required_i32(character, "first_seen_chapter")?;
        let latest_seen = required_i32(character, "latest_seen_chapter")?;
        if !analyzed_chapters.contains(&first_seen) {
            return Err(format!(
                "characters[{index}].first_seen_chapter 第 {first_seen} 章不在本次角色发现输入章节中"
            ));
        }
        if !analyzed_chapters.contains(&latest_seen) {
            return Err(format!(
                "characters[{index}].latest_seen_chapter 第 {latest_seen} 章不在本次角色发现输入章节中"
            ));
        }
    }

    Ok(())
}

fn parse_and_validate_character_discovery_content(
    content: &str,
    analyzed_chapters: &BTreeSet<i32>,
    chapters: &[chapter::Model],
    text_anchors: &[text_anchor::Model],
) -> Result<Vec<Value>, String> {
    let discovered = parse_character_discovery_output(content, chapters, text_anchors)?;
    validate_discovered_character_chapter_scope(&discovered, analyzed_chapters)?;
    Ok(discovered)
}

async fn bind_character_profile_evidence(
    state: &AppState,
    project_id: i64,
    profile_id: i64,
    character: &Value,
) -> Result<(), String> {
    let text_anchor_id = required_value(character, "evidence_anchor_id")?
        .as_i64()
        .ok_or_else(|| "角色发现字段 evidence_anchor_id 必须是 JSON 整数数字".to_string())?;
    state
        .text_anchor
        .merge_profile_evidence(
            project_id,
            None,
            PROFILE_KIND_CHARACTER,
            profile_id,
            vec![ProfileEvidenceInput {
                text_anchor_id,
                evidence_role: EVIDENCE_ROLE_FIRST_SEEN.to_string(),
            }],
        )
        .await
        .map_err(|err| err.to_string())?;
    Ok(())
}

async fn existing_character_profile_by_name(
    state: &AppState,
    project_id: i64,
    name: &str,
) -> Result<protagonist_profile::Model, String> {
    protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .filter(protagonist_profile::Column::Name.eq(name))
        .one(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| {
            format!("角色档案已存在但无法重新读取: project_id={project_id}, name={name}")
        })
}

fn report_character_discovery_progress(
    progress: Option<&CharacterAiProgress<'_>>,
    current: u32,
    total: u32,
    message: &str,
    cancelled_message: &str,
) -> Result<(), String> {
    if let Some(progress) = progress {
        progress.report(current, total, message, cancelled_message)?;
    }
    Ok(())
}
