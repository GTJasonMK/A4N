//! 角色隐性属性复审 action。
//!
//! Web 和 Tauri 共用这里的统计、模型请求、决策应用与 RAG 同步逻辑。

mod decision;
mod prompt;
mod stats;
mod types;

use crate::AppState;
use crate::actions::{
    access,
    llm_request::{
        ContextAwareJsonLlmRequestInput, PACK_KIND_IMPLICIT_UPDATE_REVIEW_CONTEXT,
        context_aware_system_user_json_llm_request,
    },
    request_preview::{
        ChatRequestStepInput, ModelRequestPreview, chat_request_step_for_state,
        model_request_preview,
    },
};
use a4n_core::config::app_config::llm_token_profiles;
use a4n_core::models::character::{
    attribute_event, protagonist_behavior_record, protagonist_deletion_mark, protagonist_profile,
};
use a4n_engine::llm::provider::{ChatRequest, is_truncated_finish_reason};
use a4n_engine::prompt::engine::PromptEngine;
use a4n_services::character::DELETION_THRESHOLD;
use a4n_services::generation::context_aware_chat_request_trace_json;
use a4n_services::llm_checkpoint::{
    CompletedLlmCallCheckpointInput, llm_call_checkpoint_fingerprint_sha256,
    load_completed_llm_call_checkpoint, required_llm_call_checkpoint_json,
    save_completed_llm_call_checkpoint,
};
use a4n_services::workflow::{
    UpsertWorkflowRunInput, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED,
    WORKFLOW_STATUS_RUNNING,
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub(crate) use decision::parse_implicit_update_decision;
pub(crate) use prompt::IMPLICIT_UPDATE_PROMPT_NAME;
use prompt::{
    build_implicit_update_manifest_user_prompt, implicit_update_stable_context,
    render_implicit_update_system_prompt,
};
pub(crate) use stats::{
    build_implicit_attribute_review, build_implicit_review_summary, validate_implicit_review_params,
};
pub use types::{
    AUTO_IMPLICIT_REVIEW_EVIDENCE_LIMIT, AUTO_IMPLICIT_REVIEW_THRESHOLD,
    AUTO_IMPLICIT_REVIEW_WINDOW_CHAPTERS, BehaviorForReview, ImplicitAttributeReviewSummary,
    ImplicitReviewSummary, ImplicitUpdateDecisionSummary,
};

#[derive(Debug, Deserialize)]
pub struct GetImplicitReviewInput {
    pub profile_id: i64,
    pub threshold: usize,
    pub window_chapters: usize,
    pub evidence_limit: usize,
}

#[derive(Debug, Deserialize)]
pub struct CheckImplicitUpdateInput {
    pub profile_id: i64,
    pub attribute_key: String,
    pub threshold: usize,
    pub window_chapters: usize,
    pub evidence_limit: usize,
}

#[derive(Debug, Deserialize)]
pub struct ApplyImplicitUpdateDecisionInput {
    pub decision: ImplicitUpdateDecisionSummary,
    pub chapter_number: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeEventSummary {
    pub id: i64,
    pub profile_id: i64,
    pub chapter_number: i32,
    pub timestamp: Option<String>,
    pub category: String,
    pub key: String,
    pub operation: String,
    pub change_description: String,
    pub old_value: serde_json::Value,
    pub new_value: serde_json::Value,
    pub cause: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletionMarkSummary {
    pub id: i64,
    pub profile_id: i64,
    pub category: String,
    pub key: String,
    pub chapter_number: i32,
    pub reason: String,
    pub evidence: String,
    pub consecutive_count: i32,
    pub threshold: i32,
    pub last_marked_chapter: i32,
    pub is_executed: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyImplicitUpdateDecisionResultSummary {
    pub action: String,
    pub event: Option<AttributeEventSummary>,
    pub deletion_mark: Option<DeletionMarkSummary>,
    pub ready_for_deletion: bool,
    pub threshold: i32,
}

struct CheckImplicitUpdateRequestContext {
    project_id: i64,
    current_value: serde_json::Value,
    stats: ImplicitAttributeReviewSummary,
    request: ChatRequest,
}

pub(crate) struct ImplicitUpdateLlmRequestInput<'a> {
    pub(crate) project_id: i64,
    pub(crate) profile_id: Option<i64>,
    pub(crate) profile_name: &'a str,
    pub(crate) attribute_key: &'a str,
    pub(crate) current_value: &'a Value,
    pub(crate) stats: &'a ImplicitAttributeReviewSummary,
    pub(crate) threshold: usize,
    pub(crate) window_chapters: usize,
    pub(crate) source_refs_json: Option<Value>,
    pub(crate) call_key: Option<String>,
}

#[derive(Debug)]
pub(crate) enum AppliedImplicitDecisionAction {
    Keep,
    Modify,
    MarkDeletion,
}

#[derive(Debug)]
pub(crate) struct AppliedImplicitDecision {
    profile: protagonist_profile::Model,
    pub(crate) action: AppliedImplicitDecisionAction,
    pub(crate) event: Option<attribute_event::Model>,
    pub(crate) deletion_mark: Option<protagonist_deletion_mark::Model>,
}

pub async fn get_implicit_review(
    state: &AppState,
    user_id: i64,
    input: GetImplicitReviewInput,
) -> Result<ImplicitReviewSummary, String> {
    let (threshold, window_chapters, evidence_limit) = validate_implicit_review_params(
        input.threshold,
        input.window_chapters,
        input.evidence_limit,
    )?;
    let (profile, records) =
        implicit_review_context_for_user(state, user_id, input.profile_id).await?;

    build_implicit_review_summary(
        input.profile_id,
        &profile.current_state,
        &records,
        threshold,
        window_chapters,
        evidence_limit,
    )
}

pub async fn check_implicit_update(
    state: &AppState,
    user_id: i64,
    input: CheckImplicitUpdateInput,
) -> Result<ImplicitUpdateDecisionSummary, String> {
    let attribute_key = input.attribute_key.trim();
    if attribute_key.is_empty() {
        return Err("隐性属性 key 不能为空".to_string());
    }

    let mut request_context = build_check_implicit_update_request(state, user_id, &input).await?;

    let request_json = context_aware_chat_request_trace_json(&request_context.request)
        .map_err(|err| format!("LLM 请求序列化失败: {err}"))?;
    let call_key = format!(
        "implicit_update_review:{}:{attribute_key}",
        input.profile_id
    );
    let input_fingerprint_json = json!({
        "prompt_names": [IMPLICIT_UPDATE_PROMPT_NAME],
        "request": request_json,
        "input_variables": {
            "profile_id": input.profile_id,
            "attribute_key": attribute_key,
            "threshold": input.threshold,
            "window_chapters": input.window_chapters,
            "evidence_limit": input.evidence_limit,
        },
    });
    let workflow_run = state
        .workflow_run
        .upsert_run(UpsertWorkflowRunInput {
            business_key: format!(
                "implicit_update_review:{}:{attribute_key}",
                input.profile_id
            ),
            workflow_kind: "implicit_update_review".to_string(),
            project_id: Some(request_context.project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("protagonist_profile".to_string()),
            scope_id: Some(input.profile_id),
            chapter_id: None,
            mode: Some(attribute_key.to_string()),
            input_fingerprint_json: Some(input_fingerprint_json.clone()),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some(call_key.clone()),
            active_task_id: None,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map_err(|err| err.to_string())?;
    let workflow_run_id = workflow_run.id;
    let input_fingerprint_sha256 = llm_call_checkpoint_fingerprint_sha256(&input_fingerprint_json)
        .map_err(|err| err.to_string())?;
    if let Some(trace_context) = request_context.request.trace_context.as_mut() {
        trace_context.workflow_run_id = Some(workflow_run_id);
        trace_context.call_key = Some(call_key.clone());
        trace_context.input_fingerprint_sha256 = Some(input_fingerprint_sha256);
    }
    state
        .workflow_run
        .mark_current_step(workflow_run_id, &call_key)
        .await
        .map_err(|err| err.to_string())?;

    let result = async {
        let (response_text, response_usage_json, response_finish_reason, restored) =
            if let Some(checkpoint) = load_completed_llm_call_checkpoint(
                state.db.clone(),
                workflow_run_id,
                &call_key,
                &input_fingerprint_json,
            )
            .await
            .map_err(|err| err.to_string())?
            {
                let checkpoint_json = required_llm_call_checkpoint_json(checkpoint, &call_key)
                    .map_err(|err| err.to_string())?;
                let (response_text, usage, finish_reason) =
                    implicit_update_review_checkpoint_response(
                        &checkpoint_json,
                        &call_key,
                        &request_context.stats,
                    )?;
                (response_text, usage, finish_reason, true)
            } else {
                let response = state
                    .llm
                    .chat(request_context.request)
                    .await
                    .map_err(|e| e.to_string())?;
                if is_truncated_finish_reason(response.finish_reason.as_deref()) {
                    return Err("隐性属性复审输出达到当前 max_tokens 上限，返回 JSON 被截断；请在设置页调大“JSON 标准”输出档位，或减少本次复审上下文后重试".to_string());
                }
                let response_text = response.message.content.clone();
                let response_usage_json = response
                    .usage
                    .as_ref()
                    .map(serde_json::to_value)
                    .transpose()
                    .map_err(|err| format!("LLM 用量序列化失败: {err}"))?;
                let response_finish_reason = response.finish_reason.clone();
                (response_text, response_usage_json, response_finish_reason, false)
            };
        let parsed = parse_implicit_update_decision(&response_text, &request_context.stats)?;
        if !restored {
            save_completed_llm_call_checkpoint(
                state.db.clone(),
                CompletedLlmCallCheckpointInput {
                    workflow_run_id,
                    call_key: call_key.clone(),
                    call_order: 1,
                    input_fingerprint_json: input_fingerprint_json.clone(),
                    checkpoint_json: json!({
                        "output": implicit_update_decision_output_json(&parsed),
                        "parsed_output": implicit_update_decision_output_json(&parsed),
                        "response_content": response_text,
                        "usage": response_usage_json,
                        "finish_reason": response_finish_reason,
                    }),
                    artifacts_json: None,
                },
            )
            .await
            .map_err(|err| err.to_string())?;
        }

        Ok(ImplicitUpdateDecisionSummary {
            profile_id: input.profile_id,
            attribute_key: attribute_key.to_string(),
            current_value: request_context.current_value,
            decision: parsed.decision,
            reasoning: parsed.reasoning,
            suggested_new_value: parsed.suggested_new_value,
            evidence_summary: parsed.evidence_summary,
            stats: request_context.stats,
        })
    }
    .await;

    if let Err(status_err) =
        finalize_implicit_update_review_workflow(state, workflow_run_id, &result).await
    {
        return match result {
            Ok(_) => Err(format!(
                "隐性属性复审完成，但工作流状态写入失败: {status_err}"
            )),
            Err(err) => Err(format!("{err}；工作流失败状态写入失败: {status_err}")),
        };
    }
    result
}

async fn build_check_implicit_update_request(
    state: &AppState,
    user_id: i64,
    input: &CheckImplicitUpdateInput,
) -> Result<CheckImplicitUpdateRequestContext, String> {
    let attribute_key = input.attribute_key.trim();
    if attribute_key.is_empty() {
        return Err("隐性属性 key 不能为空".to_string());
    }

    let (threshold, window_chapters, evidence_limit) = validate_implicit_review_params(
        input.threshold,
        input.window_chapters,
        input.evidence_limit,
    )?;

    let (profile, records) =
        implicit_review_context_for_user(state, user_id, input.profile_id).await?;
    let current_value = implicit_attribute_value(&profile.current_state, attribute_key)?;
    let stats = build_implicit_attribute_review(
        attribute_key,
        current_value.clone(),
        &records,
        threshold,
        window_chapters,
        evidence_limit,
    )?;

    let request = build_implicit_update_request(
        state.engine.as_ref(),
        ImplicitUpdateLlmRequestInput {
            project_id: profile.project_id,
            profile_id: Some(profile.id),
            profile_name: &profile.name,
            attribute_key,
            current_value: &current_value,
            stats: &stats,
            threshold,
            window_chapters,
            source_refs_json: Some(json!({
                "trigger": "manual_review",
                "profile_id": profile.id,
                "attribute_key": attribute_key,
                "window_chapters": window_chapters,
                "evidence_limit": evidence_limit,
            })),
            call_key: Some(format!(
                "implicit_update_review:{}:{attribute_key}",
                profile.id
            )),
        },
    )?;

    Ok(CheckImplicitUpdateRequestContext {
        project_id: profile.project_id,
        current_value,
        stats,
        request,
    })
}

pub(crate) fn build_implicit_update_request(
    prompt_engine: &dyn PromptEngine,
    input: ImplicitUpdateLlmRequestInput<'_>,
) -> Result<ChatRequest, String> {
    let system_prompt = render_implicit_update_system_prompt(prompt_engine, input.project_id)?;
    let dynamic_user_prompt =
        build_implicit_update_manifest_user_prompt(input.profile_name, input.attribute_key)?;
    let stable_context_json = implicit_update_stable_context(
        input.profile_name,
        input.attribute_key,
        input.current_value,
        input.stats,
        input.threshold,
        input.window_chapters,
    );
    let mut source_refs = input.source_refs_json.unwrap_or_else(|| json!({}));
    let Some(source_refs_object) = source_refs.as_object_mut() else {
        return Err("隐性属性复审 source_refs_json 必须是 JSON 对象".to_string());
    };
    source_refs_object.insert("profile_name".to_string(), json!(input.profile_name));
    source_refs_object.insert("attribute_key".to_string(), json!(input.attribute_key));
    if let Some(profile_id) = input.profile_id {
        source_refs_object.insert("profile_id".to_string(), json!(profile_id));
    }
    source_refs_object.insert(
        "evidence_count".to_string(),
        json!(input.stats.evidence.len()),
    );

    context_aware_system_user_json_llm_request(
        system_prompt,
        ContextAwareJsonLlmRequestInput {
            project_id: input.project_id,
            workflow_run_id: None,
            workflow: "implicit_update_review".to_string(),
            step_key: "implicit_update_decision".to_string(),
            scope_kind: "profile_sync".to_string(),
            pack_kind: PACK_KIND_IMPLICIT_UPDATE_REVIEW_CONTEXT.to_string(),
            source_refs_json: Some(source_refs),
            config_json: json!({
                "ledger_version": "implicit_update_review_context_v1",
                "prompt_name": IMPLICIT_UPDATE_PROMPT_NAME,
                "threshold": input.threshold,
                "window_chapters": input.window_chapters,
            }),
            stable_context_json,
            dynamic_user_prompt,
            temperature: Some(0.2),
            max_tokens_profile: llm_token_profiles::JSON_STANDARD.to_string(),
            call_key: input.call_key,
        },
    )
}

async fn finalize_implicit_update_review_workflow(
    state: &AppState,
    workflow_run_id: i64,
    result: &Result<ImplicitUpdateDecisionSummary, String>,
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

pub(crate) fn implicit_update_decision_output_json(
    parsed: &types::ParsedImplicitDecision,
) -> Value {
    json!({
        "decision": parsed.decision,
        "reasoning": parsed.reasoning,
        "suggested_new_value": parsed.suggested_new_value,
        "evidence_summary": parsed.evidence_summary,
    })
}

pub(crate) fn implicit_update_review_checkpoint_response(
    checkpoint_json: &Value,
    call_key: &str,
    stats: &ImplicitAttributeReviewSummary,
) -> Result<(String, Option<Value>, Option<String>), String> {
    let response_text = checkpoint_json
        .get("response_content")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| format!("{call_key}.response_content 必须是字符串"))?;
    let parsed = parse_implicit_update_decision(&response_text, stats)?;
    let reparsed_output = implicit_update_decision_output_json(&parsed);
    let parsed_output = checkpoint_json
        .get("parsed_output")
        .ok_or_else(|| format!("{call_key}.parsed_output 缺失"))?;
    if parsed_output != &reparsed_output {
        return Err(format!(
            "{call_key} checkpoint 的 parsed_output 与 response_content 重新解析结果不一致"
        ));
    }
    let usage = checkpoint_json
        .get("usage")
        .filter(|value| !value.is_null())
        .cloned();
    let finish_reason = match checkpoint_json.get("finish_reason") {
        Some(Value::Null) | None => None,
        Some(value) => Some(
            value
                .as_str()
                .map(ToString::to_string)
                .ok_or_else(|| format!("{call_key}.finish_reason 必须是字符串或 null"))?,
        ),
    };
    if is_truncated_finish_reason(finish_reason.as_deref()) {
        return Err(format!(
            "{call_key} checkpoint 标记为截断响应，不能用于恢复"
        ));
    }
    Ok((response_text, usage, finish_reason))
}

pub async fn preview_check_implicit_update_request(
    state: &AppState,
    user_id: i64,
    input: &CheckImplicitUpdateInput,
) -> Result<ModelRequestPreview, String> {
    let context = build_check_implicit_update_request(state, user_id, input).await?;
    Ok(model_request_preview(
        "implicit_attribute_update_review",
        "隐性属性复审 LLM 请求预览",
        vec![
            chat_request_step_for_state(
                state,
                ChatRequestStepInput {
                    key: "implicit_update_review".to_string(),
                    title: "隐性属性复审 LLM 请求".to_string(),
                    prompt_name: Some(IMPLICIT_UPDATE_PROMPT_NAME.to_string()),
                    state: "ready".to_string(),
                    note: None,
                    request: context.request,
                },
            )
            .await?,
        ],
    ))
}

pub async fn apply_implicit_update_decision(
    state: &AppState,
    user_id: i64,
    input: ApplyImplicitUpdateDecisionInput,
) -> Result<ApplyImplicitUpdateDecisionResultSummary, String> {
    let profile_id = input.decision.profile_id;
    verify_profile_ownership(state, user_id, profile_id).await?;
    let txn = state.db.begin().await.map_err(|e| e.to_string())?;
    let applied = apply_implicit_update_decision_in(
        state,
        &txn,
        profile_id,
        input.chapter_number,
        &input.decision,
    )
    .await?;
    txn.commit().await.map_err(|e| e.to_string())?;

    if let Some(event) = applied.event.as_ref() {
        ingest_attribute_event_after_write(
            state,
            applied.profile.project_id,
            &applied.profile.name,
            event,
        )
        .await?;
    }

    let ready_for_deletion = applied
        .deletion_mark
        .as_ref()
        .is_some_and(|mark| !mark.is_executed && mark.consecutive_count >= DELETION_THRESHOLD);

    Ok(ApplyImplicitUpdateDecisionResultSummary {
        action: match applied.action {
            AppliedImplicitDecisionAction::Keep => "keep",
            AppliedImplicitDecisionAction::Modify => "modify",
            AppliedImplicitDecisionAction::MarkDeletion => "mark_deletion",
        }
        .to_string(),
        event: applied.event.as_ref().map(to_event_summary),
        deletion_mark: applied.deletion_mark.as_ref().map(to_deletion_mark_summary),
        ready_for_deletion,
        threshold: DELETION_THRESHOLD,
    })
}

async fn implicit_review_context_for_user(
    state: &AppState,
    user_id: i64,
    profile_id: i64,
) -> Result<(protagonist_profile::Model, Vec<BehaviorForReview>), String> {
    let profile = require_profile(state.db.as_ref(), profile_id).await?;
    access::verify_project_editor_string(state, user_id, profile.project_id).await?;
    let records = behavior_records_for_review(state.db.as_ref(), profile_id).await?;
    Ok((profile, records))
}

async fn behavior_records_for_review<C>(
    db: &C,
    profile_id: i64,
) -> Result<Vec<BehaviorForReview>, String>
where
    C: ConnectionTrait,
{
    let records = protagonist_behavior_record::Entity::find()
        .filter(protagonist_behavior_record::Column::ProfileId.eq(profile_id))
        .order_by_desc(protagonist_behavior_record::Column::ChapterNumber)
        .order_by_desc(protagonist_behavior_record::Column::Id)
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    Ok(records.iter().map(BehaviorForReview::from).collect())
}

async fn verify_profile_ownership(
    state: &AppState,
    user_id: i64,
    profile_id: i64,
) -> Result<(), String> {
    let profile = require_profile(state.db.as_ref(), profile_id).await?;
    access::verify_project_editor_string(state, user_id, profile.project_id).await
}

fn implicit_attribute_value(
    current_state: &serde_json::Value,
    attribute_key: &str,
) -> Result<serde_json::Value, String> {
    category_attributes(current_state, "implicit")
        .and_then(|attributes| attributes.get(attribute_key))
        .cloned()
        .ok_or_else(|| format!("隐性属性不存在: {attribute_key}"))
}

fn validate_implicit_decision_for_apply(
    profile_id: i64,
    current_state: &serde_json::Value,
    decision: &ImplicitUpdateDecisionSummary,
) -> Result<(), String> {
    if decision.profile_id != profile_id {
        return Err(format!(
            "复审决策 profile_id 不匹配: expected={}, actual={}",
            profile_id, decision.profile_id
        ));
    }

    let attribute_key = decision.attribute_key.trim();
    if attribute_key.is_empty() {
        return Err("隐性属性 key 不能为空".to_string());
    }
    if decision.stats.attribute_key.trim() != attribute_key {
        return Err(format!(
            "复审统计 attribute_key 不匹配: expected={}, actual={}",
            attribute_key, decision.stats.attribute_key
        ));
    }

    let current_value = implicit_attribute_value(current_state, attribute_key)?;
    if decision.stats.current_value != decision.current_value {
        return Err(format!(
            "复审统计 current_value 与决策当前值不一致: {attribute_key}"
        ));
    }
    if current_value != decision.current_value {
        return Err(format!(
            "隐性属性 {attribute_key} 当前值已变化，请重新生成复审建议"
        ));
    }

    match decision.decision.as_str() {
        "keep" => Ok(()),
        "modify" => {
            let new_value = decision
                .suggested_new_value
                .as_ref()
                .ok_or_else(|| "复审修改建议缺少新值".to_string())?;
            if new_value.is_null() {
                return Err("复审修改建议的新值不能为 null".to_string());
            }
            if decision.reasoning.trim().is_empty() {
                return Err("复审修改建议缺少原因".to_string());
            }
            if decision.evidence_summary.trim().is_empty() {
                return Err("复审修改建议缺少证据".to_string());
            }
            Ok(())
        }
        "delete" => {
            if decision.reasoning.trim().is_empty() {
                return Err("复审删除建议缺少原因".to_string());
            }
            if decision.evidence_summary.trim().is_empty() {
                return Err("复审删除建议缺少证据".to_string());
            }
            Ok(())
        }
        other => Err(format!("未知隐性属性复审决策: {other}")),
    }
}

pub(crate) async fn apply_implicit_update_decision_in<C>(
    state: &AppState,
    db: &C,
    profile_id: i64,
    chapter_number: Option<i32>,
    decision: &ImplicitUpdateDecisionSummary,
) -> Result<AppliedImplicitDecision, String>
where
    C: ConnectionTrait,
{
    let profile = require_profile(db, profile_id).await?;
    let chapter_number = required_positive_chapter_number(chapter_number, "应用隐性属性复审")?;

    validate_implicit_decision_for_apply(profile_id, &profile.current_state, decision)?;

    match decision.decision.as_str() {
        "keep" => Ok(AppliedImplicitDecision {
            profile,
            action: AppliedImplicitDecisionAction::Keep,
            event: None,
            deletion_mark: None,
        }),
        "modify" => {
            let change_description = format!("隐性属性复审更新: {}", decision.attribute_key.trim());
            let cause = format!("隐性属性复审: {}", decision.reasoning.trim());
            let event = state
                .character
                .modify_attribute_in(
                    db,
                    a4n_services::character::AttributeValueChangeInput {
                        profile_id,
                        chapter_number,
                        category: "implicit",
                        key: decision.attribute_key.trim(),
                        value: decision
                            .suggested_new_value
                            .clone()
                            .ok_or_else(|| "复审修改建议缺少新值".to_string())?,
                        change_description: Some(&change_description),
                        cause: &cause,
                        evidence: decision.evidence_summary.trim(),
                    },
                )
                .await
                .map_err(|e| e.to_string())?;
            Ok(AppliedImplicitDecision {
                profile,
                action: AppliedImplicitDecisionAction::Modify,
                event: Some(event),
                deletion_mark: None,
            })
        }
        "delete" => {
            let mark = state
                .character
                .request_attribute_deletion_in(
                    db,
                    a4n_services::character::AttributeDeletionRequestInput {
                        profile_id,
                        category: "implicit",
                        key: decision.attribute_key.trim(),
                        chapter_number,
                        reason: decision.reasoning.trim(),
                        evidence: decision.evidence_summary.trim(),
                    },
                )
                .await
                .map_err(|e| e.to_string())?;
            Ok(AppliedImplicitDecision {
                profile,
                action: AppliedImplicitDecisionAction::MarkDeletion,
                event: None,
                deletion_mark: Some(mark),
            })
        }
        other => Err(format!("未知隐性属性复审决策: {other}")),
    }
}

fn to_event_summary(e: &attribute_event::Model) -> AttributeEventSummary {
    AttributeEventSummary {
        id: e.id,
        profile_id: e.profile_id,
        chapter_number: e.chapter_number,
        timestamp: Some(e.timestamp.to_string()),
        category: e.category.clone(),
        key: e.key.clone(),
        operation: e.operation.clone(),
        change_description: e.change_description.clone(),
        old_value: e.old_value.clone(),
        new_value: e.new_value.clone(),
        cause: e.cause.clone(),
        evidence: e.evidence.clone(),
    }
}

fn to_deletion_mark_summary(mark: &protagonist_deletion_mark::Model) -> DeletionMarkSummary {
    DeletionMarkSummary {
        id: mark.id,
        profile_id: mark.profile_id,
        category: mark.category.clone(),
        key: mark.key.clone(),
        chapter_number: mark.chapter_number,
        reason: mark.reason.clone(),
        evidence: mark.evidence.clone(),
        consecutive_count: mark.consecutive_count,
        threshold: DELETION_THRESHOLD,
        last_marked_chapter: mark.last_marked_chapter,
        is_executed: mark.is_executed,
        created_at: mark.created_at.to_string(),
        updated_at: mark.updated_at.to_string(),
    }
}

async fn ingest_attribute_event_after_write(
    state: &AppState,
    project_id: i64,
    character_name: &str,
    event: &attribute_event::Model,
) -> Result<(), String> {
    crate::workflow::ingest_character_attribute_event_rag(state, project_id, event, character_name)
        .await
        .map_err(|err| format!("角色属性事件写入后同步 RAG 失败: {err}"))
}

fn required_positive_chapter_number(value: Option<i32>, label: &str) -> Result<i32, String> {
    let chapter_number = value.ok_or_else(|| format!("{label}缺少章节号"))?;
    if chapter_number <= 0 {
        return Err(format!("{label}章节号必须大于 0"));
    }
    Ok(chapter_number)
}

async fn require_profile<C>(db: &C, profile_id: i64) -> Result<protagonist_profile::Model, String>
where
    C: ConnectionTrait,
{
    protagonist_profile::Entity::find_by_id(profile_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("角色档案不存在: {profile_id}"))
}

fn category_attributes<'a>(
    value: &'a serde_json::Value,
    category: &str,
) -> Option<&'a serde_json::Map<String, serde_json::Value>> {
    let key = match category {
        "explicit" => "explicit_attributes",
        "implicit" => "implicit_attributes",
        "social" => "social_attributes",
        other => other,
    };
    value.get(key)?.as_object()
}

#[cfg(test)]
mod tests {
    use super::*;
    use types::{ImplicitAttributeReviewSummary, ImplicitEvidenceSummary};

    fn review_stats() -> ImplicitAttributeReviewSummary {
        ImplicitAttributeReviewSummary {
            attribute_key: "谨慎".to_string(),
            current_value: json!("遇事先观察"),
            total: 1,
            conform_count: 0,
            non_conform_count: 1,
            conform_rate: 0.0,
            threshold_reached: true,
            evidence: vec![ImplicitEvidenceSummary {
                behavior_id: 11,
                chapter_number: 3,
                behavior_type: "decision".to_string(),
                behavior_description: "主动推进调查".to_string(),
                original_text: "她没有继续等待。".to_string(),
                behavior_tags: json!(["主动"]),
                classification: json!("non-conform"),
                created_at: None,
            }],
        }
    }

    #[test]
    fn implicit_update_review_checkpoint_revalidates_response_content() {
        let checkpoint = json!({
            "response_content": r#"{"decision":"modify","reasoning":"连续行为显示更主动","new_value":"主动谨慎","supporting_behavior_ids":[11]}"#,
            "parsed_output": {
                "decision": "modify",
                "reasoning": "连续行为显示更主动",
                "suggested_new_value": "主动谨慎",
                "evidence_summary": "行为11 / 第3章：主动推进调查（non-conform）"
            },
            "usage": null,
            "finish_reason": "stop",
        });

        let (response_text, usage, finish_reason) = implicit_update_review_checkpoint_response(
            &checkpoint,
            "implicit_update_review:1:谨慎",
            &review_stats(),
        )
        .expect("valid checkpoint should restore");

        assert!(response_text.contains("\"decision\":\"modify\""));
        assert_eq!(usage, None);
        assert_eq!(finish_reason.as_deref(), Some("stop"));
    }

    #[test]
    fn implicit_update_review_checkpoint_rejects_mismatched_parsed_output() {
        let checkpoint = json!({
            "response_content": r#"{"decision":"modify","reasoning":"连续行为显示更主动","new_value":"主动谨慎","supporting_behavior_ids":[11]}"#,
            "parsed_output": {
                "decision": "keep",
                "reasoning": "连续行为显示更主动",
                "suggested_new_value": null,
                "evidence_summary": "行为11 / 第3章：主动推进调查（non-conform）"
            },
            "usage": null,
            "finish_reason": "stop",
        });

        let err = implicit_update_review_checkpoint_response(
            &checkpoint,
            "implicit_update_review:1:谨慎",
            &review_stats(),
        )
        .expect_err("checkpoint parsed output must match reparsed response");

        assert!(err.contains("重新解析结果不一致"));
    }
}
