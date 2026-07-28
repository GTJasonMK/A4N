use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::generation_tasks::GenerationStartRequestMeta;

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
        #[schemars(range(min = 1))]
        part_count: Option<i32>,
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
