use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, de};
use serde_json::Value;

use super::generation_tasks::GenerationStartRequestMeta;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ChapterOutlineSummary {
    pub id: i64,
    pub project_id: i64,
    pub part_id: Option<i64>,
    pub chapter_number: i32,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub beats: Option<Value>,
    pub key_scenes: Option<Value>,
    pub pacing_score: Option<f32>,
    pub foreshadowing_setup: Option<Value>,
    pub foreshadowing_payoff: Option<Value>,
    pub characters_involved: Option<Value>,
    pub word_count_target: Option<i32>,
    pub status: String,
    /// 自动派生来源；`null` 表示用户/正式流程拥有的章纲。
    pub source_kind: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct PartOutlineSummary {
    pub id: i64,
    pub project_id: i64,
    pub part_number: i32,
    pub title: String,
    pub start_chapter: i32,
    pub end_chapter: i32,
    pub summary: Option<String>,
    pub theme: Option<String>,
    pub key_events: Option<Value>,
    pub character_focus: Option<Value>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SaveChapterOutlineInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[serde(default)]
    #[schemars(range(min = 1))]
    pub outline_id: Option<i64>,
    /// 编辑既有章纲时必填；创建时为 `null`。
    #[serde(default)]
    pub expected_updated_at: Option<String>,
    #[schemars(range(min = 1))]
    pub chapter_number: i32,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SavePartOutlineInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[serde(default)]
    #[schemars(range(min = 1))]
    pub part_id: Option<i64>,
    /// 编辑既有分部时必填；创建时为 `null`。
    #[serde(default)]
    pub expected_updated_at: Option<String>,
    #[schemars(range(min = 1))]
    pub part_number: i32,
    #[schemars(range(min = 1))]
    pub start_chapter: i32,
    #[schemars(range(min = 1))]
    pub end_chapter: i32,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub theme: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OutlineDeleteTarget {
    #[schemars(range(min = 1))]
    pub id: i64,
    #[schemars(length(min = 1, max = 64))]
    pub expected_updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeleteLatestOutlineRecordsInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(length(min = 1, max = 1024))]
    pub targets: Vec<OutlineDeleteTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GeneratePartsInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[serde(default)]
    #[schemars(range(min = 1))]
    pub part_count: Option<i32>,
    #[serde(default)]
    #[schemars(range(min = 1))]
    pub chapters_per_part: Option<i32>,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    #[schemars(with = "Option<BTreeMap<String, Value>>")]
    pub variables: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RegeneratePartsInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(length(min = 1, max = 1024))]
    pub expected_parts: Vec<OutlineDeleteTarget>,
    #[serde(default)]
    #[schemars(range(min = 1))]
    pub chapters_per_part: Option<i32>,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    #[schemars(with = "Option<BTreeMap<String, Value>>")]
    pub variables: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RegenerateLastPartInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    #[schemars(with = "Option<BTreeMap<String, Value>>")]
    pub variables: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ContinuePartOutlinesInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub part_number: i32,
    #[schemars(range(min = 1))]
    pub start_chapter: i32,
    #[schemars(range(min = 1))]
    pub end_chapter: i32,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    #[schemars(with = "Option<BTreeMap<String, Value>>")]
    pub variables: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RegenerateChapterOutlineInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub outline_id: i64,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    #[schemars(with = "Option<BTreeMap<String, Value>>")]
    pub variables: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RegeneratePartOutlineInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub part_id: i64,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    #[schemars(with = "Option<BTreeMap<String, Value>>")]
    pub variables: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GeneratePartChapterOutlinesInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub part_id: i64,
    #[serde(default)]
    pub task_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BatchGeneratePartOutlinesInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(length(min = 1, max = 1024))]
    pub part_ids: Vec<i64>,
    #[serde(default)]
    pub task_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ApplyBeatMappingInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub part_id: i64,
    #[schemars(length(min = 1, max = 262144))]
    pub beat_sheet: String,
    #[serde(default)]
    pub task_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GenerateChapterOutlinesByCountInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub start_chapter: i32,
    #[schemars(range(min = 1))]
    pub count: i32,
    #[serde(default)]
    pub task_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UpdateChapterOutlineInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub outline_id: i64,
    #[schemars(length(min = 1, max = 64))]
    pub expected_updated_at: String,
    #[serde(default, deserialize_with = "deserialize_optional_outline_title")]
    #[schemars(with = "String", length(min = 1))]
    pub title: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_outline_summary")]
    #[schemars(with = "String", length(min = 1))]
    pub summary: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_outline_beats")]
    #[schemars(with = "Vec<Value>")]
    pub beats: Option<Value>,
    #[serde(default, deserialize_with = "deserialize_optional_outline_key_scenes")]
    #[schemars(with = "Vec<Value>")]
    pub key_scenes: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct SaveChapterOutlineResult {
    pub outline: ChapterOutlineSummary,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct SavePartOutlineResult {
    pub part: PartOutlineSummary,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct UpdateChapterOutlineResult {
    pub outline: ChapterOutlineSummary,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct OutlineMutationResult {
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct OutlineGenerationProgressSummary {
    pub total_parts: u64,
    pub completed_parts: u64,
    pub total_chapter_outlines: u64,
    pub completed_chapter_outlines: u64,
}

/// Host-facing outline read boundary.
///
/// Outline visibility, source-kind projection and generation progress remain
/// owned by the outline application; Runtime only routes complete DTOs.
#[async_trait::async_trait]
pub trait OutlineReadApplicationPort: Send + Sync {
    async fn list_chapters(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<ChapterOutlineSummary>, a4n_error::AppError>;

    async fn list_parts(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<PartOutlineSummary>, a4n_error::AppError>;

    async fn generation_progress(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<OutlineGenerationProgressSummary, a4n_error::AppError>;

    async fn ensure_chapter_outline_access(
        &self,
        user_id: i64,
        project_id: i64,
        outline_id: i64,
    ) -> Result<(), a4n_error::AppError>;

    async fn ensure_part_access(
        &self,
        user_id: i64,
        project_id: i64,
        part_id: i64,
    ) -> Result<(), a4n_error::AppError>;
}

/// Host-facing outline mutation boundary.
///
/// Permission checks, input normalization, persistence and outline DTO
/// projection stay with the outline owner. Runtime may append index warnings
/// from its RAG adapter after the durable mutation succeeds.
#[async_trait::async_trait]
pub trait OutlineMutationApplicationPort: Send + Sync {
    async fn delete_latest_chapters(
        &self,
        user_id: i64,
        input: DeleteLatestOutlineRecordsInput,
    ) -> Result<OutlineMutationResult, a4n_error::AppError>;

    async fn delete_latest_parts(
        &self,
        user_id: i64,
        input: DeleteLatestOutlineRecordsInput,
    ) -> Result<OutlineMutationResult, a4n_error::AppError>;

    async fn save_chapter(
        &self,
        user_id: i64,
        input: SaveChapterOutlineInput,
    ) -> Result<SaveChapterOutlineResult, a4n_error::AppError>;

    async fn update_chapter(
        &self,
        user_id: i64,
        input: UpdateChapterOutlineInput,
    ) -> Result<UpdateChapterOutlineResult, a4n_error::AppError>;

    async fn save_part(
        &self,
        user_id: i64,
        input: SavePartOutlineInput,
    ) -> Result<SavePartOutlineResult, a4n_error::AppError>;
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ChapterOutlinePacingAnalysis {
    pub chapter_number: i32,
    pub title: Option<String>,
    pub pacing_score: f64,
    pub word_count_target: Option<i32>,
    pub has_beats: bool,
    pub has_key_scenes: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ChapterOutlinePacing {
    pub pacing_score: f64,
    pub chapter_count: usize,
    pub summary: Option<String>,
    pub analysis: Vec<ChapterOutlinePacingAnalysis>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StartOutlineGenerationInput {
    pub meta: GenerationStartRequestMeta,
    #[schemars(range(min = 1))]
    pub project_id: i64,
    pub operation: OutlineGenerationOperation,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OutlineGenerationOperation {
    GenerateParts {
        #[schemars(range(min = 1))]
        part_count: Option<i32>,
        #[schemars(range(min = 1))]
        chapters_per_part: Option<i32>,
        variables: Option<BTreeMap<String, Value>>,
    },
    RegenerateParts {
        #[schemars(length(min = 1, max = 1024))]
        expected_parts: Vec<OutlineDeleteTarget>,
        #[schemars(range(min = 1))]
        chapters_per_part: Option<i32>,
        variables: Option<BTreeMap<String, Value>>,
    },
    RegenerateLastPart {
        variables: Option<BTreeMap<String, Value>>,
    },
    ContinuePartOutlines {
        #[schemars(range(min = 1))]
        part_number: i32,
        #[schemars(range(min = 1))]
        start_chapter: i32,
        #[schemars(range(min = 1))]
        end_chapter: i32,
        variables: Option<BTreeMap<String, Value>>,
    },
    RegeneratePartOutline {
        #[schemars(range(min = 1))]
        part_id: i64,
        variables: Option<BTreeMap<String, Value>>,
    },
    GeneratePartChapterOutlines {
        #[schemars(range(min = 1))]
        part_id: i64,
    },
    BatchGeneratePartOutlines {
        #[schemars(length(min = 1, max = 1024))]
        part_ids: Vec<i64>,
    },
    GenerateChapterOutlinesByCount {
        #[schemars(range(min = 1))]
        start_chapter: i32,
        #[schemars(range(min = 1))]
        count: i32,
    },
    RegenerateChapterOutline {
        #[schemars(range(min = 1))]
        outline_id: i64,
        variables: Option<BTreeMap<String, Value>>,
    },
    ApplyBeatMapping {
        #[schemars(range(min = 1))]
        part_id: i64,
        #[schemars(length(min = 1, max = 262144))]
        beat_sheet: String,
    },
}

fn deserialize_optional_outline_title<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_optional_outline_text(deserializer, "title")
}

fn deserialize_optional_outline_summary<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_optional_outline_text(deserializer, "summary")
}

fn deserialize_optional_outline_text<'de, D>(
    deserializer: D,
    field: &str,
) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return Err(de::Error::custom(format!("章节大纲字段 {field} 不能为空")));
            }
            Ok(Some(trimmed.to_string()))
        }
        Value::Null => Err(de::Error::custom(format!(
            "章节大纲字段 {field} 不能为 null"
        ))),
        _ => Err(de::Error::custom(format!(
            "章节大纲字段 {field} 必须是字符串"
        ))),
    }
}

fn deserialize_optional_outline_beats<'de, D>(deserializer: D) -> Result<Option<Value>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_optional_outline_array(deserializer, "beats")
}

fn deserialize_optional_outline_key_scenes<'de, D>(
    deserializer: D,
) -> Result<Option<Value>, D::Error>
where
    D: Deserializer<'de>,
{
    deserialize_optional_outline_array(deserializer, "key_scenes")
}

fn deserialize_optional_outline_array<'de, D>(
    deserializer: D,
    field: &str,
) -> Result<Option<Value>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::Array(_) => Ok(Some(value)),
        Value::Null => Err(de::Error::custom(format!(
            "章节大纲字段 {field} 不能为 null"
        ))),
        _ => Err(de::Error::custom(format!(
            "章节大纲字段 {field} 必须是数组"
        ))),
    }
}
