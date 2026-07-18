use std::collections::{BTreeSet, HashMap};

use a4n_core::error::AppError;
use serde_json::{Value, json};

use super::{
    COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedger, LlmContextLedgerPackDraft,
    PromptRenderer, VIDEO_BUILDER_RULES_TEMPLATE, VIDEO_CONTEXT_LEDGER_VERSION,
    VIDEO_CONTEXT_SCOPE, source_content_sha256,
};

pub(super) async fn add_builder_rules_pack(
    ledger: &mut LlmContextLedger,
    prompt_renderer: &dyn PromptRenderer,
    project_id: i64,
) -> Result<(), AppError> {
    let rendered = prompt_renderer
        .render_prompt(
            VIDEO_BUILDER_RULES_TEMPLATE,
            &HashMap::new(),
            Some(project_id),
        )
        .await?;
    let rendered_sha256 = source_content_sha256(&rendered);
    let rules = serde_json::from_str::<Value>(&rendered).map_err(|err| {
        AppError::ConfigError(format!(
            "视频 builder 规则模板必须渲染为合法 JSON 对象：{err}"
        ))
    })?;
    ledger.add_pack(a4n_llm::context::LlmContextLedgerPackInput {
        scope_kind: (VIDEO_CONTEXT_SCOPE).into(),
        pack_kind: ("video_workflow_rules").into(),
        source_refs_json: Some(json!({"prompt_name": VIDEO_BUILDER_RULES_TEMPLATE})),
        config_identity_json: json!({
            "ledger_version": VIDEO_CONTEXT_LEDGER_VERSION,
            "prompt_name": VIDEO_BUILDER_RULES_TEMPLATE,
            "rendered_sha256": rendered_sha256
        }),
        required: true,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FULL_RAW,
                json!({
                    "prompt_name": VIDEO_BUILDER_RULES_TEMPLATE,
                    "rules": rules
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_MINIMAL,
                json!({
                    "prompt_name": VIDEO_BUILDER_RULES_TEMPLATE,
                    "rendered_sha256": rendered_sha256
                }),
            ),
        ],
    })
}

pub(super) async fn add_prompt_rule_hash_pack(
    ledger: &mut LlmContextLedger,
    prompt_renderer: &dyn PromptRenderer,
    project_id: i64,
    variables: &HashMap<String, Value>,
    prompt_names: &[&str],
) -> Result<(), AppError> {
    let builder_rules = prompt_renderer
        .render_prompt(
            VIDEO_BUILDER_RULES_TEMPLATE,
            &HashMap::new(),
            Some(project_id),
        )
        .await?;
    let mut prompts = Vec::new();
    prompts.push(json!({
        "prompt_name": VIDEO_BUILDER_RULES_TEMPLATE,
        "rendered_sha256": source_content_sha256(&builder_rules)
    }));
    let mut seen = BTreeSet::new();
    for prompt_name in prompt_names {
        if !seen.insert(*prompt_name) {
            continue;
        }
        let rendered = prompt_renderer
            .render_prompt(prompt_name, variables, Some(project_id))
            .await?;
        prompts.push(json!({
            "prompt_name": prompt_name,
            "rendered_sha256": source_content_sha256(&rendered)
        }));
    }
    ledger.add_pack(a4n_llm::context::LlmContextLedgerPackInput {
        scope_kind: (VIDEO_CONTEXT_SCOPE).into(),
        pack_kind: ("video_prompt_rule_hashes").into(),
        source_refs_json: Some(json!({
            "prompt_names": prompts
                .iter()
                .filter_map(|prompt| prompt.get("prompt_name").and_then(Value::as_str))
                .collect::<Vec<_>>()
        })),
        config_identity_json: json!({
            "ledger_version": VIDEO_CONTEXT_LEDGER_VERSION,
            "kind": "prompt_rule_hashes",
            "prompt_names": prompts
                .iter()
                .filter_map(|prompt| prompt.get("prompt_name").and_then(Value::as_str))
                .collect::<Vec<_>>()
        }),
        required: true,
        stable_prefix: true,
        variants: vec![LlmContextLedgerPackDraft::new(
            COMPRESSION_MINIMAL,
            json!({
                "kind": "prompt_rule_hashes",
                "prompts": prompts
            }),
        )],
    })
}

pub(super) async fn add_prompt_rule_packs(
    ledger: &mut LlmContextLedger,
    prompt_renderer: &dyn PromptRenderer,
    project_id: i64,
    variables: &HashMap<String, Value>,
    prompt_names: &[&str],
) -> Result<(), AppError> {
    let mut seen = BTreeSet::new();
    for prompt_name in prompt_names {
        if !seen.insert(*prompt_name) {
            continue;
        }
        let rendered = prompt_renderer
            .render_prompt(prompt_name, variables, Some(project_id))
            .await?;
        let rendered_sha256 = source_content_sha256(&rendered);
        let pack_kind = format!("video_prompt_rules:{prompt_name}");
        ledger.add_pack(a4n_llm::context::LlmContextLedgerPackInput {
            scope_kind: (VIDEO_CONTEXT_SCOPE).into(),
            pack_kind,
            source_refs_json: Some(json!({"prompt_name": prompt_name})),
            config_identity_json: json!({
                "ledger_version": VIDEO_CONTEXT_LEDGER_VERSION,
                "prompt_name": prompt_name,
                "rendered_sha256": rendered_sha256
            }),
            required: true,
            stable_prefix: true,
            variants: vec![
                LlmContextLedgerPackDraft::new(
                    COMPRESSION_FULL_RAW,
                    json!({
                        "prompt_name": prompt_name,
                        "rendered_prompt": rendered
                    }),
                ),
                LlmContextLedgerPackDraft::new(
                    COMPRESSION_MINIMAL,
                    json!({
                        "prompt_name": prompt_name,
                        "rendered_sha256": rendered_sha256
                    }),
                ),
            ],
        })?;
    }
    Ok(())
}
