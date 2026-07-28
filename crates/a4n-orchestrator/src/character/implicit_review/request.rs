use std::collections::HashMap;

use a4n_core::{config::app_config::llm_token_profiles, error::AppError};
use a4n_llm::{
    context::{
        ContextAwareJsonLlmRequestInput, PACK_KIND_IMPLICIT_UPDATE_REVIEW_CONTEXT,
        context_aware_system_user_json_llm_request,
    },
    provider::ChatRequest,
};
use a4n_prompt::{rendering::PromptRenderer, request::build_json_task_user_message};
use serde_json::{Value, json};

use super::{
    IMPLICIT_UPDATE_PROMPT_NAME,
    contracts::{ImplicitAttributeReviewSummary, ImplicitUpdateLlmRequestInput},
};

pub(super) async fn build_implicit_update_request(
    prompt_renderer: &dyn PromptRenderer,
    input: ImplicitUpdateLlmRequestInput<'_>,
) -> Result<ChatRequest, AppError> {
    let system_prompt = prompt_renderer
        .render_prompt(
            IMPLICIT_UPDATE_PROMPT_NAME,
            &HashMap::new(),
            Some(input.project_id),
        )
        .await?;
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
    let mut source_refs = input.source_refs_json;
    let Some(source_refs_object) = source_refs.as_object_mut() else {
        return Err(AppError::InvalidArgument(
            "隐性属性复审 source_refs_json 必须是 JSON 对象".to_string(),
        ));
    };
    source_refs_object.insert("profile_name".to_string(), json!(input.profile_name));
    source_refs_object.insert("attribute_key".to_string(), json!(input.attribute_key));
    source_refs_object.insert("profile_id".to_string(), json!(input.profile_id));
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
            call_key: Some(input.call_key),
        },
    )
    .map_err(AppError::InvalidArgument)
}

fn build_implicit_update_manifest_user_prompt(
    profile_name: &str,
    attribute_key: &str,
) -> Result<String, AppError> {
    build_implicit_update_prompt_from_payload(implicit_update_payload(
        profile_name,
        attribute_key,
        json!({
            "context_refs": {
                "attribute_key": format!("稳定上下文包 {PACK_KIND_IMPLICIT_UPDATE_REVIEW_CONTEXT}.attribute_key"),
                "current_value": format!("稳定上下文包 {PACK_KIND_IMPLICIT_UPDATE_REVIEW_CONTEXT}.current_value"),
                "review_window": format!("稳定上下文包 {PACK_KIND_IMPLICIT_UPDATE_REVIEW_CONTEXT}.review_window"),
                "statistics": format!("稳定上下文包 {PACK_KIND_IMPLICIT_UPDATE_REVIEW_CONTEXT}.statistics"),
                "evidence": format!("稳定上下文包 {PACK_KIND_IMPLICIT_UPDATE_REVIEW_CONTEXT}.evidence"),
            }
        }),
    ))
}

fn implicit_update_stable_context(
    profile_name: &str,
    attribute_key: &str,
    current_value: &Value,
    stats: &ImplicitAttributeReviewSummary,
    threshold: usize,
    window_chapters: usize,
) -> Value {
    json!({
        "profile_name": profile_name,
        "attribute_key": attribute_key,
        "current_value": current_value,
        "review_window": {
            "window_chapters": window_chapters,
            "non_conform_threshold": threshold,
        },
        "statistics": {
            "total": stats.total,
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
            "name": "implicit_attribute_update_review",
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
                "reasoning": "字符串，100-200 字，引用章节号、行为描述或 behavior_id",
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

fn build_implicit_update_prompt_from_payload(payload: Value) -> Result<String, AppError> {
    build_json_task_user_message(
        "判断隐性属性是否需要更新",
        "只返回 output_contract 要求的严格 JSON 对象，不要输出 Markdown、代码块、解释文字或额外字段",
        &payload,
    )
}

#[cfg(test)]
pub(super) fn build_implicit_update_user_prompt(
    profile_name: &str,
    attribute_key: &str,
    current_value: &Value,
    stats: &ImplicitAttributeReviewSummary,
    threshold: usize,
    window_chapters: usize,
) -> Result<String, AppError> {
    build_implicit_update_prompt_from_payload(implicit_update_payload(
        profile_name,
        attribute_key,
        implicit_update_stable_context(
            profile_name,
            attribute_key,
            current_value,
            stats,
            threshold,
            window_chapters,
        ),
    ))
}
