use std::collections::BTreeSet;

use a4n_core::error::AppError;

use super::super::contracts::{CONTEXT_MANIFEST_SCHEMA_VERSION, LlmContextManifest};

mod pack;
use pack::validate_manifest_pack;

pub fn validate_context_manifest(manifest: &LlmContextManifest) -> Result<(), AppError> {
    if manifest.schema_version != CONTEXT_MANIFEST_SCHEMA_VERSION {
        return Err(AppError::InvalidArgument(format!(
            "不支持的 LLM 上下文 manifest 版本：{}",
            manifest.schema_version
        )));
    }
    if manifest.packs.is_empty() {
        return Err(AppError::InvalidArgument(
            "LLM 上下文 manifest 至少需要一个上下文包".to_string(),
        ));
    }
    let mut pack_kinds = BTreeSet::new();
    for pack in &manifest.packs {
        if !pack_kinds.insert(pack.pack_kind.as_str()) {
            return Err(AppError::InvalidArgument(format!(
                "LLM 上下文 manifest 存在重复 pack_kind：{}",
                pack.pack_kind
            )));
        }
        validate_manifest_pack(pack)?;
    }
    Ok(())
}
