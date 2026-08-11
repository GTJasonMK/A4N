use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
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
