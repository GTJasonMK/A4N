use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FactSourceRevision {
    pub chapter_id: i64,
    pub chapter_number: i32,
    pub content_revision: String,
    pub analysis_hash: String,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FactConsolidationOutput {
    pub changes: Vec<FactMaterialChange>,
}

pub const DEFAULT_FEEDBACK_INTERVAL_CHAPTERS: i32 = 5;
pub const FACT_FEEDBACK_SCHEMA_VERSION: u32 = 1;
pub const FACT_APPLY_STEP: &str = "project_fact_consolidation.apply";
pub const CHAPTER_FACT_FEEDBACK_STEP: &str = "chapter_analysis.fact_feedback";
pub const CHAPTER_FACT_INDEX_STEP: &str = "chapter_analysis.index_sync";
