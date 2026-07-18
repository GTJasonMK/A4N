use a4n_core::error::AppError;
use serde_json::{Value, json};

use a4n_core::source_spans::source_text_hash as source_content_sha256;

pub(in crate::manga) fn manga_request_context_manifest_summary(
    manifest_json: Option<&Value>,
) -> Result<Option<Value>, AppError> {
    let Some(manifest_json) = manifest_json else {
        return Ok(None);
    };
    let Some(manifest) = manifest_json.as_object() else {
        return Err(AppError::InvalidArgument(
            "漫画流水线 LLM 上下文 manifest 必须是对象".to_string(),
        ));
    };
    let packs = manifest
        .get("packs")
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::InvalidArgument("漫画流水线 LLM 上下文 packs 缺失".to_string()))?;
    let pack_summaries = packs
        .iter()
        .map(manga_context_pack_summary)
        .collect::<Result<Vec<_>, AppError>>()?;
    let dynamic_user_content = manifest
        .get("dynamic_user_content")
        .and_then(Value::as_str)
        .unwrap_or_default();
    Ok(Some(json!({
        "schema_version": manifest.get("schema_version").cloned().unwrap_or(Value::Null),
        "pack_count": pack_summaries.len(),
        "required_pack_count": pack_summaries
            .iter()
            .filter(|pack| pack.get("required").and_then(Value::as_bool).unwrap_or(false))
            .count(),
        "stable_prefix_pack_count": pack_summaries
            .iter()
            .filter(|pack| pack.get("stable_prefix").and_then(Value::as_bool).unwrap_or(false))
            .count(),
        "dynamic_user_content_chars": dynamic_user_content.chars().count(),
        "dynamic_user_content_sha256": source_content_sha256(dynamic_user_content),
        "output_contract_present": manifest
            .get("output_contract")
            .and_then(Value::as_str)
            .is_some_and(|value| !value.trim().is_empty()),
        "packs": pack_summaries,
    })))
}

fn manga_context_pack_summary(pack: &Value) -> Result<Value, AppError> {
    let Some(pack_object) = pack.as_object() else {
        return Err(AppError::InvalidArgument(
            "漫画流水线 LLM 上下文 pack 必须是对象".to_string(),
        ));
    };
    let variants = pack_object
        .get("variants")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            AppError::InvalidArgument("漫画流水线 LLM 上下文 variants 缺失".to_string())
        })?;
    let variant_summaries = variants
        .iter()
        .map(|variant| {
            let Some(variant_object) = variant.as_object() else {
                return Err(AppError::InvalidArgument(
                    "漫画流水线 LLM 上下文 variant 必须是对象".to_string(),
                ));
            };
            let canonical_content_json = variant_object
                .get("canonical_content_json")
                .cloned()
                .unwrap_or(Value::Null);
            let canonical_content_text =
                serde_json::to_string(&canonical_content_json).map_err(AppError::Serialization)?;
            Ok(json!({
                "compression_level": variant_object
                    .get("compression_level")
                    .cloned()
                    .unwrap_or(Value::Null),
                "canonical_content_chars": canonical_content_text.chars().count(),
                "canonical_content_sha256": source_content_sha256(&canonical_content_text),
            }))
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    Ok(json!({
        "scope_kind": pack_object.get("scope_kind").cloned().unwrap_or(Value::Null),
        "pack_kind": pack_object.get("pack_kind").cloned().unwrap_or(Value::Null),
        "source_refs_json": pack_object.get("source_refs_json").cloned().unwrap_or(Value::Null),
        "config_hash": pack_object.get("config_hash").cloned().unwrap_or(Value::Null),
        "required": pack_object.get("required").cloned().unwrap_or(Value::Null),
        "stable_prefix": pack_object.get("stable_prefix").cloned().unwrap_or(Value::Null),
        "variants": variant_summaries,
    }))
}
