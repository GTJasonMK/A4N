use std::collections::HashMap;

use a4n_domain::llm::llm_token_profiles;
use a4n_error::AppError;
use a4n_llm::{
    context::{
        ContextAwareJsonLlmRequestInput, PACK_KIND_IMPLICIT_CLASSIFICATION_CONTEXT,
        context_aware_system_user_json_llm_request,
    },
    provider::ChatRequest,
    trace::context_aware_chat_request_trace_json,
};
use a4n_prompt::{rendering::PromptRenderer, request::build_json_task_user_message};
use serde_json::{Value, json};

use crate::{
    character::{required_string, required_value},
    workflow,
};

use super::{
    IMPLICIT_CLASSIFICATION_CONTEXT_LEDGER_VERSION, IMPLICIT_CLASSIFICATION_KIND,
    IMPLICIT_CLASSIFICATION_STEP_KEY,
};

pub(super) struct PreparedImplicitClassificationCall {
    pub request: ChatRequest,
    pub call_key: String,
    pub workflow_run_id: i64,
    pub input_fingerprint_json: Value,
}

pub(super) async fn prepare_implicit_classification_call(
    prompt_renderer: &dyn PromptRenderer,
    project_id: i64,
    workflow_run_id: i64,
    profile_id: i64,
    profile_name: &str,
    behavior_index: usize,
    chapter: i32,
    behavior: &Value,
    implicit_attributes: &Value,
) -> Result<PreparedImplicitClassificationCall, AppError> {
    let system_prompt = prompt_renderer
        .render_prompt(
            IMPLICIT_CLASSIFICATION_KIND,
            &HashMap::new(),
            Some(project_id),
        )
        .await?;
    let user_prompt =
        build_implicit_classification_manifest_user_prompt(profile_name, chapter, behavior)
            .map_err(AppError::InvalidArgument)?;
    let stable_context = json!({
        "current_implicit_attributes": implicit_attributes,
    });
    let call_key =
        format!("{IMPLICIT_CLASSIFICATION_KIND}:{profile_id}:{chapter}:{behavior_index}");
    let mut request = context_aware_system_user_json_llm_request(
        system_prompt,
        ContextAwareJsonLlmRequestInput {
            project_id,
            workflow_run_id: Some(workflow_run_id),
            workflow: IMPLICIT_CLASSIFICATION_KIND.to_string(),
            step_key: IMPLICIT_CLASSIFICATION_STEP_KEY.to_string(),
            scope_kind: "profile_sync".to_string(),
            pack_kind: PACK_KIND_IMPLICIT_CLASSIFICATION_CONTEXT.to_string(),
            source_refs_json: Some(json!({
                "profile_id": profile_id,
                "profile_name": profile_name,
                "behavior_index": behavior_index,
                "chapter_number": chapter,
            })),
            config_json: json!({
                "ledger_version": IMPLICIT_CLASSIFICATION_CONTEXT_LEDGER_VERSION,
                "prompt_name": IMPLICIT_CLASSIFICATION_KIND,
            }),
            stable_context_json: stable_context.clone(),
            dynamic_user_prompt: user_prompt,
            temperature: Some(0.2),
            max_tokens_profile: llm_token_profiles::JSON_STANDARD.to_string(),
            call_key: Some(call_key.clone()),
        },
    )
    .map_err(AppError::InvalidArgument)?;
    let request_json = context_aware_chat_request_trace_json(&request)?;
    let input_fingerprint_json = json!({
        "request": request_json,
        "stable_context": stable_context,
    });
    workflow::attach_llm_call_checkpoint_trace(
        &mut request,
        workflow_run_id,
        &call_key,
        &input_fingerprint_json,
    )?;

    Ok(PreparedImplicitClassificationCall {
        request,
        call_key,
        workflow_run_id,
        input_fingerprint_json,
    })
}

fn build_implicit_classification_manifest_user_prompt(
    profile_name: &str,
    chapter: i32,
    behavior: &Value,
) -> Result<String, String> {
    let behavior_type = required_string(behavior, "behavior_type")?;
    let description = required_string(behavior, "behavior_description")?;
    let original_text = required_string(behavior, "original_text")?;
    let behavior_tags = required_value(behavior, "behavior_tags")?;
    build_json_task_user_message(
        "执行行为隐性属性分类",
        "只返回 output_contract 要求的严格 JSON 对象，不要输出 Markdown、代码块、解释文字或额外字段",
        &implicit_classification_payload(
            profile_name,
            json!({
                "context_refs": {
                    "current_implicit_attributes": format!("稳定上下文包 {PACK_KIND_IMPLICIT_CLASSIFICATION_CONTEXT}.current_implicit_attributes"),
                },
                "behavior": {
                    "chapter_number": chapter,
                    "behavior_type": behavior_type,
                    "behavior_description": description,
                    "behavior_tags": behavior_tags,
                    "original_text": original_text,
                },
            }),
        ),
    )
    .map_err(|error| format!("行为隐性属性分类上下文序列化失败: {error}"))
}

fn implicit_classification_payload(profile_name: &str, input: Value) -> Value {
    json!({
        "task": {
            "name": IMPLICIT_CLASSIFICATION_KIND,
            "instruction": "判断行为是否符合角色当前已有的隐性属性；如发现新隐性属性，只能放入 suggested_new_attributes",
        },
        "character": { "profile_name": profile_name },
        "input": input,
        "hard_rules": [
            "classifications 的每个 key 必须逐字来自 current_implicit_attributes 对象的 key，不能自行创造 key",
            "行为体现的新隐性属性必须写入 suggested_new_attributes，绝不能作为 classifications key",
            "suggested_new_attributes 每项只能输出 key 和 value；不要输出 evidence，证据由系统使用当前 behavior.original_text 回填",
            "没有任何当前隐性属性适用时，classifications 必须是 {}",
        ],
        "output_contract": {
            "format": "json_object",
            "required_fields": {
                "classifications": "对象，key 只能来自 current_implicit_attributes；value 只能是 conform 或 non-conform",
                "reasoning": "字符串",
                "suggested_new_attributes": "数组，可为空；每项必须且只能包含 key / value",
            },
        },
    })
}
