use chrono::Utc;
use sea_orm::*;
use serde_json::{Value, json};
use std::collections::HashMap;

use a4n_core::config::app_config::llm_token_profiles;
use a4n_core::error::AppError;
use a4n_core::models::chapter::{chapter_outline, part_outline};
use a4n_engine::llm::provider::{ChatRequest, LlmProvider, LlmTraceContext};
use a4n_engine::prompt::engine::PromptEngine;

use crate::generation::{
    GenerationRunTraceTarget, RecordCompletedGenerationRunInput, chat_request_messages_json,
    context_aware_chat_request_trace_json, record_completed_generation_run,
    text_response_summary_json,
};
use crate::llm_checkpoint::{
    CompletedLlmCallCheckpointInput, llm_call_checkpoint_fingerprint_sha256,
    load_completed_llm_call_checkpoint, required_llm_call_checkpoint_json,
    required_llm_call_workflow_run_id, save_completed_llm_call_checkpoint,
};
use crate::llm_context::{
    COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedger,
    LlmContextLedgerPackDraft, LlmContextManifest, context_aware_system_user_request,
};
use crate::source::source_content_sha256;
use crate::workflow::WorkflowRunService;

use super::chapter_contract::{
    chapter_outline_active_model, chapter_outline_generation_manifest_user_message,
    chapter_outline_generation_max_tokens, ensure_focus_part_in_context,
    parse_chapter_outline_batch, requested_part_for_chapter, required_part_summary,
    update_chapter_outline_active_model, validate_chapter_generation_range,
    validate_chapter_outline_generation_request, validate_requested_part_context,
};
use super::context::{
    ChapterOutlineGenerationContext, load_chapter_outline_generation_context, part_context_json,
};
use super::{
    ChapterOutlineCountGenerationInput, ChapterOutlineGenerationInput,
    ChapterOutlineRegenerationInput, OutlineRequestPreviewStep, OutlineService, ProgressReporter,
    blocked_outline_preview_step, ensure_response_not_truncated, ready_outline_preview_step,
    report_progress, reused_outline_preview_step, validate_existing_chapter_outline_for_request,
};

mod beat_mapping;

#[derive(Debug, Clone)]
struct GeneratedChapterOutline {
    chapter_json: Value,
    request: ChatRequest,
    response_content: String,
    finish_reason: Option<String>,
}

struct ChapterOutlineSequenceInput<'a, 'progress> {
    project_id: i64,
    start_chapter: i32,
    count: i32,
    focus_part: Option<&'a part_outline::Model>,
    llm: &'a dyn LlmProvider,
    prompt_engine: &'a dyn PromptEngine,
    variables: &'a HashMap<String, Value>,
    progress: ProgressReporter<'progress>,
    workflow_run_id: Option<i64>,
}

struct SingleChapterOutlineJsonInput<'a> {
    project_id: i64,
    context: &'a ChapterOutlineGenerationContext,
    focus_part: Option<&'a part_outline::Model>,
    chapter_number: i32,
    llm: &'a dyn LlmProvider,
    prompt_engine: &'a dyn PromptEngine,
    variables: &'a HashMap<String, Value>,
    workflow_run_id: Option<i64>,
}

struct ChapterOutlineRequestInput<'a> {
    project_id: i64,
    context: &'a ChapterOutlineGenerationContext,
    focus_part: Option<&'a part_outline::Model>,
    start_chapter: i32,
    count: i32,
    end_chapter: i32,
    prompt_engine: &'a dyn PromptEngine,
    variables: &'a HashMap<String, Value>,
}

pub(super) struct ContextAwareChapterOutlineRequestInput<'a> {
    pub project_id: i64,
    pub context: &'a ChapterOutlineGenerationContext,
    pub prompt_content: String,
    pub user_content: String,
    pub count: i32,
    pub start_chapter: i32,
    pub end_chapter: i32,
    pub temperature: Option<f32>,
    pub workflow: &'a str,
    pub step_key: String,
}

impl OutlineService {
    /// 列出项目所有章节大纲
    pub async fn list_chapter_outlines(
        &self,
        project_id: i64,
    ) -> Result<Vec<chapter_outline::Model>, AppError> {
        chapter_outline::Entity::find()
            .filter(chapter_outline::Column::ProjectId.eq(project_id))
            .order_by_asc(chapter_outline::Column::ChapterNumber)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    /// 生成章节大纲（增量）
    pub async fn generate_chapter_outlines(
        &self,
        input: ChapterOutlineGenerationInput<'_, '_>,
    ) -> Result<Vec<chapter_outline::Model>, AppError> {
        let ChapterOutlineGenerationInput {
            project_id,
            part_id,
            start_chapter,
            count,
            llm,
            prompt_engine,
            variables,
            progress,
            workflow_run_id,
        } = input;
        let part = self.get_part(part_id).await?;
        if part.project_id != project_id {
            return Err(AppError::InvalidArgument(format!(
                "分部大纲 {part_id} 不属于项目 {project_id}"
            )));
        }
        let (start_num, end_num) = validate_chapter_generation_range(&part, start_chapter, count)?;
        self.generate_chapter_outline_sequence(ChapterOutlineSequenceInput {
            project_id,
            start_chapter: start_num,
            count: end_num - start_num + 1,
            focus_part: Some(&part),
            llm,
            prompt_engine,
            variables,
            progress,
            workflow_run_id,
        })
        .await
    }

    /// 按数量追加章节大纲（短篇项目或已有分部范围内的追加生成）。
    pub async fn generate_chapter_outlines_by_count(
        &self,
        input: ChapterOutlineCountGenerationInput<'_, '_>,
    ) -> Result<Vec<chapter_outline::Model>, AppError> {
        let ChapterOutlineCountGenerationInput {
            project_id,
            start_chapter,
            count,
            llm,
            prompt_engine,
            variables,
            progress,
            workflow_run_id,
        } = input;
        self.generate_chapter_outline_sequence(ChapterOutlineSequenceInput {
            project_id,
            start_chapter,
            count,
            focus_part: None,
            llm,
            prompt_engine,
            variables,
            progress,
            workflow_run_id,
        })
        .await
    }

    pub async fn regenerate_chapter_outline(
        &self,
        input: ChapterOutlineRegenerationInput<'_, '_>,
    ) -> Result<chapter_outline::Model, AppError> {
        let ChapterOutlineRegenerationInput {
            project_id,
            outline_id,
            llm,
            prompt_engine,
            variables,
            progress,
            workflow_run_id,
        } = input;
        let old = chapter_outline::Entity::find_by_id(outline_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("章节大纲不存在: {outline_id}")))?;
        if old.project_id != project_id {
            return Err(AppError::InvalidArgument(format!(
                "章节大纲 {outline_id} 不属于项目 {project_id}"
            )));
        }

        let chapter_number = old.chapter_number;
        let context = load_chapter_outline_generation_context(self.db.as_ref(), project_id).await?;
        validate_requested_part_context(&context.parts, chapter_number, chapter_number)?;
        let focus_part = match old.part_id {
            Some(part_id) => {
                let part = self.get_part(part_id).await?;
                if part.project_id != project_id {
                    return Err(AppError::InvalidArgument(format!(
                        "章节大纲关联的分部 {part_id} 不属于项目 {project_id}"
                    )));
                }
                Some(part)
            }
            None => None,
        };
        ensure_focus_part_in_context(focus_part.as_ref(), &context.parts)?;
        self.ensure_no_other_outline_for_chapter(project_id, chapter_number, outline_id)
            .await?;

        report_progress(
            progress,
            0,
            1,
            &format!("正在重新生成第 {chapter_number} 章章纲"),
        )?;
        let generated = self
            .generate_single_chapter_outline_json(SingleChapterOutlineJsonInput {
                project_id,
                context: &context,
                focus_part: focus_part.as_ref(),
                chapter_number,
                llm,
                prompt_engine,
                variables,
                workflow_run_id,
            })
            .await?;
        let part_id =
            requested_part_for_chapter(&context.parts, chapter_number)?.map(|part| part.id);
        let now = Utc::now().naive_utc();
        let active =
            update_chapter_outline_active_model(old, part_id, &generated.chapter_json, now)?;

        let updated = active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        self.record_chapter_outline_generation_run(
            &updated,
            &generated.request,
            &generated.response_content,
            generated.finish_reason.as_deref(),
            llm,
            "重新生成章节大纲",
        )
        .await?;
        report_progress(
            progress,
            1,
            1,
            &format!("已重新生成第 {chapter_number} 章章纲"),
        )?;
        Ok(updated)
    }

    async fn generate_single_chapter_outline_json(
        &self,
        input: SingleChapterOutlineJsonInput<'_>,
    ) -> Result<GeneratedChapterOutline, AppError> {
        let SingleChapterOutlineJsonInput {
            project_id,
            context,
            focus_part,
            chapter_number,
            llm,
            prompt_engine,
            variables,
            workflow_run_id,
        } = input;
        let mut request = build_chapter_outline_request(ChapterOutlineRequestInput {
            project_id,
            context,
            focus_part,
            start_chapter: chapter_number,
            count: 1,
            end_chapter: chapter_number,
            prompt_engine,
            variables,
        })?;
        let call_key = format!("chapter_{chapter_number}");
        let workflow_run_id = required_llm_call_workflow_run_id(workflow_run_id, &call_key)?;
        let fingerprint_json =
            chapter_outline_call_fingerprint_json(&call_key, chapter_number, &request)?;
        let input_fingerprint_sha256 = llm_call_checkpoint_fingerprint_sha256(&fingerprint_json)?;
        if let Some(trace_context) = request.trace_context.as_mut() {
            trace_context.workflow_run_id = Some(workflow_run_id);
            trace_context.call_key = Some(call_key.clone());
            trace_context.input_fingerprint_sha256 = Some(input_fingerprint_sha256);
        }
        WorkflowRunService::new(self.db.clone())
            .mark_current_step(workflow_run_id, &call_key)
            .await?;
        if let Some(checkpoint) = load_completed_llm_call_checkpoint(
            self.db.clone(),
            workflow_run_id,
            &call_key,
            &fingerprint_json,
        )
        .await?
        {
            let checkpoint_json = required_llm_call_checkpoint_json(checkpoint, &call_key)?;
            return restored_chapter_outline_checkpoint(checkpoint_json, request, chapter_number);
        }
        let response = llm.chat(request.clone()).await?;

        let response_usage_json = response
            .usage
            .as_ref()
            .map(serde_json::to_value)
            .transpose()?;
        let content = response.message.content;
        let finish_reason = response.finish_reason;
        ensure_response_not_truncated(
            finish_reason.as_deref(),
            &format!("第 {chapter_number} 章章纲"),
        )?;
        let mut chapters = parse_chapter_outline_batch(&content, chapter_number, 1)?;
        let chapter_json = chapters
            .pop()
            .ok_or_else(|| AppError::InvalidArgument("outline.chapters 不能为空".to_string()))?;
        let generated = GeneratedChapterOutline {
            chapter_json,
            request,
            response_content: content,
            finish_reason,
        };
        save_completed_llm_call_checkpoint(
            self.db.clone(),
            CompletedLlmCallCheckpointInput {
                workflow_run_id,
                call_key,
                call_order: chapter_number,
                input_fingerprint_json: fingerprint_json,
                checkpoint_json: json!({
                    "output": generated.chapter_json.clone(),
                    "parsed_output": generated.chapter_json.clone(),
                    "response_content": generated.response_content.clone(),
                    "usage": response_usage_json,
                    "finish_reason": generated.finish_reason.clone(),
                }),
                artifacts_json: None,
            },
        )
        .await?;
        Ok(generated)
    }

    pub async fn preview_regenerate_chapter_outline_request(
        &self,
        project_id: i64,
        outline_id: i64,
        prompt_engine: &dyn PromptEngine,
        variables: &HashMap<String, serde_json::Value>,
    ) -> Result<ChatRequest, AppError> {
        let old = chapter_outline::Entity::find_by_id(outline_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("章节大纲不存在: {outline_id}")))?;
        if old.project_id != project_id {
            return Err(AppError::InvalidArgument(format!(
                "章节大纲 {outline_id} 不属于项目 {project_id}"
            )));
        }
        let chapter_number = old.chapter_number;
        let context = load_chapter_outline_generation_context(self.db.as_ref(), project_id).await?;
        validate_requested_part_context(&context.parts, chapter_number, chapter_number)?;
        let focus_part = match old.part_id {
            Some(part_id) => {
                let part = self.get_part(part_id).await?;
                if part.project_id != project_id {
                    return Err(AppError::InvalidArgument(format!(
                        "章节大纲关联的分部 {part_id} 不属于项目 {project_id}"
                    )));
                }
                Some(part)
            }
            None => None,
        };
        ensure_focus_part_in_context(focus_part.as_ref(), &context.parts)?;
        self.ensure_no_other_outline_for_chapter(project_id, chapter_number, outline_id)
            .await?;
        build_chapter_outline_request(ChapterOutlineRequestInput {
            project_id,
            context: &context,
            focus_part: focus_part.as_ref(),
            start_chapter: chapter_number,
            count: 1,
            end_chapter: chapter_number,
            prompt_engine,
            variables,
        })
    }

    pub async fn preview_generate_chapter_outlines_by_count_requests(
        &self,
        project_id: i64,
        start_chapter: i32,
        count: i32,
        prompt_engine: &dyn PromptEngine,
        variables: &HashMap<String, serde_json::Value>,
    ) -> Result<Vec<OutlineRequestPreviewStep>, AppError> {
        self.preview_chapter_outline_sequence_requests(
            project_id,
            start_chapter,
            count,
            None,
            prompt_engine,
            variables,
        )
        .await
    }

    pub async fn preview_generate_part_chapter_outline_requests(
        &self,
        project_id: i64,
        part_id: i64,
        prompt_engine: &dyn PromptEngine,
        variables: &HashMap<String, serde_json::Value>,
    ) -> Result<Vec<OutlineRequestPreviewStep>, AppError> {
        let part = self.get_part(part_id).await?;
        if part.project_id != project_id {
            return Err(AppError::InvalidArgument(format!(
                "分部大纲 {part_id} 不属于项目 {project_id}"
            )));
        }
        let count = part.end_chapter - part.start_chapter + 1;
        self.preview_chapter_outline_sequence_requests(
            project_id,
            part.start_chapter,
            count,
            Some(&part),
            prompt_engine,
            variables,
        )
        .await
    }

    async fn preview_chapter_outline_sequence_requests(
        &self,
        project_id: i64,
        start_chapter: i32,
        count: i32,
        focus_part: Option<&part_outline::Model>,
        prompt_engine: &dyn PromptEngine,
        variables: &HashMap<String, serde_json::Value>,
    ) -> Result<Vec<OutlineRequestPreviewStep>, AppError> {
        let end_chapter = validate_chapter_outline_generation_request(start_chapter, count)?;
        if let Some(part) = focus_part {
            required_part_summary(part)?;
            if start_chapter < part.start_chapter || end_chapter > part.end_chapter {
                return Err(AppError::InvalidArgument(format!(
                    "章节大纲生成范围 {start_chapter}..={end_chapter} 超出分部范围 {}..={}",
                    part.start_chapter, part.end_chapter
                )));
            }
        }

        let context = load_chapter_outline_generation_context(self.db.as_ref(), project_id).await?;
        validate_requested_part_context(&context.parts, start_chapter, end_chapter)?;
        ensure_focus_part_in_context(focus_part, &context.parts)?;

        let mut steps = Vec::new();
        let mut first_pending_seen = false;
        for chapter_number in start_chapter..=end_chapter {
            let expected_part_id =
                requested_part_for_chapter(&context.parts, chapter_number)?.map(|part| part.id);
            let title = format!("第 {chapter_number} 章章纲");
            if let Some(existing) = self
                .existing_chapter_outline_for_number(project_id, chapter_number)
                .await?
            {
                validate_existing_chapter_outline_for_request(
                    &existing,
                    chapter_number,
                    expected_part_id,
                )?;
                steps.push(reused_outline_preview_step(
                    format!("chapter_{chapter_number}"),
                    title,
                    "outline",
                    "已有章节大纲，真实生成会保留并跳过该请求。",
                ));
                continue;
            }

            if first_pending_seen {
                steps.push(blocked_outline_preview_step(
                    format!("chapter_{chapter_number}"),
                    title,
                    "outline",
                    "该章节请求依赖前一章纲生成并写入后的上下文，需先完成前序章节。",
                ));
                continue;
            }

            let request = build_chapter_outline_request(ChapterOutlineRequestInput {
                project_id,
                context: &context,
                focus_part,
                start_chapter: chapter_number,
                count: 1,
                end_chapter: chapter_number,
                prompt_engine,
                variables,
            })?;
            steps.push(ready_outline_preview_step(
                format!("chapter_{chapter_number}"),
                title,
                "outline",
                request,
            ));
            first_pending_seen = true;
        }

        Ok(steps)
    }

    async fn generate_chapter_outline_sequence(
        &self,
        input: ChapterOutlineSequenceInput<'_, '_>,
    ) -> Result<Vec<chapter_outline::Model>, AppError> {
        let ChapterOutlineSequenceInput {
            project_id,
            start_chapter,
            count,
            focus_part,
            llm,
            prompt_engine,
            variables,
            progress,
            workflow_run_id,
        } = input;
        let end_chapter = validate_chapter_outline_generation_request(start_chapter, count)?;
        if let Some(part) = focus_part {
            required_part_summary(part)?;
            if start_chapter < part.start_chapter || end_chapter > part.end_chapter {
                return Err(AppError::InvalidArgument(format!(
                    "章节大纲生成范围 {start_chapter}..={end_chapter} 超出分部范围 {}..={}",
                    part.start_chapter, part.end_chapter
                )));
            }
        }

        let context = load_chapter_outline_generation_context(self.db.as_ref(), project_id).await?;
        validate_requested_part_context(&context.parts, start_chapter, end_chapter)?;
        ensure_focus_part_in_context(focus_part, &context.parts)?;

        report_progress(progress, 0, count, &format!("准备生成 {count} 个章节大纲"))?;
        let mut results = Vec::new();
        for chapter_number in start_chapter..=end_chapter {
            let current = chapter_number - start_chapter + 1;
            let expected_part_id =
                requested_part_for_chapter(&context.parts, chapter_number)?.map(|part| part.id);

            if let Some(existing) = self
                .existing_chapter_outline_for_number(project_id, chapter_number)
                .await?
            {
                validate_existing_chapter_outline_for_request(
                    &existing,
                    chapter_number,
                    expected_part_id,
                )?;
                report_progress(
                    progress,
                    current,
                    count,
                    &format!("第 {current}/{count} 个章节大纲已存在，保留第 {chapter_number} 章"),
                )?;
                results.push(existing);
                continue;
            }

            report_progress(
                progress,
                current - 1,
                count,
                &format!("正在生成第 {current}/{count} 个章节大纲（第 {chapter_number} 章）"),
            )?;
            let generated = self
                .generate_single_chapter_outline_json(SingleChapterOutlineJsonInput {
                    project_id,
                    context: &context,
                    focus_part,
                    chapter_number,
                    llm,
                    prompt_engine,
                    variables,
                    workflow_run_id,
                })
                .await?;
            let part_id =
                requested_part_for_chapter(&context.parts, chapter_number)?.map(|part| part.id);
            let outline = chapter_outline_active_model(
                project_id,
                part_id,
                generated.chapter_json,
                Utc::now().naive_utc(),
            )?
            .insert(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
            self.record_chapter_outline_generation_run(
                &outline,
                &generated.request,
                &generated.response_content,
                generated.finish_reason.as_deref(),
                llm,
                "生成章节大纲",
            )
            .await?;
            results.push(outline);
            report_progress(
                progress,
                current,
                count,
                &format!("已生成第 {current}/{count} 个章节大纲（第 {chapter_number} 章）"),
            )?;
        }

        Ok(results)
    }

    async fn record_chapter_outline_generation_run(
        &self,
        outline: &chapter_outline::Model,
        request: &ChatRequest,
        response_content: &str,
        finish_reason: Option<&str>,
        llm: &dyn LlmProvider,
        title: &str,
    ) -> Result<(), AppError> {
        record_completed_generation_run(
            self.db.as_ref(),
            RecordCompletedGenerationRunInput {
                project_id: outline.project_id,
                workflow: "chapter_outline_generation".to_string(),
                step_key: format!("chapter_{}", outline.chapter_number),
                title: title.to_string(),
                model_kind: "llm".to_string(),
                provider: None,
                model: Some(llm.model_name().to_string()),
                prompt_template_names_json: Some(json!(["outline"])),
                prompt_template_hashes_json: None,
                rendered_messages_json: Some(chat_request_messages_json(request)?),
                rendered_prompts_json: None,
                input_variables_json: Some(json!({
                    "chapter_number": outline.chapter_number,
                    "part_id": outline.part_id,
                })),
                source_refs_json: Some(json!({
                    "source_node": if outline.part_id.is_some() {
                        "part_outlines"
                    } else {
                        "blueprint"
                    },
                    "source_kind": if outline.part_id.is_some() {
                        "part_outline"
                    } else {
                        "novel_blueprint"
                    },
                    "part_id": outline.part_id,
                })),
                media_refs_json: None,
                request_json: Some(context_aware_chat_request_trace_json(request)?),
                response_summary_json: Some(text_response_summary_json(
                    response_content,
                    finish_reason,
                    json!({
                        "chapter_number": outline.chapter_number,
                        "title": outline.title,
                    }),
                )),
                targets: vec![GenerationRunTraceTarget {
                    target_kind: "chapter_outline".to_string(),
                    target_table: "chapter_outline".to_string(),
                    target_id: outline.id,
                }],
            },
        )
        .await?;
        Ok(())
    }

    async fn existing_chapter_outline_for_number(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<Option<chapter_outline::Model>, AppError> {
        let outlines = chapter_outline::Entity::find()
            .filter(chapter_outline::Column::ProjectId.eq(project_id))
            .filter(chapter_outline::Column::ChapterNumber.eq(chapter_number))
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        if outlines.len() > 1 {
            let ids = outlines
                .iter()
                .map(|outline| outline.id.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(AppError::InvalidArgument(format!(
                "第 {chapter_number} 章已有重复章纲记录：{ids}，请先清理后再生成"
            )));
        }
        Ok(outlines.into_iter().next())
    }

    async fn ensure_no_other_outline_for_chapter(
        &self,
        project_id: i64,
        chapter_number: i32,
        outline_id: i64,
    ) -> Result<(), AppError> {
        let conflicts = chapter_outline::Entity::find()
            .filter(chapter_outline::Column::ProjectId.eq(project_id))
            .filter(chapter_outline::Column::ChapterNumber.eq(chapter_number))
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .into_iter()
            .filter(|outline| outline.id != outline_id)
            .map(|outline| outline.id)
            .collect::<Vec<_>>();
        if conflicts.is_empty() {
            return Ok(());
        }

        Err(AppError::InvalidArgument(format!(
            "第 {chapter_number} 章存在其他章纲记录：{}",
            conflicts
                .iter()
                .map(i64::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        )))
    }
}

fn build_chapter_outline_request(
    input: ChapterOutlineRequestInput<'_>,
) -> Result<ChatRequest, AppError> {
    let ChapterOutlineRequestInput {
        project_id,
        context,
        focus_part,
        start_chapter,
        count,
        end_chapter,
        prompt_engine,
        variables,
    } = input;
    let prompt_content = prompt_engine.render("outline", variables, Some(project_id))?;
    let user_content = chapter_outline_generation_manifest_user_message(
        focus_part,
        start_chapter,
        count,
        end_chapter,
    )?;
    build_context_aware_chapter_outline_request(ContextAwareChapterOutlineRequestInput {
        project_id,
        context,
        prompt_content,
        user_content,
        count,
        start_chapter,
        end_chapter,
        temperature: Some(0.7),
        workflow: "chapter_outline_generation",
        step_key: format!("chapter_{start_chapter}"),
    })
}

pub(super) fn build_context_aware_chapter_outline_request(
    input: ContextAwareChapterOutlineRequestInput<'_>,
) -> Result<ChatRequest, AppError> {
    let ContextAwareChapterOutlineRequestInput {
        project_id,
        context,
        prompt_content,
        user_content,
        count,
        start_chapter,
        end_chapter,
        temperature,
        workflow,
        step_key,
    } = input;
    let dynamic_user_content_sha256 = source_content_sha256(&user_content);
    let manifest = build_chapter_outline_context_manifest(context, user_content)?;
    Ok(context_aware_system_user_request(
        prompt_content,
        manifest,
        temperature,
        Some(chapter_outline_generation_max_tokens(count)),
        Some(llm_token_profiles::OUTLINE.into()),
    )?
    .with_json_response_format()
    .with_trace_context(LlmTraceContext {
        project_id: Some(project_id),
        root_project_id: None,
        workflow: workflow.to_string(),
        step_key: step_key.clone(),
        generation_run_id: None,
        workflow_run_id: None,
        call_key: Some(step_key),
        input_fingerprint_sha256: Some(source_content_sha256(&serde_json::to_string(&json!({
            "blueprint_context": context.blueprint_payload,
            "part_outlines": context.parts.iter().map(part_context_json).collect::<Vec<_>>(),
            "range": {
                "start_chapter": start_chapter,
                "count": count,
                "end_chapter": end_chapter,
            },
            "dynamic_user_content_sha256": dynamic_user_content_sha256,
        }))?)),
        context_plan_json: None,
        context_pack_refs_json: None,
    }))
}

fn chapter_outline_call_fingerprint_json(
    call_key: &str,
    chapter_number: i32,
    request: &ChatRequest,
) -> Result<Value, AppError> {
    Ok(json!({
        "schema_version": 1,
        "workflow": "chapter_outline_generation",
        "call_key": call_key,
        "prompt_name": "outline",
        "messages": request.messages,
        "context_manifest": request.context_manifest_json,
        "max_tokens": request.max_tokens,
        "max_tokens_profile": request.max_tokens_profile,
        "response_format": request.response_format,
        "target": {
            "chapter_number": chapter_number,
            "count": 1,
        },
    }))
}

fn restored_chapter_outline_checkpoint(
    checkpoint_json: Value,
    request: ChatRequest,
    chapter_number: i32,
) -> Result<GeneratedChapterOutline, AppError> {
    let parsed_output = checkpoint_json
        .get("parsed_output")
        .cloned()
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_checkpoint_parsed_output".to_string(),
            operation: format!("读取第 {chapter_number} 章章纲调用级断点"),
        })?;
    let response_content = checkpoint_json
        .get("response_content")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_checkpoint_response_content".to_string(),
            operation: format!("读取第 {chapter_number} 章章纲调用级断点"),
        })?
        .to_string();
    let finish_reason = checkpoint_json
        .get("finish_reason")
        .and_then(Value::as_str)
        .map(str::to_string);
    let mut parsed = parse_chapter_outline_batch(&response_content, chapter_number, 1)?;
    let parsed = parsed.pop().ok_or_else(|| {
        AppError::InvalidArgument(format!("第 {chapter_number} 章章纲断点结果为空"))
    })?;
    if parsed != parsed_output {
        return Err(AppError::InvalidState {
            current: "checkpoint_parsed_output_mismatch".to_string(),
            operation: format!("校验第 {chapter_number} 章章纲调用级断点"),
        });
    }
    Ok(GeneratedChapterOutline {
        chapter_json: parsed,
        request,
        response_content,
        finish_reason,
    })
}

fn build_chapter_outline_context_manifest(
    context: &ChapterOutlineGenerationContext,
    dynamic_user_content: String,
) -> Result<LlmContextManifest, AppError> {
    let mut ledger = LlmContextLedger::new();
    let blueprint_hash = source_content_sha256(&serde_json::to_string(&context.blueprint_payload)?);
    ledger.add_pack(crate::llm_context::LlmContextLedgerPackInput {
        scope_kind: ("project").into(),
        pack_kind: ("outline_blueprint_context").into(),
        source_refs_json: Some(json!({
            "source": "novel_blueprint",
            "content_sha256": blueprint_hash,
        })),
        config_identity_json: json!({
            "ledger_version": "outline_chapter_context_v1",
            "kind": "blueprint_context",
        }),
        required: true,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FULL_RAW,
                json!({
                    "kind": "blueprint_context",
                    "blueprint": context.blueprint_payload,
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FOCUSED_RAW,
                json!({
                    "kind": "blueprint_context",
                    "blueprint": context.blueprint_payload,
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_MINIMAL,
                json!({
                    "kind": "blueprint_context",
                    "blueprint": minimal_chapter_blueprint_context(&context.blueprint_payload),
                }),
            ),
        ],
    })?;

    let part_context = context
        .parts
        .iter()
        .map(part_context_json)
        .collect::<Vec<_>>();
    let part_hash = source_content_sha256(&serde_json::to_string(&part_context)?);
    ledger.add_pack(crate::llm_context::LlmContextLedgerPackInput {
        scope_kind: ("project").into(),
        pack_kind: ("outline_part_outlines").into(),
        source_refs_json: Some(json!({
            "source": "part_outline",
            "content_sha256": part_hash,
            "part_numbers": context.parts.iter().map(|part| part.part_number).collect::<Vec<_>>(),
        })),
        config_identity_json: json!({
            "ledger_version": "outline_chapter_context_v1",
            "kind": "part_outlines",
        }),
        required: true,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FULL_RAW,
                json!({
                    "kind": "part_outlines",
                    "parts": part_context,
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FOCUSED_RAW,
                json!({
                    "kind": "part_outlines",
                    "parts": context.parts.iter().map(focused_chapter_part_context).collect::<Vec<_>>(),
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_MINIMAL,
                json!({
                    "kind": "part_outlines",
                    "parts": context.parts.iter().map(minimal_chapter_part_context).collect::<Vec<_>>(),
                }),
            ),
        ],
    })?;

    Ok(ledger.into_manifest(
        dynamic_user_content,
        Some("章节大纲必须返回严格 JSON 对象；根对象必须且只能包含 chapters。".to_string()),
    ))
}

fn minimal_chapter_blueprint_context(blueprint_context: &Value) -> Value {
    let Some(object) = blueprint_context.as_object() else {
        return blueprint_context.clone();
    };
    let mut minimal = serde_json::Map::new();
    for key in [
        "title",
        "genre",
        "style",
        "tone",
        "one_sentence_summary",
        "full_synopsis",
        "chapters_per_part",
        "blueprint_chapters_per_part",
    ] {
        if let Some(value) = object.get(key) {
            minimal.insert(key.to_string(), value.clone());
        }
    }
    Value::Object(minimal)
}

fn focused_chapter_part_context(part: &part_outline::Model) -> Value {
    json!({
        "part_number": part.part_number,
        "title": part.title,
        "start_chapter": part.start_chapter,
        "end_chapter": part.end_chapter,
        "summary": part.summary,
        "theme": part.theme,
        "key_events": part.key_events,
    })
}

fn minimal_chapter_part_context(part: &part_outline::Model) -> Value {
    json!({
        "part_number": part.part_number,
        "title": part.title,
        "start_chapter": part.start_chapter,
        "end_chapter": part.end_chapter,
        "summary": part.summary,
    })
}

#[cfg(test)]
mod chapter_outline_request_tests {
    use super::*;
    use a4n_engine::llm::provider::MessageRole;
    use a4n_engine::prompt::tera_engine::TeraPromptEngine;
    use std::path::Path;

    fn prompts_dir() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../prompts")
    }

    fn prompt_engine() -> TeraPromptEngine {
        TeraPromptEngine::from_directory(&prompts_dir()).expect("prompts should load")
    }

    fn sample_part() -> part_outline::Model {
        let now = Utc::now().naive_utc();
        part_outline::Model {
            id: 8,
            project_id: 42,
            part_number: 1,
            title: "星火初燃".to_string(),
            start_chapter: 1,
            end_chapter: 20,
            summary: Some("分部完整摘要不应进入章节动态 user prompt".to_string()),
            theme: Some("代价".to_string()),
            key_events: Some(json!(["分部关键事件"])),
            character_focus: Some(json!({"林烬": "承担责任"})),
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn chapter_outline_request_moves_blueprint_and_parts_into_manifest() {
        let engine = prompt_engine();
        let part = sample_part();
        let context = ChapterOutlineGenerationContext {
            blueprint_payload: json!({
                "title": "星潮档案",
                "genre": "科幻",
                "one_sentence_summary": "调查星潮真相",
            }),
            parts: vec![part.clone()],
        };
        let variables = HashMap::new();
        let request = build_chapter_outline_request(ChapterOutlineRequestInput {
            project_id: 42,
            context: &context,
            focus_part: Some(&part),
            start_chapter: 3,
            count: 1,
            end_chapter: 3,
            prompt_engine: &engine,
            variables: &variables,
        })
        .expect("chapter outline request should build");

        let manifest = request
            .context_manifest_json
            .as_ref()
            .expect("context manifest should be attached");
        let pack_kinds = manifest["packs"]
            .as_array()
            .expect("packs should be array")
            .iter()
            .filter_map(|pack| pack["pack_kind"].as_str())
            .collect::<Vec<_>>();
        assert!(pack_kinds.contains(&"outline_blueprint_context"));
        assert!(pack_kinds.contains(&"outline_part_outlines"));

        let user_message = request
            .messages
            .iter()
            .find(|message| matches!(message.role, MessageRole::User))
            .expect("user message should exist");
        assert!(user_message.content.contains("context_refs"));
        assert!(user_message.content.contains("outline_blueprint_context"));
        assert!(user_message.content.contains("outline_part_outlines"));
        assert!(!user_message.content.contains("星潮档案"));
        assert!(
            !user_message
                .content
                .contains("分部完整摘要不应进入章节动态 user prompt")
        );

        let trace_context = request
            .trace_context
            .as_ref()
            .expect("trace context should be attached");
        assert_eq!(trace_context.workflow, "chapter_outline_generation");
        assert_eq!(trace_context.step_key, "chapter_3");
        assert_eq!(trace_context.call_key.as_deref(), Some("chapter_3"));
    }
}
