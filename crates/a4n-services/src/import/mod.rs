//! 导入服务：外部文件解析 + 去重 + AI 分析 + 审核队列
//!
//! 对应文档 20-外部数据导入系统

pub mod dedup;
pub mod file_parsers;
pub mod reverse;
mod reverse_json;

use chrono::Utc;
use sea_orm::*;
use std::sync::Arc;

use a4n_core::error::AppError;
use a4n_core::models::{chapter::chapter, project::import_review_queue};
use a4n_engine::llm::provider::{
    ChatRequest, LlmProvider, LlmTraceContext, is_truncated_finish_reason,
};
use a4n_engine::prompt::engine::PromptEngine;
use a4n_engine::prompt::request::build_json_task_user_message;
use serde_json::{Value, json};

use crate::llm_checkpoint::{
    CompletedLlmCallCheckpointInput, llm_call_checkpoint_fingerprint_sha256,
    load_completed_llm_call_checkpoint, required_llm_call_checkpoint_json,
    required_llm_call_workflow_run_id, save_completed_llm_call_checkpoint,
};
use crate::llm_context::{
    COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedger,
    LlmContextLedgerPackDraft, context_aware_system_user_request,
};
use crate::source::source_content_sha256;
use crate::workflow::WorkflowRunService;

pub struct ReverseEngineerRequestInput<'a> {
    pub project_id: i64,
    pub prompt_name: &'a str,
    pub user_prefix: &'a str,
    pub content_sample: &'a str,
    pub prompt_engine: &'a dyn PromptEngine,
    pub variables: &'a std::collections::HashMap<String, serde_json::Value>,
    pub max_tokens_profile: &'a str,
}

const IMPORT_REVIEW_STATUS_PENDING: &str = "pending";
const IMPORT_REVIEW_STATUS_ANALYZING: &str = "analyzing";
const IMPORT_REVIEW_STATUS_REVIEWING: &str = "reviewing";
const IMPORT_REVIEW_STATUS_ACCEPTED: &str = "accepted";
const IMPORT_REVIEW_STATUS_REJECTED: &str = "rejected";
const IMPORT_REVIEW_STATUS_MODIFIED: &str = "modified";
const ALLOWED_IMPORT_REVIEW_STATUSES: [&str; 6] = [
    IMPORT_REVIEW_STATUS_PENDING,
    IMPORT_REVIEW_STATUS_ANALYZING,
    IMPORT_REVIEW_STATUS_REVIEWING,
    IMPORT_REVIEW_STATUS_ACCEPTED,
    IMPORT_REVIEW_STATUS_REJECTED,
    IMPORT_REVIEW_STATUS_MODIFIED,
];

#[derive(Clone)]
pub struct ImportService {
    db: Arc<DatabaseConnection>,
}

#[derive(Debug, Clone)]
pub struct AcceptedImportReview {
    pub review: import_review_queue::Model,
    pub chapter_id: i64,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ModifyImportReviewInput {
    pub content: Option<String>,
    pub analysis_result: Option<Value>,
    pub ai_summary: Option<String>,
    pub ai_characters: Option<Value>,
    pub ai_world_setting: Option<Value>,
}

impl ImportService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 列出项目的导入审核队列
    pub async fn list_queue(
        &self,
        project_id: i64,
    ) -> Result<Vec<import_review_queue::Model>, AppError> {
        import_review_queue::Entity::find()
            .filter(import_review_queue::Column::ProjectId.eq(project_id))
            .order_by_desc(import_review_queue::Column::CreatedAt)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    /// 更新审核状态
    pub async fn update_status(
        &self,
        review_id: i64,
        status: &str,
    ) -> Result<import_review_queue::Model, AppError> {
        let status = normalize_import_review_status(status)?;
        let review = import_review_queue::Entity::find_by_id(review_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound("导入记录不存在".into()))?;

        let mut active: import_review_queue::ActiveModel = review.into();
        active.review_status = Set(status.to_string());
        active.updated_at = Set(Utc::now().naive_utc());
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    /// 删除导入记录
    pub async fn delete_review(&self, review_id: i64) -> Result<(), AppError> {
        import_review_queue::Entity::delete_by_id(review_id)
            .exec(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        Ok(())
    }

    pub async fn accept_review(
        &self,
        review: import_review_queue::Model,
    ) -> Result<AcceptedImportReview, AppError> {
        let tx = self.db.begin().await.map_err(AppError::Database)?;
        let chapter = upsert_imported_chapter_from_review(&tx, &review).await?;
        let mut active: import_review_queue::ActiveModel = review.into();
        active.review_status = Set(IMPORT_REVIEW_STATUS_ACCEPTED.to_string());
        active.updated_at = Set(Utc::now().naive_utc());
        let updated = active.update(&tx).await.map_err(AppError::Database)?;
        tx.commit().await.map_err(AppError::Database)?;
        Ok(AcceptedImportReview {
            review: updated,
            chapter_id: chapter.id,
        })
    }

    pub async fn modify_review(
        &self,
        review: import_review_queue::Model,
        input: ModifyImportReviewInput,
    ) -> Result<AcceptedImportReview, AppError> {
        let review_id = review.id;
        let mut updated_result = input.analysis_result.clone().map_or_else(
            || {
                review.analysis_result.clone().ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "导入审核缺少分析结果，无法修改并接受: review_id={review_id}"
                    ))
                })
            },
            Ok,
        )?;
        let updated_object = updated_result.as_object_mut().ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "导入审核分析结果必须是 JSON object: review_id={review_id}"
            ))
        })?;
        if let Some(content) = &input.content {
            updated_object.insert("modified_content".to_string(), json!(content));
        }
        if let Some(summary) = &input.ai_summary {
            updated_object.insert("modified_summary".to_string(), json!(summary));
        }
        if let Some(chars) = &input.ai_characters {
            updated_object.insert("modified_characters".to_string(), chars.clone());
        }
        if let Some(world) = &input.ai_world_setting {
            updated_object.insert("modified_world_setting".to_string(), world.clone());
        }

        let tx = self.db.begin().await.map_err(AppError::Database)?;
        let mut active: import_review_queue::ActiveModel = review.clone().into();
        active.analysis_result = Set(Some(updated_result.clone()));
        active.review_status = Set(IMPORT_REVIEW_STATUS_MODIFIED.to_string());
        active.updated_at = Set(Utc::now().naive_utc());
        let updated = active.update(&tx).await.map_err(AppError::Database)?;

        let mut review_for_upsert = review;
        review_for_upsert.analysis_result = Some(updated_result);
        let chapter = upsert_imported_chapter_from_review(&tx, &review_for_upsert).await?;
        tx.commit().await.map_err(AppError::Database)?;

        Ok(AcceptedImportReview {
            review: updated,
            chapter_id: chapter.id,
        })
    }

    pub fn build_reverse_engineer_request(
        &self,
        input: ReverseEngineerRequestInput<'_>,
    ) -> Result<ChatRequest, AppError> {
        let ReverseEngineerRequestInput {
            project_id,
            prompt_name,
            user_prefix,
            content_sample,
            prompt_engine,
            variables,
            max_tokens_profile,
        } = input;
        let sys = prompt_engine.render(prompt_name, variables, Some(project_id))?;
        let source_artifact = reverse_engineer_source_artifact(content_sample)?;
        let source_hash = source_content_sha256(&serde_json::to_string(&source_artifact)?);
        let mut ledger = LlmContextLedger::new();
        ledger.add_pack(crate::llm_context::LlmContextLedgerPackInput {
            scope_kind: ("import_reverse_engineer").into(),
            pack_kind: ("source_artifact").into(),
            source_refs_json: Some(json!({
                "source": "import_content_sample",
                "content_sha256": source_hash,
            })),
            config_identity_json: json!({
                "ledger_version": "import_reverse_engineer_context_v1",
                "prompt_name": prompt_name,
            }),
            required: true,
            stable_prefix: true,
            variants: vec![
                LlmContextLedgerPackDraft::new(COMPRESSION_FULL_RAW, source_artifact.clone()),
                LlmContextLedgerPackDraft::new(COMPRESSION_FOCUSED_RAW, source_artifact.clone()),
                LlmContextLedgerPackDraft::new(COMPRESSION_MINIMAL, source_artifact),
            ],
        })?;
        let manifest = ledger.into_manifest(
            build_reverse_engineer_user_message(prompt_name, user_prefix)?,
            Some(
                "导入反推必须只基于稳定上下文包 source_artifact，不得编造样本之外的项目事实。"
                    .to_string(),
            ),
        );
        let fingerprint = source_content_sha256(&serde_json::to_string(&json!({
            "prompt_name": prompt_name,
            "source_artifact_sha256": source_hash,
            "instruction": user_prefix.trim(),
            "max_tokens_profile": max_tokens_profile,
        }))?);
        Ok(context_aware_system_user_request(
            sys,
            manifest,
            Some(0.3),
            None,
            Some(max_tokens_profile.into()),
        )?
        .with_json_response_format()
        .with_trace_context(LlmTraceContext {
            project_id: Some(project_id),
            root_project_id: None,
            workflow: "import_reverse_engineer".to_string(),
            step_key: prompt_name.to_string(),
            generation_run_id: None,
            workflow_run_id: None,
            call_key: Some(prompt_name.to_string()),
            input_fingerprint_sha256: Some(fingerprint),
            context_plan_json: None,
            context_pack_refs_json: None,
        }))
    }

    pub async fn reverse_engineer_request(
        &self,
        prompt_name: &str,
        mut request: ChatRequest,
        llm: &dyn LlmProvider,
        workflow_run_id: Option<i64>,
    ) -> Result<serde_json::Value, AppError> {
        let call_key = prompt_name.to_string();
        let workflow_run_id = required_llm_call_workflow_run_id(workflow_run_id, &call_key)?;
        let fingerprint_json = import_reverse_call_fingerprint_json(prompt_name, &request)?;
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
            return restored_import_reverse_checkpoint(checkpoint_json, prompt_name);
        }
        let resp = llm.chat(request).await?;
        if is_truncated_finish_reason(resp.finish_reason.as_deref()) {
            return Err(AppError::InvalidArgument(format!(
                "{prompt_name} 输出达到当前导入反推 max_tokens 上限，返回 JSON 被截断；请在设置页调大对应输出档位，或缩短导入样本后重试"
            )));
        }
        let output = parse_llm_json_object(&resp.message.content, prompt_name)?;
        let response_usage_json = resp.usage.as_ref().map(serde_json::to_value).transpose()?;
        save_completed_llm_call_checkpoint(
            self.db.clone(),
            CompletedLlmCallCheckpointInput {
                workflow_run_id,
                call_key,
                call_order: 1,
                input_fingerprint_json: fingerprint_json,
                checkpoint_json: json!({
                    "output": output.clone(),
                    "parsed_output": output.clone(),
                    "response_content": resp.message.content,
                    "usage": response_usage_json,
                    "finish_reason": resp.finish_reason,
                }),
                artifacts_json: None,
            },
        )
        .await?;
        Ok(output)
    }
}

pub fn normalize_import_review_status(status: &str) -> Result<&'static str, AppError> {
    let status = status.trim();
    if status.is_empty() {
        return Err(AppError::InvalidArgument("导入审核状态不能为空".into()));
    }
    ALLOWED_IMPORT_REVIEW_STATUSES
        .iter()
        .copied()
        .find(|allowed| *allowed == status)
        .ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "不支持的导入审核状态: {status}；允许值: {}",
                ALLOWED_IMPORT_REVIEW_STATUSES.join(", ")
            ))
        })
}

fn json_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn import_reverse_call_fingerprint_json(
    prompt_name: &str,
    request: &ChatRequest,
) -> Result<Value, AppError> {
    Ok(json!({
        "schema_version": 1,
        "workflow": "import_reverse_engineer",
        "call_key": prompt_name,
        "prompt_name": prompt_name,
        "messages": request.messages,
        "context_manifest": request.context_manifest_json,
        "max_tokens": request.max_tokens,
        "max_tokens_profile": request.max_tokens_profile,
        "response_format": request.response_format,
    }))
}

fn restored_import_reverse_checkpoint(
    checkpoint_json: Value,
    prompt_name: &str,
) -> Result<Value, AppError> {
    let parsed_output = checkpoint_json
        .get("parsed_output")
        .cloned()
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_checkpoint_parsed_output".to_string(),
            operation: format!("读取导入反推 {prompt_name} 调用级断点"),
        })?;
    let response_content = checkpoint_json
        .get("response_content")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_checkpoint_response_content".to_string(),
            operation: format!("读取导入反推 {prompt_name} 调用级断点"),
        })?;
    let parsed = parse_llm_json_object(response_content, prompt_name)?;
    if parsed != parsed_output {
        return Err(AppError::InvalidState {
            current: "checkpoint_parsed_output_mismatch".to_string(),
            operation: format!("校验导入反推 {prompt_name} 调用级断点"),
        });
    }
    Ok(parsed)
}

fn json_i32(value: &Value, key: &str) -> Option<i32> {
    value
        .get(key)
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
}

fn required_import_chapter_number(analysis: &Value, review_id: i64) -> Result<i32, AppError> {
    let chapter_number = json_i32(analysis, "chapter_number").ok_or_else(|| {
        AppError::InvalidArgument(format!("导入审核缺少章节序号: review_id={review_id}"))
    })?;
    if chapter_number <= 0 {
        return Err(AppError::InvalidArgument(format!(
            "导入审核章节序号必须为正整数: review_id={review_id}, chapter_number={chapter_number}"
        )));
    }
    Ok(chapter_number)
}

fn required_import_chapter_title(analysis: &Value, review_id: i64) -> Result<String, AppError> {
    json_string(analysis, "modified_title")
        .or_else(|| json_string(analysis, "chapter_title"))
        .ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "导入审核缺少章节标题，无法接受。请在审核中补充标题后再接受: review_id={review_id}"
            ))
        })
}

fn ensure_chapter_not_generating(
    chapter: &chapter::Model,
    operation: &str,
) -> Result<(), AppError> {
    if chapter.status == "generating" {
        return Err(AppError::InvalidState {
            current: chapter.status.clone(),
            operation: operation.to_string(),
        });
    }
    Ok(())
}

async fn upsert_imported_chapter_from_review<C>(
    db: &C,
    review: &import_review_queue::Model,
) -> Result<chapter::Model, AppError>
where
    C: ConnectionTrait,
{
    let analysis = review.analysis_result.as_ref().ok_or_else(|| {
        AppError::InvalidArgument(format!("导入审核缺少分析结果: review_id={}", review.id))
    })?;
    let chapter_number = required_import_chapter_number(analysis, review.id)?;
    let title = required_import_chapter_title(analysis, review.id)?;
    let content = json_string(analysis, "modified_content")
        .or_else(|| json_string(analysis, "content"))
        .ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "导入审核缺少完整正文，无法接受。请重新导入该文件后再接受: review_id={}",
                review.id
            ))
        })?;
    let word_count = content.chars().count() as i32;
    let summary = json_string(analysis, "modified_summary");
    let now = Utc::now().naive_utc();

    let existing = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(review.project_id))
        .filter(chapter::Column::ChapterNumber.eq(chapter_number))
        .one(db)
        .await
        .map_err(AppError::Database)?;

    if let Some(existing) = existing {
        ensure_chapter_not_generating(&existing, "接受导入审核")?;
        let mut active: chapter::ActiveModel = existing.into();
        active.title = Set(Some(title));
        active.content = Set(content);
        active.word_count = Set(word_count);
        active.summary = Set(summary);
        active.style_tags = Set(None);
        active.analysis_data = Set(None);
        active.status = Set("imported".to_string());
        active.updated_at = Set(now);
        active.update(db).await.map_err(AppError::Database)
    } else {
        chapter::ActiveModel {
            project_id: Set(review.project_id),
            outline_id: Set(None),
            chapter_number: Set(chapter_number),
            title: Set(Some(title)),
            content: Set(content),
            word_count: Set(word_count),
            status: Set("imported".to_string()),
            summary: Set(summary),
            style_tags: Set(None),
            analysis_data: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(AppError::Database)
    }
}

fn build_reverse_engineer_user_message(
    prompt_name: &str,
    instruction: &str,
) -> Result<String, AppError> {
    let instruction = instruction.trim();
    if instruction.is_empty() {
        return Err(AppError::InvalidArgument(
            "反推任务 instruction 不能为空".to_string(),
        ));
    }

    let payload = json!({
        "context_refs": {
            "source": "稳定上下文包 source_artifact",
        },
        "task": {
            "prompt_name": prompt_name,
            "instruction": instruction,
        },
        "output_contract": reverse_output_contract(prompt_name),
    });
    build_json_task_user_message(
        "执行导入反推",
        "只返回系统提示词要求的严格 JSON 对象，不要输出 Markdown、代码块、解释文字或额外字段",
        &payload,
    )
}

fn reverse_engineer_source_artifact(content_sample: &str) -> Result<Value, AppError> {
    let content_sample = content_sample.trim();
    if content_sample.is_empty() {
        return Err(AppError::InvalidArgument(
            "反推任务 content_sample 不能为空".to_string(),
        ));
    }

    Ok(match serde_json::from_str::<Value>(content_sample) {
        Ok(value) => json!({ "source_context": value }),
        Err(_) => json!({ "source_text": content_sample }),
    })
}

fn reverse_output_contract(prompt_name: &str) -> Value {
    match prompt_name {
        "reverse_blueprint" => json!({
            "format": "json_object",
            "required_top_level_fields": [
                "title",
                "genre",
                "style",
                "tone",
                "target_audience",
                "one_sentence_summary",
                "full_synopsis",
                "world_setting",
                "characters",
                "relationships"
            ],
            "optional_top_level_fields": [
                "total_chapters",
                "needs_part_outlines"
            ],
            "consistency_rules": {
                "total_chapters": "如输出，必须为正整数；已有蓝图总章节数时必须一致",
                "needs_part_outlines": "如输出，必须是布尔值，且必须与有效总章节数推导结果一致"
            },
            "system_derived_or_downstream_fields_do_not_output": [
                "chapters_per_part",
                "chapter_outline",
                "chapter_outlines",
                "part_outlines"
            ],
            "length_limits": {
                "one_sentence_summary": "30 个中文字符以内",
                "full_synopsis": "300-500 个中文字符，保留起承转合但不要展开到章节级",
                "world_setting_text_fields": "每项 20-80 个中文字符",
                "character_text_fields": "每项 10-60 个中文字符",
                "relationship_description": "20-80 个中文字符"
            },
            "count_limits": {
                "characters": "3-8 个主要角色；缺少主要角色时宁缺毋滥",
                "relationships": "0-12 条高价值关系",
                "world_setting.key_locations": "0-6 条",
                "world_setting.factions": "0-6 条",
                "world_setting.history_timeline": "0-6 条",
                "world_setting.conflict_sources": "1-6 条",
                "world_setting.mysteries_and_foreshadowing": "0-6 条"
            }
        }),
        "reverse_outline" => json!({
            "format": "json_object",
            "required_root_field": "outlines",
            "forbidden_root_fields": ["chapters", "chapter_outlines", "items"],
            "item_fields": ["chapter_number", "title", "summary"],
            "optional_item_fields": ["word_count_target"],
            "numeric_rules": {
                "word_count_target": "如输出，必须为正整数"
            },
            "system_derived_fields_do_not_output": ["id", "project_id", "outline_id", "created_at", "updated_at"],
            "length_limits": {
                "title": "15 个中文字符以内",
                "summary": "50-100 个中文字符"
            },
            "count_rule": "outlines 数量必须等于输入 source_context.summaries 数量；不要合并或新增章节"
        }),
        "reverse_part_outline" => json!({
            "format": "json_object",
            "required_root_field": "parts",
            "forbidden_root_fields": ["part_outlines", "outlines", "chapters"],
            "item_fields": [
                "part_number",
                "title",
                "start_chapter",
                "end_chapter",
                "summary",
                "theme",
                "key_events",
                "character_focus"
            ],
            "system_derived_fields_do_not_output": ["id", "project_id", "created_at", "updated_at"],
            "length_limits": {
                "title": "20 个中文字符以内",
                "summary": "100-160 个中文字符",
                "theme": "30 个中文字符以内",
                "key_events_each": "20-60 个中文字符",
                "character_focus_each": "20-80 个中文字符"
            },
            "count_limits": {
                "parts": "按剧情自然阶段划分，通常 2-6 部",
                "key_events": "每部分 3-5 条"
            },
            "range_rule": "章节范围必须连续、无重叠、无遗漏"
        }),
        _ => json!({
            "format": "json_object"
        }),
    }
}

fn parse_llm_json_object(content: &str, context: &str) -> Result<serde_json::Value, AppError> {
    let value: serde_json::Value = serde_json::from_str(content.trim())
        .map_err(|e| AppError::InvalidArgument(format!("{context} 必须返回 JSON 对象: {e}")))?;

    if !value.is_object() {
        return Err(AppError::InvalidArgument(format!(
            "{context} 必须返回 JSON 对象"
        )));
    }

    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalize_import_review_status_accepts_defined_statuses() {
        for status in ALLOWED_IMPORT_REVIEW_STATUSES {
            assert_eq!(
                normalize_import_review_status(status).expect("status should be valid"),
                status
            );
            assert_eq!(
                normalize_import_review_status(&format!(" {status}\n"))
                    .expect("trimmed status should be valid"),
                status
            );
        }
    }

    #[test]
    fn normalize_import_review_status_rejects_unknown_status() {
        let err = normalize_import_review_status("approved")
            .expect_err("legacy status should be rejected");

        assert!(matches!(err, AppError::InvalidArgument(_)));
        assert!(err.to_string().contains("不支持的导入审核状态"));
    }

    #[test]
    fn normalize_import_review_status_rejects_blank_status() {
        let err =
            normalize_import_review_status(" \n\t").expect_err("blank status should be rejected");

        assert!(matches!(err, AppError::InvalidArgument(_)));
        assert!(err.to_string().contains("不能为空"));
    }

    #[test]
    fn parse_llm_json_object_accepts_object() {
        let value = parse_llm_json_object(r#"{"chapter_number":1,"summary":"ok"}"#, "导入分析")
            .expect("valid json object");

        assert_eq!(value["chapter_number"], json!(1));
    }

    #[test]
    fn parse_llm_json_object_rejects_plain_text() {
        let err = parse_llm_json_object("这是一段分析文本", "导入分析")
            .expect_err("plain text should be rejected");

        assert!(matches!(err, AppError::InvalidArgument(_)));
    }

    #[test]
    fn parse_llm_json_object_rejects_array() {
        let err = parse_llm_json_object(r#"[{"chapter_number":1}]"#, "reverse_outline")
            .expect_err("array should be rejected");

        assert!(matches!(err, AppError::InvalidArgument(_)));
    }

    #[test]
    fn parse_llm_json_object_rejects_markdown_fence() {
        let err = parse_llm_json_object("```json\n{\"title\":\"x\"}\n```", "reverse_blueprint")
            .expect_err("markdown fence should be rejected");

        assert!(matches!(err, AppError::InvalidArgument(_)));
    }

    #[test]
    fn required_import_chapter_number_rejects_non_positive_number() {
        let err = required_import_chapter_number(&json!({"chapter_number": 0}), 42)
            .expect_err("chapter_number 0 must fail before writing chapters");

        assert!(err.to_string().contains("章节序号必须为正整数"));
        assert!(err.to_string().contains("review_id=42"));
    }

    #[test]
    fn build_reverse_engineer_user_message_references_source_artifact() {
        let message = build_reverse_engineer_user_message("reverse_outline", "生成大纲")
            .expect("reverse message should serialize");

        assert!(message.contains(r#""prompt_name": "reverse_outline""#));
        assert!(message.contains("source_artifact"));
        assert!(message.contains(r#""required_root_field": "outlines""#));
        assert!(message.contains(r#""optional_item_fields""#));
        assert!(message.contains(r#""word_count_target""#));
        assert!(message.contains(r#""forbidden_root_fields""#));
        assert!(!message.contains(r#""summaries": []"#));
        assert!(!message.contains(r#""source_text""#));
    }

    #[test]
    fn reverse_engineer_source_artifact_accepts_json_context() {
        let source = reverse_engineer_source_artifact(r#"{"summaries":[]}"#)
            .expect("source artifact should parse json sample");

        assert_eq!(source["source_context"]["summaries"], json!([]));
        assert!(source.get("source_text").is_none());
    }

    #[test]
    fn reverse_engineer_source_artifact_accepts_plain_text_sample() {
        let source = reverse_engineer_source_artifact("第一章 正文")
            .expect("source artifact should accept plain text sample");

        assert_eq!(source["source_text"], json!("第一章 正文"));
        assert!(source.get("source_context").is_none());
    }

    #[test]
    fn build_reverse_engineer_blueprint_message_keeps_contract() {
        let message = build_reverse_engineer_user_message("reverse_blueprint", "生成蓝图")
            .expect("reverse message should serialize");

        assert!(message.contains(r#""system_derived_or_downstream_fields_do_not_output""#));
        assert!(message.contains(r#""optional_top_level_fields""#));
        assert!(message.contains(r#""needs_part_outlines""#));
        assert!(message.contains(r#""full_synopsis": "300-500 个中文字符"#));
    }

    #[test]
    fn reverse_blueprint_output_contract_does_not_mark_optional_fields_as_required_or_forbidden() {
        let contract = reverse_output_contract("reverse_blueprint");
        let required = contract["required_top_level_fields"]
            .as_array()
            .expect("required fields should be an array");
        let optional = contract["optional_top_level_fields"]
            .as_array()
            .expect("optional fields should be an array");
        let forbidden = contract["system_derived_or_downstream_fields_do_not_output"]
            .as_array()
            .expect("forbidden fields should be an array");

        assert!(!required.iter().any(|field| field == "total_chapters"));
        assert!(!required.iter().any(|field| field == "needs_part_outlines"));
        assert!(optional.iter().any(|field| field == "total_chapters"));
        assert!(optional.iter().any(|field| field == "needs_part_outlines"));
        assert!(!forbidden.iter().any(|field| field == "needs_part_outlines"));
    }

    #[test]
    fn reverse_outline_output_contract_marks_word_count_target_as_optional_positive_number() {
        let contract = reverse_output_contract("reverse_outline");
        let item_fields = contract["item_fields"]
            .as_array()
            .expect("item fields should be an array");
        let optional = contract["optional_item_fields"]
            .as_array()
            .expect("optional item fields should be an array");

        assert!(!item_fields.iter().any(|field| field == "word_count_target"));
        assert!(optional.iter().any(|field| field == "word_count_target"));
        assert_eq!(
            contract["numeric_rules"]["word_count_target"],
            "如输出，必须为正整数"
        );
    }
}
