use std::collections::HashMap;

use a4n_error::AppError;
use a4n_llm::{
    context::{
        ContextAwareJsonLlmRequestInput, PACK_KIND_IMPLICIT_UPDATE_REVIEW_CONTEXT,
        context_aware_system_user_json_llm_request,
    },
    trace::context_aware_chat_request_trace_json,
};
use a4n_prompt::{rendering::PromptRenderer, request::build_json_task_user_message};
use serde_json::{Value, json};

use super::{
    IMPLICIT_UPDATE_PROMPT_NAME, IMPLICIT_UPDATE_STEP_KEY, IMPLICIT_UPDATE_WORKFLOW_KIND,
    contracts::{
        ImplicitAttributeReviewSummary, ImplicitUpdateLlmRequestInput,
        PreparedImplicitUpdateRequest,
    },
};
use crate::character::implicit_review_workflow_rules::implicit_update_call_key;

pub(super) async fn build_implicit_update_request(
    prompt_renderer: &dyn PromptRenderer,
    input: ImplicitUpdateLlmRequestInput<'_>,
) -> Result<PreparedImplicitUpdateRequest, AppError> {
    let call_key =
        implicit_update_call_key(input.profile_id, input.attribute_key, input.start_chapter)
            .map_err(AppError::InvalidArgument)?;
    let system_prompt = prompt_renderer
        .render_prompt(
            IMPLICIT_UPDATE_PROMPT_NAME,
            &HashMap::new(),
            Some(input.project_id),
        )
        .await?;
    let dynamic_user_prompt = build_json_task_user_message(
        "判断隐性属性是否需要更新",
        "只返回 output_contract 要求的严格 JSON 对象，不要输出 Markdown、代码块、解释文字或额外字段",
        &implicit_update_payload(
            input.profile_name,
            input.attribute_key,
            json!({
                "context_refs": {
                    "attribute_key": format!("稳定上下文包 {PACK_KIND_IMPLICIT_UPDATE_REVIEW_CONTEXT}.attribute_key"),
                    "current_value": format!("稳定上下文包 {PACK_KIND_IMPLICIT_UPDATE_REVIEW_CONTEXT}.current_value"),
                    "review_window": format!("稳定上下文包 {PACK_KIND_IMPLICIT_UPDATE_REVIEW_CONTEXT}.review_window"),
                    "statistics": format!("稳定上下文包 {PACK_KIND_IMPLICIT_UPDATE_REVIEW_CONTEXT}.statistics"),
                    "evidence": format!("稳定上下文包 {PACK_KIND_IMPLICIT_UPDATE_REVIEW_CONTEXT}.evidence"),
                }
            }),
        ),
    )?;
    let stable_context_json = implicit_update_stable_context(
        input.profile_name,
        input.attribute_key,
        input.stats,
        input.threshold,
        input.window_chapters,
    );
    let request = context_aware_system_user_json_llm_request(
        system_prompt,
        ContextAwareJsonLlmRequestInput {
            project_id: input.project_id,
            workflow_run_id: None,
            workflow: IMPLICIT_UPDATE_WORKFLOW_KIND.to_string(),
            step_key: IMPLICIT_UPDATE_STEP_KEY.to_string(),
            scope_kind: "profile_sync".to_string(),
            pack_kind: PACK_KIND_IMPLICIT_UPDATE_REVIEW_CONTEXT.to_string(),
            source_refs_json: Some(json!({
                "trigger": input.trigger,
                "profile_id": input.profile_id,
                "profile_name": input.profile_name,
                "attribute_key": input.attribute_key,
                "start_chapter": input.start_chapter,
            })),
            config_json: json!({
                "ledger_version": "implicit_update_review_context_v1",
                "prompt_name": IMPLICIT_UPDATE_PROMPT_NAME,
                "threshold": input.threshold,
                "window_chapters": input.window_chapters,
            }),
            stable_context_json,
            dynamic_user_prompt,
            temperature: Some(0.2),
            call_key: Some(call_key.clone()),
        },
    )
    .map_err(AppError::InvalidArgument)?;
    let request_json = context_aware_chat_request_trace_json(&request)?;
    Ok(PreparedImplicitUpdateRequest {
        call_key,
        input_fingerprint_json: json!({
            "request": request_json,
            "profile_id": input.profile_id,
            "attribute_key": input.attribute_key,
            "start_chapter": input.start_chapter,
            "stats": input.stats,
            "threshold": input.threshold,
            "window_chapters": input.window_chapters,
            "evidence_limit": input.evidence_limit,
        }),
        request,
    })
}

fn implicit_update_stable_context(
    profile_name: &str,
    attribute_key: &str,
    stats: &ImplicitAttributeReviewSummary,
    threshold: usize,
    window_chapters: usize,
) -> Value {
    json!({
        "profile_name": profile_name,
        "attribute_key": attribute_key,
        "current_value": stats.current_value,
        "review_window": {
            "window_chapters": window_chapters,
            "non_conform_threshold": threshold,
        },
        "statistics": {
            "total": stats.conform_count + stats.non_conform_count,
            "conform_count": stats.conform_count,
            "non_conform_count": stats.non_conform_count,
            "conform_rate": stats.conform_rate,
            "threshold_reached": stats.threshold_reached,
        },
        "evidence": stats.evidence.iter().map(|item| json!({
            "behavior_id": item.behavior_id,
            "chapter_number": item.chapter_number,
            "behavior_type": item.behavior_type,
            "behavior_description": item.behavior_description,
            "original_text": item.original_text,
            "behavior_tags": item.behavior_tags,
            "classification": item.classification,
        })).collect::<Vec<_>>(),
    })
}

fn implicit_update_payload(profile_name: &str, attribute_key: &str, input: Value) -> Value {
    json!({
        "task": {
            "name": IMPLICIT_UPDATE_WORKFLOW_KIND,
            "instruction": "判断隐性属性是否需要更新；只根据输入 evidence 和统计结果做 keep/modify/delete 决策",
        },
        "character": { "profile_name": profile_name },
        "target_attribute": { "key": attribute_key },
        "input": input,
        "hard_rules": [
            "supporting_behavior_ids 只能从 evidence[].behavior_id 中选择，不能自造 ID",
            "输入 evidence 非空时 supporting_behavior_ids 至少包含 1 个 ID",
            "keep/delete 的 new_value 必须为 null",
            "modify 的 new_value 必须非 null，且应概括 evidence 中一致的新行为模式",
            "不要输出 evidence_summary，系统会根据 supporting_behavior_ids 回填证据摘要",
        ],
        "output_contract": {
            "format": "json_object",
            "required_fields": {
                "decision": "字符串，只能是 keep / modify / delete",
                "reasoning": "非空字符串，说明判断依据并引用章节号、行为描述或 behavior_id",
                "new_value": "modify 时为新隐性属性值；keep/delete 时为 null",
                "supporting_behavior_ids": "整数数组，只能来自 evidence[].behavior_id",
            },
            "forbidden": [
                "Markdown 代码块",
                "解释性前后缀",
                "evidence_summary",
                "输入 evidence 外的 behavior_id",
            ],
        },
    })
}
