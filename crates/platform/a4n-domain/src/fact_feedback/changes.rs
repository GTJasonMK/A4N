use super::{FactChangeOperation, FactMaterialChange, FactMaterialKind, WorldFact, WorldFactField};
use serde_json::Value;
use std::collections::BTreeSet;

impl WorldFact {
    pub fn validate(&self) -> Result<(), String> {
        if self.subject.trim().is_empty() || self.statement.trim().is_empty() {
            return Err("世界事实的 subject 和 statement 不能为空".into());
        }
        validate_evidence_ids(&self.source_span_ids)
    }

    pub fn identity(&self) -> String {
        format!(
            "{}:{}",
            self.field.as_str(),
            self.subject.trim().to_lowercase()
        )
    }
}

impl FactMaterialChange {
    pub fn validate(&self) -> Result<(), String> {
        validate_evidence_ids(&self.source_span_ids)?;
        if self.reason.trim().is_empty() || self.target_id.is_some_and(|id| id <= 0) {
            return Err("资料变更必须有说明和有效的目标 ID".into());
        }
        match self.material_kind {
            FactMaterialKind::WorldSetting => {
                let field = self
                    .path
                    .strip_prefix('/')
                    .and_then(|path| path.split('/').next())
                    .ok_or_else(|| "世界观变更必须指定字段路径".to_string())?;
                serde_json::from_value::<WorldFactField>(Value::String(field.into()))
                    .map_err(|_| format!("不支持的世界观字段: {field}"))?;
            }
            FactMaterialKind::BlueprintSynopsis
            | FactMaterialKind::CharacterVoice
            | FactMaterialKind::CharacterArcProgress => {
                if !self.path.is_empty() {
                    return Err("该资料变更的 path 必须为空，目标由 material_kind 指定".into());
                }
            }
            FactMaterialKind::Planning => {}
        }
        if self.operation == FactChangeOperation::Remove && !self.after.is_null() {
            return Err("删除变更的 after 必须为 null".into());
        }
        Ok(())
    }
}

pub fn validate_evidence_ids(ids: &[String]) -> Result<(), String> {
    if ids.is_empty() || ids.iter().any(|id| id.trim().is_empty() || id.trim() != id) {
        return Err("事实变更必须引用有效的 source_span_ids".into());
    }
    if ids.iter().collect::<BTreeSet<_>>().len() != ids.len() {
        return Err("source_span_ids 不能重复".into());
    }
    Ok(())
}

pub fn world_setting_document(value: Option<&Value>) -> Value {
    match value {
        Some(Value::String(text)) => serde_json::json!({"premise": text}),
        Some(value) if !value.is_null() => value.clone(),
        _ => serde_json::json!({}),
    }
}
