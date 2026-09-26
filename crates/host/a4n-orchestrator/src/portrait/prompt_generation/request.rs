use std::collections::HashMap;

use a4n_error::AppError;
use a4n_llm_provider_contract::provider::{ChatRequest, LlmTraceContext};
use serde_json::{Value, json};

use a4n_domain::source_text_hash as source_content_sha256;
use a4n_llm_context::{
    COMPRESSION_FULL_RAW, LlmContextLedger, LlmContextLedgerPackDraft, LlmContextLedgerPackInput,
    context_aware_system_user_request,
};

use a4n_api_contract::portrait::PortraitType;

use super::contracts::PortraitPromptRequestInput;
use super::payload::{
    PortraitPromptPayloadInput, optional_prompt_text, portrait_prompt_payload_value,
};

pub async fn build_generate_prompt_request(
    input: PortraitPromptRequestInput<'_>,
) -> Result<ChatRequest, AppError> {
    let PortraitPromptRequestInput {
        project_id,
        spec,
        prompt_renderer,
        variables,
    } = input;
    let prompt_content = prompt_renderer
        .render_prompt("character_portrait_generation", variables, Some(project_id))
        .await?;

    let type_desc = portrait_type_description(spec.portrait_type);
    let dynamic_payload = portrait_prompt_payload_value(PortraitPromptPayloadInput {
        spec,
        portrait_type_text: type_desc,
    })?;
    let mut user_variables = HashMap::new();
    user_variables.insert(
        "payload_json".to_string(),
        Value::String(serde_json::to_string_pretty(&dynamic_payload)?),
    );
    let user_prompt = prompt_renderer
        .render_prompt(
            "character_portrait_generation_user",
            &user_variables,
            Some(project_id),
        )
        .await?;
    let stable_context = json!({
        "profile_name": spec.profile_name,
        "appearance": optional_prompt_text(spec.appearance),
        "gender": optional_prompt_text(spec.gender),
        "age": optional_prompt_text(spec.age),
        "personality": optional_prompt_text(spec.personality),
        "background": optional_prompt_text(spec.background),
        "profile_context": spec.profile_context.cloned().unwrap_or_else(|| json!({})),
        "project_context": spec.project_context.cloned().unwrap_or_else(|| json!({})),
    });
    let context_hash = source_content_sha256(&serde_json::to_string(&stable_context)?);
    let mut ledger = LlmContextLedger::new();
    ledger.add_pack(LlmContextLedgerPackInput {
        scope_kind: ("portrait_prompt_generation").into(),
        pack_kind: ("portrait_profile_project_context").into(),
        source_refs_json: Some(json!({
            "profile_name": spec.profile_name,
            "content_sha256": context_hash,
        })),
        config_identity_json: json!({
            "ledger_version": "portrait_prompt_generation_context_v1",
        }),
        required: true,
        stable_prefix: true,
        variants: vec![LlmContextLedgerPackDraft::new(
            COMPRESSION_FULL_RAW,
            stable_context,
        )],
    })?;
    let manifest = ledger.into_manifest(
            user_prompt,
            Some("角色画像提示词必须基于稳定上下文包 portrait_profile_project_context，输出只能是可直接用于绘图模型的中文提示词正文。".to_string()),
        );
    let fingerprint = source_content_sha256(&serde_json::to_string(&json!({
        "profile_name": spec.profile_name,
        "context_sha256": context_hash,
        "portrait_type": spec.portrait_type,
        "art_style": optional_prompt_text(spec.art_style),
        "expression": optional_prompt_text(spec.expression),
        "existing_portrait_count": spec.existing_portrait_count,
    }))?);

    Ok(
        context_aware_system_user_request(prompt_content, manifest, Some(0.8), None)?
            .with_trace_context(LlmTraceContext {
                project_id: Some(project_id),
                root_project_id: None,
                workflow: "portrait_prompt_generation".to_string(),
                step_key: "character_portrait_generation".to_string(),
                generation_run_id: None,
                workflow_run_id: None,
                call_key: Some(format!(
                    "portrait_prompt:{}:{}",
                    spec.profile_name, spec.portrait_type
                )),
                input_fingerprint_sha256: Some(fingerprint),
                context_plan_json: None,
                context_pack_refs_json: None,
            }),
    )
}

pub(super) fn portrait_type_description(portrait_type: PortraitType) -> &'static str {
    match portrait_type {
        PortraitType::FullBody => {
            "人物三视图与拆解设定图（正面、侧面、背面，全身，含服饰配饰拆解）"
        }
        PortraitType::HalfBody => {
            "人物三视图与拆解设定图（正面、侧面、背面，重点拆解服饰层级与配饰细节）"
        }
        PortraitType::Face => "人物三视图与拆解设定图（含面部与发型拆解参考）",
        PortraitType::Chibi => "Q版人物三视图与拆解设定图",
    }
}
