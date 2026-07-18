use std::collections::HashMap;

use a4n_core::error::AppError;

use super::super::super::super::contracts::{
    LlmContextManifest, LlmContextPackRef, LlmContextPackRequirement,
};
use super::super::super::super::planner::{canonical_json_text, estimate_context_tokens};
use super::PreviewPackContentKey;
use a4n_core::source_spans::source_text_hash as source_content_sha256;

pub(super) struct ManifestRequirementInputs {
    pub(super) requirements: Vec<LlmContextPackRequirement>,
    pub(super) content_by_key: HashMap<PreviewPackContentKey, String>,
}

pub(super) fn manifest_requirement_inputs(
    manifest: &LlmContextManifest,
) -> Result<ManifestRequirementInputs, AppError> {
    let mut requirements = Vec::with_capacity(manifest.packs.len());
    let mut content_by_key = HashMap::new();
    for pack in &manifest.packs {
        let mut variants = Vec::with_capacity(pack.variants.len());
        for variant in &pack.variants {
            let canonical_content_text = canonical_json_text(&variant.canonical_content_json)?;
            let pack_ref = LlmContextPackRef {
                pack_kind: pack.pack_kind.clone(),
                content_sha256: source_content_sha256(&canonical_content_text),
                compression_level: variant.compression_level.clone(),
                token_estimate: estimate_context_tokens(&canonical_content_text),
            };
            content_by_key.insert(
                (
                    pack_ref.pack_kind.clone(),
                    pack_ref.compression_level.clone(),
                    pack_ref.content_sha256.clone(),
                ),
                canonical_content_text,
            );
            variants.push(pack_ref);
        }
        requirements.push(LlmContextPackRequirement {
            pack_kind: pack.pack_kind.clone(),
            required: pack.required,
            stable_prefix: pack.stable_prefix,
            variants,
        });
    }
    Ok(ManifestRequirementInputs {
        requirements,
        content_by_key,
    })
}
