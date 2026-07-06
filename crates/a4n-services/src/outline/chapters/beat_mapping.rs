use std::collections::HashMap;

use a4n_core::error::AppError;
use a4n_core::models::chapter::{chapter_outline, part_outline};
use a4n_engine::llm::provider::ChatRequest;
use a4n_engine::prompt::engine::PromptEngine;
use chrono::Utc;
use sea_orm::TransactionTrait;
use serde_json::{Value, json};

use crate::llm_checkpoint::{
    CompletedLlmCallCheckpointInput, llm_call_checkpoint_fingerprint_sha256,
    load_completed_llm_call_checkpoint, required_llm_call_checkpoint_json,
    required_llm_call_workflow_run_id, save_completed_llm_call_checkpoint,
};
use crate::workflow::WorkflowRunService;

use super::super::chapter_contract::{
    beat_mapping_manifest_user_message, ensure_focus_part_in_context, parse_beat_sheet_json,
    parse_chapter_outline_batch, required_i32, required_part_summary,
    sync_chapter_outlines_by_number, validate_requested_part_context,
};
use super::super::context::{
    ChapterOutlineGenerationContext, load_chapter_outline_generation_context,
};
use super::super::{
    BeatMappingApplyInput, OutlineService, ensure_response_not_truncated, report_progress,
};
use super::{ContextAwareChapterOutlineRequestInput, build_context_aware_chapter_outline_request};

impl OutlineService {
    /// 将节拍表映射为指定分部的章节大纲。
    pub async fn apply_beat_mapping(
        &self,
        input: BeatMappingApplyInput<'_, '_>,
    ) -> Result<Vec<chapter_outline::Model>, AppError> {
        let BeatMappingApplyInput {
            project_id,
            part_id,
            beat_sheet,
            llm,
            prompt_engine,
            variables,
            progress,
            workflow_run_id,
        } = input;
        let beat_payload = parse_beat_sheet_json(beat_sheet)?;
        let part = self.get_part(part_id).await?;
        if part.project_id != project_id {
            return Err(AppError::InvalidArgument(format!(
                "分部大纲 {part_id} 不属于项目 {project_id}"
            )));
        }
        required_part_summary(&part)?;
        let count = part
            .end_chapter
            .checked_sub(part.start_chapter)
            .and_then(|span| span.checked_add(1))
            .ok_or_else(|| AppError::InvalidArgument("分部章节范围无效".to_string()))?;
        if count <= 0 {
            return Err(AppError::InvalidArgument(
                "分部章节范围必须至少包含 1 章".to_string(),
            ));
        }

        let context = load_chapter_outline_generation_context(self.db.as_ref(), project_id).await?;
        validate_requested_part_context(&context.parts, part.start_chapter, part.end_chapter)?;
        ensure_focus_part_in_context(Some(&part), &context.parts)?;

        report_progress(
            progress,
            0,
            count,
            &format!("正在为第 {} 部映射 {} 个章节大纲", part.part_number, count),
        )?;
        let mut request = build_beat_mapping_request(
            project_id,
            &context,
            &part,
            &beat_payload,
            count,
            prompt_engine,
            variables,
        )?;
        let call_key = format!("part_{}_beat_mapping", part.part_number);
        let workflow_run_id = required_llm_call_workflow_run_id(workflow_run_id, &call_key)?;
        let fingerprint_json =
            beat_mapping_call_fingerprint_json(&call_key, &part, &beat_payload, count, &request)?;
        let input_fingerprint_sha256 = llm_call_checkpoint_fingerprint_sha256(&fingerprint_json)?;
        if let Some(trace_context) = request.trace_context.as_mut() {
            trace_context.workflow_run_id = Some(workflow_run_id);
            trace_context.call_key = Some(call_key.clone());
            trace_context.input_fingerprint_sha256 = Some(input_fingerprint_sha256);
        }
        WorkflowRunService::new(self.db.clone())
            .mark_current_step(workflow_run_id, &call_key)
            .await?;
        let chapters = if let Some(checkpoint) = load_completed_llm_call_checkpoint(
            self.db.clone(),
            workflow_run_id,
            &call_key,
            &fingerprint_json,
        )
        .await?
        {
            let checkpoint_json = required_llm_call_checkpoint_json(checkpoint, &call_key)?;
            restored_beat_mapping_checkpoint(
                checkpoint_json,
                part.part_number,
                part.start_chapter,
                count,
            )?
        } else {
            let response = llm.chat(request.clone()).await?;
            let content = response.message.content;
            ensure_response_not_truncated(
                response.finish_reason.as_deref(),
                &format!("第 {} 部节拍映射", part.part_number),
            )?;
            let chapters = parse_chapter_outline_batch(&content, part.start_chapter, count)?;
            save_completed_llm_call_checkpoint(
                self.db.clone(),
                CompletedLlmCallCheckpointInput {
                    workflow_run_id,
                    call_key,
                    call_order: part.part_number,
                    input_fingerprint_json: fingerprint_json,
                    checkpoint_json: json!({
                        "output": chapters.clone(),
                        "parsed_output": chapters.clone(),
                        "response_content": content,
                        "usage": response.usage.as_ref().map(serde_json::to_value).transpose()?,
                        "finish_reason": response.finish_reason,
                    }),
                    artifacts_json: None,
                },
            )
            .await?;
            chapters
        };
        let now = Utc::now().naive_utc();
        let txn = self.db.begin().await.map_err(AppError::Database)?;

        for (index, chapter_json) in chapters.iter().enumerate() {
            let current = index as i32 + 1;
            let chapter_number = required_i32(
                chapter_json,
                "chapter_number",
                part.start_chapter + index as i32,
            )?;
            report_progress(
                progress,
                current - 1,
                count,
                &format!("正在保存第 {current}/{count} 个章节大纲（第 {chapter_number} 章）"),
            )?;
            report_progress(
                progress,
                current,
                count,
                &format!("已准备第 {current}/{count} 个章节大纲（第 {chapter_number} 章）"),
            )?;
        }
        let results =
            sync_chapter_outlines_by_number(&txn, project_id, Some(part.id), chapters, now).await?;
        txn.commit().await.map_err(AppError::Database)?;

        Ok(results)
    }

    pub async fn preview_beat_mapping_request(
        &self,
        project_id: i64,
        part_id: i64,
        beat_sheet: &str,
        prompt_engine: &dyn PromptEngine,
        variables: &HashMap<String, serde_json::Value>,
    ) -> Result<ChatRequest, AppError> {
        let beat_payload = parse_beat_sheet_json(beat_sheet)?;
        let part = self.get_part(part_id).await?;
        if part.project_id != project_id {
            return Err(AppError::InvalidArgument(format!(
                "分部大纲 {part_id} 不属于项目 {project_id}"
            )));
        }
        required_part_summary(&part)?;
        let count = part
            .end_chapter
            .checked_sub(part.start_chapter)
            .and_then(|span| span.checked_add(1))
            .ok_or_else(|| AppError::InvalidArgument("分部章节范围无效".to_string()))?;
        if count <= 0 {
            return Err(AppError::InvalidArgument(
                "分部章节范围必须至少包含 1 章".to_string(),
            ));
        }

        let context = load_chapter_outline_generation_context(self.db.as_ref(), project_id).await?;
        validate_requested_part_context(&context.parts, part.start_chapter, part.end_chapter)?;
        ensure_focus_part_in_context(Some(&part), &context.parts)?;
        build_beat_mapping_request(
            project_id,
            &context,
            &part,
            &beat_payload,
            count,
            prompt_engine,
            variables,
        )
    }
}

fn beat_mapping_call_fingerprint_json(
    call_key: &str,
    part: &part_outline::Model,
    beat_payload: &Value,
    count: i32,
    request: &ChatRequest,
) -> Result<Value, AppError> {
    Ok(json!({
        "schema_version": 1,
        "workflow": "chapter_outline_beat_mapping",
        "call_key": call_key,
        "prompt_name": "outline",
        "messages": request.messages,
        "context_manifest": request.context_manifest_json,
        "max_tokens": request.max_tokens,
        "max_tokens_profile": request.max_tokens_profile,
        "response_format": request.response_format,
        "target": {
            "part_id": part.id,
            "part_number": part.part_number,
            "start_chapter": part.start_chapter,
            "end_chapter": part.end_chapter,
            "count": count,
        },
        "beat_payload": beat_payload,
    }))
}

fn restored_beat_mapping_checkpoint(
    checkpoint_json: Value,
    part_number: i32,
    start_chapter: i32,
    count: i32,
) -> Result<Vec<Value>, AppError> {
    let parsed_output = checkpoint_json
        .get("parsed_output")
        .cloned()
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_checkpoint_parsed_output".to_string(),
            operation: format!("读取第 {part_number} 部节拍映射调用级断点"),
        })?;
    let response_content = checkpoint_json
        .get("response_content")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_checkpoint_response_content".to_string(),
            operation: format!("读取第 {part_number} 部节拍映射调用级断点"),
        })?;
    let parsed = parse_chapter_outline_batch(response_content, start_chapter, count)?;
    if Value::Array(parsed.clone()) != parsed_output {
        return Err(AppError::InvalidState {
            current: "checkpoint_parsed_output_mismatch".to_string(),
            operation: format!("校验第 {part_number} 部节拍映射调用级断点"),
        });
    }
    Ok(parsed)
}

fn build_beat_mapping_request(
    project_id: i64,
    context: &ChapterOutlineGenerationContext,
    part: &part_outline::Model,
    beat_payload: &serde_json::Value,
    count: i32,
    prompt_engine: &dyn PromptEngine,
    variables: &HashMap<String, serde_json::Value>,
) -> Result<ChatRequest, AppError> {
    let prompt_content = prompt_engine.render("outline", variables, Some(project_id))?;
    let user_content = beat_mapping_manifest_user_message(part, beat_payload, count)?;
    build_context_aware_chapter_outline_request(ContextAwareChapterOutlineRequestInput {
        project_id,
        context,
        prompt_content,
        user_content,
        count,
        start_chapter: part.start_chapter,
        end_chapter: part.end_chapter,
        temperature: Some(0.6),
        workflow: "chapter_outline_beat_mapping",
        step_key: format!("part_{}_beat_mapping", part.part_number),
    })
}
