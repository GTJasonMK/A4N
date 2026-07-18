use std::collections::{HashMap, HashSet};

use a4n_core::error::AppError;
use serde_json::{Value, json};

use a4n_core::source_spans::source_text_hash as source_content_sha256;
use a4n_llm::context::{
    COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedger, LlmContextLedgerPackDraft,
    LlmContextLedgerPackInput,
};
use a4n_prompt::rendering::PromptRenderer;

pub(in crate::manga::pipeline_prompt::manifest) async fn add_manga_prompt_rule_packs(
    ledger: &mut LlmContextLedger,
    prompt_renderer: &dyn PromptRenderer,
    project_id: i64,
    variables: &HashMap<String, Value>,
    prompt_names: &[&str],
) -> Result<(), AppError> {
    let mut seen = HashSet::new();
    for prompt_name in prompt_names {
        if !seen.insert(*prompt_name) {
            continue;
        }
        let rendered = prompt_renderer
            .render_prompt(prompt_name, variables, Some(project_id))
            .await?;
        let rendered_sha256 = source_content_sha256(&rendered);
        ledger.add_pack(LlmContextLedgerPackInput {
            scope_kind: ("chapter").into(),
            pack_kind: format!("manga_prompt_rules:{prompt_name}"),
            source_refs_json: Some(json!({"prompt_name": prompt_name})),
            config_identity_json: json!({
                "ledger_version": "manga_context_ledger_v1",
                "kind": "prompt_rules",
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
