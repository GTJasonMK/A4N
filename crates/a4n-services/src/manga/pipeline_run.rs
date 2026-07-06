use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_manga_prompt;
use serde_json::{Map, Value, json};

use crate::chapter::source_spans::source_spans_for_content;
use crate::generation::llm_usage_summary_text;
use crate::llm_checkpoint::{
    CompletedLlmCallCheckpointInput, required_llm_call_workflow_run_id,
    save_completed_llm_call_checkpoint,
};

mod drawing_prompt;

use self::drawing_prompt::DrawingPromptStepInput;
use super::checkpoint::{checkpoint_insert, checkpoint_insert_trace};
use super::pipeline_json::{
    parse_json_object, required_array_field, required_object_field, validate_page_drawing_prompts,
};
use super::pipeline_persist::MangaPipelinePersistInput;
use super::pipeline_prompt::{
    MANGA_PREVIOUS_OUTPUT_CHARACTER_BINDING, MANGA_PREVIOUS_OUTPUT_EVENT_BEATS,
    MANGA_PREVIOUS_OUTPUT_EXTRACTION_SUMMARY, MANGA_PREVIOUS_OUTPUT_ITEM_EXTRACTION,
    MANGA_PREVIOUS_OUTPUT_SPEECH_SELECTION, MANGA_PREVIOUS_OUTPUT_VISUAL_ANCHOR_EXTRACTION,
    MangaContextManifestInput, MangaContextPreviousOutput, MangaPromptGenerationTrace,
    MangaRenderTraceInput, build_manga_context_manifest,
    manga_chapter_comic_synthesis_user_message, manga_character_binding_user_message,
    manga_event_beats_user_message, manga_item_available_source_span_ids,
    manga_item_extraction_user_message, manga_page_planning_user_message,
    manga_pipeline_prompt_fingerprint, manga_speech_selection_user_message,
    manga_visual_anchor_extraction_user_message, render_and_chat_trace,
    validate_manga_chapter_comic_synthesis, validate_manga_character_binding_source_span_ids,
    validate_manga_event_beats_source_span_ids, validate_manga_item_extraction_source_span_ids,
    validate_manga_speech_selection_source_span_ids, validate_manga_visual_anchor_source_span_ids,
};
use super::pipeline_state::should_restore_manga_stage_checkpoint;
use super::pipeline_status::{MangaPipelineCheckpointInput, MangaPipelineStatusInput};
use super::reference_assets::{
    character_portraits_json, format_character_portrait_context, manga_reference_assets_metadata,
};
use super::{
    MangaGeneratePanelsInput, MangaPipelineStartStage, MangaService, PIPELINE_TOTAL_STEPS,
    manga_source_content_sha256, manga_source_metadata,
};

struct MangaRunningPipelineCheckpointInput<'a> {
    chapter_id: i64,
    project_id: i64,
    stage: &'a str,
    current: i32,
    message: &'a str,
    task_id: Option<&'a str>,
    source_metadata: &'a Value,
    reference_assets_metadata: &'a Value,
    prompt_fingerprint: &'a Value,
    checkpoint: &'a Map<String, Value>,
}

impl MangaService {
    async fn save_or_validate_manga_llm_checkpoint(
        &self,
        trace: &MangaPromptGenerationTrace,
        parsed_output: &Value,
    ) -> Result<(), AppError> {
        if let Some(restored) = trace.restored_parsed_output.as_ref() {
            if restored != parsed_output {
                let call_key = trace.call_key.as_deref().ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "漫画 LLM trace 缺少调用级 call_key: {}",
                        trace.prompt_name
                    ))
                })?;
                return Err(AppError::InvalidArgument(format!(
                    "{} checkpoint 的 parsed_output 与 response_content 重新解析结果不一致",
                    call_key
                )));
            }
            return Ok(());
        }
        let call_key = trace.call_key.clone().ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "漫画 LLM trace 缺少调用级 call_key: {}",
                trace.prompt_name
            ))
        })?;
        let call_order = trace.call_order.ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "漫画 LLM trace 缺少调用级 call_order: {}",
                trace.prompt_name
            ))
        })?;
        let input_fingerprint_json = trace.input_fingerprint_json.clone().ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "漫画 LLM trace 缺少调用级 input_fingerprint_json: {}",
                trace.prompt_name
            ))
        })?;
        let response_usage_json = trace
            .usage
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(AppError::Serialization)?;
        save_completed_llm_call_checkpoint(
            self.db.clone(),
            CompletedLlmCallCheckpointInput {
                workflow_run_id: required_llm_call_workflow_run_id(
                    trace.workflow_run_id,
                    &call_key,
                )?,
                call_key,
                call_order,
                input_fingerprint_json,
                checkpoint_json: json!({
                    "output": parsed_output,
                    "parsed_output": parsed_output,
                    "response_content": trace.response_text,
                    "usage": response_usage_json,
                    "finish_reason": trace.finish_reason,
                }),
                artifacts_json: None,
            },
        )
        .await
    }

    async fn report_pipeline_llm_usage(
        &self,
        chapter_id: i64,
        project_id: i64,
        stage: &str,
        current: i32,
        task_id: Option<&str>,
        trace: &MangaPromptGenerationTrace,
    ) -> Result<(), AppError> {
        let message = format!(
            "模型请求完成：{}；{}",
            trace.prompt_name,
            llm_usage_summary_text(trace.usage.as_ref())
        );
        self.set_pipeline_status(MangaPipelineStatusInput {
            chapter_id,
            project_id,
            pipeline_status: "running",
            stage,
            current,
            total: PIPELINE_TOTAL_STEPS,
            message: &message,
            task_id,
            allow_cancel_override: true,
        })
        .await
    }

    /// 生成整页漫画提示词：完整 8 步流水线。
    ///
    /// 每步把上一步的 JSON 输出拼到下一步的 user message 中传给 LLM；prompt 模板
    /// （system message）只描述抽取/规划/编排的策略，不依赖 Tera 变量插值。
    pub async fn generate_panels(
        &self,
        input: MangaGeneratePanelsInput<'_>,
    ) -> Result<Vec<chapter_manga_prompt::Model>, AppError> {
        let chapter_id = input.chapter_id;
        let project_id = input.project_id;
        let task_id = input.task_id;
        let result = self.generate_panels_inner(input).await;

        match result {
            Ok(prompts) => Ok(prompts),
            Err(err) => {
                let is_cancelled = matches!(&err, AppError::Cancelled);
                let original_error = err.to_string();
                let (pipeline_status, stage, message) = if is_cancelled {
                    ("cancelled", "cancelled", "漫画流水线已取消".to_string())
                } else {
                    ("failed", "failed", original_error.clone())
                };
                let (current, total) = self
                    .pipeline_progress(project_id, chapter_id)
                    .await
                    .map_err(|progress_err| {
                        AppError::InvalidArgument(format!(
                            "{original_error}; 读取漫画流水线进度失败: {progress_err}"
                        ))
                    })?;
                self.set_pipeline_status(MangaPipelineStatusInput {
                    chapter_id,
                    project_id,
                    pipeline_status,
                    stage,
                    current,
                    total,
                    message: &message,
                    task_id,
                    allow_cancel_override: true,
                })
                .await
                .map_err(|status_err| {
                    AppError::InvalidArgument(format!(
                        "{original_error}; 记录漫画流水线终止状态失败: {status_err}"
                    ))
                })?;
                Err(err)
            }
        }
    }

    async fn generate_panels_inner(
        &self,
        input: MangaGeneratePanelsInput<'_>,
    ) -> Result<Vec<chapter_manga_prompt::Model>, AppError> {
        let MangaGeneratePanelsInput {
            chapter_id,
            project_id,
            source,
            llm,
            prompt_engine,
            variables,
            reference_context,
            task_id,
            workflow_run_id,
            resume_from_checkpoint,
            start_from_stage,
        } = input;
        let chapter_content = source.content;
        if chapter_content.trim().is_empty() {
            return Err(AppError::InvalidArgument(
                "章节正文为空，无法生成整页漫画提示词".into(),
            ));
        }
        let source_spans = source_spans_for_content(chapter_id, 1, chapter_content);
        let source_metadata = manga_source_metadata(source);
        if !resume_from_checkpoint && start_from_stage.is_none() {
            self.clear_pipeline_checkpoint(project_id, chapter_id)
                .await?;
        }
        let (initial_stage, initial_current, initial_message) = match start_from_stage {
            Some(stage) => stage.status(),
            None => MangaPipelineStartStage::EventBeats.status(),
        };
        self.set_pipeline_status(MangaPipelineStatusInput {
            chapter_id,
            project_id,
            pipeline_status: "running",
            stage: initial_stage,
            current: initial_current,
            total: PIPELINE_TOTAL_STEPS,
            message: initial_message,
            task_id,
            allow_cancel_override: true,
        })
        .await?;
        self.ensure_pipeline_not_cancelled(project_id, chapter_id)
            .await?;

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
        let reference_assets_metadata = manga_reference_assets_metadata(&reference_context);
        let prompt_fingerprint =
            manga_pipeline_prompt_fingerprint(prompt_engine, project_id, &vars)?;
        let mut checkpoint = Map::new();
        let should_restore_stage = |stage_order| {
            should_restore_manga_stage_checkpoint(
                resume_from_checkpoint,
                start_from_stage,
                stage_order,
            )
        };

        // ---- Step 1: 抽取漫画化事件卡片 ----
        let events_user = manga_event_beats_user_message(chapter_content, &source_spans)?;
        let events_manifest = build_manga_context_manifest(MangaContextManifestInput {
            prompt_engine,
            project_id,
            variables: &vars,
            chapter_content,
            source_spans: &source_spans,
            prompt_names: &["manga_event_beats"],
            previous_outputs: &[],
            reference_assets: Some(&reference_assets),
            dynamic_user_content: events_user,
        })?;
        let events_trace = render_and_chat_trace(MangaRenderTraceInput {
            db: self.db.clone(),
            prompt_engine,
            llm,
            prompt_name: "manga_event_beats",
            project_id,
            workflow_run_id,
            call_key: format!("manga_pipeline:{chapter_id}:manga_event_beats"),
            call_order: 1,
            restore_from_checkpoint: should_restore_stage(1),
            variables: &vars,
            manifest: events_manifest,
            temperature: 0.3,
        })
        .await?;
        self.report_pipeline_llm_usage(
            chapter_id,
            project_id,
            "event_beats",
            1,
            task_id,
            &events_trace,
        )
        .await?;
        let events_out = events_trace.response_text.clone();
        checkpoint_insert_trace(&mut checkpoint, "events_trace", &events_trace)?;
        let events_stage_json: Value = parse_json_object(&events_out, "manga_event_beats")?;
        validate_manga_event_beats_source_span_ids(&events_stage_json, &source_spans)?;
        self.save_or_validate_manga_llm_checkpoint(&events_trace, &events_stage_json)
            .await?;
        let events_json = required_array_field(&events_stage_json, "events", "manga_event_beats")?;
        checkpoint_insert(&mut checkpoint, "events_out", json!(events_out.clone()));
        checkpoint_insert(&mut checkpoint, "events", events_json.clone());
        self.save_running_pipeline_checkpoint(MangaRunningPipelineCheckpointInput {
            chapter_id,
            project_id,
            stage: "event_beats",
            current: 1,
            message: "已完成漫画事件卡片抽取",
            task_id,
            source_metadata: &source_metadata,
            reference_assets_metadata: &reference_assets_metadata,
            prompt_fingerprint: &prompt_fingerprint,
            checkpoint: &checkpoint,
        })
        .await?;

        self.ensure_pipeline_not_cancelled(project_id, chapter_id)
            .await?;
        self.set_pipeline_status(MangaPipelineStatusInput {
            chapter_id,
            project_id,
            pipeline_status: "running",
            stage: "character_binding",
            current: 2,
            total: PIPELINE_TOTAL_STEPS,
            message: "正在绑定出场角色身份与外观",
            task_id,
            allow_cancel_override: false,
        })
        .await?;

        // ---- Step 2: 绑定本章出场角色身份与外观 ----
        let characters_user = manga_character_binding_user_message(
            chapter_content,
            events_json.clone(),
            &source_spans,
            reference_assets.clone(),
        )?;
        let events_context = json!({ "events": &events_json });
        let characters_manifest = build_manga_context_manifest(MangaContextManifestInput {
            prompt_engine,
            project_id,
            variables: &vars,
            chapter_content,
            source_spans: &source_spans,
            prompt_names: &["manga_event_beats", "manga_character_binding"],
            previous_outputs: &[MangaContextPreviousOutput {
                pack_kind: MANGA_PREVIOUS_OUTPUT_EVENT_BEATS,
                prompt_name: "manga_event_beats",
                parsed_output: &events_context,
                raw_output: Some(&events_out),
                placeholder: false,
            }],
            reference_assets: Some(&reference_assets),
            dynamic_user_content: characters_user,
        })?;
        let characters_trace = render_and_chat_trace(MangaRenderTraceInput {
            db: self.db.clone(),
            prompt_engine,
            llm,
            prompt_name: "manga_character_binding",
            project_id,
            workflow_run_id,
            call_key: format!("manga_pipeline:{chapter_id}:manga_character_binding"),
            call_order: 2,
            restore_from_checkpoint: should_restore_stage(2),
            variables: &vars,
            manifest: characters_manifest,
            temperature: 0.3,
        })
        .await?;
        self.report_pipeline_llm_usage(
            chapter_id,
            project_id,
            "character_binding",
            2,
            task_id,
            &characters_trace,
        )
        .await?;
        let characters_out = characters_trace.response_text.clone();
        checkpoint_insert_trace(&mut checkpoint, "characters_trace", &characters_trace)?;
        let characters_stage_json: Value =
            parse_json_object(&characters_out, "manga_character_binding")?;
        validate_manga_character_binding_source_span_ids(&characters_stage_json, &source_spans)?;
        self.save_or_validate_manga_llm_checkpoint(&characters_trace, &characters_stage_json)
            .await?;
        let characters_json = required_object_field(
            &characters_stage_json,
            "characters",
            "manga_character_binding",
        )?;
        checkpoint_insert(
            &mut checkpoint,
            "characters_out",
            json!(characters_out.clone()),
        );
        checkpoint_insert(&mut checkpoint, "characters", characters_json.clone());
        self.save_running_pipeline_checkpoint(MangaRunningPipelineCheckpointInput {
            chapter_id,
            project_id,
            stage: "character_binding",
            current: 2,
            message: "已完成角色绑定",
            task_id,
            source_metadata: &source_metadata,
            reference_assets_metadata: &reference_assets_metadata,
            prompt_fingerprint: &prompt_fingerprint,
            checkpoint: &checkpoint,
        })
        .await?;
        self.ensure_pipeline_not_cancelled(project_id, chapter_id)
            .await?;
        self.set_pipeline_status(MangaPipelineStatusInput {
            chapter_id,
            project_id,
            pipeline_status: "running",
            stage: "speech_selection",
            current: 3,
            total: PIPELINE_TOTAL_STEPS,
            message: "正在选择关键对白、想法与旁白",
            task_id,
            allow_cancel_override: false,
        })
        .await?;

        // ---- Step 3: 选择关键对白、想法与旁白 ----
        let speech_user = manga_speech_selection_user_message(
            characters_json.clone(),
            events_json.clone(),
            &source_spans,
            reference_assets.clone(),
        )?;
        let events_context = json!({ "events": &events_json });
        let characters_context = json!({ "characters": &characters_json });
        let speech_manifest = build_manga_context_manifest(MangaContextManifestInput {
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
                    parsed_output: &events_context,
                    raw_output: Some(&events_out),
                    placeholder: false,
                },
                MangaContextPreviousOutput {
                    pack_kind: MANGA_PREVIOUS_OUTPUT_CHARACTER_BINDING,
                    prompt_name: "manga_character_binding",
                    parsed_output: &characters_context,
                    raw_output: Some(&characters_out),
                    placeholder: false,
                },
            ],
            reference_assets: Some(&reference_assets),
            dynamic_user_content: speech_user,
        })?;
        let speech_trace = render_and_chat_trace(MangaRenderTraceInput {
            db: self.db.clone(),
            prompt_engine,
            llm,
            prompt_name: "manga_speech_selection",
            project_id,
            workflow_run_id,
            call_key: format!("manga_pipeline:{chapter_id}:manga_speech_selection"),
            call_order: 3,
            restore_from_checkpoint: should_restore_stage(3),
            variables: &vars,
            manifest: speech_manifest,
            temperature: 0.3,
        })
        .await?;
        self.report_pipeline_llm_usage(
            chapter_id,
            project_id,
            "speech_selection",
            3,
            task_id,
            &speech_trace,
        )
        .await?;
        let speech_out = speech_trace.response_text.clone();
        checkpoint_insert_trace(&mut checkpoint, "speech_trace", &speech_trace)?;
        let speech_json = parse_json_object(&speech_out, "manga_speech_selection")?;
        validate_manga_speech_selection_source_span_ids(&speech_json, &source_spans)?;
        self.save_or_validate_manga_llm_checkpoint(&speech_trace, &speech_json)
            .await?;
        checkpoint_insert(&mut checkpoint, "speech_out", json!(speech_out.clone()));
        self.save_running_pipeline_checkpoint(MangaRunningPipelineCheckpointInput {
            chapter_id,
            project_id,
            stage: "speech_selection",
            current: 3,
            message: "已完成对白旁白选择",
            task_id,
            source_metadata: &source_metadata,
            reference_assets_metadata: &reference_assets_metadata,
            prompt_fingerprint: &prompt_fingerprint,
            checkpoint: &checkpoint,
        })
        .await?;
        self.ensure_pipeline_not_cancelled(project_id, chapter_id)
            .await?;
        self.set_pipeline_status(MangaPipelineStatusInput {
            chapter_id,
            project_id,
            pipeline_status: "running",
            stage: "visual_anchor_extraction",
            current: 4,
            total: PIPELINE_TOTAL_STEPS,
            message: "正在提取场景和视觉锚点",
            task_id,
            allow_cancel_override: false,
        })
        .await?;

        // ---- Step 4: 提取场景和视觉锚点 ----
        let visual_user =
            manga_visual_anchor_extraction_user_message(events_json.clone(), &source_spans)?;
        let events_context = json!({ "events": &events_json });
        let visual_manifest = build_manga_context_manifest(MangaContextManifestInput {
            prompt_engine,
            project_id,
            variables: &vars,
            chapter_content,
            source_spans: &source_spans,
            prompt_names: &["manga_event_beats", "manga_visual_anchor_extraction"],
            previous_outputs: &[MangaContextPreviousOutput {
                pack_kind: MANGA_PREVIOUS_OUTPUT_EVENT_BEATS,
                prompt_name: "manga_event_beats",
                parsed_output: &events_context,
                raw_output: Some(&events_out),
                placeholder: false,
            }],
            reference_assets: Some(&reference_assets),
            dynamic_user_content: visual_user,
        })?;
        let visual_trace = render_and_chat_trace(MangaRenderTraceInput {
            db: self.db.clone(),
            prompt_engine,
            llm,
            prompt_name: "manga_visual_anchor_extraction",
            project_id,
            workflow_run_id,
            call_key: format!("manga_pipeline:{chapter_id}:manga_visual_anchor_extraction"),
            call_order: 4,
            restore_from_checkpoint: should_restore_stage(4),
            variables: &vars,
            manifest: visual_manifest,
            temperature: 0.3,
        })
        .await?;
        self.report_pipeline_llm_usage(
            chapter_id,
            project_id,
            "visual_anchor_extraction",
            4,
            task_id,
            &visual_trace,
        )
        .await?;
        let visual_out = visual_trace.response_text.clone();
        checkpoint_insert_trace(&mut checkpoint, "visual_trace", &visual_trace)?;
        let visual_json = parse_json_object(&visual_out, "manga_visual_anchor_extraction")?;
        validate_manga_visual_anchor_source_span_ids(&visual_json, &source_spans)?;
        self.save_or_validate_manga_llm_checkpoint(&visual_trace, &visual_json)
            .await?;
        checkpoint_insert(&mut checkpoint, "visual_out", json!(visual_out.clone()));
        self.save_running_pipeline_checkpoint(MangaRunningPipelineCheckpointInput {
            chapter_id,
            project_id,
            stage: "visual_anchor_extraction",
            current: 4,
            message: "已完成视觉锚点抽取",
            task_id,
            source_metadata: &source_metadata,
            reference_assets_metadata: &reference_assets_metadata,
            prompt_fingerprint: &prompt_fingerprint,
            checkpoint: &checkpoint,
        })
        .await?;
        self.ensure_pipeline_not_cancelled(project_id, chapter_id)
            .await?;
        self.set_pipeline_status(MangaPipelineStatusInput {
            chapter_id,
            project_id,
            pipeline_status: "running",
            stage: "item_extraction",
            current: 5,
            total: PIPELINE_TOTAL_STEPS,
            message: "正在抽取漫画关键物品",
            task_id,
            allow_cancel_override: false,
        })
        .await?;

        // ---- Step 5: 抽取漫画关键物品 ----
        let items_user =
            manga_item_extraction_user_message(speech_json.clone(), visual_json.clone())?;
        let speech_context = json!({ "speech": &speech_json });
        let visual_context = json!({ "visual_anchors": &visual_json });
        let items_manifest = build_manga_context_manifest(MangaContextManifestInput {
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
                    parsed_output: &speech_context,
                    raw_output: Some(&speech_out),
                    placeholder: false,
                },
                MangaContextPreviousOutput {
                    pack_kind: MANGA_PREVIOUS_OUTPUT_VISUAL_ANCHOR_EXTRACTION,
                    prompt_name: "manga_visual_anchor_extraction",
                    parsed_output: &visual_context,
                    raw_output: Some(&visual_out),
                    placeholder: false,
                },
            ],
            reference_assets: Some(&reference_assets),
            dynamic_user_content: items_user,
        })?;
        let items_trace = render_and_chat_trace(MangaRenderTraceInput {
            db: self.db.clone(),
            prompt_engine,
            llm,
            prompt_name: "manga_item_extraction",
            project_id,
            workflow_run_id,
            call_key: format!("manga_pipeline:{chapter_id}:manga_item_extraction"),
            call_order: 5,
            restore_from_checkpoint: should_restore_stage(5),
            variables: &vars,
            manifest: items_manifest,
            temperature: 0.3,
        })
        .await?;
        self.report_pipeline_llm_usage(
            chapter_id,
            project_id,
            "item_extraction",
            5,
            task_id,
            &items_trace,
        )
        .await?;
        let items_out = items_trace.response_text.clone();
        checkpoint_insert_trace(&mut checkpoint, "items_trace", &items_trace)?;
        let items_json = parse_json_object(&items_out, "manga_item_extraction")?;
        let allowed_source_span_ids =
            manga_item_available_source_span_ids(&speech_json, &visual_json);
        validate_manga_item_extraction_source_span_ids(
            &items_json,
            &source_spans,
            &allowed_source_span_ids,
        )?;
        self.save_or_validate_manga_llm_checkpoint(&items_trace, &items_json)
            .await?;
        checkpoint_insert(&mut checkpoint, "items_out", json!(items_out.clone()));
        self.save_running_pipeline_checkpoint(MangaRunningPipelineCheckpointInput {
            chapter_id,
            project_id,
            stage: "item_extraction",
            current: 5,
            message: "已完成关键物品抽取",
            task_id,
            source_metadata: &source_metadata,
            reference_assets_metadata: &reference_assets_metadata,
            prompt_fingerprint: &prompt_fingerprint,
            checkpoint: &checkpoint,
        })
        .await?;
        self.ensure_pipeline_not_cancelled(project_id, chapter_id)
            .await?;
        self.set_pipeline_status(MangaPipelineStatusInput {
            chapter_id,
            project_id,
            pipeline_status: "running",
            stage: "comic_synthesis",
            current: 6,
            total: PIPELINE_TOTAL_STEPS,
            message: "正在综合漫画章节节奏",
            task_id,
            allow_cancel_override: false,
        })
        .await?;

        // ---- Step 6: 综合漫画章节节奏 ----
        let synthesis_user = manga_chapter_comic_synthesis_user_message(
            events_json.clone(),
            speech_json.clone(),
            visual_json.clone(),
            items_json.clone(),
        )?;
        let events_context = json!({ "events": &events_json });
        let speech_context = json!({ "speech": &speech_json });
        let visual_context = json!({ "visual_anchors": &visual_json });
        let items_context = json!({ "items": &items_json });
        let synthesis_manifest = build_manga_context_manifest(MangaContextManifestInput {
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
                    parsed_output: &events_context,
                    raw_output: Some(&events_out),
                    placeholder: false,
                },
                MangaContextPreviousOutput {
                    pack_kind: MANGA_PREVIOUS_OUTPUT_SPEECH_SELECTION,
                    prompt_name: "manga_speech_selection",
                    parsed_output: &speech_context,
                    raw_output: Some(&speech_out),
                    placeholder: false,
                },
                MangaContextPreviousOutput {
                    pack_kind: MANGA_PREVIOUS_OUTPUT_VISUAL_ANCHOR_EXTRACTION,
                    prompt_name: "manga_visual_anchor_extraction",
                    parsed_output: &visual_context,
                    raw_output: Some(&visual_out),
                    placeholder: false,
                },
                MangaContextPreviousOutput {
                    pack_kind: MANGA_PREVIOUS_OUTPUT_ITEM_EXTRACTION,
                    prompt_name: "manga_item_extraction",
                    parsed_output: &items_context,
                    raw_output: Some(&items_out),
                    placeholder: false,
                },
            ],
            reference_assets: Some(&reference_assets),
            dynamic_user_content: synthesis_user,
        })?;
        let comic_synthesis_trace = render_and_chat_trace(MangaRenderTraceInput {
            db: self.db.clone(),
            prompt_engine,
            llm,
            prompt_name: "manga_chapter_comic_synthesis",
            project_id,
            workflow_run_id,
            call_key: format!("manga_pipeline:{chapter_id}:manga_chapter_comic_synthesis"),
            call_order: 6,
            restore_from_checkpoint: should_restore_stage(6),
            variables: &vars,
            manifest: synthesis_manifest,
            temperature: 0.3,
        })
        .await?;
        self.report_pipeline_llm_usage(
            chapter_id,
            project_id,
            "comic_synthesis",
            6,
            task_id,
            &comic_synthesis_trace,
        )
        .await?;
        let comic_synthesis_out = comic_synthesis_trace.response_text.clone();
        checkpoint_insert_trace(
            &mut checkpoint,
            "comic_synthesis_trace",
            &comic_synthesis_trace,
        )?;
        let synthesis_json =
            parse_json_object(&comic_synthesis_out, "manga_chapter_comic_synthesis")?;
        validate_manga_chapter_comic_synthesis(&synthesis_json)?;
        self.save_or_validate_manga_llm_checkpoint(&comic_synthesis_trace, &synthesis_json)
            .await?;
        checkpoint_insert(
            &mut checkpoint,
            "comic_synthesis_out",
            json!(comic_synthesis_out.clone()),
        );
        self.save_running_pipeline_checkpoint(MangaRunningPipelineCheckpointInput {
            chapter_id,
            project_id,
            stage: "comic_synthesis",
            current: 6,
            message: "已完成漫画章节综合",
            task_id,
            source_metadata: &source_metadata,
            reference_assets_metadata: &reference_assets_metadata,
            prompt_fingerprint: &prompt_fingerprint,
            checkpoint: &checkpoint,
        })
        .await?;
        self.ensure_pipeline_not_cancelled(project_id, chapter_id)
            .await?;
        self.set_pipeline_status(MangaPipelineStatusInput {
            chapter_id,
            project_id,
            pipeline_status: "running",
            stage: "page_planning",
            current: 7,
            total: PIPELINE_TOTAL_STEPS,
            message: "正在生成漫画分页规划",
            task_id,
            allow_cancel_override: false,
        })
        .await?;

        let extraction_summary = json!({
            "characters": characters_json,
            "events": events_json,
            "reference_assets": reference_assets,
            "speech": speech_json,
            "visual_anchors": visual_json,
            "items": items_json.get("items").cloned().unwrap_or_else(|| json!([])),
            "comic_synthesis": synthesis_json
        });

        // ---- Step 7: 分页规划 ----
        let plan_user = manga_page_planning_user_message(extraction_summary.clone())?;
        let planning_manifest = build_manga_context_manifest(MangaContextManifestInput {
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
            dynamic_user_content: plan_user,
        })?;
        let plan_trace = render_and_chat_trace(MangaRenderTraceInput {
            db: self.db.clone(),
            prompt_engine,
            llm,
            prompt_name: "manga_page_planning",
            project_id,
            workflow_run_id,
            call_key: format!("manga_pipeline:{chapter_id}:manga_page_planning"),
            call_order: 7,
            restore_from_checkpoint: should_restore_stage(7),
            variables: &vars,
            manifest: planning_manifest,
            temperature: 0.3,
        })
        .await?;
        self.report_pipeline_llm_usage(
            chapter_id,
            project_id,
            "page_planning",
            7,
            task_id,
            &plan_trace,
        )
        .await?;
        let plan_out = plan_trace.response_text.clone();
        checkpoint_insert_trace(&mut checkpoint, "plan_trace", &plan_trace)?;
        let plan_json: Value = parse_json_object(&plan_out, "manga_page_planning")?;
        self.save_or_validate_manga_llm_checkpoint(&plan_trace, &plan_json)
            .await?;
        checkpoint_insert(&mut checkpoint, "plan_out", json!(plan_out.clone()));
        checkpoint_insert(&mut checkpoint, "plan_json", plan_json.clone());
        self.save_running_pipeline_checkpoint(MangaRunningPipelineCheckpointInput {
            chapter_id,
            project_id,
            stage: "page_planning",
            current: 7,
            message: "已完成漫画分页规划",
            task_id,
            source_metadata: &source_metadata,
            reference_assets_metadata: &reference_assets_metadata,
            prompt_fingerprint: &prompt_fingerprint,
            checkpoint: &checkpoint,
        })
        .await?;
        self.ensure_pipeline_not_cancelled(project_id, chapter_id)
            .await?;
        self.set_pipeline_status(MangaPipelineStatusInput {
            chapter_id,
            project_id,
            pipeline_status: "running",
            stage: "drawing_prompt",
            current: 8,
            total: PIPELINE_TOTAL_STEPS,
            message: "正在生成整页漫画提示词",
            task_id,
            allow_cancel_override: false,
        })
        .await?;

        // ---- Step 8: 整页绘图 prompt 编排 ----
        let dp_step = self
            .run_page_drawing_prompt_step(DrawingPromptStepInput {
                chapter_id,
                project_id,
                llm,
                prompt_engine,
                vars: &vars,
                task_id,
                workflow_run_id,
                source_metadata: &source_metadata,
                reference_assets_metadata: &reference_assets_metadata,
                prompt_fingerprint: &prompt_fingerprint,
                checkpoint: &mut checkpoint,
                chapter_content,
                source_spans: &source_spans,
                reference_assets: &reference_assets,
                plan_json: &plan_json,
                extraction_summary: &extraction_summary,
                restore_from_checkpoint: should_restore_stage(8),
            })
            .await?;
        let page_traces = dp_step.page_traces;
        let dp_json = dp_step.json;
        let dp_out = serde_json::to_string(&dp_json)?;
        let page_prompts = validate_page_drawing_prompts(&dp_json, "manga_page_drawing_prompt")?;
        checkpoint_insert(&mut checkpoint, "dp_out", json!(dp_out.clone()));
        checkpoint_insert(&mut checkpoint, "dp_json", dp_json.clone());
        self.save_running_pipeline_checkpoint(MangaRunningPipelineCheckpointInput {
            chapter_id,
            project_id,
            stage: "drawing_prompt",
            current: 8,
            message: "已完成整页漫画提示词编排",
            task_id,
            source_metadata: &source_metadata,
            reference_assets_metadata: &reference_assets_metadata,
            prompt_fingerprint: &prompt_fingerprint,
            checkpoint: &checkpoint,
        })
        .await?;
        self.ensure_pipeline_not_cancelled(project_id, chapter_id)
            .await?;
        self.set_pipeline_status(MangaPipelineStatusInput {
            chapter_id,
            project_id,
            pipeline_status: "running",
            stage: "persisting",
            current: 8,
            total: PIPELINE_TOTAL_STEPS,
            message: "正在写入整页漫画提示词",
            task_id,
            allow_cancel_override: false,
        })
        .await?;

        let extraction_summary_out = serde_json::to_string(&extraction_summary)?;
        let stage_traces = vec![
            events_trace,
            characters_trace,
            speech_trace,
            visual_trace,
            items_trace,
            comic_synthesis_trace,
        ];

        let results = self
            .persist_pipeline_outputs(MangaPipelinePersistInput {
                chapter_id,
                project_id,
                source,
                source_metadata: &source_metadata,
                reference_assets_metadata: &reference_assets_metadata,
                prompt_fingerprint: &prompt_fingerprint,
                extraction_summary_out: &extraction_summary_out,
                extraction_summary: &extraction_summary,
                characters_json: &characters_json,
                events_json: &events_json,
                stage_traces,
                plan_out: &plan_out,
                plan_trace: Some(plan_trace),
                plan_json: &plan_json,
                dp_out: &dp_out,
                page_traces,
                dp_json: &dp_json,
                page_prompts,
            })
            .await?;

        self.ensure_pipeline_not_cancelled(project_id, chapter_id)
            .await?;
        self.set_pipeline_status(MangaPipelineStatusInput {
            chapter_id,
            project_id,
            pipeline_status: "completed",
            stage: "completed",
            current: PIPELINE_TOTAL_STEPS,
            total: PIPELINE_TOTAL_STEPS,
            message: &format!("漫画流水线完成，生成 {} 页整页提示词", results.len()),
            task_id,
            allow_cancel_override: false,
        })
        .await?;
        self.clear_pipeline_checkpoint(project_id, chapter_id)
            .await?;

        Ok(results)
    }

    async fn save_running_pipeline_checkpoint(
        &self,
        input: MangaRunningPipelineCheckpointInput<'_>,
    ) -> Result<(), AppError> {
        let checkpoint_data = Value::Object(input.checkpoint.clone());
        self.save_pipeline_checkpoint(MangaPipelineCheckpointInput {
            status: MangaPipelineStatusInput {
                chapter_id: input.chapter_id,
                project_id: input.project_id,
                pipeline_status: "running",
                stage: input.stage,
                current: input.current,
                total: PIPELINE_TOTAL_STEPS,
                message: input.message,
                task_id: input.task_id,
                allow_cancel_override: false,
            },
            source_metadata: input.source_metadata,
            reference_assets_metadata: input.reference_assets_metadata,
            prompt_fingerprint: input.prompt_fingerprint,
            checkpoint_data: &checkpoint_data,
        })
        .await
    }
}
