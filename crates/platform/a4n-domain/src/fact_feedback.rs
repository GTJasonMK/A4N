//! Evidence-backed story changes. Persistent material owners remain authoritative.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub const DEFAULT_FEEDBACK_INTERVAL_CHAPTERS: i32 = 5;
pub const FACT_FEEDBACK_SCHEMA_VERSION: u32 = 1;
pub const FACT_APPLY_STEP: &str = "project_fact_consolidation.apply";
pub const CHAPTER_FACT_FEEDBACK_STEP: &str = "chapter_analysis.fact_feedback";
pub const CHAPTER_FACT_INDEX_STEP: &str = "chapter_analysis.index_sync";

/// Text-mode world settings participate in the same field updates as structured settings.
pub fn world_setting_document(value: Option<&Value>) -> Value {
    match value {
        Some(Value::String(text)) => serde_json::json!({"premise": text}),
        Some(value) if !value.is_null() => value.clone(),
        _ => serde_json::json!({}),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FactAssertion {
    Observed,
    Inferred,
    Reported,
    Hypothetical,
    Recollection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorldFactField {
    Premise,
    CoreRules,
    TechnologyOrMagic,
    KeyLocations,
    Factions,
    HistoryTimeline,
    CultureAndSociety,
    ConflictSources,
    MysteriesAndForeshadowing,
    GenrePromises,
}

impl WorldFactField {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Premise => "premise",
            Self::CoreRules => "core_rules",
            Self::TechnologyOrMagic => "technology_or_magic",
            Self::KeyLocations => "key_locations",
            Self::Factions => "factions",
            Self::HistoryTimeline => "history_timeline",
            Self::CultureAndSociety => "culture_and_society",
            Self::ConflictSources => "conflict_sources",
            Self::MysteriesAndForeshadowing => "mysteries_and_foreshadowing",
            Self::GenrePromises => "genre_promises",
        }
    }

    pub fn requires_immediate_review(self) -> bool {
        matches!(
            self,
            Self::Premise | Self::CoreRules | Self::TechnologyOrMagic
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorldFact {
    pub field: WorldFactField,
    pub subject: String,
    pub statement: String,
    pub assertion: FactAssertion,
    pub source_span_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<String>,
}

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FactSourceRevision {
    pub chapter_id: i64,
    pub chapter_number: i32,
    pub content_revision: String,
    pub analysis_hash: String,
}

pub fn pending_fact_sources<'a>(
    current: &'a [FactSourceRevision],
    consolidated: &[FactSourceRevision],
) -> Vec<&'a FactSourceRevision> {
    let consumed: BTreeMap<_, _> = consolidated
        .iter()
        .map(|source| (source.chapter_id, source))
        .collect();
    current
        .iter()
        .filter(|source| consumed.get(&source.chapter_id).copied() != Some(*source))
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FactMaterialKind {
    WorldSetting,
    BlueprintSynopsis,
    CharacterVoice,
    CharacterArcProgress,
    Planning,
}

impl FactMaterialKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WorldSetting => "world_setting",
            Self::BlueprintSynopsis => "blueprint_synopsis",
            Self::CharacterVoice => "character_voice",
            Self::CharacterArcProgress => "character_arc_progress",
            Self::Planning => "planning",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FactChangeOperation {
    Add,
    Replace,
    Remove,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FactMaterialChange {
    pub material_kind: FactMaterialKind,
    pub target_id: Option<i64>,
    pub path: String,
    pub operation: FactChangeOperation,
    pub before: Value,
    pub after: Value,
    pub reason: String,
    pub source_span_ids: Vec<String>,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FactConsolidationOutput {
    pub changes: Vec<FactMaterialChange>,
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

/// Apply one revision-fenced JSON edit. Callers validate the domain and commit atomically.
pub fn apply_fact_json_change(
    document: &mut Value,
    change: &FactMaterialChange,
) -> Result<bool, String> {
    use FactChangeOperation::{Add, Remove, Replace};
    let current = document.pointer(&change.path);
    if change.operation == Add {
        if current.is_some() || !change.before.is_null() {
            return Err(format!("新增字段已经存在或 before 非空: {}", change.path));
        }
    } else if current != Some(&change.before) {
        return Err(format!("资料已变化，不能应用旧变更: {}", change.path));
    }
    if change.operation == Replace && change.before == change.after {
        return Ok(false);
    }
    if change.path.is_empty() {
        *document = if change.operation == Remove {
            Value::Null
        } else {
            change.after.clone()
        };
        return Ok(true);
    }
    let (parent, key) = change
        .path
        .rsplit_once('/')
        .ok_or("资料字段路径必须是 JSON Pointer")?;
    let key = decode_pointer_key(key)?;
    let parent = document
        .pointer_mut(parent)
        .ok_or("资料字段的父节点不存在")?;
    match parent {
        Value::Object(object) => {
            if change.operation == Remove {
                object.remove(&key);
            } else {
                object.insert(key, change.after.clone());
            }
        }
        Value::Array(items) => {
            if change.operation == Add && key == "-" {
                items.push(change.after.clone());
            } else {
                let index = key.parse::<usize>().map_err(|_| "数组字段索引无效")?;
                if index >= items.len() {
                    return Err("数组字段索引超出范围".into());
                }
                match change.operation {
                    Add => return Err("数组新增只支持 /- 追加".into()),
                    Remove => {
                        items.remove(index);
                    }
                    Replace => items[index] = change.after.clone(),
                }
            }
        }
        _ => return Err("资料字段父节点必须是对象或数组".into()),
    }
    Ok(true)
}

fn decode_pointer_key(key: &str) -> Result<String, String> {
    let mut decoded = String::new();
    let mut chars = key.chars();
    while let Some(character) = chars.next() {
        decoded.push(if character == '~' {
            match chars.next() {
                Some('0') => '~',
                Some('1') => '/',
                _ => return Err("资料字段路径包含无效转义".into()),
            }
        } else {
            character
        });
    }
    Ok(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn revision_fenced_partial_changes_preserve_unrelated_rules() {
        assert_eq!(
            world_setting_document(Some(&json!("记忆之城"))),
            json!({"premise":"记忆之城"})
        );
        assert_eq!(world_setting_document(None), json!({}));
        let mut world = json!({"core_rules": ["原规则"], "factions": ["旧城议会"]});
        let mut change = FactMaterialChange {
            material_kind: FactMaterialKind::WorldSetting,
            target_id: Some(1),
            path: "/core_rules/-".into(),
            operation: FactChangeOperation::Add,
            before: Value::Null,
            after: json!("施法损失记忆"),
            reason: "第六章明示".into(),
            source_span_ids: vec!["ch6:s1:f1".into()],
        };
        change.validate().unwrap();
        assert!(apply_fact_json_change(&mut world, &change).unwrap());
        assert_eq!(world["factions"], json!(["旧城议会"]));
        change.path = "/core_rules/0".into();
        change.operation = FactChangeOperation::Replace;
        assert!(apply_fact_json_change(&mut world, &change).is_err());
        change.before = json!("原规则");
        assert!(apply_fact_json_change(&mut world, &change).unwrap());
        change.before = change.after.clone();
        assert!(!apply_fact_json_change(&mut world, &change).unwrap());
        change.path = "/total_chapters".into();
        assert!(change.validate().is_err());
    }

    #[test]
    fn coverage_counts_changed_chapters_instead_of_analysis_attempts() {
        let source = FactSourceRevision {
            chapter_id: 7,
            chapter_number: 4,
            content_revision: "body-a".into(),
            analysis_hash: "facts-a".into(),
        };
        assert!(
            pending_fact_sources(std::slice::from_ref(&source), std::slice::from_ref(&source))
                .is_empty()
        );
        let mut edited = source.clone();
        edited.analysis_hash = "facts-b".into();
        assert_eq!(pending_fact_sources(&[edited], &[source]).len(), 1);
    }
}
