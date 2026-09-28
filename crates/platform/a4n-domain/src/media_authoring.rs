//! Project-owned narrative adaptation. Film shots are optional provenance,
//! never the identity of a game's scene or a comic's panel.

use std::collections::BTreeSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod legacy;
pub const MEDIA_AUTHORING_PIPELINE_VERSION: &str = "media_authoring_v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AdaptationTarget {
    Manga,
    Galgame,
}

impl AdaptationTarget {
    pub const fn project_kind(self) -> &'static str {
        match self {
            Self::Manga => crate::project::ProjectKind::Comic.as_str(),
            Self::Galgame => crate::project::ProjectKind::Galgame.as_str(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum AdaptationPlanVersion {
    #[serde(rename = "adaptation_plan_v1")]
    V1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AdaptationOrigin {
    Source,
    Adapted,
    Added,
    Historical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AdaptationImportance {
    Required,
    Important,
    Supporting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AdaptationBeatKind {
    Action,
    Dialogue,
    Reaction,
    Revelation,
    Transition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AdaptationBeat {
    pub id: String,
    pub kind: AdaptationBeatKind,
    pub is_climax: bool,
    pub origin: AdaptationOrigin,
    pub importance: AdaptationImportance,
    pub action: String,
    pub reaction: Option<String>,
    pub dialogue_intent: Option<String>,
    pub entry_state: String,
    pub exit_state: String,
    pub source_span_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AdaptationUnit {
    pub id: String,
    pub origin: AdaptationOrigin,
    pub title: String,
    pub location: Option<String>,
    pub participants: Vec<String>,
    pub dramatic_question: String,
    pub objective: String,
    pub obstacle: String,
    pub entry_state: String,
    pub exit_state: String,
    pub source_span_ids: Vec<String>,
    pub beats: Vec<AdaptationBeat>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AdaptationDecision {
    pub description: String,
    pub reason: String,
    pub source_span_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AdaptationPlanV1 {
    pub schema_version: AdaptationPlanVersion,
    pub target: AdaptationTarget,
    pub intention: String,
    pub decisions: Vec<AdaptationDecision>,
    pub units: Vec<AdaptationUnit>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct AdaptationBeatRef {
    pub unit_id: String,
    pub beat_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AdaptationUnitRef {
    pub source_chapter_id: i64,
    pub unit_id: String,
}

impl AdaptationUnitRef {
    pub fn validate(&self) -> Result<(), String> {
        if self.source_chapter_id <= 0 {
            return Err("原文章节 ID 必须大于 0".into());
        }
        required(&self.unit_id, "unit_id")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GalgameBranchDesign {
    pub choices: Vec<GalgamePipelineChoicePlan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GalgamePipelineChoicePlan {
    pub after_beat_ref: crate::galgame::GalgamePipelineBeatRefV4,
    pub question: String,
    pub options: Vec<GalgamePipelineChoiceOptionPlan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GalgamePipelineChoiceOptionPlan {
    pub branch_id: String,
    pub option_text: String,
    pub branch_title: String,
    pub branch_objective: String,
    pub dramatic_action: String,
    pub reaction: String,
    pub player_knowledge: Vec<String>,
    pub returns_to: AdaptationUnitRef,
}

impl AdaptationBeatRef {
    pub fn validate(&self) -> Result<(), String> {
        required(&self.unit_id, "unit_id")?;
        required(&self.beat_id, "beat_id")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AuthoringSourceRef {
    pub root_project_id: i64,
    pub source_chapter_id: i64,
    pub source_snapshot_id: i64,
    pub source_snapshot_chapter_id: i64,
    pub content_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VideoAdaptationReference {
    /// A lookup hint for checking updates, never required to consume the snapshot.
    pub video_project_id: Option<i64>,
    pub project_title: String,
    pub source: AuthoringSourceRef,
    pub script_id: i64,
    pub script_revision: i32,
    pub script_sha256: String,
    pub units: Vec<AdaptationUnit>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AuthoringDocumentInputs {
    pub sources: Vec<AuthoringSourceRef>,
    pub video_reference: Option<VideoAdaptationReference>,
    pub dependencies: Vec<AuthoringDocumentRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AuthoringDocumentRef {
    pub id: i64,
    pub revision: i32,
    pub sha256: String,
}

impl AuthoringDocumentRecord {
    pub fn reference(&self) -> AuthoringDocumentRef {
        AuthoringDocumentRef {
            id: self.id,
            revision: self.revision,
            sha256: self.sha256.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AuthoringDocumentKind {
    AdaptationPlan,
    GalgameBranchPlan,
    MangaPagePlan,
}

impl AuthoringDocumentKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AdaptationPlan => "adaptation_plan",
            Self::GalgameBranchPlan => "galgame_branch_plan",
            Self::MangaPagePlan => "manga_page_plan",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AuthoringDocumentStatus {
    Draft,
    Adopted,
    Superseded,
}

impl AuthoringDocumentStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Adopted => "adopted",
            Self::Superseded => "superseded",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AuthoringDocumentRecord {
    pub id: i64,
    pub project_id: i64,
    pub source_snapshot_id: i64,
    pub source_chapter_id: Option<i64>,
    pub kind: AuthoringDocumentKind,
    pub version: i32,
    pub revision: i32,
    pub status: AuthoringDocumentStatus,
    pub inputs: AuthoringDocumentInputs,
    pub document: serde_json::Value,
    pub sha256: String,
}

impl AdaptationPlanV1 {
    pub fn validate(&self, source_span_ids: &BTreeSet<String>) -> Result<(), String> {
        required(&self.intention, "intention")?;
        if self.units.is_empty() {
            return Err("改编计划至少需要一个叙事单元".into());
        }
        let mut units = BTreeSet::new();
        for unit in &self.units {
            required(&unit.id, "unit.id")?;
            if !units.insert(&unit.id) {
                return Err(format!("改编单元 ID 重复: {}", unit.id));
            }
            required(&unit.title, "unit.title")?;
            required(&unit.objective, "unit.objective")?;
            validate_adaptation_evidence(unit.origin, &unit.source_span_ids, source_span_ids)?;
            if unit.beats.is_empty() {
                return Err(format!("改编单元 {} 缺少节拍", unit.id));
            }
            let mut beats = BTreeSet::new();
            for beat in &unit.beats {
                required(&beat.id, "beat.id")?;
                if !beats.insert(&beat.id) {
                    return Err(format!("改编单元 {} 的节拍 ID 重复: {}", unit.id, beat.id));
                }
                required(&beat.action, "beat.action")?;
                validate_adaptation_evidence(beat.origin, &beat.source_span_ids, source_span_ids)?;
                if beat
                    .source_span_ids
                    .iter()
                    .any(|id| !unit.source_span_ids.contains(id))
                {
                    return Err(format!("单元 {} 的原文依据未覆盖节拍 {}", unit.id, beat.id));
                }
            }
        }
        for decision in &self.decisions {
            required(&decision.description, "decision.description")?;
            required(&decision.reason, "decision.reason")?;
            validate_adaptation_evidence(
                AdaptationOrigin::Historical,
                &decision.source_span_ids,
                source_span_ids,
            )?;
        }
        Ok(())
    }

    pub fn beat(&self, reference: &AdaptationBeatRef) -> Option<&AdaptationBeat> {
        self.units
            .iter()
            .find(|unit| unit.id == reference.unit_id)?
            .beats
            .iter()
            .find(|beat| beat.id == reference.beat_id)
    }
}

fn required(value: &str, field: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{field} 不能为空"));
    }
    Ok(())
}

pub fn validate_adaptation_evidence(
    origin: AdaptationOrigin,
    spans: &[String],
    available: &BTreeSet<String>,
) -> Result<(), String> {
    if matches!(origin, AdaptationOrigin::Source | AdaptationOrigin::Adapted) && spans.is_empty() {
        return Err("来自原文的改编内容必须保留原文引用".into());
    }
    if origin == AdaptationOrigin::Added && !spans.is_empty() {
        return Err("新增改编内容不能声明为原文事实".into());
    }
    let mut seen = BTreeSet::new();
    for span in spans {
        if !available.contains(span) || !seen.insert(span) {
            return Err(format!("改编原文引用不存在或重复: {span}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adaptation_identity_survives_reordering_and_rejects_fabricated_sources() {
        let mut plan = AdaptationPlanV1 {
            schema_version: AdaptationPlanVersion::V1,
            target: AdaptationTarget::Manga,
            intention: "保留发现真相的因果".into(),
            decisions: vec![],
            units: vec![AdaptationUnit {
                id: "discovery".into(),
                origin: AdaptationOrigin::Source,
                title: "发现信件".into(),
                location: None,
                participants: vec![],
                dramatic_question: "信是谁寄的".into(),
                objective: "找到寄件人".into(),
                obstacle: "落款被撕掉".into(),
                entry_state: "尚未读信".into(),
                exit_state: "发现缺失落款".into(),
                source_span_ids: vec!["p1".into()],
                beats: vec![AdaptationBeat {
                    id: "letter".into(),
                    kind: AdaptationBeatKind::Revelation,
                    is_climax: true,
                    origin: AdaptationOrigin::Source,
                    importance: AdaptationImportance::Required,
                    action: "翻到信纸背面".into(),
                    reaction: None,
                    dialogue_intent: None,
                    entry_state: "信纸正面朝上".into(),
                    exit_state: "露出被撕去的落款".into(),
                    source_span_ids: vec!["p1".into()],
                }],
            }],
        };
        let available = BTreeSet::from(["p1".into()]);
        plan.validate(&available).unwrap();
        let reference = AdaptationBeatRef {
            unit_id: "discovery".into(),
            beat_id: "letter".into(),
        };
        let mut added = plan.units[0].clone();
        added.id = "reaction".into();
        plan.units.insert(0, added);
        assert_eq!(plan.beat(&reference).unwrap().action, "翻到信纸背面");
        plan.validate(&available).unwrap();
        plan.units[0].beats[0].origin = AdaptationOrigin::Added;
        assert!(plan.validate(&available).unwrap_err().contains("新增改编"));
        plan.units[0].beats[0].origin = AdaptationOrigin::Source;
        plan.units[0].beats[0].source_span_ids = vec!["missing".into()];
        assert!(plan.validate(&available).unwrap_err().contains("不存在"));
    }
}
