use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use a4n_core::config::app_config::llm_token_profiles;
use a4n_core::error::AppError;
use a4n_engine::llm::provider::{
    ChatRequest, LlmProvider, LlmTraceContext, UsageInfo, is_truncated_finish_reason,
};
use a4n_engine::prompt::engine::PromptEngine;
use a4n_engine::prompt::request::build_json_task_user_message;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::chapter::source_spans::{
    SourceSpan, resolve_source_span_text, source_span_prompt_value, source_span_prompt_values,
};
use crate::generation::context_aware_chat_request_trace_json;
use crate::llm_checkpoint::{
    llm_call_checkpoint_fingerprint_sha256, load_completed_llm_call_checkpoint,
    required_llm_call_checkpoint_json, required_llm_call_workflow_run_id,
};
use crate::llm_context::{
    COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedger,
    LlmContextLedgerPackDraft, LlmContextManifest, context_aware_system_user_request,
};
use crate::source::source_content_sha256;

const MANGA_PIPELINE_MAX_TOKENS_PROFILE: &str = llm_token_profiles::EXTRA_LONG;
const MANGA_PIPELINE_WORKFLOW: &str = "manga_pipeline";
const MANGA_PIPELINE_PROMPT_NAMES: &[&str] = &[
    "manga_event_beats",
    "manga_character_binding",
    "manga_speech_selection",
    "manga_visual_anchor_extraction",
    "manga_item_extraction",
    "manga_chapter_comic_synthesis",
    "manga_page_planning",
    "manga_page_drawing_prompt",
];
pub(super) const MANGA_PREVIOUS_OUTPUT_EVENT_BEATS: &str = "manga_previous_output:event_beats";
pub(super) const MANGA_PREVIOUS_OUTPUT_CHARACTER_BINDING: &str =
    "manga_previous_output:character_binding";
pub(super) const MANGA_PREVIOUS_OUTPUT_SPEECH_SELECTION: &str =
    "manga_previous_output:speech_selection";
pub(super) const MANGA_PREVIOUS_OUTPUT_VISUAL_ANCHOR_EXTRACTION: &str =
    "manga_previous_output:visual_anchor_extraction";
pub(super) const MANGA_PREVIOUS_OUTPUT_ITEM_EXTRACTION: &str =
    "manga_previous_output:item_extraction";
pub(super) const MANGA_PREVIOUS_OUTPUT_EXTRACTION_SUMMARY: &str =
    "manga_previous_output:extraction_summary";
pub(super) const MANGA_PREVIOUS_OUTPUT_PAGE_PLANNING: &str = "manga_previous_output:page_planning";

#[derive(Debug, Clone, Copy)]
pub(super) struct MangaContextPreviousOutput<'a> {
    pub(super) pack_kind: &'a str,
    pub(super) prompt_name: &'a str,
    pub(super) parsed_output: &'a Value,
    pub(super) raw_output: Option<&'a str>,
    pub(super) placeholder: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct MangaPromptGenerationTrace {
    pub(super) prompt_name: String,
    pub(super) request_json: Value,
    pub(super) messages_json: Value,
    pub(super) response_text: String,
    #[serde(default)]
    pub(super) workflow_run_id: Option<i64>,
    #[serde(default)]
    pub(super) call_key: Option<String>,
    #[serde(default)]
    pub(super) call_order: Option<i32>,
    #[serde(default)]
    pub(super) input_fingerprint_json: Option<Value>,
    #[serde(default)]
    pub(super) usage: Option<UsageInfo>,
    #[serde(default)]
    pub(super) finish_reason: Option<String>,
    #[serde(default)]
    pub(super) context_manifest_summary_json: Option<Value>,
    #[serde(skip)]
    pub(super) restored_parsed_output: Option<Value>,
}

pub(super) struct MangaPreparedLlmCall {
    pub(super) request: ChatRequest,
    pub(super) request_json: Value,
    pub(super) messages_json: Value,
    pub(super) call_key: String,
    pub(super) call_order: i32,
    pub(super) input_fingerprint_json: Value,
    pub(super) context_manifest_summary_json: Option<Value>,
}

pub(super) struct MangaRestoredLlmCheckpoint {
    pub(super) response_text: String,
    pub(super) parsed_output: Value,
    pub(super) usage: Option<UsageInfo>,
    pub(super) finish_reason: Option<String>,
}

pub(super) struct MangaContextManifestInput<'a> {
    pub(super) prompt_engine: &'a dyn PromptEngine,
    pub(super) project_id: i64,
    pub(super) variables: &'a HashMap<String, Value>,
    pub(super) chapter_content: &'a str,
    pub(super) source_spans: &'a [SourceSpan],
    pub(super) prompt_names: &'a [&'a str],
    pub(super) previous_outputs: &'a [MangaContextPreviousOutput<'a>],
    pub(super) reference_assets: Option<&'a Value>,
    pub(super) dynamic_user_content: String,
}

pub(super) struct MangaLlmCallInput<'a> {
    pub(super) prompt_engine: &'a dyn PromptEngine,
    pub(super) prompt_name: &'a str,
    pub(super) project_id: i64,
    pub(super) workflow_run_id: Option<i64>,
    pub(super) call_key: String,
    pub(super) call_order: i32,
    pub(super) variables: &'a HashMap<String, Value>,
    pub(super) manifest: LlmContextManifest,
    pub(super) temperature: f32,
}

pub(super) fn build_manga_context_manifest(
    input: MangaContextManifestInput<'_>,
) -> Result<LlmContextManifest, AppError> {
    let mut ledger = LlmContextLedger::new();
    add_manga_source_pack(&mut ledger, input.chapter_content, input.source_spans)?;
    add_manga_prompt_rule_packs(
        &mut ledger,
        input.prompt_engine,
        input.project_id,
        input.variables,
        input.prompt_names,
    )?;
    for previous_output in input.previous_outputs {
        add_manga_previous_output_pack(&mut ledger, previous_output)?;
    }
    if let Some(reference_assets) = input.reference_assets {
        ledger.add_pack(crate::llm_context::LlmContextLedgerPackInput {
            scope_kind: ("chapter").into(),
            pack_kind: ("manga_reference_assets").into(),
            source_refs_json: None,
            config_identity_json: json!({
                "ledger_version": "manga_context_ledger_v1",
                "kind": "reference_assets",
                "sha256": source_content_sha256(&serde_json::to_string(reference_assets).map_err(AppError::Serialization)?)
            }),
            required: false,
            stable_prefix: true,
            variants: vec![
                LlmContextLedgerPackDraft::new(COMPRESSION_FULL_RAW, reference_assets.clone()),
                LlmContextLedgerPackDraft::new(
                    COMPRESSION_MINIMAL,
                    json!({
                        "reference_assets_sha256": source_content_sha256(&serde_json::to_string(reference_assets).map_err(AppError::Serialization)?)
                    }),
                ),
            ],
        })?;
    }
    Ok(ledger.into_manifest(input.dynamic_user_content, None))
}

fn add_manga_source_pack(
    ledger: &mut LlmContextLedger,
    chapter_content: &str,
    source_spans: &[SourceSpan],
) -> Result<(), AppError> {
    let content_sha256 = source_content_sha256(chapter_content);
    ledger.add_pack(crate::llm_context::LlmContextLedgerPackInput {
        scope_kind: ("chapter").into(),
        pack_kind: ("manga_chapter_source").into(),
        source_refs_json: Some(json!({
            "content_sha256": content_sha256,
            "source_span_count": source_spans.len()
        })),
        config_identity_json: json!({
            "ledger_version": "manga_context_ledger_v1",
            "kind": "chapter_source",
            "content_sha256": content_sha256
        }),
        required: true,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FULL_RAW,
                json!({
                    "kind": "chapter_source",
                    "content": chapter_content,
                    "source_spans": source_span_prompt_values(source_spans)
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FOCUSED_RAW,
                json!({
                    "kind": "chapter_source",
                    "source_spans": source_span_prompt_values(source_spans)
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_MINIMAL,
                json!({
                    "kind": "chapter_source",
                    "content_sha256": content_sha256,
                    "source_span_count": source_spans.len()
                }),
            ),
        ],
    })
}

fn add_manga_prompt_rule_packs(
    ledger: &mut LlmContextLedger,
    prompt_engine: &dyn PromptEngine,
    project_id: i64,
    variables: &HashMap<String, Value>,
    prompt_names: &[&str],
) -> Result<(), AppError> {
    let mut seen = HashSet::new();
    for prompt_name in prompt_names {
        if !seen.insert(*prompt_name) {
            continue;
        }
        let rendered = prompt_engine.render(prompt_name, variables, Some(project_id))?;
        let rendered_sha256 = source_content_sha256(&rendered);
        ledger.add_pack(crate::llm_context::LlmContextLedgerPackInput {
            scope_kind: ("chapter").into(),
            pack_kind: format!("manga_prompt_rules:{prompt_name}"),
            source_refs_json: Some(json!({"prompt_name": prompt_name})),
            config_identity_json: json!({
                "ledger_version": "manga_context_ledger_v1",
                "kind": "prompt_rules",
                "prompt_name": prompt_name,
                "rendered_sha256": rendered_sha256
            }),
            required: true,
            stable_prefix: true,
            variants: vec![
                LlmContextLedgerPackDraft::new(
                    COMPRESSION_FULL_RAW,
                    json!({
                        "prompt_name": prompt_name,
                        "rendered_prompt": rendered
                    }),
                ),
                LlmContextLedgerPackDraft::new(
                    COMPRESSION_MINIMAL,
                    json!({
                        "prompt_name": prompt_name,
                        "rendered_sha256": rendered_sha256
                    }),
                ),
            ],
        })?;
    }
    Ok(())
}

fn add_manga_previous_output_pack(
    ledger: &mut LlmContextLedger,
    output: &MangaContextPreviousOutput<'_>,
) -> Result<(), AppError> {
    let raw_sha256 = output.raw_output.map(source_content_sha256);
    ledger.add_pack(crate::llm_context::LlmContextLedgerPackInput {
        scope_kind: ("chapter").into(),
        pack_kind: (output.pack_kind).into(),
        source_refs_json: Some(json!({
            "prompt_name": output.prompt_name,
            "placeholder": output.placeholder
        })),
        config_identity_json: json!({
            "ledger_version": "manga_context_ledger_v1",
            "kind": "previous_output",
            "pack_kind": output.pack_kind,
            "prompt_name": output.prompt_name,
            "raw_sha256": raw_sha256,
            "placeholder": output.placeholder
        }),
        required: true,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FULL_RAW,
                json!({
                    "prompt_name": output.prompt_name,
                    "placeholder": output.placeholder,
                    "raw_output": output.raw_output,
                    "parsed_output": output.parsed_output
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FOCUSED_RAW,
                json!({
                    "prompt_name": output.prompt_name,
                    "placeholder": output.placeholder,
                    "parsed_output": output.parsed_output
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_MINIMAL,
                json!({
                    "prompt_name": output.prompt_name,
                    "placeholder": output.placeholder,
                    "raw_sha256": raw_sha256
                }),
            ),
        ],
    })
}

pub(super) fn manga_pipeline_prompt_fingerprint(
    prompt_engine: &dyn PromptEngine,
    project_id: i64,
    variables: &HashMap<String, Value>,
) -> Result<Value, AppError> {
    let mut prompts = Vec::with_capacity(MANGA_PIPELINE_PROMPT_NAMES.len());
    for name in MANGA_PIPELINE_PROMPT_NAMES {
        let rendered = prompt_engine.render(name, variables, Some(project_id))?;
        prompts.push(json!({
            "name": name,
            "sha256": source_content_sha256(&rendered),
        }));
    }
    Ok(json!({
        "schema_version": 2,
        "prompts": prompts,
    }))
}

pub(super) fn manga_item_extraction_input(speech: Value, visual_anchors: Value) -> Value {
    let available_source_span_ids = manga_item_available_source_span_ids(&speech, &visual_anchors);
    json!({
        "available_source_span_ids": available_source_span_ids,
        "previous_steps": {
            "speech": speech,
            "visual_anchors": visual_anchors
        }
    })
}

pub(super) fn manga_item_available_source_span_ids(
    speech: &Value,
    visual_anchors: &Value,
) -> Vec<String> {
    let mut ids = Vec::new();
    let mut seen = HashSet::new();
    for field in ["dialogues", "narrations"] {
        for item in speech
            .get(field)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            push_unique_source_span_id(item.get("source_span_id"), &mut ids, &mut seen);
        }
    }
    for scene in visual_anchors
        .get("scenes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        for id in scene
            .get("source_span_ids")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            push_unique_source_span_id(Some(id), &mut ids, &mut seen);
        }
    }
    ids
}

fn push_unique_source_span_id(
    value: Option<&Value>,
    ids: &mut Vec<String>,
    seen: &mut HashSet<String>,
) {
    let Some(source_span_id) = value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return;
    };
    if seen.insert(source_span_id.to_string()) {
        ids.push(source_span_id.to_string());
    }
}

fn push_unique_source_span_ids_from_array(
    value: Option<&Value>,
    ids: &mut Vec<String>,
    seen: &mut HashSet<String>,
) {
    for item in value.and_then(Value::as_array).into_iter().flatten() {
        push_unique_source_span_id(Some(item), ids, seen);
    }
}

fn event_source_span_windows(events_json: &Value, source_spans: &[SourceSpan]) -> Value {
    let mut windows = Vec::new();
    let mut available_source_span_ids = Vec::new();
    let mut seen_available = HashSet::new();
    let Some(events) = events_json.as_array() else {
        return json!({
            "source_windows": windows,
            "available_source_spans": available_source_span_ids
        });
    };

    for (event_index, event) in events.iter().enumerate() {
        let mut ids = Vec::new();
        let mut seen_ids = HashSet::new();
        push_unique_source_span_ids_from_array(
            event.get("source_span_ids"),
            &mut ids,
            &mut seen_ids,
        );
        if ids.is_empty() {
            continue;
        }
        let mut text_parts = Vec::new();
        for id in &ids {
            if let Some(span) = source_spans.iter().find(|span| span.id == *id) {
                text_parts.push(span.text.clone());
                if seen_available.insert(span.id.clone()) {
                    available_source_span_ids.push(source_span_prompt_value(span));
                }
            }
        }
        windows.push(json!({
            "window_id": format!("event_window_{}", event_index + 1),
            "event_index": event_index,
            "event": event,
            "source_span_ids": ids,
            "text": text_parts.join("\n")
        }));
    }

    json!({
        "source_windows": windows,
        "available_source_spans": available_source_span_ids
    })
}

pub(super) fn manga_page_planning_input(extraction_summary: Value) -> Value {
    json!({
        "extraction_summary": extraction_summary
    })
}

pub(super) fn manga_page_drawing_prompt_page_input(
    page_plan: &Value,
    extraction_summary: &Value,
) -> Value {
    let event_indices = event_indices_from_page(page_plan);
    let events = filter_array_by_index(extraction_summary.get("events"), &event_indices);
    let character_names = character_names_for_page(page_plan, &events);
    let characters = filter_characters(extraction_summary.get("characters"), &character_names);
    let reference_assets =
        filter_reference_assets(extraction_summary.get("reference_assets"), &character_names);
    let speech = extraction_summary.get("speech").unwrap_or(&Value::Null);
    let visual_anchors = extraction_summary
        .get("visual_anchors")
        .unwrap_or(&Value::Null);
    let items = extraction_summary.get("items").unwrap_or(&Value::Null);
    let comic_synthesis = extraction_summary
        .get("comic_synthesis")
        .unwrap_or(&Value::Null);

    json!({
        "current_page": page_plan,
        "page_context": {
            "characters": characters,
            "events": events,
            "dialogues": filter_array_by_event_index(speech.get("dialogues"), &event_indices),
            "narrations": filter_array_by_event_index(speech.get("narrations"), &event_indices),
            "scenes": filter_scenes_by_event_indices(visual_anchors.get("scenes"), &event_indices),
            "items": filter_items_by_first_event(Some(items), &event_indices),
            "chapter_summary": comic_synthesis.get("chapter_summary").cloned().unwrap_or(Value::Null),
            "mood_progression": comic_synthesis.get("mood_progression").cloned().unwrap_or(Value::Null),
        },
        "reference_assets": reference_assets,
    })
}

pub(super) fn build_manga_step_user_message(
    prompt_name: &str,
    instruction: &str,
    input: Value,
    output_contract: Value,
) -> Result<String, AppError> {
    let payload = json!({
        "task": {
            "prompt_name": prompt_name,
            "instruction": instruction
        },
        "output_contract": output_contract,
        "input": input
    });
    build_json_task_user_message(
        "执行漫画流水线步骤",
        "只返回 output_contract 要求的严格 JSON 对象，不要输出 Markdown、代码块、解释文字或额外字段",
        &payload,
    )
}

pub(super) fn manga_event_beats_user_message(
    chapter_content: &str,
    source_spans: &[SourceSpan],
) -> Result<String, AppError> {
    build_manga_step_user_message(
        "manga_event_beats",
        "从章节正文中抽取适合漫画分页和分镜的事件卡片",
        json!({
            "chapter": {
                "content": chapter_content,
                "source_spans": source_span_prompt_values(source_spans)
            }
        }),
        json!({
            "format": "json_object",
            "prompt_name": "manga_event_beats",
            "required_fields": ["events"],
            "source_span_rule": "events[].source_span_ids 必须从 input.chapter.source_spans 中选择，至少 1 个，不要自造 ID"
        }),
    )
}

pub(super) fn manga_character_binding_user_message(
    chapter_content: &str,
    events_json: Value,
    source_spans: &[SourceSpan],
    reference_assets: Value,
) -> Result<String, AppError> {
    build_manga_step_user_message(
        "manga_character_binding",
        "基于事件卡片、章节正文和角色立绘参考绑定本章出场角色身份与外观",
        json!({
            "chapter": {
                "content": chapter_content,
                "source_spans": source_span_prompt_values(source_spans)
            },
            "previous_step": {
                "events": events_json
            },
            "reference_assets": reference_assets
        }),
        json!({
            "format": "json_object",
            "prompt_name": "manga_character_binding",
            "required_fields": ["characters"],
            "source_span_rule": "characters.*.source_span_ids 必须从 input.chapter.source_spans 中选择，至少 1 个，不要自造 ID"
        }),
    )
}

pub(super) fn validate_manga_event_beats_source_span_ids(
    events_json: &Value,
    source_spans: &[SourceSpan],
) -> Result<(), AppError> {
    reject_manga_unknown_fields(events_json, &["events"], "manga_event_beats")?;
    let events = events_json
        .get("events")
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::InvalidArgument("manga_event_beats.events 必须是数组".into()))?;
    validate_manga_events_array(events, source_spans, "manga_event_beats.events")
}

pub(super) fn validate_manga_character_binding_source_span_ids(
    characters_json: &Value,
    source_spans: &[SourceSpan],
) -> Result<(), AppError> {
    reject_manga_unknown_fields(characters_json, &["characters"], "manga_character_binding")?;
    let characters = characters_json
        .get("characters")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            AppError::InvalidArgument("manga_character_binding.characters 必须是对象".into())
        })?;
    validate_manga_characters_object(
        characters,
        source_spans,
        "manga_character_binding.characters",
    )
}

fn validate_manga_characters_object(
    characters: &Map<String, Value>,
    source_spans: &[SourceSpan],
    path: &str,
) -> Result<(), AppError> {
    for (name, character) in characters {
        let item_path = format!("{path}.{name}");
        reject_manga_unknown_fields(
            character,
            &[
                "name",
                "appearance",
                "personality",
                "role",
                "gender",
                "age_description",
                "source_span_ids",
            ],
            &item_path,
        )?;
        required_manga_string(character, "name", &item_path)?;
        required_manga_string(character, "appearance", &item_path)?;
        required_manga_string(character, "personality", &item_path)?;
        required_manga_string(character, "role", &item_path)?;
        required_manga_string(character, "gender", &item_path)?;
        required_manga_string(character, "age_description", &item_path)?;
        validate_manga_source_span_id_array(
            character,
            source_spans,
            &format!("{item_path}.source_span_ids"),
        )?;
    }
    Ok(())
}

fn validate_manga_events_array(
    events: &[Value],
    source_spans: &[SourceSpan],
    path: &str,
) -> Result<(), AppError> {
    for (index, event) in events.iter().enumerate() {
        let item_path = format!("{path}[{index}]");
        reject_manga_unknown_fields(
            event,
            &[
                "index",
                "type",
                "description",
                "participants",
                "importance",
                "is_climax",
                "source_span_ids",
            ],
            &item_path,
        )?;
        validate_manga_event_index(event.get("index"), &format!("{item_path}.index"))?;
        required_manga_string(event, "type", &item_path)?;
        required_manga_string(event, "description", &item_path)?;
        validate_manga_string_array(
            event.get("participants"),
            &format!("{item_path}.participants"),
        )?;
        required_manga_string(event, "importance", &item_path)?;
        required_manga_bool(event.get("is_climax"), &format!("{item_path}.is_climax"))?;
        validate_manga_source_span_id_array(
            event,
            source_spans,
            &format!("{item_path}.source_span_ids"),
        )?;
    }
    Ok(())
}

pub(super) fn manga_speech_selection_user_message(
    characters_json: Value,
    events_json: Value,
    source_spans: &[SourceSpan],
    reference_assets: Value,
) -> Result<String, AppError> {
    let chapter_context = event_source_span_windows(&events_json, source_spans);
    build_manga_step_user_message(
        "manga_speech_selection",
        "基于事件窗口、角色绑定和事件卡片，选择关键对白、想法与旁白",
        json!({
            "chapter": chapter_context,
            "previous_steps": {
                "characters": characters_json,
                "events": events_json
            },
            "reference_assets": reference_assets
        }),
        json!({
            "format": "json_object",
            "prompt_name": "manga_speech_selection",
            "required_fields": ["dialogues", "narrations"],
            "source_span_rule": "dialogues[] 和 narrations[] 每项必须输出 source_span_id；只能从 input.chapter.available_source_spans 中选择，不要自造 ID",
            "limits": {
                "dialogues_max": 24,
                "narrations_max": 8,
                "dialogue_content_max_chars": 30,
                "narration_content_max_chars": 40
            }
        }),
    )
}

pub(super) fn validate_manga_speech_selection_source_span_ids(
    speech_json: &Value,
    source_spans: &[SourceSpan],
) -> Result<(), AppError> {
    reject_manga_unknown_fields(
        speech_json,
        &["dialogues", "narrations"],
        "manga_speech_selection",
    )?;
    validate_manga_speech_source_span_array(
        speech_json,
        source_spans,
        "dialogues",
        "manga_speech_selection",
    )?;
    validate_manga_speech_source_span_array(
        speech_json,
        source_spans,
        "narrations",
        "manga_speech_selection",
    )?;
    Ok(())
}

fn validate_manga_speech_source_span_array(
    speech_json: &Value,
    source_spans: &[SourceSpan],
    field: &str,
    context: &str,
) -> Result<(), AppError> {
    let items = speech_json
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::InvalidArgument(format!("{context}.{field} 必须是数组")))?;
    for (index, item) in items.iter().enumerate() {
        let item_context = format!("{context}.{field}[{index}]");
        let allowed = match field {
            "dialogues" => &[
                "index",
                "speaker",
                "content",
                "source_span_id",
                "emotion",
                "target",
                "event_index",
                "is_internal",
                "bubble_type",
            ][..],
            "narrations" => &[
                "index",
                "content",
                "source_span_id",
                "narration_type",
                "event_index",
                "position",
            ][..],
            _ => &[][..],
        };
        reject_manga_unknown_fields(item, allowed, &item_context)?;
        validate_manga_event_index(item.get("index"), &format!("{item_context}.index"))?;
        required_manga_string(item, "content", &item_context)?;
        if field == "dialogues" {
            required_manga_string(item, "speaker", &item_context)?;
            required_manga_string(item, "emotion", &item_context)?;
            validate_manga_nullable_string(item.get("target"), &format!("{item_context}.target"))?;
            required_manga_bool(
                item.get("is_internal"),
                &format!("{item_context}.is_internal"),
            )?;
            required_manga_string(item, "bubble_type", &item_context)?;
        } else {
            required_manga_string(item, "narration_type", &item_context)?;
            required_manga_string(item, "position", &item_context)?;
        }
        let source_span_id = item
            .get("source_span_id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "{context}.{field}[{index}].source_span_id 不能为空"
                ))
            })?;
        resolve_source_span_text(source_spans, source_span_id).map_err(|err| {
            AppError::InvalidArgument(format!(
                "{context}.{field}[{index}].source_span_id 无效：{err}"
            ))
        })?;
        validate_manga_event_index(
            item.get("event_index"),
            &format!("{context}.{field}[{index}].event_index"),
        )?;
    }
    Ok(())
}

fn validate_manga_event_index(value: Option<&Value>, path: &str) -> Result<i64, AppError> {
    let event_index = value
        .and_then(Value::as_i64)
        .ok_or_else(|| AppError::InvalidArgument(format!("{path} 必须是非负整数")))?;
    if event_index < 0 {
        return Err(AppError::InvalidArgument(format!("{path} 必须是非负整数")));
    }
    Ok(event_index)
}

fn validate_manga_event_indices(value: Option<&Value>, path: &str) -> Result<(), AppError> {
    let indices = value
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::InvalidArgument(format!("{path} 必须是数组")))?;
    if indices.is_empty() {
        return Err(AppError::InvalidArgument(format!("{path} 不能为空")));
    }

    let mut seen = HashSet::with_capacity(indices.len());
    for (index, value) in indices.iter().enumerate() {
        let event_index = validate_manga_event_index(Some(value), &format!("{path}[{index}]"))?;
        if !seen.insert(event_index) {
            return Err(AppError::InvalidArgument(format!(
                "{path} 包含重复事件索引：{event_index}"
            )));
        }
    }
    Ok(())
}

fn validate_manga_source_span_id_array(
    value: &Value,
    source_spans: &[SourceSpan],
    path: &str,
) -> Result<(), AppError> {
    let ids = value
        .get("source_span_ids")
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::InvalidArgument(format!("{path} 必须是数组")))?;
    if ids.is_empty() {
        return Err(AppError::InvalidArgument(format!("{path} 不能为空")));
    }
    for (index, id) in ids.iter().enumerate() {
        let source_span_id = id
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                AppError::InvalidArgument(format!("{path}[{index}] 必须是非空字符串"))
            })?;
        resolve_source_span_text(source_spans, source_span_id)
            .map_err(|err| AppError::InvalidArgument(format!("{path}[{index}] 无效：{err}")))?;
    }
    Ok(())
}

pub(super) fn manga_visual_anchor_extraction_user_message(
    events_json: Value,
    source_spans: &[SourceSpan],
) -> Result<String, AppError> {
    let chapter_context = event_source_span_windows(&events_json, source_spans);
    build_manga_step_user_message(
        "manga_visual_anchor_extraction",
        "基于事件窗口和事件卡片提取场景边界、地点氛围与视觉锚点",
        json!({
            "chapter": chapter_context,
            "previous_step": {
                "events": events_json
            }
        }),
        json!({
            "format": "json_object",
            "prompt_name": "manga_visual_anchor_extraction",
            "required_fields": ["scenes"],
            "source_span_rule": "scenes[] 每项必须输出 source_span_ids；只能从 input.chapter.available_source_spans 中选择，至少 1 个，不要自造 ID"
        }),
    )
}

pub(super) fn validate_manga_visual_anchor_source_span_ids(
    visual_json: &Value,
    source_spans: &[SourceSpan],
) -> Result<(), AppError> {
    reject_manga_unknown_fields(visual_json, &["scenes"], "manga_visual_anchor_extraction")?;
    let scenes = visual_json
        .get("scenes")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            AppError::InvalidArgument("manga_visual_anchor_extraction.scenes 必须是数组".into())
        })?;
    for (index, scene) in scenes.iter().enumerate() {
        let path = format!("manga_visual_anchor_extraction.scenes[{index}]");
        reject_manga_unknown_fields(
            scene,
            &[
                "index",
                "location",
                "location_en",
                "time_of_day",
                "atmosphere",
                "weather",
                "lighting",
                "indoor_outdoor",
                "description",
                "event_indices",
                "source_span_ids",
            ],
            &path,
        )?;
        validate_manga_event_index(scene.get("index"), &format!("{path}.index"))?;
        required_manga_string(scene, "location", &path)?;
        required_manga_string(scene, "location_en", &path)?;
        required_manga_string(scene, "time_of_day", &path)?;
        required_manga_string(scene, "atmosphere", &path)?;
        validate_manga_nullable_string(scene.get("weather"), &format!("{path}.weather"))?;
        required_manga_string(scene, "lighting", &path)?;
        required_manga_string(scene, "indoor_outdoor", &path)?;
        required_manga_string(scene, "description", &path)?;
        validate_manga_source_span_id_array(
            scene,
            source_spans,
            &format!("manga_visual_anchor_extraction.scenes[{index}].source_span_ids"),
        )?;
        validate_manga_event_indices(
            scene.get("event_indices"),
            &format!("manga_visual_anchor_extraction.scenes[{index}].event_indices"),
        )?;
    }
    Ok(())
}

pub(super) fn manga_item_extraction_user_message(
    speech_json: Value,
    visual_json: Value,
) -> Result<String, AppError> {
    build_manga_step_user_message(
        "manga_item_extraction",
        "基于对白旁白和视觉锚点抽取漫画页需要稳定绘制的关键物品",
        manga_item_extraction_input(speech_json, visual_json),
        json!({
            "format": "json_object",
            "prompt_name": "manga_item_extraction",
            "required_fields": ["items"],
            "source_span_rule": "items[] 每项必须输出 source_span_ids；只能从 input.available_source_span_ids 中选择，至少 1 个，不要自造 ID；没有关键物品时 items 输出空数组"
        }),
    )
}

pub(super) fn manga_chapter_comic_synthesis_user_message(
    events_json: Value,
    speech_json: Value,
    visual_json: Value,
    items_json: Value,
) -> Result<String, AppError> {
    build_manga_step_user_message(
        "manga_chapter_comic_synthesis",
        "基于事件、对白旁白、视觉锚点和物品生成漫画分页所需章节综合信息",
        json!({
            "previous_steps": {
                "events": events_json,
                "speech": speech_json,
                "visual_anchors": visual_json,
                "items": items_json
            }
        }),
        json!({
            "format": "json_object",
            "prompt_name": "manga_chapter_comic_synthesis",
            "required_fields": ["chapter_summary", "mood_progression", "total_estimated_pages"],
            "limits": {
                "total_estimated_pages_min": 5,
                "total_estimated_pages_max": 15
            }
        }),
    )
}

pub(super) fn validate_manga_item_extraction_source_span_ids(
    items_json: &Value,
    source_spans: &[SourceSpan],
    allowed_source_span_ids: &[String],
) -> Result<(), AppError> {
    reject_manga_unknown_fields(items_json, &["items"], "manga_item_extraction")?;
    let items = items_json
        .get("items")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            AppError::InvalidArgument("manga_item_extraction.items 必须是数组".into())
        })?;
    let allowed = allowed_source_span_ids
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    for (index, item) in items.iter().enumerate() {
        let item_context = format!("manga_item_extraction.items[{index}]");
        reject_manga_unknown_fields(
            item,
            &[
                "name",
                "description",
                "description_en",
                "importance",
                "first_appearance_event",
                "visual_features",
                "source_span_ids",
            ],
            &item_context,
        )?;
        required_manga_string(item, "name", &item_context)?;
        required_manga_string(item, "description", &item_context)?;
        required_manga_string(item, "description_en", &item_context)?;
        required_manga_string(item, "importance", &item_context)?;
        required_manga_string(item, "visual_features", &item_context)?;
        let path = format!("manga_item_extraction.items[{index}].source_span_ids");
        validate_manga_source_span_id_array(item, source_spans, &path)?;
        validate_manga_event_index(
            item.get("first_appearance_event"),
            &format!("manga_item_extraction.items[{index}].first_appearance_event"),
        )?;
        let ids = item
            .get("source_span_ids")
            .and_then(Value::as_array)
            .expect("validate_manga_source_span_id_array already checked source_span_ids");
        for (id_index, id) in ids.iter().enumerate() {
            let source_span_id = id
                .as_str()
                .map(str::trim)
                .expect("validate_manga_source_span_id_array already checked string ids");
            if !allowed.contains(source_span_id) {
                return Err(AppError::InvalidArgument(format!(
                    "{path}[{id_index}] 不在 input.available_source_span_ids 中: {source_span_id}"
                )));
            }
        }
    }
    Ok(())
}

pub(super) fn validate_manga_chapter_comic_synthesis(
    synthesis_json: &Value,
) -> Result<(), AppError> {
    reject_manga_unknown_fields(
        synthesis_json,
        &[
            "chapter_summary",
            "mood_progression",
            "total_estimated_pages",
        ],
        "manga_chapter_comic_synthesis",
    )?;
    required_manga_string(
        synthesis_json,
        "chapter_summary",
        "manga_chapter_comic_synthesis",
    )?;
    validate_manga_string_array(
        synthesis_json.get("mood_progression"),
        "manga_chapter_comic_synthesis.mood_progression",
    )?;
    let total = validate_manga_positive_i64(
        synthesis_json.get("total_estimated_pages"),
        "manga_chapter_comic_synthesis.total_estimated_pages",
    )?;
    if !(5..=15).contains(&total) {
        return Err(AppError::InvalidArgument(format!(
            "manga_chapter_comic_synthesis.total_estimated_pages 必须在 5-15 之间，实际为 {total}"
        )));
    }
    Ok(())
}

fn reject_manga_unknown_fields(
    value: &Value,
    allowed: &[&str],
    path: &str,
) -> Result<(), AppError> {
    let object = value
        .as_object()
        .ok_or_else(|| AppError::InvalidArgument(format!("{path} 必须是 JSON 对象")))?;
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(AppError::InvalidArgument(format!(
                "{path} 包含未知字段 {key}；允许字段：{}",
                allowed.join(", ")
            )));
        }
    }
    Ok(())
}

fn required_manga_string(value: &Value, key: &str, path: &str) -> Result<String, AppError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| AppError::InvalidArgument(format!("{path}.{key} 必须是非空字符串")))
}

fn required_manga_bool(value: Option<&Value>, path: &str) -> Result<(), AppError> {
    if value.and_then(Value::as_bool).is_some() {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(format!("{path} 必须是布尔值")))
    }
}

fn validate_manga_nullable_string(value: Option<&Value>, path: &str) -> Result<(), AppError> {
    match value {
        Some(Value::Null) => Ok(()),
        Some(Value::String(text)) if !text.trim().is_empty() => Ok(()),
        Some(Value::String(_)) => Err(AppError::InvalidArgument(format!(
            "{path} 必须是非空字符串或 null"
        ))),
        Some(_) => Err(AppError::InvalidArgument(format!(
            "{path} 必须是字符串或 null"
        ))),
        None => Err(AppError::InvalidArgument(format!("{path} 缺失"))),
    }
}

fn validate_manga_string_array(value: Option<&Value>, path: &str) -> Result<(), AppError> {
    let values = value
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::InvalidArgument(format!("{path} 必须是数组")))?;
    for (index, value) in values.iter().enumerate() {
        let text = value.as_str().map(str::trim).ok_or_else(|| {
            AppError::InvalidArgument(format!("{path}[{index}] 必须是非空字符串"))
        })?;
        if text.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "{path}[{index}] 必须是非空字符串"
            )));
        }
    }
    Ok(())
}

fn validate_manga_positive_i64(value: Option<&Value>, path: &str) -> Result<i64, AppError> {
    let number = value
        .and_then(Value::as_i64)
        .ok_or_else(|| AppError::InvalidArgument(format!("{path} 必须是正整数")))?;
    if number <= 0 {
        return Err(AppError::InvalidArgument(format!("{path} 必须是正整数")));
    }
    Ok(number)
}

pub(super) fn manga_page_planning_user_message(
    extraction_summary: Value,
) -> Result<String, AppError> {
    build_manga_step_user_message(
        "manga_page_planning",
        "基于完整抽取信息规划漫画分页结构",
        manga_page_planning_input(extraction_summary),
        json!({
            "format": "json_object",
            "prompt_name": "manga_page_planning"
        }),
    )
}

pub(super) fn manga_page_drawing_prompt_user_message(
    page_number: i32,
    page_plan: &Value,
    extraction_summary: &Value,
) -> Result<String, AppError> {
    build_manga_step_user_message(
        "manga_page_drawing_prompt",
        &format!(
            "只为第 {page_number} 页生成一个整页漫画绘图 prompt。输出 JSON 的 pages 数组必须且只能包含这一页。"
        ),
        manga_page_drawing_prompt_page_input(page_plan, extraction_summary),
        json!({
            "format": "json_object",
            "prompt_name": "manga_page_drawing_prompt",
            "mode": "single_page",
            "required_root_field": "pages",
            "pages": "array",
            "constraints": {
                "pages_length": 1,
                "page_number": page_number
            }
        }),
    )
}

pub(super) fn build_manga_chat_request(
    prompt_engine: &dyn PromptEngine,
    prompt_name: &str,
    project_id: i64,
    variables: &HashMap<String, Value>,
    manifest: LlmContextManifest,
    temperature: f32,
) -> Result<ChatRequest, AppError> {
    let sys = prompt_engine.render(prompt_name, variables, Some(project_id))?;
    Ok(context_aware_system_user_request(
        sys,
        manifest,
        Some(temperature),
        None,
        Some(MANGA_PIPELINE_MAX_TOKENS_PROFILE.into()),
    )?
    .with_json_response_format()
    .with_trace_context(LlmTraceContext {
        project_id: Some(project_id),
        root_project_id: None,
        workflow: MANGA_PIPELINE_WORKFLOW.to_string(),
        step_key: prompt_name.to_string(),
        generation_run_id: None,
        workflow_run_id: None,
        call_key: None,
        input_fingerprint_sha256: None,
        context_plan_json: None,
        context_pack_refs_json: None,
    }))
}

pub(super) fn prepare_manga_llm_call(
    input: MangaLlmCallInput<'_>,
) -> Result<MangaPreparedLlmCall, AppError> {
    let MangaLlmCallInput {
        prompt_engine,
        prompt_name,
        project_id,
        workflow_run_id,
        call_key,
        call_order,
        variables,
        manifest,
        temperature,
    } = input;
    let mut request = build_manga_chat_request(
        prompt_engine,
        prompt_name,
        project_id,
        variables,
        manifest,
        temperature,
    )?;
    let context_manifest_summary_json =
        manga_request_context_manifest_summary(request.context_manifest_json.as_ref())?;
    if let Some(trace_context) = request.trace_context.as_mut() {
        trace_context.workflow_run_id = workflow_run_id;
        trace_context.call_key = Some(call_key.clone());
    }
    let fingerprint_request_json = context_aware_chat_request_trace_json(&request)?;
    let input_fingerprint_json = json!({
        "prompt_name": prompt_name,
        "project_id": project_id,
        "request": fingerprint_request_json,
        "context_manifest_summary": context_manifest_summary_json.clone(),
    });
    let input_fingerprint_sha256 = llm_call_checkpoint_fingerprint_sha256(&input_fingerprint_json)?;
    if let Some(trace_context) = request.trace_context.as_mut() {
        trace_context.input_fingerprint_sha256 = Some(input_fingerprint_sha256);
    }
    let request_json = context_aware_chat_request_trace_json(&request)?;
    let messages_json = serde_json::to_value(&request.messages).map_err(|err| {
        AppError::InvalidArgument(format!("漫画流水线 LLM 消息序列化失败: {err}"))
    })?;
    Ok(MangaPreparedLlmCall {
        request,
        request_json,
        messages_json,
        call_key,
        call_order,
        input_fingerprint_json,
        context_manifest_summary_json,
    })
}

pub(super) async fn load_manga_restored_llm_checkpoint(
    db: Arc<DatabaseConnection>,
    workflow_run_id: i64,
    call_key: &str,
    input_fingerprint_json: &Value,
    prompt_name: &str,
) -> Result<Option<MangaRestoredLlmCheckpoint>, AppError> {
    let Some(checkpoint) =
        load_completed_llm_call_checkpoint(db, workflow_run_id, call_key, input_fingerprint_json)
            .await?
    else {
        return Ok(None);
    };
    let checkpoint_json = required_llm_call_checkpoint_json(checkpoint, call_key)?;
    let response_text =
        manga_llm_checkpoint_string(&checkpoint_json, call_key, "response_content")?;
    let finish_reason = manga_llm_checkpoint_optional_string(&checkpoint_json, "finish_reason")?;
    if is_truncated_finish_reason(finish_reason.as_deref()) {
        return Err(AppError::InvalidArgument(format!(
            "{prompt_name} checkpoint 标记为截断响应，不能用于恢复"
        )));
    }
    let parsed_output = checkpoint_json
        .get("parsed_output")
        .cloned()
        .ok_or_else(|| AppError::InvalidArgument(format!("{call_key}.parsed_output 缺失")))?;
    let usage = checkpoint_json
        .get("usage")
        .filter(|value| !value.is_null())
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(AppError::Serialization)?;
    Ok(Some(MangaRestoredLlmCheckpoint {
        response_text,
        parsed_output,
        usage,
        finish_reason,
    }))
}

pub(super) struct MangaRenderTraceInput<'a> {
    pub db: Arc<DatabaseConnection>,
    pub prompt_engine: &'a dyn PromptEngine,
    pub llm: &'a dyn LlmProvider,
    pub prompt_name: &'a str,
    pub project_id: i64,
    pub workflow_run_id: Option<i64>,
    pub call_key: String,
    pub call_order: i32,
    pub restore_from_checkpoint: bool,
    pub variables: &'a HashMap<String, Value>,
    pub manifest: LlmContextManifest,
    pub temperature: f32,
}

pub(super) async fn render_and_chat_trace(
    input: MangaRenderTraceInput<'_>,
) -> Result<MangaPromptGenerationTrace, AppError> {
    let MangaRenderTraceInput {
        db,
        prompt_engine,
        llm,
        prompt_name,
        project_id,
        workflow_run_id,
        call_key,
        call_order,
        restore_from_checkpoint,
        variables,
        manifest,
        temperature,
    } = input;
    let workflow_run_id = required_llm_call_workflow_run_id(workflow_run_id, &call_key)?;
    let prepared = prepare_manga_llm_call(MangaLlmCallInput {
        prompt_engine,
        prompt_name,
        project_id,
        workflow_run_id: Some(workflow_run_id),
        call_key,
        call_order,
        variables,
        manifest,
        temperature,
    })?;
    let (response_text, usage, finish_reason, restored_parsed_output) = if restore_from_checkpoint
        && let Some(restored) = load_manga_restored_llm_checkpoint(
            db,
            workflow_run_id,
            &prepared.call_key,
            &prepared.input_fingerprint_json,
            prompt_name,
        )
        .await?
    {
        (
            restored.response_text,
            restored.usage,
            restored.finish_reason,
            Some(restored.parsed_output),
        )
    } else {
        let resp = llm.chat(prepared.request).await?;
        if is_truncated_finish_reason(resp.finish_reason.as_deref()) {
            return Err(AppError::InvalidArgument(format!(
                "{prompt_name} 输出被 max_tokens 截断，未形成完整 JSON。当前使用 token 档位 {MANGA_PIPELINE_MAX_TOKENS_PROFILE}，请减少本章输入规模或增大该档位后重试。"
            )));
        }
        (resp.message.content, resp.usage, resp.finish_reason, None)
    };
    Ok(MangaPromptGenerationTrace {
        prompt_name: prompt_name.to_string(),
        request_json: prepared.request_json,
        messages_json: prepared.messages_json,
        response_text,
        workflow_run_id: Some(workflow_run_id),
        call_key: Some(prepared.call_key),
        call_order: Some(prepared.call_order),
        input_fingerprint_json: Some(prepared.input_fingerprint_json),
        usage,
        finish_reason,
        context_manifest_summary_json: prepared.context_manifest_summary_json,
        restored_parsed_output,
    })
}

fn manga_llm_checkpoint_string(
    checkpoint_json: &Value,
    call_key: &str,
    field: &str,
) -> Result<String, AppError> {
    checkpoint_json
        .get(field)
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| AppError::InvalidArgument(format!("{call_key}.{field} 必须是字符串")))
}

fn manga_llm_checkpoint_optional_string(
    checkpoint_json: &Value,
    field: &str,
) -> Result<Option<String>, AppError> {
    match checkpoint_json.get(field) {
        Some(Value::Null) | None => Ok(None),
        Some(value) => value
            .as_str()
            .map(ToString::to_string)
            .map(Some)
            .ok_or_else(|| AppError::InvalidArgument(format!("{field} 必须是字符串或 null"))),
    }
}

pub(super) fn manga_request_context_manifest_summary(
    manifest_json: Option<&Value>,
) -> Result<Option<Value>, AppError> {
    let Some(manifest_json) = manifest_json else {
        return Ok(None);
    };
    let Some(manifest) = manifest_json.as_object() else {
        return Err(AppError::InvalidArgument(
            "漫画流水线 LLM 上下文 manifest 必须是对象".to_string(),
        ));
    };
    let packs = manifest
        .get("packs")
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::InvalidArgument("漫画流水线 LLM 上下文 packs 缺失".to_string()))?;
    let pack_summaries = packs
        .iter()
        .map(manga_context_pack_summary)
        .collect::<Result<Vec<_>, AppError>>()?;
    let dynamic_user_content = manifest
        .get("dynamic_user_content")
        .and_then(Value::as_str)
        .unwrap_or_default();
    Ok(Some(json!({
        "schema_version": manifest.get("schema_version").cloned().unwrap_or(Value::Null),
        "pack_count": pack_summaries.len(),
        "required_pack_count": pack_summaries
            .iter()
            .filter(|pack| pack.get("required").and_then(Value::as_bool).unwrap_or(false))
            .count(),
        "stable_prefix_pack_count": pack_summaries
            .iter()
            .filter(|pack| pack.get("stable_prefix").and_then(Value::as_bool).unwrap_or(false))
            .count(),
        "dynamic_user_content_chars": dynamic_user_content.chars().count(),
        "dynamic_user_content_sha256": source_content_sha256(dynamic_user_content),
        "output_contract_present": manifest
            .get("output_contract")
            .and_then(Value::as_str)
            .is_some_and(|value| !value.trim().is_empty()),
        "packs": pack_summaries,
    })))
}

fn manga_context_pack_summary(pack: &Value) -> Result<Value, AppError> {
    let Some(pack_object) = pack.as_object() else {
        return Err(AppError::InvalidArgument(
            "漫画流水线 LLM 上下文 pack 必须是对象".to_string(),
        ));
    };
    let variants = pack_object
        .get("variants")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            AppError::InvalidArgument("漫画流水线 LLM 上下文 variants 缺失".to_string())
        })?;
    let variant_summaries = variants
        .iter()
        .map(|variant| {
            let Some(variant_object) = variant.as_object() else {
                return Err(AppError::InvalidArgument(
                    "漫画流水线 LLM 上下文 variant 必须是对象".to_string(),
                ));
            };
            let canonical_content_json = variant_object
                .get("canonical_content_json")
                .cloned()
                .unwrap_or(Value::Null);
            let canonical_content_text =
                serde_json::to_string(&canonical_content_json).map_err(AppError::Serialization)?;
            Ok(json!({
                "compression_level": variant_object
                    .get("compression_level")
                    .cloned()
                    .unwrap_or(Value::Null),
                "canonical_content_chars": canonical_content_text.chars().count(),
                "canonical_content_sha256": source_content_sha256(&canonical_content_text),
            }))
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    Ok(json!({
        "scope_kind": pack_object.get("scope_kind").cloned().unwrap_or(Value::Null),
        "pack_kind": pack_object.get("pack_kind").cloned().unwrap_or(Value::Null),
        "source_refs_json": pack_object.get("source_refs_json").cloned().unwrap_or(Value::Null),
        "config_hash": pack_object.get("config_hash").cloned().unwrap_or(Value::Null),
        "required": pack_object.get("required").cloned().unwrap_or(Value::Null),
        "stable_prefix": pack_object.get("stable_prefix").cloned().unwrap_or(Value::Null),
        "variants": variant_summaries,
    }))
}

fn event_indices_from_page(page_plan: &Value) -> HashSet<i64> {
    page_plan
        .get("event_indices")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_i64)
        .collect()
}

fn character_names_for_page(page_plan: &Value, events: &[Value]) -> HashSet<String> {
    let mut names = HashSet::new();
    collect_strings(page_plan.get("key_characters"), &mut names);
    for event in events {
        collect_strings(event.get("participants"), &mut names);
    }
    names
}

fn collect_strings(value: Option<&Value>, target: &mut HashSet<String>) {
    match value {
        Some(Value::String(text)) => {
            let text = text.trim();
            if !text.is_empty() {
                target.insert(text.to_string());
            }
        }
        Some(Value::Array(items)) => {
            for item in items {
                collect_strings(Some(item), target);
            }
        }
        _ => {}
    }
}

fn filter_characters(value: Option<&Value>, names: &HashSet<String>) -> Value {
    let Some(object) = value.and_then(Value::as_object) else {
        return json!({});
    };
    if names.is_empty() {
        return Value::Object(object.clone());
    }
    Value::Object(
        object
            .iter()
            .filter(|(key, value)| {
                names.contains(key.as_str())
                    || value
                        .get("name")
                        .and_then(Value::as_str)
                        .is_some_and(|name| names.contains(name.trim()))
            })
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect::<Map<_, _>>(),
    )
}

fn filter_reference_assets(value: Option<&Value>, names: &HashSet<String>) -> Value {
    let Some(object) = value.and_then(Value::as_object) else {
        return Value::Null;
    };
    if names.is_empty() {
        return Value::Object(object.clone());
    }

    let portraits = object
        .get("character_portraits")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter(|item| {
                    item.get("character_name")
                        .and_then(Value::as_str)
                        .is_some_and(|name| names.contains(name.trim()))
                })
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let summary = portraits
        .iter()
        .filter_map(|item| {
            let name = item.get("character_name")?.as_str()?.trim();
            let prompt = item
                .get("prompt")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim();
            if prompt.is_empty() {
                Some(name.to_string())
            } else {
                Some(format!("{name}: {prompt}"))
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    json!({
        "character_portraits": portraits,
        "character_portrait_summary": summary,
    })
}

fn filter_array_by_index(value: Option<&Value>, event_indices: &HashSet<i64>) -> Vec<Value> {
    filter_array(value, |item| {
        item.get("index")
            .and_then(Value::as_i64)
            .is_some_and(|index| in_event_scope(index, event_indices))
    })
}

fn filter_array_by_event_index(value: Option<&Value>, event_indices: &HashSet<i64>) -> Vec<Value> {
    filter_array(value, |item| {
        item.get("event_index")
            .and_then(Value::as_i64)
            .is_some_and(|index| in_event_scope(index, event_indices))
    })
}

fn filter_items_by_first_event(value: Option<&Value>, event_indices: &HashSet<i64>) -> Vec<Value> {
    filter_array(value, |item| {
        item.get("first_appearance_event")
            .and_then(Value::as_i64)
            .is_some_and(|index| in_event_scope(index, event_indices))
    })
}

fn filter_scenes_by_event_indices(
    value: Option<&Value>,
    event_indices: &HashSet<i64>,
) -> Vec<Value> {
    filter_array(value, |item| {
        if event_indices.is_empty() {
            return true;
        }
        item.get("event_indices")
            .and_then(Value::as_array)
            .is_some_and(|indices| {
                indices
                    .iter()
                    .filter_map(Value::as_i64)
                    .any(|index| event_indices.contains(&index))
            })
    })
}

fn filter_array<F>(value: Option<&Value>, keep: F) -> Vec<Value>
where
    F: Fn(&Value) -> bool,
{
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|item| keep(item))
        .cloned()
        .collect()
}

fn in_event_scope(index: i64, event_indices: &HashSet<i64>) -> bool {
    event_indices.is_empty() || event_indices.contains(&index)
}
