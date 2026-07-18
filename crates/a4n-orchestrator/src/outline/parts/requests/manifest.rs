use a4n_core::error::AppError;
use a4n_core::models::chapter::part_outline;
use serde_json::{Value, json};

use a4n_core::source_spans::source_text_hash as source_content_sha256;
use a4n_llm::context::LlmContextManifest;
use a4n_llm::context::{
    COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedger,
    LlmContextLedgerPackDraft,
};

use super::super::super::context::part_context_json;
use super::super::super::part_contract::PART_GENERATION_OUTPUT_CONTRACT;

pub(super) fn build_part_outline_context_manifest(
    blueprint_context: &Value,
    previous_parts: &[part_outline::Model],
    dynamic_user_content: String,
    part_num: i32,
    start_ch: i32,
    end_ch: i32,
) -> Result<LlmContextManifest, AppError> {
    let mut ledger = LlmContextLedger::new();
    let blueprint_hash = source_content_sha256(&serde_json::to_string(blueprint_context)?);
    ledger.add_pack(a4n_llm::context::LlmContextLedgerPackInput {
        scope_kind: ("project").into(),
        pack_kind: ("outline_blueprint_context").into(),
        source_refs_json: Some(json!({
            "source": "novel_blueprint",
            "content_sha256": blueprint_hash,
        })),
        config_identity_json: json!({
            "ledger_version": "outline_part_context_v1",
            "kind": "blueprint_context",
        }),
        required: true,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FULL_RAW,
                json!({
                    "kind": "blueprint_context",
                    "blueprint": blueprint_context,
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FOCUSED_RAW,
                json!({
                    "kind": "blueprint_context",
                    "blueprint": blueprint_context,
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_MINIMAL,
                json!({
                    "kind": "blueprint_context",
                    "blueprint": minimal_outline_blueprint_context(blueprint_context),
                }),
            ),
        ],
    })?;

    if !previous_parts.is_empty() {
        let previous_part_context = previous_parts
            .iter()
            .map(part_context_json)
            .collect::<Vec<_>>();
        let previous_hash = source_content_sha256(&serde_json::to_string(&previous_part_context)?);
        ledger.add_pack(a4n_llm::context::LlmContextLedgerPackInput {
            scope_kind: ("project").into(),
            pack_kind: ("outline_previous_part_outlines").into(),
            source_refs_json: Some(json!({
                "source": "part_outline",
                "content_sha256": previous_hash,
                "part_numbers": previous_parts.iter().map(|part| part.part_number).collect::<Vec<_>>(),
            })),
            config_identity_json: json!({
                "ledger_version": "outline_part_context_v1",
                "kind": "previous_part_outlines",
            }),
            required: true,
            stable_prefix: true,
            variants: vec![
                LlmContextLedgerPackDraft::new(
                    COMPRESSION_FULL_RAW,
                    json!({
                        "kind": "previous_part_outlines",
                        "parts": previous_part_context,
                    }),
                ),
                LlmContextLedgerPackDraft::new(
                    COMPRESSION_FOCUSED_RAW,
                    json!({
                        "kind": "previous_part_outlines",
                        "parts": previous_parts
                            .iter()
                            .map(focused_part_context)
                            .collect::<Vec<_>>(),
                    }),
                ),
                LlmContextLedgerPackDraft::new(
                    COMPRESSION_MINIMAL,
                    json!({
                        "kind": "previous_part_outlines",
                        "parts": previous_parts
                            .iter()
                            .map(minimal_part_context)
                            .collect::<Vec<_>>(),
                    }),
                ),
            ],
        })?;
    }

    Ok(ledger.into_manifest(
        dynamic_user_content,
        Some(format!(
            "第 {part_num} 分部（第 {start_ch}-{end_ch} 章）必须返回严格 JSON 对象。{}",
            PART_GENERATION_OUTPUT_CONTRACT
        )),
    ))
}

fn minimal_outline_blueprint_context(blueprint_context: &Value) -> Value {
    let Some(object) = blueprint_context.as_object() else {
        return blueprint_context.clone();
    };
    let mut minimal = serde_json::Map::new();
    for key in [
        "title",
        "genre",
        "style",
        "tone",
        "one_sentence_summary",
        "full_synopsis",
        "chapters_per_part",
        "blueprint_chapters_per_part",
    ] {
        if let Some(value) = object.get(key) {
            minimal.insert(key.to_string(), value.clone());
        }
    }
    Value::Object(minimal)
}

fn focused_part_context(part: &part_outline::Model) -> Value {
    json!({
        "part_number": part.part_number,
        "title": part.title,
        "start_chapter": part.start_chapter,
        "end_chapter": part.end_chapter,
        "summary": part.summary,
        "theme": part.theme,
        "key_events": part.key_events,
    })
}

fn minimal_part_context(part: &part_outline::Model) -> Value {
    json!({
        "part_number": part.part_number,
        "title": part.title,
        "start_chapter": part.start_chapter,
        "end_chapter": part.end_chapter,
        "summary": part.summary,
    })
}
