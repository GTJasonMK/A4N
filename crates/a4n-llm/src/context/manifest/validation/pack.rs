use std::collections::BTreeSet;

use a4n_core::error::AppError;

use super::super::super::contracts::LlmContextManifestPack;

pub(super) fn validate_manifest_pack(pack: &LlmContextManifestPack) -> Result<(), AppError> {
    if pack.pack_kind.trim().is_empty() {
        return Err(AppError::InvalidArgument(
            "LLM 上下文包 pack_kind 不能为空".to_string(),
        ));
    }
    if pack.config_hash.trim().is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "LLM 上下文包 {} 缺少 config_hash",
            pack.pack_kind
        )));
    }
    if pack.variants.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "LLM 上下文包 {} 至少需要一个压缩变体",
            pack.pack_kind
        )));
    }
    let mut compression_levels = BTreeSet::new();
    for variant in &pack.variants {
        if variant.compression_level.trim().is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "LLM 上下文包 {} 存在空 compression_level",
                pack.pack_kind
            )));
        }
        if !compression_levels.insert(variant.compression_level.as_str()) {
            return Err(AppError::InvalidArgument(format!(
                "LLM 上下文包 {} 存在重复 compression_level：{}",
                pack.pack_kind, variant.compression_level
            )));
        }
    }
    Ok(())
}
