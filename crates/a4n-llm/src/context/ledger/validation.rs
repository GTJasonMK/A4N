use super::super::contracts::{LlmContextLedgerPackDraft, LlmContextManifestPackVariant};
use a4n_core::error::AppError;
use std::collections::BTreeSet;

pub(super) fn validate_variants(
    pack_kind: &str,
    variants: Vec<LlmContextLedgerPackDraft>,
) -> Result<Vec<LlmContextManifestPackVariant>, AppError> {
    if variants.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "LLM 上下文包 {pack_kind} 至少需要一个压缩变体"
        )));
    }
    let mut levels = BTreeSet::new();
    variants
        .into_iter()
        .map(|variant| {
            if variant.compression_level.trim().is_empty() {
                return Err(AppError::InvalidArgument(format!(
                    "LLM 上下文包 {pack_kind} 存在空 compression_level"
                )));
            }
            if !levels.insert(variant.compression_level.clone()) {
                return Err(AppError::InvalidArgument(format!(
                    "LLM 上下文包 {pack_kind} 存在重复 compression_level：{}",
                    variant.compression_level
                )));
            }
            Ok(LlmContextManifestPackVariant {
                compression_level: variant.compression_level,
                canonical_content_json: variant.canonical_content_json,
            })
        })
        .collect()
}
