use crate::actions::{
    character_implicit_review::{
        AUTO_IMPLICIT_REVIEW_EVIDENCE_LIMIT, AUTO_IMPLICIT_REVIEW_THRESHOLD,
        AUTO_IMPLICIT_REVIEW_WINDOW_CHAPTERS, BehaviorForReview, ImplicitUpdateDecisionSummary,
        ImplicitUpdateLlmRequestInput, build_implicit_review_summary,
        build_implicit_update_request, implicit_update_decision_output_json,
        implicit_update_review_checkpoint_response, parse_implicit_update_decision,
        validate_implicit_review_params,
    },
    llm_request::PACK_KIND_IMPLICIT_CLASSIFICATION_CONTEXT,
    llm_request::{ContextAwareJsonLlmRequestInput, context_aware_system_user_json_llm_request},
};
use crate::state::AppState;
use a4n_core::config::app_config::llm_token_profiles;
use a4n_core::models::character::{protagonist_behavior_record, protagonist_profile};
use a4n_engine::llm::provider::is_truncated_finish_reason;
use a4n_services::generation::context_aware_chat_request_trace_json;
use a4n_services::llm_checkpoint::{
    CompletedLlmCallCheckpointInput, llm_call_checkpoint_fingerprint_sha256,
    load_completed_llm_call_checkpoint, required_llm_call_checkpoint_json,
    required_llm_call_workflow_run_id, save_completed_llm_call_checkpoint,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde_json::{Value, json};
use std::collections::{BTreeSet, HashMap};

use super::implicit_classification::{
    apply_implicit_classification_to_behavior, build_implicit_classification_manifest_user_prompt,
    implicit_classification_stable_context, parse_implicit_classification_output,
    set_behavior_classification_results,
};
use super::json_utils::{
    category_attributes, required_category, required_i32, required_string, required_value,
};
use super::state_projection::{apply_sync_change_to_state, project_attribute_changes};

pub(super) async fn classify_behaviors(
    state: &AppState,
    project_id: i64,
    workflow_run_id: Option<i64>,
    profile_name: &str,
    changes: &[Value],
    behaviors: &mut [Value],
    final_attributes: &mut Value,
) -> Result<Vec<Value>, String> {
    if behaviors.is_empty() {
        *final_attributes = project_attribute_changes(final_attributes.clone(), changes)?;
        return Ok(Vec::new());
    }

    let system_prompt = state
        .engine
        .render("implicit_classification", &HashMap::new(), Some(project_id))
        .map_err(|e| e.to_string())?;
    let mut change_index = 0usize;
    let mut suggestions = Vec::new();
    let mut suggested_keys = BTreeSet::new();

    for (behavior_index, behavior) in behaviors.iter_mut().enumerate() {
        let chapter = required_i32(behavior, "chapter_number")?;
        while change_index < changes.len()
            && required_i32(&changes[change_index], "chapter_number")? <= chapter
        {
            apply_sync_change_to_state(final_attributes, &changes[change_index])?;
            change_index += 1;
        }

        let implicit_attributes = category_attributes(final_attributes, "implicit")?;
        if implicit_attributes
            .as_object()
            .is_none_or(serde_json::Map::is_empty)
        {
            set_behavior_classification_results(behavior, json!({}))?;
            continue;
        }

        let user_prompt =
            build_implicit_classification_manifest_user_prompt(profile_name, behavior)?;
        let stable_context = implicit_classification_stable_context(&implicit_attributes)?;
        let call_key = format!("implicit_classification:{profile_name}:{chapter}:{behavior_index}");
        let workflow_run_id = required_llm_call_workflow_run_id(workflow_run_id, &call_key)
            .map_err(|err| err.to_string())?;
        let mut request = context_aware_system_user_json_llm_request(
            system_prompt.clone(),
            ContextAwareJsonLlmRequestInput {
                project_id,
                workflow_run_id: Some(workflow_run_id),
                workflow: "implicit_classification".to_string(),
                step_key: "behavior_classification".to_string(),
                scope_kind: "profile_sync".to_string(),
                pack_kind: PACK_KIND_IMPLICIT_CLASSIFICATION_CONTEXT.to_string(),
                source_refs_json: Some(json!({
                    "profile_name": profile_name,
                    "behavior_index": behavior_index,
                    "chapter_number": chapter,
                })),
                config_json: json!({
                    "ledger_version": "implicit_classification_context_v1",
                    "prompt_name": "implicit_classification",
                }),
                stable_context_json: stable_context,
                dynamic_user_prompt: user_prompt,
                temperature: Some(0.2),
                max_tokens_profile: llm_token_profiles::JSON_STANDARD.to_string(),
                call_key: Some(call_key.clone()),
            },
        )?;
        let request_json = context_aware_chat_request_trace_json(&request)
            .map_err(|err| format!("行为隐性属性分类 LLM 请求序列化失败: {err}"))?;
        let input_fingerprint_json = json!({
            "request": request_json,
            "profile_name": profile_name,
            "behavior_index": behavior_index,
            "chapter_number": chapter,
            "implicit_attributes": implicit_attributes,
            "behavior": &*behavior,
        });
        let input_fingerprint_sha256 =
            llm_call_checkpoint_fingerprint_sha256(&input_fingerprint_json)
                .map_err(|err| err.to_string())?;
        if let Some(trace_context) = request.trace_context.as_mut() {
            trace_context.workflow_run_id = Some(workflow_run_id);
            trace_context.call_key = Some(call_key.clone());
            trace_context.input_fingerprint_sha256 = Some(input_fingerprint_sha256);
        }
        state
            .workflow_run
            .mark_current_step(workflow_run_id, &call_key)
            .await
            .map_err(|err| err.to_string())?;

        let (response_text, response_usage_json, response_finish_reason, restored_parsed_output) =
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
                let response_text = required_sync_implicit_checkpoint_string(
                    &checkpoint_json,
                    &call_key,
                    "response_content",
                )?;
                let finish_reason =
                    optional_sync_implicit_checkpoint_string(&checkpoint_json, "finish_reason")?;
                if is_truncated_finish_reason(finish_reason.as_deref()) {
                    return Err(
                        "行为隐性属性分类 checkpoint 标记为截断响应，不能用于恢复".to_string()
                    );
                }
                let parsed_output = checkpoint_json
                    .get("parsed_output")
                    .cloned()
                    .ok_or_else(|| format!("{call_key}.parsed_output 缺失"))?;
                let usage = checkpoint_json
                    .get("usage")
                    .filter(|value| !value.is_null())
                    .cloned();
                (response_text, usage, finish_reason, Some(parsed_output))
            } else {
                let response = state.llm.chat(request).await.map_err(|e| e.to_string())?;
                if is_truncated_finish_reason(response.finish_reason.as_deref()) {
                    return Err("行为隐性属性分类输出达到当前 max_tokens 上限，返回 JSON 被截断；请在设置页调大“JSON 标准”输出档位，或减少本次分类上下文后重试".to_string());
                }
                let usage = response
                    .usage
                    .as_ref()
                    .map(serde_json::to_value)
                    .transpose()
                    .map_err(|err| format!("行为隐性属性分类 LLM 用量序列化失败: {err}"))?;
                (
                    response.message.content,
                    usage,
                    response.finish_reason,
                    None,
                )
            };
        let parsed = parse_implicit_classification_output(&response_text, &implicit_attributes)
            .map_err(|err| format!("行为隐性属性分类输出不符合契约：{err}"))?;
        if let Some(parsed_output) = restored_parsed_output {
            if parsed_output != parsed {
                return Err(format!(
                    "{call_key} checkpoint 的 parsed_output 与 response_content 重新解析结果不一致"
                ));
            }
        } else {
            save_completed_llm_call_checkpoint(
                state.db.clone(),
                CompletedLlmCallCheckpointInput {
                    workflow_run_id,
                    call_key: call_key.clone(),
                    call_order: 1000 + behavior_index as i32,
                    input_fingerprint_json: input_fingerprint_json.clone(),
                    checkpoint_json: json!({
                        "output": parsed.clone(),
                        "parsed_output": parsed.clone(),
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
        suggestions.extend(apply_implicit_classification_to_behavior(
            behavior,
            &parsed,
            final_attributes,
            &mut suggested_keys,
            behavior_index,
        )?);
    }

    while change_index < changes.len() {
        apply_sync_change_to_state(final_attributes, &changes[change_index])?;
        change_index += 1;
    }

    Ok(suggestions)
}

pub(super) async fn decide_implicit_reviews(
    state: &AppState,
    workflow_run_id: Option<i64>,
    profile: &protagonist_profile::Model,
    start_chapter: i32,
    final_attributes: &Value,
    behaviors: &[Value],
    deletion_candidates: &[Value],
) -> Result<Vec<ImplicitUpdateDecisionSummary>, String> {
    let (threshold, window_chapters, evidence_limit) = validate_implicit_review_params(
        AUTO_IMPLICIT_REVIEW_THRESHOLD,
        AUTO_IMPLICIT_REVIEW_WINDOW_CHAPTERS,
        AUTO_IMPLICIT_REVIEW_EVIDENCE_LIMIT,
    )?;
    let records = behavior_records_for_review(state, profile.id, start_chapter, behaviors).await?;
    if records.is_empty() {
        return Ok(Vec::new());
    }

    let excluded_keys = implicit_deletion_candidate_keys(deletion_candidates)?;
    let review = build_implicit_review_summary(
        profile.id,
        final_attributes,
        &records,
        threshold,
        window_chapters,
        evidence_limit,
    )?;
    let mut decisions = Vec::new();
    for stats in review
        .attributes
        .into_iter()
        .filter(|stats| stats.threshold_reached && !excluded_keys.contains(&stats.attribute_key))
    {
        let call_key = format!(
            "implicit_update_review:{}:{}:{}",
            profile.id, start_chapter, stats.attribute_key
        );
        let workflow_run_id = required_llm_call_workflow_run_id(workflow_run_id, &call_key)
            .map_err(|err| err.to_string())?;
        let mut request = build_implicit_update_request(
            state.engine.as_ref(),
            ImplicitUpdateLlmRequestInput {
                project_id: profile.project_id,
                profile_id: Some(profile.id),
                profile_name: &profile.name,
                attribute_key: &stats.attribute_key,
                current_value: &stats.current_value,
                stats: &stats,
                threshold,
                window_chapters,
                source_refs_json: Some(json!({
                    "trigger": "character_sync_auto_review",
                    "profile_id": profile.id,
                    "start_chapter": start_chapter,
                    "attribute_key": &stats.attribute_key,
                    "window_chapters": window_chapters,
                    "evidence_limit": evidence_limit,
                })),
                call_key: Some(call_key.clone()),
            },
        )?;
        let request_json = context_aware_chat_request_trace_json(&request)
            .map_err(|err| format!("隐性属性更新决策 LLM 请求序列化失败: {err}"))?;
        let input_fingerprint_json = json!({
            "request": request_json,
            "profile_id": profile.id,
            "start_chapter": start_chapter,
            "attribute_key": &stats.attribute_key,
            "stats": &stats,
            "threshold": threshold,
            "window_chapters": window_chapters,
            "evidence_limit": evidence_limit,
        });
        let input_fingerprint_sha256 =
            llm_call_checkpoint_fingerprint_sha256(&input_fingerprint_json)
                .map_err(|err| err.to_string())?;
        if let Some(trace_context) = request.trace_context.as_mut() {
            trace_context.workflow_run_id = Some(workflow_run_id);
            trace_context.call_key = Some(call_key.clone());
            trace_context.input_fingerprint_sha256 = Some(input_fingerprint_sha256);
        }
        state
            .workflow_run
            .mark_current_step(workflow_run_id, &call_key)
            .await
            .map_err(|err| err.to_string())?;

        let (response_text, response_usage_json, response_finish_reason, restored) = if let Some(
            checkpoint,
        ) =
            load_completed_llm_call_checkpoint(
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
                implicit_update_review_checkpoint_response(&checkpoint_json, &call_key, &stats)?;
            (response_text, usage, finish_reason, true)
        } else {
            let response = state.llm.chat(request).await.map_err(|e| e.to_string())?;
            if is_truncated_finish_reason(response.finish_reason.as_deref()) {
                return Err("隐性属性更新决策输出达到当前 max_tokens 上限，返回 JSON 被截断；请在设置页调大“JSON 标准”输出档位，或减少本次决策上下文后重试".to_string());
            }
            let usage = response
                .usage
                .as_ref()
                .map(serde_json::to_value)
                .transpose()
                .map_err(|err| format!("隐性属性更新决策 LLM 用量序列化失败: {err}"))?;
            (
                response.message.content,
                usage,
                response.finish_reason,
                false,
            )
        };
        let parsed = parse_implicit_update_decision(&response_text, &stats)?;
        if !restored {
            save_completed_llm_call_checkpoint(
                state.db.clone(),
                CompletedLlmCallCheckpointInput {
                    workflow_run_id,
                    call_key: call_key.clone(),
                    call_order: 2000 + decisions.len() as i32,
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
        decisions.push(ImplicitUpdateDecisionSummary {
            profile_id: profile.id,
            attribute_key: stats.attribute_key.clone(),
            current_value: stats.current_value.clone(),
            decision: parsed.decision,
            reasoning: parsed.reasoning,
            suggested_new_value: parsed.suggested_new_value,
            evidence_summary: parsed.evidence_summary,
            stats,
        });
    }

    Ok(decisions)
}

async fn behavior_records_for_review(
    state: &AppState,
    profile_id: i64,
    start_chapter: i32,
    behaviors: &[Value],
) -> Result<Vec<BehaviorForReview>, String> {
    let mut pending = behaviors
        .iter()
        .enumerate()
        .map(|(index, behavior)| behavior_for_review_from_sync(index, behavior))
        .collect::<Result<Vec<_>, _>>()?;
    pending.sort_by(|left, right| {
        right
            .chapter_number
            .cmp(&left.chapter_number)
            .then_with(|| left.behavior_id.cmp(&right.behavior_id))
    });

    let existing = protagonist_behavior_record::Entity::find()
        .filter(protagonist_behavior_record::Column::ProfileId.eq(profile_id))
        .filter(protagonist_behavior_record::Column::ChapterNumber.lt(start_chapter))
        .order_by_desc(protagonist_behavior_record::Column::ChapterNumber)
        .order_by_desc(protagonist_behavior_record::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    pending.extend(existing.iter().map(BehaviorForReview::from));
    Ok(pending)
}

fn behavior_for_review_from_sync(
    index: usize,
    behavior: &Value,
) -> Result<BehaviorForReview, String> {
    let behavior_id = -((index as i64) + 1);
    Ok(BehaviorForReview {
        behavior_id,
        chapter_number: required_i32(behavior, "chapter_number")?,
        behavior_type: required_string(behavior, "behavior_type")?,
        behavior_description: required_string(behavior, "behavior_description")?,
        original_text: required_string(behavior, "original_text")?,
        behavior_tags: required_value(behavior, "behavior_tags")?.clone(),
        classification_results: required_value(behavior, "classification_results")?.clone(),
        created_at: None,
    })
}

fn implicit_deletion_candidate_keys(
    deletion_candidates: &[Value],
) -> Result<BTreeSet<String>, String> {
    let mut keys = BTreeSet::new();
    for candidate in deletion_candidates {
        let category = required_category(candidate, "category")?;
        if category == "implicit" {
            keys.insert(required_string(candidate, "key")?);
        }
    }
    Ok(keys)
}

fn required_sync_implicit_checkpoint_string(
    checkpoint_json: &Value,
    call_key: &str,
    field: &str,
) -> Result<String, String> {
    checkpoint_json
        .get(field)
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| format!("{call_key}.{field} 必须是字符串"))
}

fn optional_sync_implicit_checkpoint_string(
    checkpoint_json: &Value,
    field: &str,
) -> Result<Option<String>, String> {
    match checkpoint_json.get(field) {
        Some(Value::Null) | None => Ok(None),
        Some(value) => value
            .as_str()
            .map(ToString::to_string)
            .map(Some)
            .ok_or_else(|| format!("{field} 必须是字符串或 null")),
    }
}
