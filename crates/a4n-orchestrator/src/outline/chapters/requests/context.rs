use a4n_core::error::AppError;
use a4n_core::models::chapter::part_outline;
use serde_json::{Value, json};

use crate::outline::context::{ChapterOutlineGenerationContext, part_context_json};
use a4n_core::source_spans::source_text_hash as source_content_sha256;
use a4n_llm::context::LlmContextManifest;
use a4n_llm::context::{
    COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedger,
    LlmContextLedgerPackDraft,
};

pub(super) fn build_chapter_outline_context_manifest(
    context: &ChapterOutlineGenerationContext,
    dynamic_user_content: String,
) -> Result<LlmContextManifest, AppError> {
    let mut ledger = LlmContextLedger::new();
    let blueprint_hash = source_content_sha256(&serde_json::to_string(&context.blueprint_payload)?);
    ledger.add_pack(a4n_llm::context::LlmContextLedgerPackInput {
        scope_kind: ("project").into(),
        pack_kind: ("outline_blueprint_context").into(),
        source_refs_json: Some(json!({
            "source": "novel_blueprint",
            "content_sha256": blueprint_hash,
        })),
        config_identity_json: json!({
            "ledger_version": "outline_chapter_context_v1",
            "kind": "blueprint_context",
        }),
        required: true,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FULL_RAW,
                json!({
                    "kind": "blueprint_context",
                    "blueprint": context.blueprint_payload,
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FOCUSED_RAW,
                json!({
                    "kind": "blueprint_context",
                    "blueprint": context.blueprint_payload,
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_MINIMAL,
                json!({
                    "kind": "blueprint_context",
                    "blueprint": minimal_chapter_blueprint_context(&context.blueprint_payload),
                }),
            ),
        ],
    })?;

    let part_context = context
        .parts
        .iter()
        .map(part_context_json)
        .collect::<Vec<_>>();
    let part_hash = source_content_sha256(&serde_json::to_string(&part_context)?);
    ledger.add_pack(a4n_llm::context::LlmContextLedgerPackInput {
        scope_kind: ("project").into(),
        pack_kind: ("outline_part_outlines").into(),
        source_refs_json: Some(json!({
            "source": "part_outline",
            "content_sha256": part_hash,
            "part_numbers": context.parts.iter().map(|part| part.part_number).collect::<Vec<_>>(),
        })),
        config_identity_json: json!({
            "ledger_version": "outline_chapter_context_v1",
            "kind": "part_outlines",
        }),
        required: true,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FULL_RAW,
                json!({
                    "kind": "part_outlines",
                    "parts": part_context,
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FOCUSED_RAW,
                json!({
                    "kind": "part_outlines",
                    "parts": context.parts.iter().map(focused_chapter_part_context).collect::<Vec<_>>(),
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_MINIMAL,
                json!({
                    "kind": "part_outlines",
                    "parts": context.parts.iter().map(minimal_chapter_part_context).collect::<Vec<_>>(),
                }),
            ),
        ],
    })?;

    Ok(ledger.into_manifest(
        dynamic_user_content,
        Some("章节大纲必须返回严格 JSON 对象；根对象必须且只能包含 chapters。".to_string()),
    ))
}

fn minimal_chapter_blueprint_context(blueprint_context: &Value) -> Value {
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

fn focused_chapter_part_context(part: &part_outline::Model) -> Value {
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

fn minimal_chapter_part_context(part: &part_outline::Model) -> Value {
    json!({
        "part_number": part.part_number,
        "title": part.title,
        "start_chapter": part.start_chapter,
        "end_chapter": part.end_chapter,
        "summary": part.summary,
    })
}
