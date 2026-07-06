//! 章节结构化分析
//!
//! 该服务把已确定的章节正文转为稳定的 JSON 产物，供摘要、角色状态、伏笔、
//! RAG 和后续多媒体流程复用。

mod extract;
mod normalization;
mod prompt;

#[cfg(test)]
mod tests;

use chrono::Utc;
use sea_orm::*;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;

use a4n_core::config::app_config::llm_token_profiles;
use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter;
use a4n_engine::llm::provider::{ChatRequest, LlmProvider, LlmTraceContext};
use a4n_engine::prompt::engine::PromptEngine;
use tokio_util::sync::CancellationToken;

use crate::chapter::source_spans::{
    SourceSpan, resolve_source_span_text, source_span_prompt_values, source_spans_for_chapters,
};
use crate::generation::{
    GenerationRunTraceTarget, RecordCompletedGenerationRunInput, chat_request_messages_json,
    context_aware_chat_request_trace_json, delete_generation_run_targets_for,
    generation_checkpoint_fingerprint_sha256, record_completed_generation_run,
};
use crate::llm_checkpoint::{
    CompletedLlmCallCheckpointInput, load_completed_llm_call_checkpoint,
    required_llm_call_checkpoint_json, required_llm_call_workflow_run_id,
    save_completed_llm_call_checkpoint,
};
use crate::llm_context::{
    COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedger,
    LlmContextLedgerPackDraft, context_aware_system_user_request,
};
use crate::source::source_content_sha256;
use crate::structured_llm_stream::{
    ChatJsonStreamInput, StructuredLlmProgress, chat_json_stream_with_progress,
};
use crate::structured_progress::{FieldMilestone, ProgressSpan, StructuredJsonProgressSpec};
use crate::workflow::{
    UpsertWorkflowCheckpointInput, WORKFLOW_STATUS_COMPLETED, WorkflowRunService,
};
pub use extract::{
    analysis_rag_items, character_state_snapshots, foreshadowing_entries, priority_to_importance,
    summary_text,
};
use extract::{merge_tags, metadata_tags, should_replace_summary};
use normalization::normalize_analysis;
use prompt::{build_chapter_analysis_stage_user_message, required_chapter_title};

const CHAPTER_ANALYSIS_STAGE_FACTS: &str = "chapter_fact_extraction";
const CHAPTER_ANALYSIS_STAGE_SUMMARY: &str = "chapter_summary_synthesis";
const CHAPTER_ANALYSIS_STAGE_CHARACTER_STATE: &str = "chapter_character_state";
const CHAPTER_ANALYSIS_STAGE_FORESHADOWING_EVENTS: &str = "chapter_foreshadowing_events";
const CHAPTER_ANALYSIS_STEP_FACTS: &str = "chapter_analysis.facts";
const CHAPTER_ANALYSIS_STEP_SUMMARY: &str = "chapter_analysis.summary";
const CHAPTER_ANALYSIS_STEP_CHARACTER_STATES: &str = "chapter_analysis.character_states";
const CHAPTER_ANALYSIS_STEP_FORESHADOWING_EVENTS: &str = "chapter_analysis.foreshadowing_events";
const CHAPTER_ANALYSIS_STEP_FINAL: &str = "chapter_analysis.final";
const CHAPTER_ANALYSIS_SERVICE_PROGRESS_TOTAL: u32 = 100;
const CHAPTER_ANALYSIS_FACT_FIELDS: &[FieldMilestone] = &[
    FieldMilestone::new("characters", "角色候选", 1),
    FieldMilestone::new("locations", "地点候选", 1),
    FieldMilestone::new("items", "物品候选", 1),
    FieldMilestone::new("event_candidates", "事件候选", 1),
    FieldMilestone::new("foreshadowing_candidates", "伏笔候选", 1),
];
const CHAPTER_ANALYSIS_SUMMARY_FIELDS: &[FieldMilestone] = &[
    FieldMilestone::new("metadata_additions", "元数据补充", 1),
    FieldMilestone::new("summaries", "章节摘要", 1),
];
const CHAPTER_ANALYSIS_CHARACTER_STATE_FIELDS: &[FieldMilestone] =
    &[FieldMilestone::new("character_states", "角色状态", 1)];
const CHAPTER_ANALYSIS_FORESHADOWING_FIELDS: &[FieldMilestone] = &[
    FieldMilestone::new("foreshadowing", "伏笔结构", 1),
    FieldMilestone::new("key_events", "关键事件", 1),
];

pub type ChapterAnalysisServiceProgressReporter<'a> =
    &'a mut (dyn FnMut(u32, u32, &str) -> Result<(), AppError> + Send);

pub fn validate_saved_chapter_analysis(value: &Value) -> Result<(), AppError> {
    normalization::validate_saved_analysis(value)
}

struct ChapterAnalysisServiceProgress<'a> {
    reporter: Option<ChapterAnalysisServiceProgressReporter<'a>>,
    cancel_token: Option<&'a CancellationToken>,
}

impl<'a> ChapterAnalysisServiceProgress<'a> {
    fn new(
        reporter: Option<ChapterAnalysisServiceProgressReporter<'a>>,
        cancel_token: Option<&'a CancellationToken>,
    ) -> Self {
        Self {
            reporter,
            cancel_token,
        }
    }

    fn report(&mut self, current: u32, total: u32, message: &str) -> Result<(), AppError> {
        if let Some(reporter) = self.reporter.as_deref_mut() {
            reporter(current, total.max(1), message)?;
        }
        if self
            .cancel_token
            .is_some_and(CancellationToken::is_cancelled)
        {
            return Err(AppError::Cancelled);
        }
        Ok(())
    }
}
#[derive(Debug, Clone)]
pub struct ChapterAnalysisRequestStep {
    pub key: String,
    pub title: String,
    pub prompt_name: String,
    pub request: ChatRequest,
}

#[derive(Debug, Clone)]
pub struct CharacterStateSnapshot {
    pub name: String,
    pub state_summary: String,
    pub key_attributes: Value,
    pub emotional_state: Option<String>,
    pub current_location: Option<String>,
    pub current_goal: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ForeshadowingEntry {
    pub description: String,
    pub original_text: Option<String>,
    pub category: Option<String>,
    pub priority: Option<String>,
    pub related_entities: Option<Value>,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct AnalysisRagItem {
    pub id_suffix: String,
    pub data_type: String,
    pub content_kind: String,
    pub text: String,
    pub metadata: Value,
}

#[derive(Debug, Clone)]
struct ChapterAnalysisRun {
    analysis: Value,
    stage_traces: Vec<ChapterAnalysisStageTrace>,
}

#[derive(Debug, Clone)]
struct ChapterAnalysisStageTrace {
    stage: &'static str,
    request: ChatRequest,
    output: Value,
}

fn chapter_analysis_stage_fingerprint(
    stage: &str,
    request: &ChatRequest,
) -> Result<Value, AppError> {
    Ok(json!({
        "schema_version": 1,
        "stage": stage,
        "request": context_aware_chat_request_trace_json(request)?,
    }))
}

fn restored_chapter_analysis_stage_output(
    checkpoint: Value,
    context: &str,
) -> Result<Value, AppError> {
    let parsed_output =
        checkpoint
            .get("parsed_output")
            .cloned()
            .ok_or_else(|| AppError::InvalidState {
                current: "missing_checkpoint_parsed_output".to_string(),
                operation: format!("读取{context}断点"),
            })?;
    let response_content = checkpoint
        .get("response_content")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_checkpoint_response_content".to_string(),
            operation: format!("读取{context}断点"),
        })?;
    let reparsed = parse_chapter_analysis_stage_json(response_content, context)?;
    if reparsed != parsed_output {
        return Err(AppError::InvalidState {
            current: "checkpoint_parsed_output_mismatch".to_string(),
            operation: format!("校验{context}断点"),
        });
    }
    Ok(reparsed)
}

fn ensure_chapter_has_analysis_content(ch: &chapter::Model) -> Result<(), AppError> {
    if ch.has_available_content() {
        return Ok(());
    }
    Err(AppError::InvalidArgument(
        "章节正文为空，不能生成结构化分析".to_string(),
    ))
}

#[derive(Clone)]
pub struct ChapterAnalysisService {
    db: Arc<DatabaseConnection>,
}

impl ChapterAnalysisService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn analyze_and_store_with_progress_and_workflow<'a>(
        &self,
        chapter_id: i64,
        llm: &dyn LlmProvider,
        prompt_engine: &dyn PromptEngine,
        progress: Option<ChapterAnalysisServiceProgressReporter<'a>>,
        cancel_token: Option<&'a CancellationToken>,
        workflow_run_id: Option<i64>,
    ) -> Result<Value, AppError> {
        let ch = chapter::Entity::find_by_id(chapter_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("章节不存在: {chapter_id}")))?;

        ensure_chapter_has_analysis_content(&ch)?;

        let mut progress = ChapterAnalysisServiceProgress::new(progress, cancel_token);
        let run = self
            .analyze_chapter(&ch, llm, prompt_engine, &mut progress, workflow_run_id)
            .await?;
        let analysis = run.analysis.clone();
        let summary = summary_text(&analysis);
        let tags = merge_tags(ch.style_tags.as_ref(), metadata_tags(&analysis));

        let mut active: chapter::ActiveModel = ch.clone().into();
        active.analysis_data = Set(Some(analysis.clone()));
        if should_replace_summary(ch.summary.as_deref(), ch.chapter_number)
            && let Some(summary) = summary
        {
            active.summary = Set(Some(summary));
        }
        if !tags.is_empty() {
            active.style_tags = Set(Some(json!(tags)));
        }
        active.updated_at = Set(Utc::now().naive_utc());
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        delete_generation_run_targets_for(self.db.as_ref(), "chapter_analysis", &[ch.id]).await?;
        self.record_chapter_analysis_generation_run(&ch, &run, llm)
            .await?;

        Ok(analysis)
    }

    pub async fn build_analysis_requests(
        &self,
        chapter_id: i64,
        prompt_engine: &dyn PromptEngine,
    ) -> Result<Vec<ChapterAnalysisRequestStep>, AppError> {
        let ch = chapter::Entity::find_by_id(chapter_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("章节不存在: {chapter_id}")))?;

        ensure_chapter_has_analysis_content(&ch)?;

        build_chapter_analysis_preview_requests(&ch, prompt_engine)
    }

    async fn load_completed_stage_checkpoint(
        &self,
        workflow_run_id: i64,
        step_key: &str,
        input_fingerprint_json: &Value,
    ) -> Result<Option<Value>, AppError> {
        load_completed_llm_call_checkpoint(
            self.db.clone(),
            workflow_run_id,
            step_key,
            input_fingerprint_json,
        )
        .await?
        .map(|checkpoint| required_llm_call_checkpoint_json(checkpoint, step_key))
        .transpose()
    }

    async fn save_completed_stage_checkpoint(
        &self,
        workflow_run_id: i64,
        step_key: &str,
        step_order: i32,
        input_fingerprint_json: Value,
        response_content: String,
        parsed_output: Value,
    ) -> Result<(), AppError> {
        save_completed_llm_call_checkpoint(
            self.db.clone(),
            CompletedLlmCallCheckpointInput {
                workflow_run_id,
                call_key: step_key.to_string(),
                call_order: step_order,
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
    }

    async fn save_completed_final_checkpoint(
        &self,
        workflow_run_id: i64,
        step_key: &str,
        step_order: i32,
        input_fingerprint_json: Value,
        output: Value,
    ) -> Result<(), AppError> {
        let input_fingerprint_sha256 =
            generation_checkpoint_fingerprint_sha256(&input_fingerprint_json)?;
        WorkflowRunService::new(self.db.clone())
            .upsert_checkpoint(UpsertWorkflowCheckpointInput {
                workflow_run_id,
                step_key: step_key.to_string(),
                step_order,
                status: WORKFLOW_STATUS_COMPLETED.to_string(),
                input_fingerprint_sha256: Some(input_fingerprint_sha256),
                input_fingerprint_json: Some(input_fingerprint_json),
                checkpoint_json: Some(json!({ "output": output })),
                artifacts_json: None,
                generation_run_ids_json: None,
                error_message: None,
                completed: true,
            })
            .await?;
        Ok(())
    }

    async fn mark_current_step(
        &self,
        workflow_run_id: i64,
        step_key: &str,
    ) -> Result<(), AppError> {
        WorkflowRunService::new(self.db.clone())
            .mark_current_step(workflow_run_id, step_key)
            .await?;
        Ok(())
    }

    async fn analyze_chapter(
        &self,
        ch: &chapter::Model,
        llm: &dyn LlmProvider,
        prompt_engine: &dyn PromptEngine,
        progress: &mut ChapterAnalysisServiceProgress<'_>,
        workflow_run_id: Option<i64>,
    ) -> Result<ChapterAnalysisRun, AppError> {
        let workflow_run_id =
            required_llm_call_workflow_run_id(workflow_run_id, "chapter_analysis")?;
        let source_spans = source_spans_for_chapters(std::slice::from_ref(ch));
        let mut stage_traces = Vec::new();
        let facts_request =
            build_chapter_fact_extraction_request(ch, prompt_engine, &source_spans)?;
        self.mark_current_step(workflow_run_id, CHAPTER_ANALYSIS_STEP_FACTS)
            .await?;
        let facts_fingerprint =
            chapter_analysis_stage_fingerprint(CHAPTER_ANALYSIS_STAGE_FACTS, &facts_request)?;
        let facts = if let Some(checkpoint) = self
            .load_completed_stage_checkpoint(
                workflow_run_id,
                CHAPTER_ANALYSIS_STEP_FACTS,
                &facts_fingerprint,
            )
            .await?
        {
            progress.report(
                35,
                CHAPTER_ANALYSIS_SERVICE_PROGRESS_TOTAL,
                "事实抽取阶段已复用断点",
            )?;
            restored_chapter_analysis_stage_output(checkpoint, "事实抽取阶段")?
        } else {
            let (facts, response_content) = chat_chapter_analysis_stage(
                llm,
                facts_request.clone(),
                CHAPTER_ANALYSIS_STAGE_FACTS,
                chapter_analysis_root_field_progress(
                    "事实抽取阶段",
                    CHAPTER_ANALYSIS_FACT_FIELDS,
                    ProgressSpan::new(0, 35),
                ),
                progress,
            )
            .await?;
            validate_chapter_fact_extraction(&facts, &source_spans)?;
            self.save_completed_stage_checkpoint(
                workflow_run_id,
                CHAPTER_ANALYSIS_STEP_FACTS,
                1,
                facts_fingerprint,
                response_content,
                facts.clone(),
            )
            .await?;
            facts
        };
        validate_chapter_fact_extraction(&facts, &source_spans)?;
        stage_traces.push(ChapterAnalysisStageTrace {
            stage: CHAPTER_ANALYSIS_STAGE_FACTS,
            request: facts_request,
            output: facts.clone(),
        });

        let summary_request = build_chapter_summary_synthesis_request(ch, prompt_engine, &facts)?;
        self.mark_current_step(workflow_run_id, CHAPTER_ANALYSIS_STEP_SUMMARY)
            .await?;
        let summary_fingerprint =
            chapter_analysis_stage_fingerprint(CHAPTER_ANALYSIS_STAGE_SUMMARY, &summary_request)?;
        let summary = if let Some(checkpoint) = self
            .load_completed_stage_checkpoint(
                workflow_run_id,
                CHAPTER_ANALYSIS_STEP_SUMMARY,
                &summary_fingerprint,
            )
            .await?
        {
            progress.report(
                55,
                CHAPTER_ANALYSIS_SERVICE_PROGRESS_TOTAL,
                "摘要综合阶段已复用断点",
            )?;
            restored_chapter_analysis_stage_output(checkpoint, "摘要综合阶段")?
        } else {
            let (summary, response_content) = chat_chapter_analysis_stage(
                llm,
                summary_request.clone(),
                CHAPTER_ANALYSIS_STAGE_SUMMARY,
                chapter_analysis_root_field_progress(
                    "摘要综合阶段",
                    CHAPTER_ANALYSIS_SUMMARY_FIELDS,
                    ProgressSpan::new(35, 55),
                ),
                progress,
            )
            .await?;
            validate_chapter_summary_synthesis(&summary)?;
            self.save_completed_stage_checkpoint(
                workflow_run_id,
                CHAPTER_ANALYSIS_STEP_SUMMARY,
                2,
                summary_fingerprint,
                response_content,
                summary.clone(),
            )
            .await?;
            summary
        };
        validate_chapter_summary_synthesis(&summary)?;
        stage_traces.push(ChapterAnalysisStageTrace {
            stage: CHAPTER_ANALYSIS_STAGE_SUMMARY,
            request: summary_request,
            output: summary.clone(),
        });

        let character_state_request =
            build_chapter_character_state_request(ch, prompt_engine, &facts)?;
        self.mark_current_step(workflow_run_id, CHAPTER_ANALYSIS_STEP_CHARACTER_STATES)
            .await?;
        let character_state_fingerprint = chapter_analysis_stage_fingerprint(
            CHAPTER_ANALYSIS_STAGE_CHARACTER_STATE,
            &character_state_request,
        )?;
        let character_states = if let Some(checkpoint) = self
            .load_completed_stage_checkpoint(
                workflow_run_id,
                CHAPTER_ANALYSIS_STEP_CHARACTER_STATES,
                &character_state_fingerprint,
            )
            .await?
        {
            progress.report(
                75,
                CHAPTER_ANALYSIS_SERVICE_PROGRESS_TOTAL,
                "角色状态阶段已复用断点",
            )?;
            restored_chapter_analysis_stage_output(checkpoint, "角色状态阶段")?
        } else {
            let (character_states, response_content) = chat_chapter_analysis_stage(
                llm,
                character_state_request.clone(),
                CHAPTER_ANALYSIS_STAGE_CHARACTER_STATE,
                chapter_analysis_root_field_progress(
                    "角色状态阶段",
                    CHAPTER_ANALYSIS_CHARACTER_STATE_FIELDS,
                    ProgressSpan::new(55, 75),
                ),
                progress,
            )
            .await?;
            validate_chapter_character_state(&character_states)?;
            self.save_completed_stage_checkpoint(
                workflow_run_id,
                CHAPTER_ANALYSIS_STEP_CHARACTER_STATES,
                3,
                character_state_fingerprint,
                response_content,
                character_states.clone(),
            )
            .await?;
            character_states
        };
        validate_chapter_character_state(&character_states)?;
        stage_traces.push(ChapterAnalysisStageTrace {
            stage: CHAPTER_ANALYSIS_STAGE_CHARACTER_STATE,
            request: character_state_request,
            output: character_states.clone(),
        });

        let foreshadowing_request =
            build_chapter_foreshadowing_events_request(ch, prompt_engine, &facts)?;
        self.mark_current_step(workflow_run_id, CHAPTER_ANALYSIS_STEP_FORESHADOWING_EVENTS)
            .await?;
        let foreshadowing_fingerprint = chapter_analysis_stage_fingerprint(
            CHAPTER_ANALYSIS_STAGE_FORESHADOWING_EVENTS,
            &foreshadowing_request,
        )?;
        let foreshadowing_events = if let Some(checkpoint) = self
            .load_completed_stage_checkpoint(
                workflow_run_id,
                CHAPTER_ANALYSIS_STEP_FORESHADOWING_EVENTS,
                &foreshadowing_fingerprint,
            )
            .await?
        {
            progress.report(
                95,
                CHAPTER_ANALYSIS_SERVICE_PROGRESS_TOTAL,
                "伏笔事件阶段已复用断点",
            )?;
            restored_chapter_analysis_stage_output(checkpoint, "伏笔事件阶段")?
        } else {
            let (foreshadowing_events, response_content) = chat_chapter_analysis_stage(
                llm,
                foreshadowing_request.clone(),
                CHAPTER_ANALYSIS_STAGE_FORESHADOWING_EVENTS,
                chapter_analysis_root_field_progress(
                    "伏笔事件阶段",
                    CHAPTER_ANALYSIS_FORESHADOWING_FIELDS,
                    ProgressSpan::new(75, 95),
                ),
                progress,
            )
            .await?;
            validate_chapter_foreshadowing_events(&foreshadowing_events)?;
            self.save_completed_stage_checkpoint(
                workflow_run_id,
                CHAPTER_ANALYSIS_STEP_FORESHADOWING_EVENTS,
                4,
                foreshadowing_fingerprint,
                response_content,
                foreshadowing_events.clone(),
            )
            .await?;
            foreshadowing_events
        };
        validate_chapter_foreshadowing_events(&foreshadowing_events)?;
        stage_traces.push(ChapterAnalysisStageTrace {
            stage: CHAPTER_ANALYSIS_STAGE_FORESHADOWING_EVENTS,
            request: foreshadowing_request,
            output: foreshadowing_events.clone(),
        });

        self.mark_current_step(workflow_run_id, CHAPTER_ANALYSIS_STEP_FINAL)
            .await?;
        progress.report(
            96,
            CHAPTER_ANALYSIS_SERVICE_PROGRESS_TOTAL,
            "正在组装章节结构化分析结果",
        )?;
        let parsed = assemble_staged_chapter_analysis(
            facts,
            summary,
            character_states,
            foreshadowing_events,
        )?;
        let materialized = materialize_analysis_source_spans(parsed, &source_spans)?;
        let analysis = normalize_analysis(materialized, ch)?;
        self.save_completed_final_checkpoint(
            workflow_run_id,
            CHAPTER_ANALYSIS_STEP_FINAL,
            5,
            json!({
                "schema_version": 1,
                "stage": "chapter_analysis.final",
                "chapter_id": ch.id,
                "chapter_number": ch.chapter_number,
                "analysis": &analysis,
            }),
            analysis.clone(),
        )
        .await?;
        progress.report(
            CHAPTER_ANALYSIS_SERVICE_PROGRESS_TOTAL,
            CHAPTER_ANALYSIS_SERVICE_PROGRESS_TOTAL,
            "章节结构化分析结果已生成",
        )?;
        Ok(ChapterAnalysisRun {
            analysis,
            stage_traces,
        })
    }

    async fn record_chapter_analysis_generation_run(
        &self,
        ch: &chapter::Model,
        run: &ChapterAnalysisRun,
        llm: &dyn LlmProvider,
    ) -> Result<(), AppError> {
        record_completed_generation_run(
            self.db.as_ref(),
            RecordCompletedGenerationRunInput {
                project_id: ch.project_id,
                workflow: "chapter_analysis".to_string(),
                step_key: format!("chapter_{}", ch.chapter_number),
                title: "生成章节分析".to_string(),
                model_kind: "llm".to_string(),
                provider: None,
                model: Some(llm.model_name().to_string()),
                prompt_template_names_json: Some(json!([
                    CHAPTER_ANALYSIS_STAGE_FACTS,
                    CHAPTER_ANALYSIS_STAGE_SUMMARY,
                    CHAPTER_ANALYSIS_STAGE_CHARACTER_STATE,
                    CHAPTER_ANALYSIS_STAGE_FORESHADOWING_EVENTS,
                ])),
                prompt_template_hashes_json: None,
                rendered_messages_json: Some(Value::Array(
                    run.stage_traces
                        .iter()
                        .map(|trace| {
                            Ok(json!({
                                "stage": trace.stage,
                                "messages": chat_request_messages_json(&trace.request)?,
                            }))
                        })
                        .collect::<Result<Vec<_>, AppError>>()?,
                )),
                rendered_prompts_json: None,
                input_variables_json: Some(json!({
                    "chapter_number": ch.chapter_number,
                    "title": ch.title,
                    "content_chars": ch.content.chars().count(),
                })),
                source_refs_json: Some(json!({
                    "source_node": "chapters",
                    "source_kind": "chapter_content",
                    "chapter_id": ch.id,
                    "content_sha256": run.analysis
                        .get("_meta")
                        .and_then(Value::as_object)
                        .and_then(|meta| meta.get("content_sha256"))
                        .cloned(),
                })),
                media_refs_json: None,
                request_json: Some(Value::Array(
                    run.stage_traces
                        .iter()
                        .map(|trace| {
                            Ok(json!({
                                "stage": trace.stage,
                                "request": context_aware_chat_request_trace_json(&trace.request)?,
                            }))
                        })
                        .collect::<Result<Vec<_>, AppError>>()?,
                )),
                response_summary_json: Some(json!({
                    "chapter_number": ch.chapter_number,
                    "stage_count": run.stage_traces.len(),
                    "stage_outputs": run
                        .stage_traces
                        .iter()
                        .map(|trace| json!({
                            "stage": trace.stage,
                            "summary": json_value_shape_summary(&trace.output),
                        }))
                        .collect::<Vec<_>>(),
                })),
                targets: vec![GenerationRunTraceTarget {
                    target_kind: "chapter_analysis".to_string(),
                    target_table: "chapter_analysis".to_string(),
                    target_id: ch.id,
                }],
            },
        )
        .await?;
        Ok(())
    }
}

fn json_value_shape_summary(value: &Value) -> Value {
    match value {
        Value::Object(object) => json!({
            "kind": "object",
            "keys": object.keys().cloned().collect::<Vec<_>>(),
            "array_lengths": object
                .iter()
                .filter_map(|(key, value)| {
                    value.as_array().map(|items| json!({
                        "key": key,
                        "len": items.len(),
                    }))
                })
                .collect::<Vec<_>>(),
        }),
        Value::Array(items) => json!({
            "kind": "array",
            "len": items.len(),
        }),
        Value::String(text) => json!({
            "kind": "string",
            "chars": text.chars().count(),
        }),
        Value::Bool(_) => json!({ "kind": "bool" }),
        Value::Number(_) => json!({ "kind": "number" }),
        Value::Null => json!({ "kind": "null" }),
    }
}

async fn chat_chapter_analysis_stage(
    llm: &dyn LlmProvider,
    request: ChatRequest,
    context: &str,
    structured_progress: StructuredLlmProgress,
    progress: &mut ChapterAnalysisServiceProgress<'_>,
) -> Result<(Value, String), AppError> {
    let cancel_token = progress.cancel_token;
    let truncated_message = format!(
        "{context} 输出达到当前 max_tokens 上限，返回 JSON 被截断；请在设置页调大“章节分析”输出档位，或减少章节正文长度后重试"
    );
    let content = chat_json_stream_with_progress(
        ChatJsonStreamInput {
            llm,
            request,
            prompt_name: context,
            truncated_message: &truncated_message,
            structured_progress: Some(structured_progress),
            cancel_token,
            cancelled_message: "章节结构化分析已停止",
        },
        |current, message| {
            progress
                .report(current, CHAPTER_ANALYSIS_SERVICE_PROGRESS_TOTAL, message)
                .map_err(|err| err.to_string())
        },
    )
    .await
    .map_err(AppError::LlmError)?;
    let parsed = parse_chapter_analysis_stage_json(&content, context)?;
    Ok((parsed, content))
}

fn chapter_analysis_root_field_progress(
    item_label: &'static str,
    milestones: &'static [FieldMilestone],
    span: ProgressSpan,
) -> StructuredLlmProgress {
    StructuredLlmProgress {
        spec: StructuredJsonProgressSpec {
            root_array_key: "",
            item_label,
            expected_items: None,
            milestones,
        },
        span,
    }
}

fn build_chapter_analysis_preview_requests(
    ch: &chapter::Model,
    prompt_engine: &dyn PromptEngine,
) -> Result<Vec<ChapterAnalysisRequestStep>, AppError> {
    let source_spans = source_spans_for_chapters(std::slice::from_ref(ch));
    let facts_placeholder = json!({
        "characters": [],
        "locations": [],
        "items": [],
        "event_candidates": [],
        "foreshadowing_candidates": [],
        "preview_note": "预览阶段尚未调用事实抽取；运行时会使用上一阶段真实 JSON"
    });
    Ok(vec![
        ChapterAnalysisRequestStep {
            key: CHAPTER_ANALYSIS_STAGE_FACTS.to_string(),
            title: "章节分析 1/4：事实候选抽取".to_string(),
            prompt_name: CHAPTER_ANALYSIS_STAGE_FACTS.to_string(),
            request: build_chapter_fact_extraction_request(ch, prompt_engine, &source_spans)?,
        },
        ChapterAnalysisRequestStep {
            key: CHAPTER_ANALYSIS_STAGE_SUMMARY.to_string(),
            title: "章节分析 2/4：摘要与标签综合".to_string(),
            prompt_name: CHAPTER_ANALYSIS_STAGE_SUMMARY.to_string(),
            request: build_chapter_summary_synthesis_request(
                ch,
                prompt_engine,
                &facts_placeholder,
            )?,
        },
        ChapterAnalysisRequestStep {
            key: CHAPTER_ANALYSIS_STAGE_CHARACTER_STATE.to_string(),
            title: "章节分析 3/4：角色状态快照".to_string(),
            prompt_name: CHAPTER_ANALYSIS_STAGE_CHARACTER_STATE.to_string(),
            request: build_chapter_character_state_request(ch, prompt_engine, &facts_placeholder)?,
        },
        ChapterAnalysisRequestStep {
            key: CHAPTER_ANALYSIS_STAGE_FORESHADOWING_EVENTS.to_string(),
            title: "章节分析 4/4：伏笔与关键事件".to_string(),
            prompt_name: CHAPTER_ANALYSIS_STAGE_FORESHADOWING_EVENTS.to_string(),
            request: build_chapter_foreshadowing_events_request(
                ch,
                prompt_engine,
                &facts_placeholder,
            )?,
        },
    ])
}

fn build_chapter_fact_extraction_request(
    ch: &chapter::Model,
    prompt_engine: &dyn PromptEngine,
    source_spans: &[SourceSpan],
) -> Result<ChatRequest, AppError> {
    let chapter_title = required_chapter_title(ch)?;
    let input = json!({
        "chapter_title": chapter_title,
        "content": ch.content,
        "source_spans": source_span_prompt_values(source_spans),
    });
    build_chapter_analysis_stage_request(
        ch,
        prompt_engine,
        CHAPTER_ANALYSIS_STAGE_FACTS,
        "从章节正文抽取角色、地点、物品、关键事件和伏笔候选",
        input,
        json!({
            "format": "json_object",
            "required_fields": ["characters", "locations", "items", "event_candidates", "foreshadowing_candidates"],
            "source_span_rule": "所有 source_span_ids 和 original_text_span_id 必须来自稳定上下文 chapter_analysis_source.source_spans[].id"
        }),
    )
}

fn build_chapter_summary_synthesis_request(
    ch: &chapter::Model,
    prompt_engine: &dyn PromptEngine,
    facts: &Value,
) -> Result<ChatRequest, AppError> {
    let chapter_title = required_chapter_title(ch)?;
    build_chapter_analysis_stage_request(
        ch,
        prompt_engine,
        CHAPTER_ANALYSIS_STAGE_SUMMARY,
        "基于章节正文和事实候选生成摘要、标签、基调和时间标记",
        json!({
            "chapter_title": chapter_title,
            "content": ch.content,
            "facts": facts,
        }),
        json!({
            "format": "json_object",
            "required_fields": ["metadata_additions", "summaries"]
        }),
    )
}

fn build_chapter_character_state_request(
    ch: &chapter::Model,
    prompt_engine: &dyn PromptEngine,
    facts: &Value,
) -> Result<ChatRequest, AppError> {
    let chapter_title = required_chapter_title(ch)?;
    build_chapter_analysis_stage_request(
        ch,
        prompt_engine,
        CHAPTER_ANALYSIS_STAGE_CHARACTER_STATE,
        "基于章节正文和事实候选生成角色章节结束状态",
        json!({
            "chapter_title": chapter_title,
            "content": ch.content,
            "characters": facts.get("characters").cloned().unwrap_or_else(|| json!([])),
            "event_candidates": facts.get("event_candidates").cloned().unwrap_or_else(|| json!([])),
        }),
        json!({
            "format": "json_object",
            "required_fields": ["character_states"]
        }),
    )
}

fn build_chapter_foreshadowing_events_request(
    ch: &chapter::Model,
    prompt_engine: &dyn PromptEngine,
    facts: &Value,
) -> Result<ChatRequest, AppError> {
    let chapter_title = required_chapter_title(ch)?;
    build_chapter_analysis_stage_request(
        ch,
        prompt_engine,
        CHAPTER_ANALYSIS_STAGE_FORESHADOWING_EVENTS,
        "从事实候选中筛选最终伏笔和关键事件",
        json!({
            "chapter_title": chapter_title,
            "facts": {
                "event_candidates": facts.get("event_candidates").cloned().unwrap_or_else(|| json!([])),
                "foreshadowing_candidates": facts.get("foreshadowing_candidates").cloned().unwrap_or_else(|| json!([])),
            }
        }),
        json!({
            "format": "json_object",
            "required_fields": ["foreshadowing", "key_events"],
            "source_span_rule": "只能使用 facts 中已有的 source_span_ids 和 original_text_span_id"
        }),
    )
}

fn build_chapter_analysis_stage_request(
    ch: &chapter::Model,
    prompt_engine: &dyn PromptEngine,
    prompt_name: &str,
    instruction: &str,
    input: Value,
    output_contract: Value,
) -> Result<ChatRequest, AppError> {
    let variables = HashMap::new();
    let system_prompt = prompt_engine.render(prompt_name, &variables, Some(ch.project_id))?;
    let source_spans = source_spans_for_chapters(std::slice::from_ref(ch));
    let dynamic_input = chapter_analysis_dynamic_input(input);
    let user_prompt = build_chapter_analysis_stage_user_message(
        prompt_name,
        instruction,
        dynamic_input,
        output_contract,
    )?;
    let mut ledger = LlmContextLedger::new();
    add_chapter_analysis_source_pack(&mut ledger, ch, &source_spans)?;
    let manifest = ledger.into_manifest(user_prompt, None);
    Ok(context_aware_system_user_request(
        system_prompt,
        manifest,
        Some(0.2),
        None,
        Some(llm_token_profiles::CHAPTER_ANALYSIS.into()),
    )?
    .with_json_response_format()
    .with_trace_context(LlmTraceContext {
        project_id: Some(ch.project_id),
        root_project_id: None,
        workflow: "chapter_analysis".to_string(),
        step_key: prompt_name.to_string(),
        generation_run_id: None,
        workflow_run_id: None,
        call_key: None,
        input_fingerprint_sha256: None,
        context_plan_json: None,
        context_pack_refs_json: None,
    }))
}

fn chapter_analysis_dynamic_input(mut input: Value) -> Value {
    if let Some(object) = input.as_object_mut() {
        object.remove("content");
        object.remove("source_spans");
        object.insert(
            "chapter_source_context".to_string(),
            json!("章节正文和 source_spans 位于稳定上下文 chapter_analysis_source"),
        );
    }
    input
}

fn add_chapter_analysis_source_pack(
    ledger: &mut LlmContextLedger,
    ch: &chapter::Model,
    source_spans: &[SourceSpan],
) -> Result<(), AppError> {
    let content_sha256 = source_content_sha256(&ch.content);
    let chapter_title = required_chapter_title(ch)?;
    ledger.add_pack(crate::llm_context::LlmContextLedgerPackInput {
        scope_kind: ("chapter").into(),
        pack_kind: ("chapter_analysis_source").into(),
        source_refs_json: Some(json!({
            "chapter_id": ch.id,
            "chapter_number": ch.chapter_number,
            "content_sha256": content_sha256,
            "source_span_count": source_spans.len(),
        })),
        config_identity_json: json!({
            "ledger_version": "chapter_analysis_context_v1",
            "kind": "chapter_source",
            "content_sha256": content_sha256,
            "source_span_count": source_spans.len(),
        }),
        required: true,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FULL_RAW,
                json!({
                    "kind": "chapter_source",
                    "chapter_id": ch.id,
                    "chapter_number": ch.chapter_number,
                    "chapter_title": chapter_title,
                    "content": ch.content,
                    "source_spans": source_span_prompt_values(source_spans),
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FOCUSED_RAW,
                json!({
                    "kind": "chapter_source",
                    "chapter_id": ch.id,
                    "chapter_number": ch.chapter_number,
                    "chapter_title": chapter_title,
                    "source_spans": source_span_prompt_values(source_spans),
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_MINIMAL,
                json!({
                    "kind": "chapter_source",
                    "chapter_id": ch.id,
                    "chapter_number": ch.chapter_number,
                    "chapter_title": chapter_title,
                    "content_sha256": content_sha256,
                    "source_span_count": source_spans.len(),
                }),
            ),
        ],
    })
}

fn parse_chapter_analysis_stage_json(raw: &str, context: &str) -> Result<Value, AppError> {
    let value: Value = serde_json::from_str(raw.trim()).map_err(|err| {
        AppError::InvalidArgument(format!("{context} 必须返回严格 JSON 对象: {err}"))
    })?;
    if !value.is_object() {
        return Err(AppError::InvalidArgument(format!(
            "{context} 必须返回 JSON 对象"
        )));
    }
    Ok(value)
}

fn assemble_staged_chapter_analysis(
    facts: Value,
    summary: Value,
    character_states: Value,
    foreshadowing_events: Value,
) -> Result<Value, AppError> {
    let metadata_additions = summary
        .get("metadata_additions")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            analysis_materialize_error("chapter_summary_synthesis.metadata_additions 必须是对象")
        })?;
    let mut metadata = serde_json::Map::new();
    for key in ["characters", "locations", "items"] {
        metadata.insert(
            key.to_string(),
            facts
                .get(key)
                .cloned()
                .unwrap_or_else(|| Value::Array(Vec::new())),
        );
    }
    metadata.insert(
        "tags".to_string(),
        metadata_additions
            .get("tags")
            .cloned()
            .unwrap_or_else(|| Value::Array(Vec::new())),
    );
    metadata.insert(
        "tone".to_string(),
        metadata_additions
            .get("tone")
            .cloned()
            .unwrap_or(Value::Null),
    );
    metadata.insert(
        "timeline_marker".to_string(),
        metadata_additions
            .get("timeline_marker")
            .cloned()
            .unwrap_or(Value::Null),
    );

    Ok(json!({
        "metadata": Value::Object(metadata),
        "summaries": summary.get("summaries").cloned().unwrap_or(Value::Null),
        "character_states": character_states.get("character_states").cloned().unwrap_or(Value::Null),
        "foreshadowing": foreshadowing_events.get("foreshadowing").cloned().unwrap_or(Value::Null),
        "key_events": foreshadowing_events.get("key_events").cloned().unwrap_or(Value::Null),
    }))
}

fn validate_chapter_fact_extraction(
    facts: &Value,
    source_spans: &[SourceSpan],
) -> Result<(), AppError> {
    reject_root_unknown_fields(
        facts,
        &[
            "characters",
            "locations",
            "items",
            "event_candidates",
            "foreshadowing_candidates",
        ],
        CHAPTER_ANALYSIS_STAGE_FACTS,
    )?;
    for key in ["characters", "locations", "items"] {
        validate_fact_entity_array(facts, key, source_spans)?;
    }
    let events = facts
        .get("event_candidates")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            analysis_materialize_error("chapter_fact_extraction.event_candidates 必须是数组")
        })?;
    for (index, event) in events.iter().enumerate() {
        let path = format!("chapter_fact_extraction.event_candidates[{index}]");
        let object = event
            .as_object()
            .ok_or_else(|| analysis_materialize_error(format!("{path} 必须是对象")))?;
        reject_materialize_unknown_fields(
            object,
            &[
                "event_ref",
                "type",
                "description",
                "importance",
                "involved_characters",
                "source_span_ids",
            ],
            &path,
        )?;
        required_materialize_string(object, "event_ref", &format!("{path}.event_ref"))?;
        required_materialize_string(object, "description", &format!("{path}.description"))?;
        validate_required_string_enum(
            object.get("type"),
            &[
                "battle",
                "revelation",
                "relationship",
                "discovery",
                "decision",
                "death",
                "arrival",
                "departure",
            ],
            &format!("{path}.type"),
        )?;
        validate_required_string_enum(
            object.get("importance"),
            &["high", "medium", "low"],
            &format!("{path}.importance"),
        )?;
        validate_string_array(
            object.get("involved_characters"),
            &format!("{path}.involved_characters"),
        )?;
        validate_source_span_ids(
            object.get("source_span_ids"),
            source_spans,
            &format!("{path}.source_span_ids"),
        )?;
    }

    let candidates = facts
        .get("foreshadowing_candidates")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            analysis_materialize_error(
                "chapter_fact_extraction.foreshadowing_candidates 必须是数组",
            )
        })?;
    for (index, item) in candidates.iter().enumerate() {
        let path = format!("chapter_fact_extraction.foreshadowing_candidates[{index}]");
        let object = item
            .as_object()
            .ok_or_else(|| analysis_materialize_error(format!("{path} 必须是对象")))?;
        reject_materialize_unknown_fields(
            object,
            &[
                "foreshadowing_ref",
                "description",
                "category",
                "priority",
                "related_entities",
                "original_text_span_id",
            ],
            &path,
        )?;
        required_materialize_string(
            object,
            "foreshadowing_ref",
            &format!("{path}.foreshadowing_ref"),
        )?;
        required_materialize_string(object, "description", &format!("{path}.description"))?;
        validate_required_string_enum(
            object.get("category"),
            &[
                "character_secret",
                "plot_twist",
                "item_mystery",
                "world_rule",
            ],
            &format!("{path}.category"),
        )?;
        validate_required_string_enum(
            object.get("priority"),
            &["high", "medium", "low"],
            &format!("{path}.priority"),
        )?;
        validate_string_array(
            object.get("related_entities"),
            &format!("{path}.related_entities"),
        )?;
        let span_id = required_materialize_string(
            object,
            "original_text_span_id",
            &format!("{path}.original_text_span_id"),
        )?;
        resolve_source_span_text(source_spans, &span_id).map_err(analysis_materialize_error)?;
    }
    Ok(())
}

fn validate_fact_entity_array(
    facts: &Value,
    key: &str,
    source_spans: &[SourceSpan],
) -> Result<(), AppError> {
    let values = facts.get(key).and_then(Value::as_array).ok_or_else(|| {
        analysis_materialize_error(format!("chapter_fact_extraction.{key} 必须是数组"))
    })?;
    for (index, value) in values.iter().enumerate() {
        let path = format!("chapter_fact_extraction.{key}[{index}]");
        let object = value
            .as_object()
            .ok_or_else(|| analysis_materialize_error(format!("{path} 必须是对象")))?;
        reject_materialize_unknown_fields(object, &["name", "source_span_ids"], &path)?;
        required_materialize_string(object, "name", &format!("{path}.name"))?;
        validate_source_span_ids(
            object.get("source_span_ids"),
            source_spans,
            &format!("{path}.source_span_ids"),
        )?;
    }
    Ok(())
}

fn validate_chapter_summary_synthesis(summary: &Value) -> Result<(), AppError> {
    reject_root_unknown_fields(
        summary,
        &["metadata_additions", "summaries"],
        CHAPTER_ANALYSIS_STAGE_SUMMARY,
    )?;
    let metadata = summary
        .get("metadata_additions")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            analysis_materialize_error("chapter_summary_synthesis.metadata_additions 必须是对象")
        })?;
    reject_materialize_unknown_fields(
        metadata,
        &["tags", "tone", "timeline_marker"],
        "chapter_summary_synthesis.metadata_additions",
    )?;
    validate_string_array(
        metadata.get("tags"),
        "chapter_summary_synthesis.metadata_additions.tags",
    )?;
    validate_optional_stage_string(
        metadata.get("tone"),
        "chapter_summary_synthesis.metadata_additions.tone",
    )?;
    validate_optional_stage_string(
        metadata.get("timeline_marker"),
        "chapter_summary_synthesis.metadata_additions.timeline_marker",
    )?;

    let summaries = summary
        .get("summaries")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            analysis_materialize_error("chapter_summary_synthesis.summaries 必须是对象")
        })?;
    reject_materialize_unknown_fields(
        summaries,
        &["compressed", "one_line", "keywords"],
        "chapter_summary_synthesis.summaries",
    )?;
    required_materialize_string(
        summaries,
        "compressed",
        "chapter_summary_synthesis.summaries.compressed",
    )?;
    required_materialize_string(
        summaries,
        "one_line",
        "chapter_summary_synthesis.summaries.one_line",
    )?;
    validate_string_array(
        summaries.get("keywords"),
        "chapter_summary_synthesis.summaries.keywords",
    )?;
    Ok(())
}

fn validate_chapter_character_state(value: &Value) -> Result<(), AppError> {
    reject_root_unknown_fields(
        value,
        &["character_states"],
        CHAPTER_ANALYSIS_STAGE_CHARACTER_STATE,
    )?;
    let states = value
        .get("character_states")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            analysis_materialize_error("chapter_character_state.character_states 必须是对象")
        })?;
    for (name, state) in states {
        if name.trim().is_empty() {
            return Err(analysis_materialize_error(
                "chapter_character_state.character_states 角色名不能为空",
            ));
        }
        let object = state.as_object().ok_or_else(|| {
            analysis_materialize_error(format!(
                "chapter_character_state.character_states.{name} 必须是对象"
            ))
        })?;
        reject_materialize_unknown_fields(
            object,
            &["location", "status", "changes", "emotional_state"],
            &format!("chapter_character_state.character_states.{name}"),
        )?;
        validate_optional_stage_string(
            object.get("location"),
            &format!("chapter_character_state.character_states.{name}.location"),
        )?;
        validate_optional_stage_string(
            object.get("status"),
            &format!("chapter_character_state.character_states.{name}.status"),
        )?;
        validate_string_array(
            object.get("changes"),
            &format!("chapter_character_state.character_states.{name}.changes"),
        )?;
        validate_optional_stage_string(
            object.get("emotional_state"),
            &format!("chapter_character_state.character_states.{name}.emotional_state"),
        )?;
    }
    Ok(())
}

fn validate_chapter_foreshadowing_events(value: &Value) -> Result<(), AppError> {
    reject_root_unknown_fields(
        value,
        &["foreshadowing", "key_events"],
        CHAPTER_ANALYSIS_STAGE_FORESHADOWING_EVENTS,
    )?;
    let root = value
        .as_object()
        .ok_or_else(|| analysis_materialize_error("chapter_foreshadowing_events 必须是对象"))?;
    if !root.contains_key("foreshadowing") {
        return Err(analysis_materialize_error(
            "chapter_foreshadowing_events 缺少 foreshadowing",
        ));
    }
    if !root.contains_key("key_events") {
        return Err(analysis_materialize_error(
            "chapter_foreshadowing_events 缺少 key_events",
        ));
    }
    Ok(())
}

fn reject_root_unknown_fields(value: &Value, allowed: &[&str], path: &str) -> Result<(), AppError> {
    let object = value
        .as_object()
        .ok_or_else(|| analysis_materialize_error(format!("{path} 必须是 JSON 对象")))?;
    reject_materialize_unknown_fields(object, allowed, path)
}

fn validate_optional_stage_string(value: Option<&Value>, path: &str) -> Result<(), AppError> {
    match value {
        Some(Value::Null) => Ok(()),
        Some(Value::String(text)) if !text.trim().is_empty() => Ok(()),
        Some(Value::String(_)) => Err(analysis_materialize_error(format!(
            "{path} 必须是非空字符串或 null"
        ))),
        Some(_) => Err(analysis_materialize_error(format!(
            "{path} 必须是字符串或 null"
        ))),
        None => Err(analysis_materialize_error(format!("{path} 缺失"))),
    }
}

fn materialize_analysis_source_spans(
    raw: Value,
    source_spans: &[SourceSpan],
) -> Result<Value, AppError> {
    let mut root = raw.as_object().cloned().ok_or_else(|| {
        AppError::InvalidArgument("chapter_analysis 必须返回 JSON 对象".to_string())
    })?;
    materialize_metadata_source_spans(&mut root, source_spans)?;
    materialize_key_event_source_spans(&mut root, source_spans)?;
    materialize_foreshadowing_source_spans(&mut root, source_spans)?;
    Ok(Value::Object(root))
}

fn materialize_metadata_source_spans(
    root: &mut serde_json::Map<String, Value>,
    source_spans: &[SourceSpan],
) -> Result<(), AppError> {
    let metadata = root
        .get_mut("metadata")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| analysis_materialize_error("chapter_analysis 缺少 metadata 对象"))?;
    for key in ["characters", "locations", "items"] {
        let values = metadata
            .get(key)
            .and_then(Value::as_array)
            .ok_or_else(|| analysis_materialize_error(format!("metadata.{key} 必须是数组")))?;
        let mut materialized = Vec::with_capacity(values.len());
        for (index, value) in values.iter().enumerate() {
            let object = value.as_object().ok_or_else(|| {
                analysis_materialize_error(format!(
                    "metadata.{key}[{index}] 必须是包含 name 和 source_span_ids 的对象"
                ))
            })?;
            reject_materialize_unknown_fields(
                object,
                &["name", "source_span_ids"],
                &format!("metadata.{key}[{index}]"),
            )?;
            let name = required_materialize_string(
                object,
                "name",
                &format!("metadata.{key}[{index}].name"),
            )?;
            validate_source_span_ids(
                object.get("source_span_ids"),
                source_spans,
                &format!("metadata.{key}[{index}].source_span_ids"),
            )?;
            materialized.push(Value::String(name));
        }
        metadata.insert(key.to_string(), Value::Array(materialized));
    }
    normalize_string_array(metadata.get_mut("tags"), "metadata.tags")?;
    normalize_optional_string(metadata.get_mut("tone"), "metadata.tone")?;
    normalize_optional_string(
        metadata.get_mut("timeline_marker"),
        "metadata.timeline_marker",
    )?;
    Ok(())
}

fn materialize_key_event_source_spans(
    root: &mut serde_json::Map<String, Value>,
    source_spans: &[SourceSpan],
) -> Result<(), AppError> {
    let Some(events) = root.get_mut("key_events").and_then(Value::as_array_mut) else {
        return Err(analysis_materialize_error(
            "chapter_analysis.key_events 必须是数组",
        ));
    };
    for (index, event) in events.iter_mut().enumerate() {
        let object = event.as_object_mut().ok_or_else(|| {
            analysis_materialize_error(format!("key_events[{index}] 必须是 JSON 对象"))
        })?;
        reject_materialize_unknown_fields(
            object,
            &[
                "type",
                "description",
                "importance",
                "involved_characters",
                "source_span_ids",
            ],
            &format!("key_events[{index}]"),
        )?;
        validate_required_string_enum(
            object.get("type"),
            &[
                "battle",
                "revelation",
                "relationship",
                "discovery",
                "decision",
                "death",
                "arrival",
                "departure",
            ],
            &format!("key_events[{index}].type"),
        )?;
        validate_required_string_enum(
            object.get("importance"),
            &["high", "medium", "low"],
            &format!("key_events[{index}].importance"),
        )?;
        required_materialize_string(
            object,
            "description",
            &format!("key_events[{index}].description"),
        )?;
        validate_string_array(
            object.get("involved_characters"),
            &format!("key_events[{index}].involved_characters"),
        )?;
        validate_source_span_ids(
            object.get("source_span_ids"),
            source_spans,
            &format!("key_events[{index}].source_span_ids"),
        )?;
        object.remove("source_span_ids");
    }
    Ok(())
}

fn materialize_foreshadowing_source_spans(
    root: &mut serde_json::Map<String, Value>,
    source_spans: &[SourceSpan],
) -> Result<(), AppError> {
    let foreshadowing = root
        .get_mut("foreshadowing")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| analysis_materialize_error("chapter_analysis.foreshadowing 必须是对象"))?;
    let Some(planted) = foreshadowing
        .get_mut("planted")
        .and_then(Value::as_array_mut)
    else {
        return Err(analysis_materialize_error(
            "chapter_analysis.foreshadowing.planted 必须是数组",
        ));
    };
    for (index, item) in planted.iter_mut().enumerate() {
        let object = item.as_object_mut().ok_or_else(|| {
            analysis_materialize_error(format!("foreshadowing.planted[{index}] 必须是 JSON 对象"))
        })?;
        reject_materialize_unknown_fields(
            object,
            &[
                "description",
                "original_text_span_id",
                "category",
                "priority",
                "related_entities",
            ],
            &format!("foreshadowing.planted[{index}]"),
        )?;
        required_materialize_string(
            object,
            "description",
            &format!("foreshadowing.planted[{index}].description"),
        )?;
        validate_required_string_enum(
            object.get("category"),
            &[
                "character_secret",
                "plot_twist",
                "item_mystery",
                "world_rule",
            ],
            &format!("foreshadowing.planted[{index}].category"),
        )?;
        validate_required_string_enum(
            object.get("priority"),
            &["high", "medium", "low"],
            &format!("foreshadowing.planted[{index}].priority"),
        )?;
        validate_string_array(
            object.get("related_entities"),
            &format!("foreshadowing.planted[{index}].related_entities"),
        )?;
        let span_id = required_materialize_string(
            object,
            "original_text_span_id",
            &format!("foreshadowing.planted[{index}].original_text_span_id"),
        )?;
        let original_text =
            resolve_source_span_text(source_spans, &span_id).map_err(analysis_materialize_error)?;
        object.insert("original_text".to_string(), Value::String(original_text));
        object.remove("original_text_span_id");
    }
    Ok(())
}

fn reject_materialize_unknown_fields(
    object: &serde_json::Map<String, Value>,
    allowed: &[&str],
    path: &str,
) -> Result<(), AppError> {
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(analysis_materialize_error(format!(
                "{path} 包含未知字段 {key}；允许字段：{}",
                allowed.join(", ")
            )));
        }
    }
    Ok(())
}

fn validate_source_span_ids(
    value: Option<&Value>,
    source_spans: &[SourceSpan],
    path: &str,
) -> Result<(), AppError> {
    let ids = value
        .and_then(Value::as_array)
        .ok_or_else(|| analysis_materialize_error(format!("{path} 必须是 source_span_id 数组")))?;
    if ids.is_empty() {
        return Err(analysis_materialize_error(format!("{path} 不能为空")));
    }
    for (index, id) in ids.iter().enumerate() {
        let id = id
            .as_str()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .ok_or_else(|| {
                analysis_materialize_error(format!("{path}[{index}] 必须是非空字符串"))
            })?;
        resolve_source_span_text(source_spans, id).map_err(analysis_materialize_error)?;
    }
    Ok(())
}

fn validate_string_array(value: Option<&Value>, path: &str) -> Result<(), AppError> {
    let values = value
        .and_then(Value::as_array)
        .ok_or_else(|| analysis_materialize_error(format!("{path} 必须是数组")))?;
    for (index, value) in values.iter().enumerate() {
        let text = value
            .as_str()
            .map(str::trim)
            .ok_or_else(|| analysis_materialize_error(format!("{path}[{index}] 必须是字符串")))?;
        if text.is_empty() {
            return Err(analysis_materialize_error(format!(
                "{path}[{index}] 不能为空"
            )));
        }
    }
    Ok(())
}

fn normalize_string_array(value: Option<&mut Value>, path: &str) -> Result<(), AppError> {
    let values = value
        .and_then(Value::as_array_mut)
        .ok_or_else(|| analysis_materialize_error(format!("{path} 必须是数组")))?;

    let mut normalized = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let text = value
            .as_str()
            .map(str::trim)
            .ok_or_else(|| analysis_materialize_error(format!("{path}[{index}] 必须是字符串")))?;
        if text.is_empty()
            || normalized
                .iter()
                .any(|item: &Value| item.as_str() == Some(text))
        {
            continue;
        }
        normalized.push(Value::String(text.to_string()));
    }
    *values = normalized;
    Ok(())
}

fn normalize_optional_string(value: Option<&mut Value>, path: &str) -> Result<(), AppError> {
    match value {
        Some(Value::Null) => Ok(()),
        Some(Value::String(text)) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return Err(analysis_materialize_error(format!(
                    "{path} 必须是非空字符串或 null"
                )));
            }
            *text = trimmed.to_string();
            Ok(())
        }
        Some(_) => Err(analysis_materialize_error(format!(
            "{path} 必须是字符串或 null"
        ))),
        None => Err(analysis_materialize_error(format!("{path} 缺失"))),
    }
}

fn validate_required_string_enum(
    value: Option<&Value>,
    allowed: &[&str],
    path: &str,
) -> Result<(), AppError> {
    let text = value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| analysis_materialize_error(format!("{path} 必须是非空字符串")))?;
    if allowed.contains(&text) {
        Ok(())
    } else {
        Err(analysis_materialize_error(format!(
            "{path} 无效：{text}；允许值：{}",
            allowed.join("/")
        )))
    }
}

fn required_materialize_string(
    object: &serde_json::Map<String, Value>,
    key: &str,
    path: &str,
) -> Result<String, AppError> {
    object
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| analysis_materialize_error(format!("{path} 必须是非空字符串")))
}

fn analysis_materialize_error(message: impl Into<String>) -> AppError {
    AppError::InvalidArgument(message.into())
}
