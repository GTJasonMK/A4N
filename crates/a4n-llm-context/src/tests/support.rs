use super::super::*;
use serde_json::json;

pub(super) fn pack_ref(kind: &str, level: &str, tokens: usize) -> LlmContextPackRef {
    LlmContextPackRef {
        pack_kind: kind.to_string(),
        content_sha256: format!("{kind}-hash"),
        compression_level: level.to_string(),
        token_estimate: tokens,
    }
}

pub(super) fn requirement(
    kind: &str,
    level: &str,
    tokens: usize,
    required: bool,
) -> LlmContextPackRequirement {
    LlmContextPackRequirement {
        pack_kind: kind.to_string(),
        required,
        stable_prefix: true,
        variants: vec![pack_ref(kind, level, tokens)],
    }
}

fn manifest_pack(kind: &str) -> LlmContextManifestPack {
    LlmContextManifestPack {
        scope_kind: "chapter".to_string(),
        pack_kind: kind.to_string(),
        source_refs_json: None,
        config_hash: format!("{kind}:config"),
        required: true,
        stable_prefix: true,
        variants: vec![LlmContextManifestPackVariant {
            compression_level: COMPRESSION_FULL_RAW.to_string(),
            canonical_content_json: json!({"kind": kind, "content": "上下文内容"}),
        }],
    }
}

pub(super) fn manifest_with_pack(dynamic_user_content: &str) -> LlmContextManifest {
    LlmContextManifest {
        schema_version: CONTEXT_MANIFEST_SCHEMA_VERSION,
        packs: vec![manifest_pack("chapter_source")],
        dynamic_user_content: dynamic_user_content.to_string(),
        output_contract: None,
    }
}
