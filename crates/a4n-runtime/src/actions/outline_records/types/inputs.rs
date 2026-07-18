use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Copy)]
pub struct OutlineTaskScope {
    pub project_id: i64,
    pub total: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratePartsInput {
    pub project_id: i64,
    pub part_count: Option<i32>,
    pub chapters_per_part: Option<i32>,
    pub task_id: Option<String>,
    pub variables: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegeneratePartsInput {
    pub project_id: i64,
    pub part_count: Option<i32>,
    pub chapters_per_part: Option<i32>,
    pub task_id: Option<String>,
    pub variables: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegenerateLastPartInput {
    pub project_id: i64,
    pub task_id: Option<String>,
    pub variables: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContinuePartOutlinesInput {
    pub project_id: i64,
    pub part_number: i32,
    pub start_chapter: i32,
    pub end_chapter: i32,
    pub task_id: Option<String>,
    pub variables: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegenerateChapterOutlineInput {
    pub project_id: i64,
    pub outline_id: i64,
    pub task_id: Option<String>,
    pub variables: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegeneratePartOutlineInput {
    pub project_id: i64,
    pub part_id: i64,
    pub task_id: Option<String>,
    pub variables: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratePartChapterOutlinesInput {
    pub project_id: i64,
    pub part_id: i64,
    pub task_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BatchGeneratePartOutlinesInput {
    pub project_id: i64,
    pub part_ids: Vec<i64>,
    pub task_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplyBeatMappingInput {
    pub project_id: i64,
    pub part_id: i64,
    pub beat_sheet: String,
    pub task_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerateChapterOutlinesByCountInput {
    pub project_id: i64,
    pub start_chapter: i32,
    pub count: i32,
    pub task_id: Option<String>,
}
