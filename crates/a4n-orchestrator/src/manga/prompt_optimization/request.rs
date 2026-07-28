use std::collections::HashMap;

use a4n_core::{
    config::app_config::llm_token_profiles, error::AppError, models::chapter::chapter_manga_prompt,
    source_spans::source_text_hash as source_content_sha256,
};
use a4n_llm::{
    context::{
        COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedger,
        LlmContextLedgerPackDraft, context_aware_system_user_request,
    },
    provider::{ChatRequest, LlmTraceContext},
};
use a4n_prompt::rendering::PromptRenderer;
use serde_json::{Value, json};

pub(super) const MANGA_PROMPT_OPTIMIZATION_PROMPT_NAME: &str = "manga_prompt_optimization";
pub(super) const MANGA_PROMPT_OPTIMIZATION_USER_PROMPT_NAME: &str =
    "manga_prompt_optimization_user";

pub(super) fn manga_prompt_optimization_call_key(prompt_id: i64) -> String {
    format!("optimize_manga_prompt:prompt:{prompt_id}")
}

pub(super) async fn build_manga_prompt_optimization_request(
    prompt_renderer: &dyn PromptRenderer,
    prompt: &chapter_manga_prompt::Model,
    instruction: &str,
) -> Result<ChatRequest, AppError> {
    let system_prompt = prompt_renderer
        .render_prompt(
            MANGA_PROMPT_OPTIMIZATION_PROMPT_NAME,
            &HashMap::new(),
            Some(prompt.project_id),
        )
        .await?;
    let dynamic_payload = json!({
        "context_refs": {"current_prompt": "稳定上下文包 source_artifact.current_prompt"},
        "instruction": instruction,
        "output_contract": {
            "format": "json_object",
            "required_fields": ["image_prompt"],
            "field_rules": {"image_prompt": "优化后的整页漫画绘图提示词，不能为空"},
        },
    });
    let mut user_variables = HashMap::new();
    user_variables.insert(
        "payload".to_string(),
        json!(serde_json::to_string_pretty(&dynamic_payload)?),
    );
    let user_prompt = prompt_renderer
        .render_prompt(
            MANGA_PROMPT_OPTIMIZATION_USER_PROMPT_NAME,
            &user_variables,
            Some(prompt.project_id),
        )
        .await?;
    let current_prompt = manga_prompt_optimization_source_artifact(prompt);
    let source_hash = source_content_sha256(
        &serde_json::to_string(&current_prompt).map_err(AppError::Serialization)?,
    );
    let mut ledger = LlmContextLedger::new();
    ledger.add_pack(a4n_llm::context::LlmContextLedgerPackInput {
        scope_kind: "prompt_optimization".into(),
        pack_kind: "source_artifact".into(),
        source_refs_json: Some(json!({
            "source": "chapter_manga_prompt",
            "prompt_id": prompt.id,
            "content_sha256": source_hash,
        })),
        config_identity_json: json!({
            "ledger_version": "manga_prompt_optimization_context_v1",
            "artifact_kind": "chapter_manga_prompt",
        }),
        required: true,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(COMPRESSION_FULL_RAW, current_prompt.clone()),
            LlmContextLedgerPackDraft::new(COMPRESSION_FOCUSED_RAW, current_prompt.clone()),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_MINIMAL,
                json!({
                    "artifact_kind": "chapter_manga_prompt",
                    "prompt_id": prompt.id,
                    "page_number": prompt.page_number,
                    "panel_number": prompt.panel_number,
                    "prompt_type": prompt.prompt_type,
                    "image_prompt": prompt.image_prompt,
                }),
            ),
        ],
    })?;
    let manifest = ledger.into_manifest(
        user_prompt,
        Some("漫画提示词优化必须保留稳定上下文包 source_artifact 中的剧情、角色、构图、参考图和风格约束，只按动态输入中的 instruction 修改表达。".to_string()),
    );
    let fingerprint = source_content_sha256(
        &serde_json::to_string(&json!({
            "prompt_id": prompt.id,
            "source_artifact_sha256": source_hash,
            "instruction": instruction,
            "prompt_names": [
                MANGA_PROMPT_OPTIMIZATION_PROMPT_NAME,
                MANGA_PROMPT_OPTIMIZATION_USER_PROMPT_NAME,
            ],
        }))
        .map_err(AppError::Serialization)?,
    );
    Ok(context_aware_system_user_request(
        system_prompt,
        manifest,
        Some(0.25),
        None,
        Some(llm_token_profiles::MEDIA_PROMPT.to_string()),
    )?
    .with_json_response_format()
    .with_trace_context(LlmTraceContext {
        project_id: Some(prompt.project_id),
        root_project_id: None,
        workflow: "manga_prompt_optimization".to_string(),
        step_key: "optimize_manga_prompt".to_string(),
        generation_run_id: None,
        workflow_run_id: None,
        call_key: Some(manga_prompt_optimization_call_key(prompt.id)),
        input_fingerprint_sha256: Some(fingerprint),
        context_plan_json: None,
        context_pack_refs_json: None,
    }))
}

pub(super) fn manga_prompt_optimization_input_fingerprint(
    panel: &chapter_manga_prompt::Model,
    instruction: &str,
    request_json: Value,
) -> Value {
    json!({
        "prompt_names": [
            MANGA_PROMPT_OPTIMIZATION_PROMPT_NAME,
            MANGA_PROMPT_OPTIMIZATION_USER_PROMPT_NAME,
        ],
        "request": request_json,
        "input_variables": {
            "instruction": instruction,
            "prompt_id": panel.id,
        },
        "source_refs": {
            "previous_prompt_id": panel.id,
            "previous_image_prompt": &panel.image_prompt,
            "dialogue_text": &panel.dialogue_text,
            "camera_angle": &panel.camera_angle,
            "dialogue_position": &panel.dialogue_position,
            "layout": &panel.layout,
            "characters_in_frame": &panel.characters_in_frame,
        },
    })
}

fn manga_prompt_optimization_source_artifact(prompt: &chapter_manga_prompt::Model) -> Value {
    json!({
        "artifact_kind": "chapter_manga_prompt",
        "current_prompt": {
            "prompt_id": prompt.id,
            "page_number": prompt.page_number,
            "panel_number": prompt.panel_number,
            "prompt_type": prompt.prompt_type,
            "dialogue_text": prompt.dialogue_text,
            "camera_angle": prompt.camera_angle,
            "dialogue_position": prompt.dialogue_position,
            "image_prompt": prompt.image_prompt,
        },
    })
}
