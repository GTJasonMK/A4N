use a4n_core::error::AppError;
use a4n_engine::llm::provider::ChatRequest;
use serde_json::{Value, json};

use crate::chapter::source_spans::source_spans_for_content;
use crate::llm_checkpoint::required_llm_call_workflow_run_id;

use super::pipeline_json::{
    page_drawing_prompt_from_response, page_plan_page_number, page_plan_pages, parse_json_object,
    required_array_field, required_object_field,
};
use super::pipeline_prompt::{
    MANGA_PREVIOUS_OUTPUT_CHARACTER_BINDING, MANGA_PREVIOUS_OUTPUT_EVENT_BEATS,
    MANGA_PREVIOUS_OUTPUT_EXTRACTION_SUMMARY, MANGA_PREVIOUS_OUTPUT_ITEM_EXTRACTION,
    MANGA_PREVIOUS_OUTPUT_PAGE_PLANNING, MANGA_PREVIOUS_OUTPUT_SPEECH_SELECTION,
    MANGA_PREVIOUS_OUTPUT_VISUAL_ANCHOR_EXTRACTION, MangaContextManifestInput,
    MangaContextPreviousOutput, MangaLlmCallInput, build_manga_context_manifest,
    load_manga_restored_llm_checkpoint, manga_chapter_comic_synthesis_user_message,
    manga_character_binding_user_message, manga_event_beats_user_message,
    manga_item_available_source_span_ids, manga_item_extraction_user_message,
    manga_page_drawing_prompt_user_message, manga_page_planning_user_message,
    manga_speech_selection_user_message, manga_visual_anchor_extraction_user_message,
    prepare_manga_llm_call, validate_manga_chapter_comic_synthesis,
    validate_manga_character_binding_source_span_ids, validate_manga_event_beats_source_span_ids,
    validate_manga_item_extraction_source_span_ids,
    validate_manga_speech_selection_source_span_ids, validate_manga_visual_anchor_source_span_ids,
};
use super::pipeline_state::should_restore_manga_stage_checkpoint;
use super::reference_assets::{character_portraits_json, format_character_portrait_context};
use super::{MangaPipelineRequestPreviewInput, MangaService, manga_source_content_sha256};

#[derive(Debug, Clone)]
pub struct MangaPipelineRequestPreviewStep {
    pub key: String,
    pub title: String,
    pub prompt_name: Option<String>,
    pub state: String,
    pub note: Option<String>,
    pub request: Option<ChatRequest>,
}

struct MangaPreviewCallInput<'a> {
    service: &'a MangaService,
    prompt_name: &'a str,
    workflow_run_id: Option<i64>,
    call_key: String,
    call_order: i32,
    restore_from_checkpoint: bool,
    manifest: MangaContextManifestInput<'a>,
    temperature: f32,
}

async fn build_manga_preview_call(
    input: MangaPreviewCallInput<'_>,
) -> Result<MangaPreviewCall, AppError> {
    let MangaPreviewCallInput {
        service,
        prompt_name,
        workflow_run_id,
        call_key,
        call_order,
        restore_from_checkpoint,
        manifest,
        temperature,
    } = input;
    let prompt_engine = manifest.prompt_engine;
    let project_id = manifest.project_id;
    let variables = manifest.variables;
    let manifest = build_manga_context_manifest(manifest)?;
    let prepared = prepare_manga_llm_call(MangaLlmCallInput {
        prompt_engine,
        prompt_name,
        project_id,
        workflow_run_id,
        call_key,
        call_order,
        variables,
        manifest,
        temperature,
    })?;
    if restore_from_checkpoint {
        let workflow_run_id =
            required_llm_call_workflow_run_id(workflow_run_id, &prepared.call_key)?;
        if let Some(restored) = load_manga_restored_llm_checkpoint(
            service.db.clone(),
            workflow_run_id,
            &prepared.call_key,
            &prepared.input_fingerprint_json,
            prompt_name,
        )
        .await?
        {
            return Ok(MangaPreviewCall::Restored {
                call_key: prepared.call_key,
                response_text: restored.response_text,
                parsed_output: restored.parsed_output,
            });
        }
    }
    Ok(MangaPreviewCall::Ready(Box::new(prepared.request)))
}

enum MangaPreviewCall {
    Ready(Box<ChatRequest>),
    Restored {
        call_key: String,
        response_text: String,
        parsed_output: Value,
    },
}

fn ensure_preview_restored_matches(
    call_key: &str,
    restored: &Value,
    parsed: &Value,
) -> Result<(), AppError> {
    if restored == parsed {
        return Ok(());
    }
    Err(AppError::InvalidArgument(format!(
        "{call_key} checkpoint 的 parsed_output 与 response_content 重新解析结果不一致"
    )))
}

impl MangaPipelineRequestPreviewStep {
    fn new(
        key: &str,
        title: &str,
        prompt_name: &str,
        state: &str,
        note: Option<String>,
        request: Option<ChatRequest>,
    ) -> Self {
        Self {
            key: key.to_string(),
            title: title.to_string(),
            prompt_name: Some(prompt_name.to_string()),
            state: state.to_string(),
            note,
            request,
        }
    }

    fn ready(
        key: &str,
        title: &str,
        prompt_name: &str,
        note: Option<String>,
        request: ChatRequest,
    ) -> Self {
        Self::new(key, title, prompt_name, "ready", note, Some(request))
    }

    fn reused(key: &str, title: &str, prompt_name: &str, note: String) -> Self {
        Self::new(key, title, prompt_name, "reused", Some(note), None)
    }

    fn reused_checkpoint(key: &str, title: &str, prompt_name: &str) -> Self {
        Self::reused(
            key,
            title,
            prompt_name,
            "已有匹配的调用级 checkpoint，本次不会重新发送该请求".to_string(),
        )
    }

    fn blocked(key: &str, title: &str, prompt_name: &str, note: &str) -> Self {
        Self::new(
            key,
            title,
            prompt_name,
            "blocked",
            Some(note.to_string()),
            None,
        )
    }
}

impl MangaService {
    pub async fn preview_generate_panels_requests(
        &self,
        input: MangaPipelineRequestPreviewInput<'_>,
    ) -> Result<Vec<MangaPipelineRequestPreviewStep>, AppError> {
        let MangaPipelineRequestPreviewInput {
            chapter_id,
            project_id,
            source,
            prompt_engine,
            variables,
            reference_context,
            workflow_run_id,
            resume_from_checkpoint,
            start_from_stage,
        } = input;
        let chapter_content = source.content;
        if chapter_content.trim().is_empty() {
            return Err(AppError::InvalidArgument(
                "章节正文为空，无法预览漫画流水线提示词".into(),
            ));
        }
        let source_spans = source_spans_for_content(chapter_id, 1, chapter_content);
        let mut vars = variables.clone();
        vars.insert("chapter_content".into(), json!(chapter_content));
        vars.insert(
            "source_content_sha256".into(),
            json!(manga_source_content_sha256(chapter_content)),
        );
        let character_portraits = character_portraits_json(&reference_context.character_portraits);
        vars.insert("character_portraits".into(), character_portraits.clone());
        vars.insert(
            "character_portrait_count".into(),
            json!(reference_context.character_portraits.len()),
        );
        let character_portrait_context = format_character_portrait_context(&reference_context);
        let reference_assets = json!({
            "character_portraits": character_portraits,
            "character_portrait_summary": character_portrait_context,
        });

        let mut steps = Vec::new();
        let should_restore_stage = |stage_order| {
            should_restore_manga_stage_checkpoint(
                resume_from_checkpoint,
                start_from_stage,
                stage_order,
            )
        };

        let events_call = build_manga_preview_call(MangaPreviewCallInput {
            service: self,
            prompt_name: "manga_event_beats",
            workflow_run_id,
            call_key: format!("manga_pipeline:{chapter_id}:manga_event_beats"),
            call_order: 1,
            restore_from_checkpoint: should_restore_stage(1),
            manifest: MangaContextManifestInput {
                prompt_engine,
                project_id,
                variables: &vars,
                chapter_content,
                source_spans: &source_spans,
                prompt_names: &["manga_event_beats"],
                previous_outputs: &[],
                reference_assets: Some(&reference_assets),
                dynamic_user_content: manga_event_beats_user_message(
                    chapter_content,
                    &source_spans,
                )?,
            },
            temperature: 0.3,
        })
        .await?;
        let (events_out, events_restored) = match events_call {
            MangaPreviewCall::Restored {
                call_key,
                response_text,
                parsed_output,
            } => {
                steps.push(MangaPipelineRequestPreviewStep::reused_checkpoint(
                    "manga_event_beats",
                    "漫画 Step 1：事件卡片抽取",
                    "manga_event_beats",
                ));
                (response_text, Some((call_key, parsed_output)))
            }
            MangaPreviewCall::Ready(request) => {
                steps.push(MangaPipelineRequestPreviewStep::ready(
                    "manga_event_beats",
                    "漫画 Step 1：事件卡片抽取",
                    "manga_event_beats",
                    None,
                    *request,
                ));
                push_blocked_after(&mut steps, "manga_character_binding");
                return Ok(steps);
            }
        };
        let events_stage_json: Value = parse_json_object(&events_out, "manga_event_beats")?;
        if let Some((call_key, parsed_output)) = events_restored.as_ref() {
            ensure_preview_restored_matches(call_key, parsed_output, &events_stage_json)?;
        }
        validate_manga_event_beats_source_span_ids(&events_stage_json, &source_spans)?;
        let events_json = required_array_field(&events_stage_json, "events", "manga_event_beats")?;

        let characters_call = build_manga_preview_call(MangaPreviewCallInput {
            service: self,
            prompt_name: "manga_character_binding",
            workflow_run_id,
            call_key: format!("manga_pipeline:{chapter_id}:manga_character_binding"),
            call_order: 2,
            restore_from_checkpoint: should_restore_stage(2),
            manifest: MangaContextManifestInput {
                prompt_engine,
                project_id,
                variables: &vars,
                chapter_content,
                source_spans: &source_spans,
                prompt_names: &["manga_event_beats", "manga_character_binding"],
                previous_outputs: &[MangaContextPreviousOutput {
                    pack_kind: MANGA_PREVIOUS_OUTPUT_EVENT_BEATS,
                    prompt_name: "manga_event_beats",
                    parsed_output: &json!({ "events": &events_json }),
                    raw_output: Some(&events_out),
                    placeholder: false,
                }],
                reference_assets: Some(&reference_assets),
                dynamic_user_content: manga_character_binding_user_message(
                    chapter_content,
                    events_json.clone(),
                    &source_spans,
                    reference_assets.clone(),
                )?,
            },
            temperature: 0.3,
        })
        .await?;
        let (characters_out, characters_restored) = match characters_call {
            MangaPreviewCall::Restored {
                call_key,
                response_text,
                parsed_output,
            } => {
                steps.push(MangaPipelineRequestPreviewStep::reused_checkpoint(
                    "manga_character_binding",
                    "漫画 Step 2：角色绑定",
                    "manga_character_binding",
                ));
                (response_text, Some((call_key, parsed_output)))
            }
            MangaPreviewCall::Ready(request) => {
                steps.push(MangaPipelineRequestPreviewStep::ready(
                    "manga_character_binding",
                    "漫画 Step 2：角色绑定",
                    "manga_character_binding",
                    None,
                    *request,
                ));
                push_blocked_after(&mut steps, "manga_speech_selection");
                return Ok(steps);
            }
        };
        let characters_stage_json: Value =
            parse_json_object(&characters_out, "manga_character_binding")?;
        if let Some((call_key, parsed_output)) = characters_restored.as_ref() {
            ensure_preview_restored_matches(call_key, parsed_output, &characters_stage_json)?;
        }
        validate_manga_character_binding_source_span_ids(&characters_stage_json, &source_spans)?;
        let characters_json = required_object_field(
            &characters_stage_json,
            "characters",
            "manga_character_binding",
        )?;

        let speech_call = build_manga_preview_call(MangaPreviewCallInput {
            service: self,
            prompt_name: "manga_speech_selection",
            workflow_run_id,
            call_key: format!("manga_pipeline:{chapter_id}:manga_speech_selection"),
            call_order: 3,
            restore_from_checkpoint: should_restore_stage(3),
            manifest: MangaContextManifestInput {
                prompt_engine,
                project_id,
                variables: &vars,
                chapter_content,
                source_spans: &source_spans,
                prompt_names: &[
                    "manga_event_beats",
                    "manga_character_binding",
                    "manga_speech_selection",
                ],
                previous_outputs: &[
                    MangaContextPreviousOutput {
                        pack_kind: MANGA_PREVIOUS_OUTPUT_EVENT_BEATS,
                        prompt_name: "manga_event_beats",
                        parsed_output: &json!({ "events": &events_json }),
                        raw_output: Some(&events_out),
                        placeholder: false,
                    },
                    MangaContextPreviousOutput {
                        pack_kind: MANGA_PREVIOUS_OUTPUT_CHARACTER_BINDING,
                        prompt_name: "manga_character_binding",
                        parsed_output: &json!({ "characters": &characters_json }),
                        raw_output: Some(&characters_out),
                        placeholder: false,
                    },
                ],
                reference_assets: Some(&reference_assets),
                dynamic_user_content: manga_speech_selection_user_message(
                    characters_json.clone(),
                    events_json.clone(),
                    &source_spans,
                    reference_assets.clone(),
                )?,
            },
            temperature: 0.3,
        })
        .await?;
        let (speech_out, speech_restored) = match speech_call {
            MangaPreviewCall::Restored {
                call_key,
                response_text,
                parsed_output,
            } => {
                steps.push(MangaPipelineRequestPreviewStep::reused_checkpoint(
                    "manga_speech_selection",
                    "漫画 Step 3：对白旁白选择",
                    "manga_speech_selection",
                ));
                (response_text, Some((call_key, parsed_output)))
            }
            MangaPreviewCall::Ready(request) => {
                steps.push(MangaPipelineRequestPreviewStep::ready(
                    "manga_speech_selection",
                    "漫画 Step 3：对白旁白选择",
                    "manga_speech_selection",
                    None,
                    *request,
                ));
                push_blocked_after(&mut steps, "manga_visual_anchor_extraction");
                return Ok(steps);
            }
        };
        let speech_json = parse_json_object(&speech_out, "manga_speech_selection")?;
        if let Some((call_key, parsed_output)) = speech_restored.as_ref() {
            ensure_preview_restored_matches(call_key, parsed_output, &speech_json)?;
        }
        validate_manga_speech_selection_source_span_ids(&speech_json, &source_spans)?;

        let visual_call = build_manga_preview_call(MangaPreviewCallInput {
            service: self,
            prompt_name: "manga_visual_anchor_extraction",
            workflow_run_id,
            call_key: format!("manga_pipeline:{chapter_id}:manga_visual_anchor_extraction"),
            call_order: 4,
            restore_from_checkpoint: should_restore_stage(4),
            manifest: MangaContextManifestInput {
                prompt_engine,
                project_id,
                variables: &vars,
                chapter_content,
                source_spans: &source_spans,
                prompt_names: &["manga_event_beats", "manga_visual_anchor_extraction"],
                previous_outputs: &[MangaContextPreviousOutput {
                    pack_kind: MANGA_PREVIOUS_OUTPUT_EVENT_BEATS,
                    prompt_name: "manga_event_beats",
                    parsed_output: &json!({ "events": &events_json }),
                    raw_output: Some(&events_out),
                    placeholder: false,
                }],
                reference_assets: Some(&reference_assets),
                dynamic_user_content: manga_visual_anchor_extraction_user_message(
                    events_json.clone(),
                    &source_spans,
                )?,
            },
            temperature: 0.3,
        })
        .await?;
        let (visual_out, visual_restored) = match visual_call {
            MangaPreviewCall::Restored {
                call_key,
                response_text,
                parsed_output,
            } => {
                steps.push(MangaPipelineRequestPreviewStep::reused_checkpoint(
                    "manga_visual_anchor_extraction",
                    "漫画 Step 4：视觉锚点抽取",
                    "manga_visual_anchor_extraction",
                ));
                (response_text, Some((call_key, parsed_output)))
            }
            MangaPreviewCall::Ready(request) => {
                steps.push(MangaPipelineRequestPreviewStep::ready(
                    "manga_visual_anchor_extraction",
                    "漫画 Step 4：视觉锚点抽取",
                    "manga_visual_anchor_extraction",
                    None,
                    *request,
                ));
                push_blocked_after(&mut steps, "manga_item_extraction");
                return Ok(steps);
            }
        };
        let visual_json = parse_json_object(&visual_out, "manga_visual_anchor_extraction")?;
        if let Some((call_key, parsed_output)) = visual_restored.as_ref() {
            ensure_preview_restored_matches(call_key, parsed_output, &visual_json)?;
        }
        validate_manga_visual_anchor_source_span_ids(&visual_json, &source_spans)?;

        let items_call = build_manga_preview_call(MangaPreviewCallInput {
            service: self,
            prompt_name: "manga_item_extraction",
            workflow_run_id,
            call_key: format!("manga_pipeline:{chapter_id}:manga_item_extraction"),
            call_order: 5,
            restore_from_checkpoint: should_restore_stage(5),
            manifest: MangaContextManifestInput {
                prompt_engine,
                project_id,
                variables: &vars,
                chapter_content,
                source_spans: &source_spans,
                prompt_names: &[
                    "manga_speech_selection",
                    "manga_visual_anchor_extraction",
                    "manga_item_extraction",
                ],
                previous_outputs: &[
                    MangaContextPreviousOutput {
                        pack_kind: MANGA_PREVIOUS_OUTPUT_SPEECH_SELECTION,
                        prompt_name: "manga_speech_selection",
                        parsed_output: &json!({ "speech": &speech_json }),
                        raw_output: Some(&speech_out),
                        placeholder: false,
                    },
                    MangaContextPreviousOutput {
                        pack_kind: MANGA_PREVIOUS_OUTPUT_VISUAL_ANCHOR_EXTRACTION,
                        prompt_name: "manga_visual_anchor_extraction",
                        parsed_output: &json!({ "visual_anchors": &visual_json }),
                        raw_output: Some(&visual_out),
                        placeholder: false,
                    },
                ],
                reference_assets: Some(&reference_assets),
                dynamic_user_content: manga_item_extraction_user_message(
                    speech_json.clone(),
                    visual_json.clone(),
                )?,
            },
            temperature: 0.3,
        })
        .await?;
        let (items_out, items_restored) = match items_call {
            MangaPreviewCall::Restored {
                call_key,
                response_text,
                parsed_output,
            } => {
                steps.push(MangaPipelineRequestPreviewStep::reused_checkpoint(
                    "manga_item_extraction",
                    "漫画 Step 5：关键物品抽取",
                    "manga_item_extraction",
                ));
                (response_text, Some((call_key, parsed_output)))
            }
            MangaPreviewCall::Ready(request) => {
                steps.push(MangaPipelineRequestPreviewStep::ready(
                    "manga_item_extraction",
                    "漫画 Step 5：关键物品抽取",
                    "manga_item_extraction",
                    None,
                    *request,
                ));
                push_blocked_after(&mut steps, "manga_chapter_comic_synthesis");
                return Ok(steps);
            }
        };
        let items_json = parse_json_object(&items_out, "manga_item_extraction")?;
        if let Some((call_key, parsed_output)) = items_restored.as_ref() {
            ensure_preview_restored_matches(call_key, parsed_output, &items_json)?;
        }
        let allowed_source_span_ids =
            manga_item_available_source_span_ids(&speech_json, &visual_json);
        validate_manga_item_extraction_source_span_ids(
            &items_json,
            &source_spans,
            &allowed_source_span_ids,
        )?;

        let comic_synthesis_call = build_manga_preview_call(MangaPreviewCallInput {
            service: self,
            prompt_name: "manga_chapter_comic_synthesis",
            workflow_run_id,
            call_key: format!("manga_pipeline:{chapter_id}:manga_chapter_comic_synthesis"),
            call_order: 6,
            restore_from_checkpoint: should_restore_stage(6),
            manifest: MangaContextManifestInput {
                prompt_engine,
                project_id,
                variables: &vars,
                chapter_content,
                source_spans: &source_spans,
                prompt_names: &[
                    "manga_event_beats",
                    "manga_speech_selection",
                    "manga_visual_anchor_extraction",
                    "manga_item_extraction",
                    "manga_chapter_comic_synthesis",
                ],
                previous_outputs: &[
                    MangaContextPreviousOutput {
                        pack_kind: MANGA_PREVIOUS_OUTPUT_EVENT_BEATS,
                        prompt_name: "manga_event_beats",
                        parsed_output: &json!({ "events": &events_json }),
                        raw_output: Some(&events_out),
                        placeholder: false,
                    },
                    MangaContextPreviousOutput {
                        pack_kind: MANGA_PREVIOUS_OUTPUT_SPEECH_SELECTION,
                        prompt_name: "manga_speech_selection",
                        parsed_output: &json!({ "speech": &speech_json }),
                        raw_output: Some(&speech_out),
                        placeholder: false,
                    },
                    MangaContextPreviousOutput {
                        pack_kind: MANGA_PREVIOUS_OUTPUT_VISUAL_ANCHOR_EXTRACTION,
                        prompt_name: "manga_visual_anchor_extraction",
                        parsed_output: &json!({ "visual_anchors": &visual_json }),
                        raw_output: Some(&visual_out),
                        placeholder: false,
                    },
                    MangaContextPreviousOutput {
                        pack_kind: MANGA_PREVIOUS_OUTPUT_ITEM_EXTRACTION,
                        prompt_name: "manga_item_extraction",
                        parsed_output: &json!({ "items": &items_json }),
                        raw_output: Some(&items_out),
                        placeholder: false,
                    },
                ],
                reference_assets: Some(&reference_assets),
                dynamic_user_content: manga_chapter_comic_synthesis_user_message(
                    events_json.clone(),
                    speech_json.clone(),
                    visual_json.clone(),
                    items_json.clone(),
                )?,
            },
            temperature: 0.3,
        })
        .await?;
        let (comic_synthesis_out, comic_synthesis_restored) = match comic_synthesis_call {
            MangaPreviewCall::Restored {
                call_key,
                response_text,
                parsed_output,
            } => {
                steps.push(MangaPipelineRequestPreviewStep::reused_checkpoint(
                    "manga_chapter_comic_synthesis",
                    "漫画 Step 6：章节节奏综合",
                    "manga_chapter_comic_synthesis",
                ));
                (response_text, Some((call_key, parsed_output)))
            }
            MangaPreviewCall::Ready(request) => {
                steps.push(MangaPipelineRequestPreviewStep::ready(
                    "manga_chapter_comic_synthesis",
                    "漫画 Step 6：章节节奏综合",
                    "manga_chapter_comic_synthesis",
                    None,
                    *request,
                ));
                push_blocked_after(&mut steps, "manga_page_planning");
                return Ok(steps);
            }
        };
        let comic_synthesis_json =
            parse_json_object(&comic_synthesis_out, "manga_chapter_comic_synthesis")?;
        if let Some((call_key, parsed_output)) = comic_synthesis_restored.as_ref() {
            ensure_preview_restored_matches(call_key, parsed_output, &comic_synthesis_json)?;
        }
        validate_manga_chapter_comic_synthesis(&comic_synthesis_json)?;

        let extraction_summary = json!({
            "characters": characters_json,
            "events": events_json,
            "reference_assets": reference_assets,
            "speech": speech_json,
            "visual_anchors": visual_json,
            "items": items_json.get("items").cloned().unwrap_or_else(|| json!([])),
            "comic_synthesis": comic_synthesis_json
        });

        let plan_call = build_manga_preview_call(MangaPreviewCallInput {
            service: self,
            prompt_name: "manga_page_planning",
            workflow_run_id,
            call_key: format!("manga_pipeline:{chapter_id}:manga_page_planning"),
            call_order: 7,
            restore_from_checkpoint: should_restore_stage(7),
            manifest: MangaContextManifestInput {
                prompt_engine,
                project_id,
                variables: &vars,
                chapter_content,
                source_spans: &source_spans,
                prompt_names: &[
                    "manga_event_beats",
                    "manga_character_binding",
                    "manga_speech_selection",
                    "manga_visual_anchor_extraction",
                    "manga_item_extraction",
                    "manga_chapter_comic_synthesis",
                    "manga_page_planning",
                ],
                previous_outputs: &[MangaContextPreviousOutput {
                    pack_kind: MANGA_PREVIOUS_OUTPUT_EXTRACTION_SUMMARY,
                    prompt_name: "manga_extraction_summary",
                    parsed_output: &extraction_summary,
                    raw_output: None,
                    placeholder: false,
                }],
                reference_assets: Some(&reference_assets),
                dynamic_user_content: manga_page_planning_user_message(extraction_summary.clone())?,
            },
            temperature: 0.3,
        })
        .await?;
        let (plan_out, plan_restored) = match plan_call {
            MangaPreviewCall::Restored {
                call_key,
                response_text,
                parsed_output,
            } => {
                steps.push(MangaPipelineRequestPreviewStep::reused_checkpoint(
                    "manga_page_planning",
                    "漫画 Step 7：分页规划",
                    "manga_page_planning",
                ));
                (response_text, Some((call_key, parsed_output)))
            }
            MangaPreviewCall::Ready(request) => {
                steps.push(MangaPipelineRequestPreviewStep::ready(
                    "manga_page_planning",
                    "漫画 Step 7：分页规划",
                    "manga_page_planning",
                    None,
                    *request,
                ));
                push_blocked_after(&mut steps, "manga_page_drawing_prompt");
                return Ok(steps);
            }
        };
        let plan_json: Value = parse_json_object(&plan_out, "manga_page_planning")?;
        if let Some((call_key, parsed_output)) = plan_restored.as_ref() {
            ensure_preview_restored_matches(call_key, parsed_output, &plan_json)?;
        }

        let pages = page_plan_pages(&plan_json, "manga_page_planning")?;
        for (index, page_plan) in pages.iter().enumerate() {
            let page_number =
                page_plan_page_number(page_plan, &format!("manga_page_planning.pages[{index}]"))?;
            let key = format!("manga_page_drawing_prompt_page_{page_number}");
            let title = format!("漫画 Step 8：第 {page_number} 页绘图提示词");
            let plan_context = json!({ "page_plan": page_plan, "plan": &plan_json });
            let page_call = build_manga_preview_call(MangaPreviewCallInput {
                service: self,
                prompt_name: "manga_page_drawing_prompt",
                workflow_run_id,
                call_key: format!(
                    "manga_pipeline:{chapter_id}:manga_page_drawing_prompt:page:{page_number}"
                ),
                call_order: 800 + index as i32,
                restore_from_checkpoint: should_restore_stage(8),
                manifest: MangaContextManifestInput {
                    prompt_engine,
                    project_id,
                    variables: &vars,
                    chapter_content,
                    source_spans: &source_spans,
                    prompt_names: &["manga_page_planning", "manga_page_drawing_prompt"],
                    previous_outputs: &[
                        MangaContextPreviousOutput {
                            pack_kind: MANGA_PREVIOUS_OUTPUT_PAGE_PLANNING,
                            prompt_name: "manga_page_planning",
                            parsed_output: &plan_context,
                            raw_output: Some(&plan_out),
                            placeholder: false,
                        },
                        MangaContextPreviousOutput {
                            pack_kind: MANGA_PREVIOUS_OUTPUT_EXTRACTION_SUMMARY,
                            prompt_name: "manga_extraction_summary",
                            parsed_output: &extraction_summary,
                            raw_output: None,
                            placeholder: false,
                        },
                    ],
                    reference_assets: Some(&reference_assets),
                    dynamic_user_content: manga_page_drawing_prompt_user_message(
                        page_number,
                        page_plan,
                        &extraction_summary,
                    )?,
                },
                temperature: 0.5,
            })
            .await?;
            match page_call {
                MangaPreviewCall::Restored {
                    call_key,
                    response_text,
                    parsed_output,
                } => {
                    let page_context = format!("manga_page_drawing_prompt.page_{page_number}");
                    let page_json = parse_json_object(&response_text, &page_context)?;
                    ensure_preview_restored_matches(&call_key, &parsed_output, &page_json)?;
                    page_drawing_prompt_from_response(&page_json, page_number, &page_context)?;
                    steps.push(MangaPipelineRequestPreviewStep::reused_checkpoint(
                        &key,
                        &title,
                        "manga_page_drawing_prompt",
                    ));
                }
                MangaPreviewCall::Ready(request) => {
                    steps.push(MangaPipelineRequestPreviewStep::ready(
                        &key,
                        &title,
                        "manga_page_drawing_prompt",
                        Some(format!("分页规划已可用；本次会发送第 {page_number} 页请求")),
                        *request,
                    ));
                }
            }
        }

        Ok(steps)
    }
}

fn push_blocked_after(steps: &mut Vec<MangaPipelineRequestPreviewStep>, next_step_key: &str) {
    for (key, title, prompt_name) in manga_pipeline_steps_after(next_step_key) {
        steps.push(MangaPipelineRequestPreviewStep::blocked(
            key,
            title,
            prompt_name,
            "该请求依赖前序模型输出，发送前无法提前构造真实请求",
        ));
    }
}

fn manga_pipeline_steps_after(
    next_step_key: &str,
) -> &'static [(&'static str, &'static str, &'static str)] {
    const STEPS: &[(&str, &str, &str)] = &[
        (
            "manga_character_binding",
            "漫画 Step 2：角色绑定",
            "manga_character_binding",
        ),
        (
            "manga_speech_selection",
            "漫画 Step 3：对白旁白选择",
            "manga_speech_selection",
        ),
        (
            "manga_visual_anchor_extraction",
            "漫画 Step 4：视觉锚点抽取",
            "manga_visual_anchor_extraction",
        ),
        (
            "manga_item_extraction",
            "漫画 Step 5：关键物品抽取",
            "manga_item_extraction",
        ),
        (
            "manga_chapter_comic_synthesis",
            "漫画 Step 6：章节节奏综合",
            "manga_chapter_comic_synthesis",
        ),
        (
            "manga_page_planning",
            "漫画 Step 7：分页规划",
            "manga_page_planning",
        ),
        (
            "manga_page_drawing_prompt",
            "漫画 Step 8：整页绘图提示词",
            "manga_page_drawing_prompt",
        ),
    ];
    let index = STEPS
        .iter()
        .position(|(key, _, _)| *key == next_step_key)
        .unwrap_or(STEPS.len());
    &STEPS[index..]
}
