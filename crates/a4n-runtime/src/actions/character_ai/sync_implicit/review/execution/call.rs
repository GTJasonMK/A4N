use crate::{
    actions::{
        character_implicit_review::{
            ImplicitAttributeReviewSummary, ImplicitUpdateDecisionSummary,
            ImplicitUpdateLlmRequestInput, build_implicit_update_request_for_state,
            implicit_update_decision_output_json, implicit_update_review_checkpoint_response,
            parse_implicit_update_decision,
        },
        workflow_checkpoint::{
            CompletedLlmCallCheckpointInput, llm_call_checkpoint_fingerprint_sha256,
            required_llm_call_checkpoint_json, required_llm_call_workflow_run_id,
        },
    },
    state::AppState,
};
use a4n_core::models::character::protagonist_profile;
use a4n_llm::{provider::is_truncated_finish_reason, trace::context_aware_chat_request_trace_json};
use serde_json::json;

pub(super) struct ImplicitReviewCallInput {
    pub(super) workflow_run_id: Option<i64>,
    pub(super) start_chapter: i32,
    pub(super) threshold: usize,
    pub(super) window_chapters: usize,
    pub(super) evidence_limit: usize,
    pub(super) call_index: usize,
}

pub(super) async fn run_implicit_review_call(
    state: &AppState,
    profile: &protagonist_profile::Model,
    stats: ImplicitAttributeReviewSummary,
    input: ImplicitReviewCallInput,
) -> Result<ImplicitUpdateDecisionSummary, String> {
    let call_key = format!(
        "implicit_update_review:{}:{}:{}",
        profile.id, input.start_chapter, stats.attribute_key
    );
    let workflow_run_id = required_llm_call_workflow_run_id(input.workflow_run_id, &call_key)
        .map_err(|err| err.to_string())?;
    let mut request = build_implicit_update_request_for_state(
        state,
        ImplicitUpdateLlmRequestInput {
            project_id: profile.project_id,
            profile_id: Some(profile.id),
            profile_name: &profile.name,
            attribute_key: &stats.attribute_key,
            current_value: &stats.current_value,
            stats: &stats,
            threshold: input.threshold,
            window_chapters: input.window_chapters,
            source_refs_json: Some(json!({
                "trigger": "character_sync_auto_review",
                "profile_id": profile.id,
                "start_chapter": input.start_chapter,
                "attribute_key": &stats.attribute_key,
                "window_chapters": input.window_chapters,
                "evidence_limit": input.evidence_limit,
            })),
            call_key: Some(call_key.clone()),
        },
    )
    .await?;
    let request_json = context_aware_chat_request_trace_json(&request)
        .map_err(|err| format!("隐性属性更新决策 LLM 请求序列化失败: {err}"))?;
    let input_fingerprint_json = json!({
        "request": request_json,
        "profile_id": profile.id,
        "start_chapter": input.start_chapter,
        "attribute_key": &stats.attribute_key,
        "stats": &stats,
        "threshold": input.threshold,
        "window_chapters": input.window_chapters,
        "evidence_limit": input.evidence_limit,
    });
    let input_fingerprint_sha256 = llm_call_checkpoint_fingerprint_sha256(&input_fingerprint_json)
        .map_err(|err| err.to_string())?;
    if let Some(trace_context) = request.trace_context.as_mut() {
        trace_context.workflow_run_id = Some(workflow_run_id);
        trace_context.call_key = Some(call_key.clone());
        trace_context.input_fingerprint_sha256 = Some(input_fingerprint_sha256);
    }
    state
        .orchestrator
        .mark_workflow_current_step(workflow_run_id, &call_key)
        .await
        .map_err(|err| err.to_string())?;

    let (response_text, response_usage_json, response_finish_reason, restored) = if let Some(
        checkpoint,
    ) = state
        .load_completed_llm_call_checkpoint(workflow_run_id, &call_key, &input_fingerprint_json)
        .await
        .map_err(|err| err.to_string())?
    {
        let checkpoint_json = required_llm_call_checkpoint_json(checkpoint, &call_key)
            .map_err(|err| err.to_string())?;
        let (response_text, usage, finish_reason) =
            implicit_update_review_checkpoint_response(&checkpoint_json, &call_key, &stats)?;
        (response_text, usage, finish_reason, true)
    } else {
        let response = state
            .orchestrator
            .chat_llm(request)
            .await
            .map_err(|err| err.to_string())?;
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
        let call_order = i32::try_from(input.call_index)
            .map_err(|_| format!("隐性属性更新决策数量超过 i32 范围：{}", input.call_index))?
            .checked_add(2000)
            .ok_or_else(|| {
                format!(
                    "隐性属性更新决策 call_order 超过 i32 范围：{}",
                    input.call_index
                )
            })?;
        state
            .save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
                workflow_run_id,
                call_key,
                call_order,
                input_fingerprint_json,
                checkpoint_json: json!({
                    "output": implicit_update_decision_output_json(&parsed),
                    "parsed_output": implicit_update_decision_output_json(&parsed),
                    "response_content": response_text,
                    "usage": response_usage_json,
                    "finish_reason": response_finish_reason,
                }),
                artifacts_json: None,
            })
            .await
            .map_err(|err| err.to_string())?;
    }
    Ok(ImplicitUpdateDecisionSummary {
        profile_id: profile.id,
        attribute_key: stats.attribute_key.clone(),
        current_value: stats.current_value.clone(),
        decision: parsed.decision,
        reasoning: parsed.reasoning,
        suggested_new_value: parsed.suggested_new_value,
        evidence_summary: parsed.evidence_summary,
        stats,
    })
}
