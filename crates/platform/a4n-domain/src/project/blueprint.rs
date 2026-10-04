use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::status::ProjectStatus;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlueprintCharacterRecord {
    pub id: i64,
    pub project_id: i64,
    pub name: String,
    pub character_type: String,
    pub gender: Option<String>,
    pub age: Option<String>,
    pub appearance: Option<String>,
    pub personality: Option<String>,
    pub background: Option<String>,
    pub motivation: Option<String>,
    pub arc_summary: Option<String>,
    pub relationship_to_protagonist: Option<String>,
    pub arc_stages: Option<Value>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlueprintRelationshipRecord {
    pub id: i64,
    pub project_id: i64,
    pub relationship_type: String,
    pub character_a: String,
    pub character_b: String,
    pub description: Option<String>,
    pub strength: Option<i32>,
    pub evolution: Option<Value>,
    pub created_at: NaiveDateTime,
}

pub fn normalized_blueprint_identity(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

pub fn blueprint_character_identity_keys<'a>(
    names: impl IntoIterator<Item = &'a str>,
) -> Result<Vec<String>, String> {
    let mut seen = std::collections::HashSet::new();
    names
        .into_iter()
        .map(|name| {
            let key = normalized_blueprint_identity(name);
            if key.is_empty() {
                return Err("蓝图角色名称不能为空".to_string());
            }
            if !seen.insert(key.clone()) {
                return Err(format!("蓝图角色重复: {name}"));
            }
            Ok(key)
        })
        .collect()
}

pub fn blueprint_relationship_identity_keys<'a>(
    relationships: impl IntoIterator<Item = (&'a str, &'a str, &'a str)>,
) -> Result<Vec<String>, String> {
    let mut seen = std::collections::HashSet::new();
    relationships
        .into_iter()
        .map(|(kind, a, b)| {
            if !BLUEPRINT_RELATIONSHIP_TYPES.contains(&kind) {
                return Err(format!(
                    "蓝图关系 relationship_type 无效：{kind}；必须是以下值之一：{}",
                    BLUEPRINT_RELATIONSHIP_TYPES.join(", ")
                ));
            }
            let keys = [kind, a, b].map(normalized_blueprint_identity);
            if keys.iter().any(String::is_empty) {
                return Err("蓝图关系的角色和类型不能为空".to_string());
            }
            let key = keys.join("|");
            if !seen.insert(key.clone()) {
                return Err(format!("蓝图关系重复: {kind} / {a} / {b}"));
            }
            Ok(key)
        })
        .collect()
}

pub const BLUEPRINT_MIN_TOTAL_CHAPTERS: i32 = 1;
pub const BLUEPRINT_MIN_GENERATION_CHAPTERS: i32 = 5;
pub const BLUEPRINT_MAX_TOTAL_CHAPTERS: i32 = 10_000;
pub const PART_OUTLINE_THRESHOLD: i32 = 50;
pub const CHAPTERS_PER_PART: i32 = 25;

pub const BLUEPRINT_GENERATION_KIND: &str = "blueprint_generation";
pub const BLUEPRINT_BASE_PROMPT: &str = "blueprint";
pub const BLUEPRINT_STAGE_SYSTEM_PROMPT: &str = "blueprint_stage_system";
pub const BLUEPRINT_STAGE_USER_PROMPT: &str = "blueprint_stage_user";
pub const BLUEPRINT_GENERATION_PROMPTS: [&str; 3] = [
    BLUEPRINT_BASE_PROMPT,
    BLUEPRINT_STAGE_SYSTEM_PROMPT,
    BLUEPRINT_STAGE_USER_PROMPT,
];
pub const BLUEPRINT_REFINEMENT_TASK_KIND: &str = "blueprint_refinement";
pub const BLUEPRINT_REFINEMENT_WORKFLOW_KIND: &str = "blueprint_refine";
pub const BLUEPRINT_REFINEMENT_MODE: &str = "refine";
pub const BLUEPRINT_REFINEMENT_PROMPT: &str = "blueprint_optimization";
pub const BLUEPRINT_REFINEMENT_USER_PROMPT: &str = "blueprint_optimization_user";
pub const BLUEPRINT_REFINEMENT_PROMPTS: [&str; 2] = [
    BLUEPRINT_REFINEMENT_PROMPT,
    BLUEPRINT_REFINEMENT_USER_PROMPT,
];
pub const BLUEPRINT_REFINEMENT_FIELDS: &[&str] = &[
    "style",
    "full_synopsis",
    "world_setting",
    "characters",
    "relationships",
];

pub const BLUEPRINT_RELATIONSHIP_TYPES: &[&str] = &[
    "family", "friend", "enemy", "lover", "rival", "mentor", "other",
];
pub const BLUEPRINT_RAW_JSON_STREAM_ID: &str = "raw_json";
pub const BLUEPRINT_GENERATE_PROGRESS_TOTAL: u32 = 12;

pub fn blueprint_refinement_call_key(project_id: i64) -> String {
    format!("{BLUEPRINT_REFINEMENT_WORKFLOW_KIND}:project:{project_id}")
}

pub fn blueprint_stage_call_key(stage_key: &str) -> String {
    format!("{BLUEPRINT_GENERATION_KIND}.{stage_key}")
}

pub fn blueprint_refinement_business_key(project_id: i64) -> String {
    format!("{BLUEPRINT_REFINEMENT_WORKFLOW_KIND}:{project_id}")
}

pub fn blueprint_generation_input_fingerprint(
    project_id: i64,
    title: &str,
    genre: &str,
    target_length: i32,
    variables: &Value,
) -> Value {
    json!({
        "version": 1,
        "task_kind": BLUEPRINT_GENERATION_KIND,
        "project_id": project_id,
        "title": title,
        "genre": genre,
        "target_length": target_length,
        "variables": variables,
    })
}

pub fn blueprint_refinement_input_fingerprint(
    project_id: i64,
    focus: Option<&str>,
    variables: &Value,
) -> Value {
    json!({
        "version": 1,
        "task_kind": BLUEPRINT_REFINEMENT_TASK_KIND,
        "project_id": project_id,
        "focus": normalized_blueprint_refinement_focus(focus),
        "variables": variables,
    })
}

pub fn normalized_blueprint_refinement_focus(focus: Option<&str>) -> Option<&str> {
    focus.map(str::trim).filter(|value| !value.is_empty())
}

pub fn validate_blueprint_chapter_count(value: i32, context: &str) -> Result<(), String> {
    if is_valid_blueprint_generation_chapters(value) {
        Ok(())
    } else {
        Err(format!(
            "{context} 必须在 {BLUEPRINT_MIN_GENERATION_CHAPTERS} 到 {BLUEPRINT_MAX_TOTAL_CHAPTERS} 之间"
        ))
    }
}

pub fn build_blueprint_generation_payload(
    novel_title: &str,
    genre: &str,
    target_chapters: i32,
    variables: &std::collections::HashMap<String, Value>,
) -> Result<Value, String> {
    validate_blueprint_chapter_count(target_chapters, "requested_chapter_count")?;

    let mut payload = serde_json::Map::new();
    payload.insert(
        "novel_title".to_string(),
        Value::String(novel_title.to_string()),
    );
    payload.insert("genre".to_string(), Value::String(genre.to_string()));
    payload.insert(
        "requested_chapter_count".to_string(),
        Value::from(target_chapters),
    );
    for key in [
        "inspiration_transcript",
        "generation_instruction",
        "conversation_state",
        "progress_summary",
        "next_question",
        "project",
    ] {
        if let Some(value) = variables.get(key) {
            payload.insert(key.to_string(), value.clone());
        }
    }
    Ok(Value::Object(payload))
}

pub fn increment_blueprint_version(current: &str) -> Result<String, String> {
    let (major, minor) = current
        .split_once('.')
        .ok_or_else(|| format!("蓝图版本号 {current:?} 必须使用 major.minor 格式"))?;
    if major.is_empty() || minor.is_empty() || minor.contains('.') {
        return Err(format!("蓝图版本号 {current:?} 必须使用 major.minor 格式"));
    }
    let major = major
        .parse::<u32>()
        .map_err(|_| format!("蓝图版本号 {current:?} 的 major 必须是非负整数"))?;
    let minor = minor
        .parse::<u32>()
        .map_err(|_| format!("蓝图版本号 {current:?} 的 minor 必须是非负整数"))?;
    let next_minor = minor
        .checked_add(1)
        .ok_or_else(|| format!("蓝图版本号 {current:?} 的 minor 已超出可递增范围"))?;
    Ok(format!("{major}.{next_minor}"))
}

pub fn project_status_after_blueprint_generation(previous: ProjectStatus) -> ProjectStatus {
    previous.max(ProjectStatus::BlueprintReady)
}

#[derive(Clone, Debug)]
pub struct ParsedBlueprint {
    pub title: String,
    pub genre: String,
    pub style: String,
    pub tone: String,
    pub target_audience: String,
    pub one_sentence_summary: String,
    pub full_synopsis: String,
    pub world_setting: Value,
    pub total_chapters: i32,
    pub needs_part_outlines: bool,
    pub chapters_per_part: i32,
    pub characters: Vec<ParsedBlueprintCharacter>,
    pub relationships: Vec<ParsedBlueprintRelationship>,
}

#[derive(Clone, Debug)]
pub struct ParsedBlueprintCharacter {
    pub name: String,
    pub character_type: String,
    pub gender: Option<String>,
    pub age: Option<String>,
    pub appearance: String,
    pub personality: String,
    pub background: String,
    pub motivation: String,
    pub arc_summary: String,
    pub relationship_to_protagonist: String,
    pub arc_stages: Option<Value>,
}

#[derive(Clone, Debug)]
pub struct ParsedBlueprintRelationship {
    pub relationship_type: String,
    pub character_a: String,
    pub character_b: String,
    pub description: String,
    pub strength: Option<i32>,
    pub evolution: Option<Value>,
}

#[derive(Clone, Debug)]
pub struct ParsedBlueprintOptimization {
    pub raw: Value,
    pub style: Option<String>,
    pub full_synopsis: Option<String>,
    pub world_setting: Option<Value>,
    pub characters: Option<Vec<ParsedBlueprintCharacter>>,
    pub relationships: Option<Vec<ParsedBlueprintRelationship>>,
}

pub const fn is_valid_blueprint_total_chapters(total_chapters: i32) -> bool {
    total_chapters >= BLUEPRINT_MIN_TOTAL_CHAPTERS && total_chapters <= BLUEPRINT_MAX_TOTAL_CHAPTERS
}

pub const fn is_valid_blueprint_generation_chapters(total_chapters: i32) -> bool {
    total_chapters >= BLUEPRINT_MIN_GENERATION_CHAPTERS
        && total_chapters <= BLUEPRINT_MAX_TOTAL_CHAPTERS
}

pub const fn needs_part_outlines_for_chapters(total_chapters: i32) -> bool {
    total_chapters > PART_OUTLINE_THRESHOLD
}
