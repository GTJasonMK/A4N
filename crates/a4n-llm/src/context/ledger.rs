mod draft;
mod hash;
mod validation;

use super::contracts::{
    CONTEXT_MANIFEST_SCHEMA_VERSION, LlmContextLedgerPackInput, LlmContextManifest,
    LlmContextManifestPack,
};
use a4n_core::error::AppError;

#[derive(Debug, Default, Clone)]
pub struct LlmContextLedger {
    packs: Vec<LlmContextManifestPack>,
}

impl LlmContextLedger {
    pub fn new() -> Self {
        Self { packs: Vec::new() }
    }

    pub fn add_pack(&mut self, input: LlmContextLedgerPackInput) -> Result<(), AppError> {
        let pack_kind = input.pack_kind;
        let variants = validation::validate_variants(&pack_kind, input.variants)?;
        let config_hash = hash::context_config_hash(&pack_kind, &input.config_identity_json)?;
        self.packs.push(LlmContextManifestPack {
            scope_kind: input.scope_kind,
            pack_kind,
            source_refs_json: input.source_refs_json,
            config_hash,
            required: input.required,
            stable_prefix: input.stable_prefix,
            variants,
        });
        Ok(())
    }

    pub fn into_manifest(
        self,
        dynamic_user_content: impl Into<String>,
        output_contract: Option<String>,
    ) -> LlmContextManifest {
        LlmContextManifest {
            schema_version: CONTEXT_MANIFEST_SCHEMA_VERSION,
            packs: self.packs,
            dynamic_user_content: dynamic_user_content.into(),
            output_contract,
        }
    }
}
