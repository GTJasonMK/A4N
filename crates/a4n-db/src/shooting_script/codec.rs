use a4n_core::models::chapter::chapter_shooting_script;
use a4n_domain::shooting_script::{CHAPTER_SHOOTING_SCRIPT_V1, ChapterShootingScriptV1};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::{
    contracts::{ParsedChapterShootingScript, ShootingScriptDbError},
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
    let actual_script_sha256 = hash_serializable(&script)?;
    if actual_script_sha256 != record.script_sha256 {
        return Err(ShootingScriptDbError::InvalidState(format!(
            "章节拍摄剧本 {} 内容哈希不一致",
            record.id
        )));
    }
    Ok(ParsedChapterShootingScript { record, script })
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
