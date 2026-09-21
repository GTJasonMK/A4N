mod entity_states;
mod occurrences;
mod profiles;
mod transitions;
mod validation;

use super::types::{ParsedSceneProfileSync, SceneProfileSyncRequestContext};
use crate::scene_profile::SceneSyncInput;
use a4n_domain::json_contract::reject_unknown_fields;
use serde_json::Value;

use entity_states::parse_scene_entity_states;
use occurrences::parse_scene_occurrences;
use profiles::parse_scene_profiles;
use transitions::parse_scene_transitions;
use validation::validate_scene_sync_references;

pub(super) fn parse_scene_stage_root_array(
    content: &str,
    prompt_name: &str,
    root_field: &str,
    context: &SceneProfileSyncRequestContext,
) -> Result<Value, String> {
    let value: Value = serde_json::from_str(content.trim()).map_err(|err| {
        let mut message = format!("{prompt_name} 结果必须是 JSON 对象: {err}");
        if err.is_eof() {
            message.push_str("；响应在 JSON 闭合前结束，通常是模型输出被截断或 max_tokens 不足");
        }
        message
    })?;
    if !value.is_object() {
        return Err(format!("{prompt_name} 结果必须是 JSON 对象"));
    }
    reject_unknown_fields(&value, prompt_name, &[root_field])?;
    // 阶段结果和恢复断点共用领域解析器，字段校验通过后才能缓存或交给下一阶段。
    match root_field {
        "scenes" => parse_scene_profiles(&value, context).map(drop),
        "occurrences" => parse_scene_occurrences(&value, context).map(drop),
        "transitions" => parse_scene_transitions(&value, context).map(drop),
        "entity_states" => parse_scene_entity_states(&value, context).map(drop),
        _ => Err(format!("不支持的场景同步阶段字段: {root_field}")),
    }?;
    value
        .get(root_field)
        .and_then(Value::as_array)
        .cloned()
        .map(Value::Array)
        .ok_or_else(|| format!("{prompt_name}.{root_field} 必须是数组"))
}

pub(super) fn parse_scene_profile_sync_content(
    content: &str,
    context: &SceneProfileSyncRequestContext,
) -> Result<ParsedSceneProfileSync, String> {
    let value: Value = serde_json::from_str(content.trim()).map_err(|err| {
        let mut message = format!("场景档案同步结果必须是 JSON 对象: {err}");
        if err.is_eof() {
            message.push_str("；响应在 JSON 闭合前结束，通常是模型输出被截断或 max_tokens 不足");
        }
        message
    })?;
    if !value.is_object() {
        return Err("场景档案同步结果必须是 JSON 对象".to_string());
    }
    reject_unknown_fields(
        &value,
        "root",
        &["scenes", "occurrences", "transitions", "entity_states"],
    )?;
    let profiles = parse_scene_profiles(&value, context)?;
    let input = SceneSyncInput {
        profiles: profiles.drafts,
        occurrences: parse_scene_occurrences(&value, context)?,
        transitions: parse_scene_transitions(&value, context)?,
        entity_states: parse_scene_entity_states(&value, context)?,
    };
    validate_scene_sync_references(&input)?;
    Ok(ParsedSceneProfileSync {
        input,
        profile_evidence: profiles.evidence_refs,
    })
}
