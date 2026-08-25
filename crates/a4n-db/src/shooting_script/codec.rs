use a4n_core::models::chapter::chapter_shooting_script;
use a4n_domain::shooting_script::{CHAPTER_SHOOTING_SCRIPT_V1, ChapterShootingScriptV1};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::{
    contracts::{ChapterVisualPlanIdentity, ParsedChapterShootingScript, ShootingScriptDbError},
    validation::validate_sha256,
};

pub(super) fn parse_record(
    record: chapter_shooting_script::Model,
) -> Result<ParsedChapterShootingScript, ShootingScriptDbError> {
    if record.contract_version != CHAPTER_SHOOTING_SCRIPT_V1 {
        return Err(ShootingScriptDbError::InvalidState(format!(
            "拍摄剧本 {} 使用不受支持的合同版本: {}",
            record.id, record.contract_version
        )));
    }
    validate_sha256(&record.source_content_sha256, "source_content_sha256")?;
    validate_sha256(&record.input_fingerprint_sha256, "input_fingerprint_sha256")?;
    validate_sha256(&record.script_sha256, "script_sha256")?;
    let visual_plan = stored_visual_plan_identity(&record)?;
    let script: ChapterShootingScriptV1 = serde_json::from_value(record.script_json.clone())
        .map_err(|error| {
            ShootingScriptDbError::InvalidState(format!(
                "解析章节拍摄剧本 {} 失败: {error}",
                record.id
            ))
        })?;
    script.validate().map_err(|error| {
        ShootingScriptDbError::InvalidState(format!("章节拍摄剧本 {} 校验失败: {error}", record.id))
    })?;
    validate_stored_script_visual_plan_identity(record.id, &script, visual_plan.as_ref())?;
    let actual_script_sha256 = hash_serializable(&script)?;
    if actual_script_sha256 != record.script_sha256 {
        return Err(ShootingScriptDbError::InvalidState(format!(
            "章节拍摄剧本 {} 内容哈希不一致",
            record.id
        )));
    }
    Ok(ParsedChapterShootingScript { record, script })
}

pub(super) fn stored_visual_plan_identity(
    record: &chapter_shooting_script::Model,
) -> Result<Option<ChapterVisualPlanIdentity>, ShootingScriptDbError> {
    match (
        record.visual_plan_id,
        record.visual_plan_version,
        record.visual_plan_sha256.as_deref(),
    ) {
        (None, None, None) => Ok(None),
        (Some(id), Some(version), Some(sha256)) if id > 0 && version > 0 => {
            validate_sha256(sha256, "visual_plan_sha256").map_err(|_| {
                ShootingScriptDbError::InvalidState(format!(
                    "章节拍摄剧本 {} 的 visual plan SHA-256 无效",
                    record.id
                ))
            })?;
            Ok(Some(ChapterVisualPlanIdentity {
                id,
                version,
                sha256: sha256.to_string(),
            }))
        }
        _ => Err(ShootingScriptDbError::InvalidState(format!(
            "章节拍摄剧本 {} 的 visual plan 身份不完整",
            record.id
        ))),
    }
}

pub(super) fn validate_script_visual_plan_identity(
    script: &ChapterShootingScriptV1,
    identity: &ChapterVisualPlanIdentity,
) -> Result<(), ShootingScriptDbError> {
    let source = script.visual_plan_source.as_ref().ok_or_else(|| {
        ShootingScriptDbError::InvalidArgument(
            "新写入的拍摄剧本必须包含 visual_plan_source".to_string(),
        )
    })?;
    if visual_plan_source_matches(source, identity) {
        return Ok(());
    }
    Err(ShootingScriptDbError::InvalidArgument(format!(
        "拍摄剧本 visual_plan_source 与写入身份不一致: script=({}, {}, {}), record=({}, {}, {})",
        source.plan_id,
        source.plan_version,
        source.plan_sha256,
        identity.id,
        identity.version,
        identity.sha256
    )))
}

fn validate_stored_script_visual_plan_identity(
    record_id: i64,
    script: &ChapterShootingScriptV1,
    identity: Option<&ChapterVisualPlanIdentity>,
) -> Result<(), ShootingScriptDbError> {
    match (script.visual_plan_source.as_ref(), identity) {
        (None, None) => Ok(()),
        (Some(source), Some(identity)) if visual_plan_source_matches(source, identity) => Ok(()),
        _ => Err(ShootingScriptDbError::InvalidState(format!(
            "章节拍摄剧本 {record_id} 的记录身份与 script.visual_plan_source 不一致"
        ))),
    }
}

fn visual_plan_source_matches(
    source: &a4n_domain::shooting_script::ShootingVisualPlanSourceV1,
    identity: &ChapterVisualPlanIdentity,
) -> bool {
    source.plan_id == identity.id
        && source.plan_version == identity.version
        && source.plan_sha256 == identity.sha256
}

pub(super) fn serialize_script(
    script: &ChapterShootingScriptV1,
) -> Result<(serde_json::Value, String), ShootingScriptDbError> {
    script
        .validate()
        .map_err(|error| ShootingScriptDbError::InvalidArgument(error.to_string()))?;
    let script_json = serde_json::to_value(script).map_err(|error| {
        ShootingScriptDbError::InvalidState(format!("序列化章节拍摄剧本失败: {error}"))
    })?;
    let script_sha256 = hash_serializable(script)?;
    Ok((script_json, script_sha256))
}

fn hash_serializable<T: Serialize>(value: &T) -> Result<String, ShootingScriptDbError> {
    let bytes = serde_json::to_vec(value).map_err(|error| {
        ShootingScriptDbError::InvalidState(format!("序列化拍摄剧本哈希输入失败: {error}"))
    })?;
    Ok(sha256_hex(&bytes))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("{digest:x}")
}
