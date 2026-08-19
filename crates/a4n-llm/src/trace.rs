use a4n_core::error::AppError;
use a4n_core::source_spans::source_text_hash;
use serde_json::{Value, json};

use crate::context::{LlmContextManifest, estimate_context_tokens};
use crate::provider::{ChatRequest, UsageInfo};

pub fn llm_usage_summary_text(usage: Option<&UsageInfo>) -> String {
    let Some(usage) = usage else {
        return "模型未返回 token usage".to_string();
    };
    let mut summary = format!(
        "Token {}（输入 {} / 输出 {}）",
        usage.total_tokens, usage.prompt_tokens, usage.completion_tokens
    );
    match (
        usage.prompt_cache_hit_tokens,
        usage.prompt_cache_miss_tokens,
    ) {
        (Some(hit), Some(miss)) => {
            let total = hit + miss;
            if total > 0 {
                let ratio = (hit as f64 / total as f64) * 100.0;
                summary.push_str(&format!(
                    "，缓存命中 {:.1}%（命中 {} / 未命中 {}）",
                    ratio, hit, miss
                ));
            } else {
                summary.push_str("，缓存命中 0.0%（命中 0 / 未命中 0）");
            }
        }
        (Some(hit), None) => {
            summary.push_str(&format!("，缓存命中 token {}", hit));
        }
        (None, Some(miss)) => {
            summary.push_str(&format!("，缓存未命中 token {}", miss));
        }
        (None, None) => {
            summary.push_str("，缓存数据未返回");
        }
    }
    summary
}

pub fn chat_request_messages_json(request: &ChatRequest) -> Result<Value, AppError> {
    serde_json::to_value(&request.messages).map_err(AppError::Serialization)
}

fn chat_request_trace_json(request: &ChatRequest) -> Result<Value, AppError> {
    serde_json::to_value(request).map_err(AppError::Serialization)
}

pub fn context_aware_chat_request_trace_json(request: &ChatRequest) -> Result<Value, AppError> {
    let mut request_json = chat_request_trace_json(request)?;
    let Some(object) = request_json.as_object_mut() else {
        return Ok(request_json);
    };
    if let Some(manifest_json) = request.context_manifest_json.as_ref() {
        object.insert(
            "context_manifest_summary".to_string(),
            context_manifest_trace_summary(manifest_json)?,
        );
    }
    if let Some(trace_context) = request.trace_context.as_ref() {
        object.insert(
            "trace_context".to_string(),
            json!({
                "project_id": trace_context.project_id,
                "root_project_id": trace_context.root_project_id,
                "workflow": trace_context.workflow,
                "step_key": trace_context.step_key,
                "generation_run_id": trace_context.generation_run_id,
                "workflow_run_id": trace_context.workflow_run_id,
                "call_key": trace_context.call_key,
                "input_fingerprint_sha256": trace_context.input_fingerprint_sha256,
                "context_plan": trace_context.context_plan_json,
                "context_pack_refs": trace_context.context_pack_refs_json,
            }),
        );
    }
    Ok(request_json)
}

fn context_manifest_trace_summary(manifest_json: &Value) -> Result<Value, AppError> {
    let manifest: LlmContextManifest =
        serde_json::from_value(manifest_json.clone()).map_err(AppError::Serialization)?;
    let packs = manifest
        .packs
        .iter()
        .map(|pack| {
            let variants = pack
                .variants
                .iter()
                .map(|variant| {
                    let text = serde_json::to_string(&variant.canonical_content_json)
                        .map_err(AppError::Serialization)?;
                    Ok(json!({
                        "compression_level": variant.compression_level,
                        "content_sha256": source_text_hash(&text),
                        "chars": text.chars().count(),
                        "tokens_estimate": estimate_context_tokens(&text),
                    }))
                })
                .collect::<Result<Vec<_>, AppError>>()?;
            Ok(json!({
                "scope_kind": pack.scope_kind,
                "pack_kind": pack.pack_kind,
                "source_refs": pack.source_refs_json,
                "config_hash": pack.config_hash,
                "required": pack.required,
                "stable_prefix": pack.stable_prefix,
                "variants": variants
            }))
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    Ok(json!({
        "schema_version": manifest.schema_version,
        "dynamic_user_chars": manifest.dynamic_user_content.chars().count(),
        "dynamic_user_tokens_estimate": estimate_context_tokens(&manifest.dynamic_user_content),
        "output_contract_chars": manifest.output_contract.as_deref().map(str::chars).map(Iterator::count),
        "output_contract_tokens_estimate": manifest.output_contract.as_deref().map(estimate_context_tokens),
        "packs": packs
    }))
}

pub fn text_response_summary_json(
    content: &str,
    finish_reason: Option<&str>,
    extra: Value,
) -> Value {
    let mut summary = json!({
        "content_chars": content.chars().count(),
        "content_sha256": source_text_hash(content),
        "content_preview": content.chars().take(240).collect::<String>(),
        "finish_reason": finish_reason,
    });
    if let (Some(summary), Some(extra)) = (summary.as_object_mut(), extra.as_object()) {
        for (key, value) in extra {
            summary.insert(key.clone(), value.clone());
        }
    }
    summary
}

#[cfg(test)]
mod tests;
