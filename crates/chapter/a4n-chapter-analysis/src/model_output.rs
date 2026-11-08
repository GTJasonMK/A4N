use a4n_domain::{
    fact_feedback::WorldFact,
    json_contract::{RequiredNullable, typed_output_schema},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub fn chapter_analysis_output_schema(stage: &str) -> Result<Value, String> {
    Ok(match stage {
        crate::CHAPTER_ANALYSIS_STAGE_FACTS => typed_output_schema::<Facts>(),
        crate::CHAPTER_ANALYSIS_STAGE_SUMMARY => typed_output_schema::<Summary>(),
        crate::CHAPTER_ANALYSIS_STAGE_CHARACTER_STATE => typed_output_schema::<CharacterStates>(),
        crate::CHAPTER_ANALYSIS_STAGE_FORESHADOWING_EVENTS => {
            typed_output_schema::<ForeshadowingEvents>()
        }
        _ => return Err(format!("未知章节分析输出合同：{stage}")),
    })
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Facts {
    characters: Vec<Entity>,
    locations: Vec<Entity>,
    items: Vec<Entity>,
    event_candidates: Vec<EventCandidate>,
    foreshadowing_candidates: Vec<ForeshadowingCandidate>,
    world_facts: Vec<WorldFact>,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Entity {
    name: String,
    source_span_ids: Vec<String>,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum EventType {
    Battle,
    Revelation,
    Relationship,
    Discovery,
    Decision,
    Death,
    Arrival,
    Departure,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum Importance {
    High,
    Medium,
    Low,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum ForeshadowingCategory {
    CharacterSecret,
    PlotTwist,
    ItemMystery,
    WorldRule,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct EventCandidate {
    event_ref: String,
    #[serde(rename = "type")]
    event_type: EventType,
    description: String,
    importance: Importance,
    involved_characters: Vec<String>,
    source_span_ids: Vec<String>,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ForeshadowingCandidate {
    foreshadowing_ref: String,
    description: String,
    category: ForeshadowingCategory,
    priority: Importance,
    related_entities: Vec<String>,
    original_text_span_id: String,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Summary {
    metadata_additions: Metadata,
    summaries: Summaries,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Metadata {
    tags: Vec<String>,
    tone: RequiredNullable<String>,
    timeline_marker: RequiredNullable<String>,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Summaries {
    compressed: String,
    one_line: String,
    keywords: Vec<String>,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct CharacterStates {
    character_states: BTreeMap<String, CharacterState>,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct CharacterState {
    location: RequiredNullable<String>,
    status: RequiredNullable<String>,
    changes: Vec<String>,
    emotional_state: RequiredNullable<String>,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ForeshadowingEvents {
    foreshadowing: Foreshadowing,
    key_events: Vec<KeyEvent>,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct KeyEvent {
    #[serde(rename = "type")]
    event_type: EventType,
    description: String,
    importance: Importance,
    involved_characters: Vec<String>,
    source_span_ids: Vec<String>,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Foreshadowing {
    planted: Vec<Planted>,
    resolved: Vec<Resolved>,
    tensions: Vec<String>,
    #[serde(default)]
    progressed: Vec<Progressed>,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Planted {
    id: Option<String>,
    description: String,
    original_text_span_id: String,
    category: ForeshadowingCategory,
    priority: Importance,
    related_entities: Vec<String>,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Resolved {
    id: String,
    resolution: String,
    source_span_ids: Vec<String>,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum ProgressStatus {
    PartiallyRevealed,
    Abandoned,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Progressed {
    id: String,
    resolution: String,
    source_span_ids: Vec<String>,
    status: ProgressStatus,
}
