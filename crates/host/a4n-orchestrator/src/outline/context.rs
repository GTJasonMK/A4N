mod contracts;
mod loading;
mod projections;

use a4n_domain::source_text_hash;
use a4n_error::AppError;
use a4n_llm_context::{COMPRESSION_FULL_RAW, LlmContextLedgerPackDraft, LlmContextLedgerPackInput};
use serde_json::{Value, json};

pub(in crate::outline) use contracts::{
    ChapterOutlineGenerationContext, PartOutlineGenerationContext,
};
pub(in crate::outline) use loading::with_locked_outline_generation_sources;
pub(in crate::outline) use loading::{
    load_chapter_outline_generation_context, load_part_generation_context,
};
pub(in crate::outline) use projections::part_context_json;
pub(crate) use projections::{chapter_outline_context_json, outline_formal_materials_context_json};

#[cfg(test)]
pub(in crate::outline) use projections::{
    has_outline_generation_source, outline_blueprint_context_json,
};

pub(in crate::outline) fn blueprint_context_pack(
    context: &Value,
) -> Result<LlmContextLedgerPackInput, AppError> {
    let blueprint_available = context["blueprint_available"].as_bool().ok_or_else(|| {
        AppError::InvalidArgument("大纲来源上下文必须明确 blueprint_available".into())
    })?;
    Ok(LlmContextLedgerPackInput {
        scope_kind: "project".into(),
        pack_kind: "outline_blueprint_context".into(),
        source_refs_json: Some(json!({
            "source": if blueprint_available {
                "novel_blueprint_and_formal_materials"
            } else {
                "formal_project_materials"
            },
            "blueprint_available": blueprint_available,
            "content_sha256": source_text_hash(&serde_json::to_string(context)?),
        })),
        config_identity_json: json!({
            "ledger_version": "outline_blueprint_context_v2",
            "kind": "blueprint_context",
        }),
        required: true,
        stable_prefix: true,
        variants: vec![LlmContextLedgerPackDraft::new(
            COMPRESSION_FULL_RAW,
            json!({"kind": "blueprint_context", "blueprint": context}),
        )],
    })
}
