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
    context_aware_chat_request_trace_json, delete_generation_run_targets_for,
    record_completed_generation_run, text_response_summary_json,
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

use super::context::{
    PartOutlineGenerationContext, load_part_generation_context, part_context_json,
};
use super::part_contract::{
    PART_GENERATION_OUTPUT_CONTRACT, PartOutlineDraft, chapter_outline_ids_in_part_ranges,
    delete_chapter_outlines_in_part_range, ensure_continue_part_request,
    ensure_no_part_outline_conflict, parse_part_outline_single_response,
    part_generation_manifest_user_message, part_outline_active_model, part_outline_model_from_json,
    sync_chapter_outline_part_assignments, sync_part_outlines, update_part_outline_active_model,
    validate_existing_parts_for_latest_delete, validate_existing_parts_for_plan,
    validate_part_outline_request,
};
use super::{
    OutlineRequestPreviewStep, OutlineService, PartOutlineContinuationInput,
    PartOutlineGenerationInput, PartOutlineRegenerationInput, blocked_outline_preview_step,
    ensure_response_not_truncated, ready_outline_preview_step, report_progress,
};

#[derive(Debug, Clone)]
struct GeneratedPartOutline {
    data: Value,
    request: ChatRequest,
    response_content: String,
    finish_reason: Option<String>,
}

struct RegeneratePartOutlineModelInput<'a> {
    project_id: i64,
    target: part_outline::Model,
    context: &'a PartOutlineGenerationContext,
    rebind_chapter_outlines: bool,
    llm: &'a dyn LlmProvider,
    prompt_engine: &'a dyn PromptEngine,
    variables: &'a HashMap<String, Value>,
    workflow_run_id: Option<i64>,
}

struct PartOutlineJsonInput<'a> {
    project_id: i64,
    part_num: i32,
    start_ch: i32,
    end_ch: i32,
    chapters_per_part: i32,
    requested_part_count: i32,
    blueprint_context: &'a Value,
    previous_parts: &'a [part_outline::Model],
    llm: &'a dyn LlmProvider,
    prompt_engine: &'a dyn PromptEngine,
    variables: &'a HashMap<String, Value>,
    workflow_run_id: Option<i64>,
}

struct PartOutlineRequestInput<'a> {
    project_id: i64,
    part_num: i32,
    start_ch: i32,
    end_ch: i32,
    chapters_per_part: i32,
    requested_part_count: i32,
    blueprint_context: &'a Value,
    previous_parts: &'a [part_outline::Model],
    prompt_engine: &'a dyn PromptEngine,
    variables: &'a HashMap<String, Value>,
}

impl OutlineService {
    /// 列出项目所有分部大纲
    pub async fn list_parts(&self, project_id: i64) -> Result<Vec<part_outline::Model>, AppError> {
        part_outline::Entity::find()
            .filter(part_outline::Column::ProjectId.eq(project_id))
            .order_by_asc(part_outline::Column::PartNumber)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    /// 获取分部大纲详情
    pub async fn get_part(&self, part_id: i64) -> Result<part_outline::Model, AppError> {
        part_outline::Entity::find_by_id(part_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("分部大纲不存在: {part_id}")))
    }

    /// 生成分部大纲
    pub async fn generate_parts(
        &self,
        input: PartOutlineGenerationInput<'_, '_>,
    ) -> Result<Vec<part_outline::Model>, AppError> {
        let PartOutlineGenerationInput {
            project_id,
            part_count,
            chapters_per_part,
            llm,
            prompt_engine,
            variables,
            progress,
            workflow_run_id,
        } = input;
        let context = load_part_generation_context(
            self.db.as_ref(),
            project_id,
            part_count,
            chapters_per_part,
        )
        .await?;
        let batch = plan_next_parts(
            &context.parts,
            context.requested_part_count,
            context.chapters_per_part,
        )?;
        let num_parts = batch.len() as i32;
        validate_existing_parts_for_plan(&context.parts)?;
        let mut results = context.parts.clone();
        report_progress(
            progress,
            0,
            num_parts,
            &format!("准备生成 {num_parts} 个分部大纲"),
        )?;

        for (index, (part_num, start_ch, end_ch)) in batch.into_iter().enumerate() {
            let progress_index = index as i32;
            report_progress(
                progress,
                progress_index,
                num_parts,
                &format!("正在生成第 {part_num}/{num_parts} 部"),
            )?;
            let generated = self
                .generate_part_outline_json(PartOutlineJsonInput {
                    project_id,
                    part_num,
                    start_ch,
                    end_ch,
                    chapters_per_part: context.chapters_per_part,
                    requested_part_count: context.requested_part_count,
                    blueprint_context: &context.blueprint_payload,
                    previous_parts: &results,
                    llm,
                    prompt_engine,
                    variables,
                    workflow_run_id,
                })
                .await?;
            let part = part_outline_active_model(
                project_id,
                part_num,
                start_ch,
                end_ch,
                &generated.data,
                Utc::now().naive_utc(),
            )?
            .insert(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
            self.record_part_outline_generation_run(&part, &generated, llm, "生成分部大纲")
                .await?;
            results.push(part);
            report_progress(
                progress,
                progress_index + 1,
                num_parts,
                &format!("已生成第 {part_num}/{num_parts} 部"),
            )?;
        }

        Ok(results)
    }

    pub async fn preview_generate_parts_requests(
        &self,
        project_id: i64,
        part_count: Option<i32>,
        chapters_per_part: Option<i32>,
        regenerate: bool,
        prompt_engine: &dyn PromptEngine,
        variables: &HashMap<String, serde_json::Value>,
    ) -> Result<Vec<OutlineRequestPreviewStep>, AppError> {
        let context = load_part_generation_context(
            self.db.as_ref(),
            project_id,
            part_count,
            chapters_per_part,
        )
        .await?;
        validate_existing_parts_for_plan(&context.parts)?;
        let batch = if regenerate {
            plan_replacement_parts(context.requested_part_count, context.chapters_per_part)?
        } else {
            plan_next_parts(
                &context.parts,
                context.requested_part_count,
                context.chapters_per_part,
            )?
        };
        let num_parts = batch.len() as i32;

        let mut steps = Vec::new();
        let previous_parts = if regenerate {
            Vec::new()
        } else {
            context.parts.clone()
        };
        let mut first_pending_seen = false;

        for (part_num, start_ch, end_ch) in batch {
            let title = format!("第 {part_num}/{num_parts} 部（第 {start_ch}-{end_ch} 章）");

            if first_pending_seen {
                steps.push(blocked_outline_preview_step(
                    format!("part_{part_num}"),
                    title,
                    "part_outline_single",
                    "该分部请求依赖前一部分部模型输出，需先完成前序阶段。",
                ));
                continue;
            }

            let request = build_part_outline_request(PartOutlineRequestInput {
                project_id,
                part_num,
                start_ch,
                end_ch,
                chapters_per_part: context.chapters_per_part,
                requested_part_count: context.requested_part_count,
                blueprint_context: &context.blueprint_payload,
                previous_parts: &previous_parts,
                prompt_engine,
                variables,
            })?;
            steps.push(ready_outline_preview_step(
                format!("part_{part_num}"),
                title,
                "part_outline_single",
                request,
            ));
            first_pending_seen = true;
        }

        Ok(steps)
    }

    /// 重新生成所有分部。先完整生成新分部，全部成功后再替换旧分部和旧章纲。
    pub async fn regenerate_parts(
        &self,
        input: PartOutlineRegenerationInput<'_, '_>,
    ) -> Result<Vec<part_outline::Model>, AppError> {
        let PartOutlineRegenerationInput {
            project_id,
            part_count,
            chapters_per_part,
            llm,
            prompt_engine,
            variables,
            progress,
            workflow_run_id,
        } = input;
        let context = load_part_generation_context(
            self.db.as_ref(),
            project_id,
            part_count,
            chapters_per_part,
        )
        .await?;
        validate_existing_parts_for_plan(&context.parts)?;
        let batch =
            plan_replacement_parts(context.requested_part_count, context.chapters_per_part)?;
        let num_parts = batch.len() as i32;
        let now = Utc::now().naive_utc();
        let mut generated_jsons = Vec::new();
        let mut generated_context_parts = Vec::new();

        report_progress(
            progress,
            0,
            num_parts,
            &format!("准备重新生成 {num_parts} 个分部大纲"),
        )?;
        for (index, (part_num, start_ch, end_ch)) in batch.into_iter().enumerate() {
            let progress_index = index as i32;
            report_progress(
                progress,
                progress_index,
                num_parts,
                &format!("正在重新生成第 {part_num}/{num_parts} 部"),
            )?;
            let generated = self
                .generate_part_outline_json(PartOutlineJsonInput {
                    project_id,
                    part_num,
                    start_ch,
                    end_ch,
                    chapters_per_part: context.chapters_per_part,
                    requested_part_count: context.requested_part_count,
                    blueprint_context: &context.blueprint_payload,
                    previous_parts: &generated_context_parts,
                    llm,
                    prompt_engine,
                    variables,
                    workflow_run_id,
                })
                .await?;
            generated_context_parts.push(part_outline_model_from_json(
                0,
                project_id,
                part_num,
                start_ch,
                end_ch,
                &generated.data,
                now,
            )?);
            generated_jsons.push((part_num, start_ch, end_ch, generated));
            report_progress(
                progress,
                progress_index + 1,
                num_parts,
                &format!("已重新生成第 {part_num}/{num_parts} 部"),
            )?;
        }

        report_progress(progress, num_parts, num_parts, "正在增量保存分部并重绑章纲")?;
        let txn = self.db.begin().await.map_err(AppError::Database)?;
        let drafts = generated_jsons
            .iter()
            .map(
                |(part_number, start_chapter, end_chapter, generated)| PartOutlineDraft {
                    part_number: *part_number,
                    start_chapter: *start_chapter,
                    end_chapter: *end_chapter,
                    data: generated.data.clone(),
                },
            )
            .collect();
        let results = sync_part_outlines(&txn, project_id, drafts, now).await?;
        sync_chapter_outline_part_assignments(&txn, project_id, &results, now).await?;
        txn.commit().await.map_err(AppError::Database)?;
        for part in &results {
            if let Some((_, _, _, generated)) = generated_jsons
                .iter()
                .find(|(part_number, _, _, _)| *part_number == part.part_number)
            {
                self.record_part_outline_generation_run(part, generated, llm, "重新生成分部大纲")
                    .await?;
            }
        }
        Ok(results)
    }

    /// 续作下一个分部大纲。
    pub async fn continue_part_outline(
        &self,
        input: PartOutlineContinuationInput<'_>,
    ) -> Result<part_outline::Model, AppError> {
        let PartOutlineContinuationInput {
            project_id,
            part_number,
            start_chapter,
            end_chapter,
            llm,
            prompt_engine,
            variables,
            workflow_run_id,
        } = input;
        let context =
            load_part_generation_context(self.db.as_ref(), project_id, None, None).await?;
        validate_part_outline_request(part_number, start_chapter, end_chapter)?;
        ensure_continue_part_request(&context.parts, part_number, start_chapter)?;
        ensure_no_part_outline_conflict(
            &context.parts,
            part_number,
            start_chapter,
            end_chapter,
            None,
        )?;

        let generated = self
            .generate_part_outline_json(PartOutlineJsonInput {
                project_id,
                part_num: part_number,
                start_ch: start_chapter,
                end_ch: end_chapter,
                chapters_per_part: context.chapters_per_part,
                requested_part_count: 1,
                blueprint_context: &context.blueprint_payload,
                previous_parts: &context.parts,
                llm,
                prompt_engine,
                variables,
                workflow_run_id,
            })
            .await?;
        let now = Utc::now().naive_utc();
        let part = part_outline_active_model(
            project_id,
            part_number,
            start_chapter,
            end_chapter,
            &generated.data,
            now,
        )?
        .insert(self.db.as_ref())
        .await
        .map_err(AppError::Database)?;
        self.record_part_outline_generation_run(&part, &generated, llm, "续写分部大纲")
            .await?;
        Ok(part)
    }

    pub async fn preview_continue_part_outline_request(
        &self,
        project_id: i64,
        part_number: i32,
        start_chapter: i32,
        end_chapter: i32,
        prompt_engine: &dyn PromptEngine,
        variables: &HashMap<String, serde_json::Value>,
    ) -> Result<ChatRequest, AppError> {
        let context =
            load_part_generation_context(self.db.as_ref(), project_id, None, None).await?;
        validate_part_outline_request(part_number, start_chapter, end_chapter)?;
        ensure_continue_part_request(&context.parts, part_number, start_chapter)?;
        ensure_no_part_outline_conflict(
            &context.parts,
            part_number,
            start_chapter,
            end_chapter,
            None,
        )?;
        build_part_outline_request(PartOutlineRequestInput {
            project_id,
            part_num: part_number,
            start_ch: start_chapter,
            end_ch: end_chapter,
            chapters_per_part: context.chapters_per_part,
            requested_part_count: 1,
            blueprint_context: &context.blueprint_payload,
            previous_parts: &context.parts,
            prompt_engine,
            variables,
        })
    }

    /// 重新生成最后一个分部。解析通过后才更新分部并删除其旧章节大纲。
    pub async fn regenerate_last_part(
        &self,
        project_id: i64,
        llm: &dyn LlmProvider,
        prompt_engine: &dyn PromptEngine,
        variables: &HashMap<String, serde_json::Value>,
        workflow_run_id: Option<i64>,
    ) -> Result<part_outline::Model, AppError> {
        let context =
            load_part_generation_context(self.db.as_ref(), project_id, None, None).await?;
        let target = context
            .parts
            .last()
            .cloned()
            .ok_or_else(|| AppError::InvalidArgument("没有分部可重新生成".to_string()))?;
        self.regenerate_part_outline_model(RegeneratePartOutlineModelInput {
            project_id,
            target,
            context: &context,
            rebind_chapter_outlines: true,
            llm,
            prompt_engine,
            variables,
            workflow_run_id,
        })
        .await
    }

    /// 重新生成指定分部。
    pub async fn regenerate_part_outline(
        &self,
        project_id: i64,
        part_id: i64,
        llm: &dyn LlmProvider,
        prompt_engine: &dyn PromptEngine,
        variables: &HashMap<String, serde_json::Value>,
        workflow_run_id: Option<i64>,
    ) -> Result<part_outline::Model, AppError> {
        let target = self.get_part(part_id).await?;
        if target.project_id != project_id {
            return Err(AppError::InvalidArgument(format!(
                "分部大纲 {part_id} 不属于项目 {project_id}"
            )));
        }

        let context =
            load_part_generation_context(self.db.as_ref(), project_id, None, None).await?;
        self.regenerate_part_outline_model(RegeneratePartOutlineModelInput {
            project_id,
            target,
            context: &context,
            rebind_chapter_outlines: false,
            llm,
            prompt_engine,
            variables,
            workflow_run_id,
        })
        .await
    }

    pub async fn preview_regenerate_last_part_request(
        &self,
        project_id: i64,
        prompt_engine: &dyn PromptEngine,
        variables: &HashMap<String, serde_json::Value>,
    ) -> Result<ChatRequest, AppError> {
        let context =
            load_part_generation_context(self.db.as_ref(), project_id, None, None).await?;
        let target = context
            .parts
            .last()
            .cloned()
            .ok_or_else(|| AppError::InvalidArgument("没有分部可重新生成".to_string()))?;
        self.preview_regenerate_part_request_from_target(
            project_id,
            target,
            &context,
            prompt_engine,
            variables,
        )
    }

    pub async fn preview_regenerate_part_outline_request(
        &self,
        project_id: i64,
        part_id: i64,
        prompt_engine: &dyn PromptEngine,
        variables: &HashMap<String, serde_json::Value>,
    ) -> Result<ChatRequest, AppError> {
        let target = self.get_part(part_id).await?;
        if target.project_id != project_id {
            return Err(AppError::InvalidArgument(format!(
                "分部大纲 {part_id} 不属于项目 {project_id}"
            )));
        }
        let context =
            load_part_generation_context(self.db.as_ref(), project_id, None, None).await?;
        self.preview_regenerate_part_request_from_target(
            project_id,
            target,
            &context,
            prompt_engine,
            variables,
        )
    }

    async fn regenerate_part_outline_model(
        &self,
        input: RegeneratePartOutlineModelInput<'_>,
    ) -> Result<part_outline::Model, AppError> {
        let RegeneratePartOutlineModelInput {
            project_id,
            target,
            context,
            rebind_chapter_outlines,
            llm,
            prompt_engine,
            variables,
            workflow_run_id,
        } = input;
        validate_part_outline_request(
            target.part_number,
            target.start_chapter,
            target.end_chapter,
        )?;
        ensure_no_part_outline_conflict(
            &context.parts,
            target.part_number,
            target.start_chapter,
            target.end_chapter,
            Some(target.id),
        )?;
        let previous_parts = context
            .parts
            .iter()
            .filter(|part| part.part_number < target.part_number)
            .cloned()
            .collect::<Vec<_>>();
        let generated = self
            .generate_part_outline_json(PartOutlineJsonInput {
                project_id,
                part_num: target.part_number,
                start_ch: target.start_chapter,
                end_ch: target.end_chapter,
                chapters_per_part: context.chapters_per_part,
                requested_part_count: 1,
                blueprint_context: &context.blueprint_payload,
                previous_parts: &previous_parts,
                llm,
                prompt_engine,
                variables,
                workflow_run_id,
            })
            .await?;
        let now = Utc::now().naive_utc();
        let active = update_part_outline_active_model(
            target.clone(),
            target.part_number,
            target.start_chapter,
            target.end_chapter,
            &generated.data,
            now,
        )?;
        if !rebind_chapter_outlines {
            let updated = active
                .update(self.db.as_ref())
                .await
                .map_err(AppError::Database)?;
            self.record_part_outline_generation_run(&updated, &generated, llm, "重新生成分部大纲")
                .await?;
            return Ok(updated);
        }

        let txn = self.db.begin().await.map_err(AppError::Database)?;
        let updated = active.update(&txn).await.map_err(AppError::Database)?;
        delete_chapter_outlines_in_part_range(&txn, project_id, &updated).await?;
        let parts = part_outline::Entity::find()
            .filter(part_outline::Column::ProjectId.eq(project_id))
            .order_by_asc(part_outline::Column::PartNumber)
            .all(&txn)
            .await
            .map_err(AppError::Database)?;
        sync_chapter_outline_part_assignments(&txn, project_id, &parts, now).await?;
        txn.commit().await.map_err(AppError::Database)?;
        self.record_part_outline_generation_run(&updated, &generated, llm, "重新生成分部大纲")
            .await?;
        Ok(updated)
    }

    fn preview_regenerate_part_request_from_target(
        &self,
        project_id: i64,
        target: part_outline::Model,
        context: &PartOutlineGenerationContext,
        prompt_engine: &dyn PromptEngine,
        variables: &HashMap<String, serde_json::Value>,
    ) -> Result<ChatRequest, AppError> {
        validate_part_outline_request(
            target.part_number,
            target.start_chapter,
            target.end_chapter,
        )?;
        ensure_no_part_outline_conflict(
            &context.parts,
            target.part_number,
            target.start_chapter,
            target.end_chapter,
            Some(target.id),
        )?;
        let previous_parts = context
            .parts
            .iter()
            .filter(|part| part.part_number < target.part_number)
            .cloned()
            .collect::<Vec<_>>();
        build_part_outline_request(PartOutlineRequestInput {
            project_id,
            part_num: target.part_number,
            start_ch: target.start_chapter,
            end_ch: target.end_chapter,
            chapters_per_part: context.chapters_per_part,
            requested_part_count: 1,
            blueprint_context: &context.blueprint_payload,
            previous_parts: &previous_parts,
            prompt_engine,
            variables,
        })
    }

    async fn generate_part_outline_json(
        &self,
        input: PartOutlineJsonInput<'_>,
    ) -> Result<GeneratedPartOutline, AppError> {
        let PartOutlineJsonInput {
            project_id,
            part_num,
            start_ch,
            end_ch,
            chapters_per_part,
            requested_part_count,
            blueprint_context,
            previous_parts,
            llm,
            prompt_engine,
            variables,
            workflow_run_id,
        } = input;
        let mut request = build_part_outline_request(PartOutlineRequestInput {
            project_id,
            part_num,
            start_ch,
            end_ch,
            chapters_per_part,
            requested_part_count,
            blueprint_context,
            previous_parts,
            prompt_engine,
            variables,
        })?;
        let call_key = format!("part_{part_num}");
        let workflow_run_id = required_llm_call_workflow_run_id(workflow_run_id, &call_key)?;
        let fingerprint_json = part_outline_call_fingerprint_json(
            &call_key,
            part_num,
            start_ch,
            end_ch,
            chapters_per_part,
            requested_part_count,
            &request,
        )?;
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
            return restored_part_outline_checkpoint(
                checkpoint_json,
                request,
                part_num,
                start_ch,
                end_ch,
            );
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
            &format!("第 {part_num} 部分部大纲"),
        )?;
        let data = parse_part_outline_single_response(&content, part_num, start_ch, end_ch)?;
        let generated = GeneratedPartOutline {
            data,
            request,
            response_content: content,
            finish_reason,
        };
        save_completed_llm_call_checkpoint(
            self.db.clone(),
            CompletedLlmCallCheckpointInput {
                workflow_run_id,
                call_key,
                call_order: part_num,
                input_fingerprint_json: fingerprint_json,
                checkpoint_json: json!({
                    "output": generated.data.clone(),
                    "parsed_output": generated.data.clone(),
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

    async fn record_part_outline_generation_run(
        &self,
        part: &part_outline::Model,
        generated: &GeneratedPartOutline,
        llm: &dyn LlmProvider,
        title: &str,
    ) -> Result<(), AppError> {
        record_completed_generation_run(
            self.db.as_ref(),
            RecordCompletedGenerationRunInput {
                project_id: part.project_id,
                workflow: "part_outline_generation".to_string(),
                step_key: format!("part_{}", part.part_number),
                title: title.to_string(),
                model_kind: "llm".to_string(),
                provider: None,
                model: Some(llm.model_name().to_string()),
                prompt_template_names_json: Some(json!(["part_outline_single"])),
                prompt_template_hashes_json: None,
                rendered_messages_json: Some(chat_request_messages_json(&generated.request)?),
                rendered_prompts_json: None,
                input_variables_json: Some(json!({
                    "part_number": part.part_number,
                    "start_chapter": part.start_chapter,
                    "end_chapter": part.end_chapter,
                })),
                source_refs_json: Some(json!({
                    "source_node": "blueprint",
                    "source_kind": "novel_blueprint",
                    "previous_part_context": true,
                })),
                media_refs_json: None,
                request_json: Some(context_aware_chat_request_trace_json(&generated.request)?),
                response_summary_json: Some(text_response_summary_json(
                    &generated.response_content,
                    generated.finish_reason.as_deref(),
                    json!({
                        "part_number": part.part_number,
                        "title": part.title,
                    }),
                )),
                targets: vec![GenerationRunTraceTarget {
                    target_kind: "part_outline".to_string(),
                    target_table: "part_outline".to_string(),
                    target_id: part.id,
                }],
            },
        )
        .await?;
        Ok(())
    }

    /// 删除最后 N 个分部
    pub async fn delete_latest_parts(&self, project_id: i64, count: u64) -> Result<(), AppError> {
        let parts = self.list_parts(project_id).await?;
        let part_ids = latest_part_ids_to_delete(&parts, count)?;
        let parts_to_delete = parts
            .iter()
            .filter(|part| part_ids.contains(&part.id))
            .cloned()
            .collect::<Vec<_>>();
        let txn = self.db.begin().await.map_err(AppError::Database)?;

        let chapter_outline_ids = chapter_outline::Entity::find()
            .filter(chapter_outline::Column::ProjectId.eq(project_id))
            .all(&txn)
            .await
            .map_err(AppError::Database)?
            .into_iter()
            .collect::<Vec<_>>();
        let chapter_outline_ids =
            chapter_outline_ids_in_part_ranges(&chapter_outline_ids, &parts_to_delete);
        // 级联删除对应章节大纲
        if !chapter_outline_ids.is_empty() {
            chapter_outline::Entity::delete_many()
                .filter(chapter_outline::Column::Id.is_in(chapter_outline_ids.iter().copied()))
                .exec(&txn)
                .await
                .map_err(AppError::Database)?;
            delete_generation_run_targets_for(&txn, "chapter_outline", &chapter_outline_ids)
                .await?;
        }

        part_outline::Entity::delete_many()
            .filter(part_outline::Column::Id.is_in(part_ids.iter().copied()))
            .exec(&txn)
            .await
            .map_err(AppError::Database)?;
        delete_generation_run_targets_for(&txn, "part_outline", &part_ids).await?;

        txn.commit().await.map_err(AppError::Database)?;
        Ok(())
    }
}

fn latest_part_ids_to_delete(
    parts: &[part_outline::Model],
    count: u64,
) -> Result<Vec<i64>, AppError> {
    if count == 0 {
        return Err(AppError::InvalidArgument(
            "删除分部数量必须大于 0".to_string(),
        ));
    }
    if count as usize > parts.len() {
        return Err(AppError::InvalidArgument(format!(
            "删除分部数量超过现有分部数量：count={count}, total={}",
            parts.len()
        )));
    }
    validate_existing_parts_for_latest_delete(parts)?;
    Ok(parts
        .iter()
        .rev()
        .take(count as usize)
        .map(|part| part.id)
        .collect())
}

fn plan_next_parts(
    existing_parts: &[part_outline::Model],
    part_count: i32,
    chapters_per_part: i32,
) -> Result<Vec<(i32, i32, i32)>, AppError> {
    if part_count <= 0 {
        return Err(AppError::InvalidArgument(
            "分部大纲 part_count 必须大于 0".to_string(),
        ));
    }
    if chapters_per_part <= 0 {
        return Err(AppError::InvalidArgument(
            "分部大纲 chapters_per_part 必须大于 0".to_string(),
        ));
    }

    let (next_part_number, next_start_chapter) = existing_parts
        .iter()
        .max_by_key(|part| part.part_number)
        .map(|part| (part.part_number + 1, part.end_chapter + 1))
        .unwrap_or((1, 1));
    Ok((0..part_count)
        .map(|offset| {
            let part_number = next_part_number + offset;
            let start_chapter = next_start_chapter + offset * chapters_per_part;
            let end_chapter = start_chapter + chapters_per_part - 1;
            (part_number, start_chapter, end_chapter)
        })
        .collect())
}

fn plan_replacement_parts(
    part_count: i32,
    chapters_per_part: i32,
) -> Result<Vec<(i32, i32, i32)>, AppError> {
    plan_next_parts(&[], part_count, chapters_per_part)
}

fn build_part_outline_request(input: PartOutlineRequestInput<'_>) -> Result<ChatRequest, AppError> {
    let PartOutlineRequestInput {
        project_id,
        part_num,
        start_ch,
        end_ch,
        chapters_per_part,
        requested_part_count,
        blueprint_context,
        previous_parts,
        prompt_engine,
        variables,
    } = input;
    let prompt_content =
        prompt_engine.render("part_outline_single", variables, Some(project_id))?;
    let user_content = part_generation_manifest_user_message(
        chapters_per_part,
        requested_part_count,
        part_num,
        start_ch,
        end_ch,
        previous_parts,
    )?;
    let manifest = build_part_outline_context_manifest(
        blueprint_context,
        previous_parts,
        user_content,
        part_num,
        start_ch,
        end_ch,
    )?;

    Ok(context_aware_system_user_request(
        prompt_content,
        manifest,
        Some(0.7),
        None,
        Some(llm_token_profiles::OUTLINE.into()),
    )?
    .with_json_response_format()
    .with_trace_context(LlmTraceContext {
        project_id: Some(project_id),
        root_project_id: None,
        workflow: "part_outline_generation".to_string(),
        step_key: format!("part_{part_num}"),
        generation_run_id: None,
        workflow_run_id: None,
        call_key: Some(format!("part_{part_num}")),
        input_fingerprint_sha256: Some(source_content_sha256(&serde_json::to_string(&json!({
            "blueprint_context": blueprint_context,
            "previous_parts": previous_parts.iter().map(part_context_json).collect::<Vec<_>>(),
            "generation_request": {
                "chapters_per_part": chapters_per_part,
                "requested_part_count": requested_part_count,
                "part_number": part_num,
                "start_chapter": start_ch,
                "end_chapter": end_ch,
            },
        }))?)),
        context_plan_json: None,
        context_pack_refs_json: None,
    }))
}

fn part_outline_call_fingerprint_json(
    call_key: &str,
    part_num: i32,
    start_ch: i32,
    end_ch: i32,
    chapters_per_part: i32,
    requested_part_count: i32,
    request: &ChatRequest,
) -> Result<Value, AppError> {
    Ok(json!({
        "schema_version": 1,
        "workflow": "part_outline_generation",
        "call_key": call_key,
        "prompt_name": "part_outline_single",
        "messages": request.messages,
        "context_manifest": request.context_manifest_json,
        "max_tokens": request.max_tokens,
        "max_tokens_profile": request.max_tokens_profile,
        "response_format": request.response_format,
        "target": {
            "part_number": part_num,
            "start_chapter": start_ch,
            "end_chapter": end_ch,
            "chapters_per_part": chapters_per_part,
            "requested_part_count": requested_part_count,
        },
    }))
}

fn restored_part_outline_checkpoint(
    checkpoint_json: Value,
    request: ChatRequest,
    part_num: i32,
    start_ch: i32,
    end_ch: i32,
) -> Result<GeneratedPartOutline, AppError> {
    let parsed_output = checkpoint_json
        .get("parsed_output")
        .cloned()
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_checkpoint_parsed_output".to_string(),
            operation: format!("读取第 {part_num} 部分部大纲调用级断点"),
        })?;
    let response_content = checkpoint_json
        .get("response_content")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_checkpoint_response_content".to_string(),
            operation: format!("读取第 {part_num} 部分部大纲调用级断点"),
        })?
        .to_string();
    let finish_reason = checkpoint_json
        .get("finish_reason")
        .and_then(Value::as_str)
        .map(str::to_string);
    let parsed = parse_part_outline_single_response(&response_content, part_num, start_ch, end_ch)?;
    if parsed != parsed_output {
        return Err(AppError::InvalidState {
            current: "checkpoint_parsed_output_mismatch".to_string(),
            operation: format!("校验第 {part_num} 部分部大纲调用级断点"),
        });
    }
    Ok(GeneratedPartOutline {
        data: parsed,
        request,
        response_content,
        finish_reason,
    })
}

fn build_part_outline_context_manifest(
    blueprint_context: &Value,
    previous_parts: &[part_outline::Model],
    dynamic_user_content: String,
    part_num: i32,
    start_ch: i32,
    end_ch: i32,
) -> Result<LlmContextManifest, AppError> {
    let mut ledger = LlmContextLedger::new();
    let blueprint_hash = source_content_sha256(&serde_json::to_string(blueprint_context)?);
    ledger.add_pack(crate::llm_context::LlmContextLedgerPackInput {
        scope_kind: ("project").into(),
        pack_kind: ("outline_blueprint_context").into(),
        source_refs_json: Some(json!({
            "source": "novel_blueprint",
            "content_sha256": blueprint_hash,
        })),
        config_identity_json: json!({
            "ledger_version": "outline_part_context_v1",
            "kind": "blueprint_context",
        }),
        required: true,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FULL_RAW,
                json!({
                    "kind": "blueprint_context",
                    "blueprint": blueprint_context,
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FOCUSED_RAW,
                json!({
                    "kind": "blueprint_context",
                    "blueprint": blueprint_context,
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_MINIMAL,
                json!({
                    "kind": "blueprint_context",
                    "blueprint": minimal_outline_blueprint_context(blueprint_context),
                }),
            ),
        ],
    })?;

    if !previous_parts.is_empty() {
        let previous_part_context = previous_parts
            .iter()
            .map(part_context_json)
            .collect::<Vec<_>>();
        let previous_hash = source_content_sha256(&serde_json::to_string(&previous_part_context)?);
        ledger.add_pack(crate::llm_context::LlmContextLedgerPackInput {
            scope_kind: ("project").into(),
            pack_kind: ("outline_previous_part_outlines").into(),
            source_refs_json: Some(json!({
                "source": "part_outline",
                "content_sha256": previous_hash,
                "part_numbers": previous_parts.iter().map(|part| part.part_number).collect::<Vec<_>>(),
            })),
            config_identity_json: json!({
                "ledger_version": "outline_part_context_v1",
                "kind": "previous_part_outlines",
            }),
            required: true,
            stable_prefix: true,
            variants: vec![
                LlmContextLedgerPackDraft::new(
                    COMPRESSION_FULL_RAW,
                    json!({
                        "kind": "previous_part_outlines",
                        "parts": previous_part_context,
                    }),
                ),
                LlmContextLedgerPackDraft::new(
                    COMPRESSION_FOCUSED_RAW,
                    json!({
                        "kind": "previous_part_outlines",
                        "parts": previous_parts
                            .iter()
                            .map(focused_part_context)
                            .collect::<Vec<_>>(),
                    }),
                ),
                LlmContextLedgerPackDraft::new(
                    COMPRESSION_MINIMAL,
                    json!({
                        "kind": "previous_part_outlines",
                        "parts": previous_parts
                            .iter()
                            .map(minimal_part_context)
                            .collect::<Vec<_>>(),
                    }),
                ),
            ],
        })?;
    }

    Ok(ledger.into_manifest(
        dynamic_user_content,
        Some(format!(
            "第 {part_num} 分部（第 {start_ch}-{end_ch} 章）必须返回严格 JSON 对象。{}",
            PART_GENERATION_OUTPUT_CONTRACT
        )),
    ))
}

fn minimal_outline_blueprint_context(blueprint_context: &Value) -> Value {
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

fn focused_part_context(part: &part_outline::Model) -> Value {
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

fn minimal_part_context(part: &part_outline::Model) -> Value {
    json!({
        "part_number": part.part_number,
        "title": part.title,
        "start_chapter": part.start_chapter,
        "end_chapter": part.end_chapter,
        "summary": part.summary,
    })
}

#[cfg(test)]
mod part_outline_request_tests {
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

    fn sample_previous_part() -> part_outline::Model {
        let now = Utc::now().naive_utc();
        part_outline::Model {
            id: 7,
            project_id: 42,
            part_number: 1,
            title: "星火初燃".to_string(),
            start_chapter: 1,
            end_chapter: 20,
            summary: Some("旧分部完整摘要不应进入动态 user prompt".to_string()),
            theme: Some("代价".to_string()),
            key_events: Some(json!(["旧分部关键事件"])),
            character_focus: Some(json!({"林烬": "承担责任"})),
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn part_outline_request_moves_blueprint_and_previous_parts_into_manifest() {
        let engine = prompt_engine();
        let previous_part = sample_previous_part();
        let request = build_part_outline_request(PartOutlineRequestInput {
            project_id: 42,
            part_num: 2,
            start_ch: 21,
            end_ch: 40,
            chapters_per_part: 20,
            requested_part_count: 1,
            blueprint_context: &json!({
                "title": "星潮档案",
                "genre": "科幻",
                "one_sentence_summary": "调查星潮真相",
                "characters": [{"name": "林烬", "motivation": "查明真相"}],
            }),
            previous_parts: std::slice::from_ref(&previous_part),
            prompt_engine: &engine,
            variables: &HashMap::new(),
        })
        .expect("part outline request should build");

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
        assert!(pack_kinds.contains(&"outline_previous_part_outlines"));

        let user_message = request
            .messages
            .iter()
            .find(|message| matches!(message.role, MessageRole::User))
            .expect("user message should exist");
        assert!(user_message.content.contains("context_refs"));
        assert!(user_message.content.contains("outline_blueprint_context"));
        assert!(
            user_message
                .content
                .contains("outline_previous_part_outlines")
        );
        assert!(!user_message.content.contains("星潮档案"));
        assert!(
            !user_message
                .content
                .contains("旧分部完整摘要不应进入动态 user prompt")
        );

        let trace_context = request
            .trace_context
            .as_ref()
            .expect("trace context should be attached");
        assert_eq!(trace_context.workflow, "part_outline_generation");
        assert_eq!(trace_context.step_key, "part_2");
        assert_eq!(trace_context.call_key.as_deref(), Some("part_2"));
    }
}

#[cfg(test)]
mod delete_latest_parts_tests {
    use super::*;

    fn part(id: i64, part_number: i32) -> part_outline::Model {
        let now = chrono::Utc::now().naive_utc();
        part_outline::Model {
            id,
            project_id: 7,
            part_number,
            title: format!("第 {part_number} 部"),
            start_chapter: (part_number - 1) * 10 + 1,
            end_chapter: part_number * 10,
            summary: Some("概要".to_string()),
            theme: None,
            key_events: None,
            character_focus: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn latest_part_ids_to_delete_rejects_zero_count() {
        let err =
            latest_part_ids_to_delete(&[part(1, 1)], 0).expect_err("zero delete count must fail");

        assert!(err.to_string().contains("删除分部数量必须大于 0"));
    }

    #[test]
    fn latest_part_ids_to_delete_rejects_count_greater_than_existing_parts() {
        let err = latest_part_ids_to_delete(&[part(1, 1), part(2, 2)], 3)
            .expect_err("oversized delete count must fail");

        assert!(err.to_string().contains("删除分部数量超过现有分部数量"));
    }

    #[test]
    fn latest_part_ids_to_delete_selects_latest_parts_by_list_order() {
        let ids = latest_part_ids_to_delete(&[part(11, 1), part(12, 2), part(13, 3)], 2)
            .expect("valid count should select latest parts");

        assert_eq!(ids, vec![13, 12]);
    }

    #[test]
    fn latest_part_ids_to_delete_rejects_invalid_existing_parts() {
        let err = latest_part_ids_to_delete(&[part(11, 1), part(12, 1)], 1)
            .expect_err("ambiguous latest part state must fail before delete");

        assert!(err.to_string().contains("重复分部大纲"));
        assert!(err.to_string().contains("请先清理分部状态后再删除"));
    }
}
