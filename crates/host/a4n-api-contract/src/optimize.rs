use super::generation_tasks::GenerationStartRequestMeta;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OptimizationAgentPermissions {
    pub read_project_detail: bool,
    pub read_blueprint: bool,
    pub read_outlines: bool,
    pub read_character_profiles: bool,
    pub read_foreshadowing: bool,
    pub automatic_writeback: bool,
}

impl OptimizationAgentPermissions {
    pub fn as_value(&self) -> Value {
        serde_json::to_value(self).expect("optimization permissions must serialize")
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OptimizationPatchOperation {
    Replace,
    Delete,
    InsertBefore,
    InsertAfter,
}

impl OptimizationPatchOperation {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Replace => "replace",
            Self::Delete => "delete",
            Self::InsertBefore => "insert_before",
            Self::InsertAfter => "insert_after",
        }
    }
}

impl fmt::Display for OptimizationPatchOperation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OptimizationPatch {
    pub patch_id: String,
    pub issue_id: Option<String>,
    pub operation: OptimizationPatchOperation,
    pub old_text: String,
    pub new_text: String,
    pub reason: String,
    pub risk: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct OptimizationSessionSummary {
    pub id: i64,
    pub project_id: i64,
    pub chapter_id: i64,
    pub name: Option<String>,
    pub status: String,
    pub issues: Option<Value>,
    pub agent_permissions: OptimizationAgentPermissions,
    pub agent_instruction: Option<String>,
    pub fixes_applied: i32,
    pub summary: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct OptimizationTurnEventSummary {
    pub id: i64,
    pub turn_id: i64,
    pub seq: i32,
    pub kind: String,
    pub status: String,
    pub public_text: String,
    pub payload: Option<Value>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct OptimizationTurnSummary {
    pub id: i64,
    pub session_id: i64,
    pub agent_turn_id: Option<i64>,
    pub user_text: String,
    pub status: String,
    pub permissions_snapshot: OptimizationAgentPermissions,
    pub final_reply: Option<String>,
    pub error: Option<String>,
    pub events: Vec<OptimizationTurnEventSummary>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct OptimizationPatchSetSummary {
    pub id: i64,
    pub turn_id: i64,
    pub session_id: i64,
    pub chapter_id: i64,
    pub base_content_hash: String,
    pub status: String,
    pub summary: String,
    pub patches: Vec<OptimizationPatch>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AnalyzeChapterInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub chapter_number: i32,
    #[schemars(length(min = 1, max = 200))]
    pub name: String,
    pub mode: String,
    pub scope: Option<String>,
    pub permissions: OptimizationAgentPermissions,
    pub agent_instruction: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CreateOptimizationChatSessionInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub chapter_number: i32,
    #[schemars(length(min = 1, max = 200))]
    pub name: String,
    pub permissions: OptimizationAgentPermissions,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PrepareOptimizationAnalysisSessionInput {
    #[schemars(range(min = 1))]
    pub session_id: i64,
    pub mode: String,
    pub scope: Option<String>,
    pub permissions: OptimizationAgentPermissions,
    pub agent_instruction: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProposeOptimizationPatchesInput {
    #[schemars(range(min = 1))]
    pub session_id: i64,
    pub issue_ids: Option<Vec<String>>,
    pub agent_instruction: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ApplyOptimizationPatchesInput {
    #[schemars(range(min = 1))]
    pub session_id: i64,
    pub base_content_hash: String,
    pub patches: Vec<OptimizationPatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ApplyOptimizationPatchesResponse {
    pub chapter_id: i64,
    pub applied_count: usize,
    pub word_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SendOptimizationMessageInput {
    #[schemars(range(min = 1))]
    pub session_id: i64,
    #[schemars(length(min = 1, max = 32768))]
    pub message: String,
    pub permissions: OptimizationAgentPermissions,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ListOptimizeSessionsInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub chapter_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OptimizeSessionInput {
    #[schemars(range(min = 1))]
    pub session_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DiscardOptimizationPatchSetInput {
    #[schemars(range(min = 1))]
    pub patch_set_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RecordOptimizeAgentEventInput {
    #[schemars(range(min = 1))]
    pub session_id: i64,
    #[schemars(length(min = 1, max = 32768))]
    pub content: String,
}

/// Agent context sources are inspected and read through the same domain owner
/// as the optimization session. The selection is intentionally explicit so a
/// model can choose paragraphs/fields/items without guessing a character page.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OptimizationContextSelection {
    #[default]
    All,
    Paragraphs {
        indexes: Vec<u32>,
    },
    Paths {
        paths: Vec<String>,
    },
    Items {
        indexes: Vec<u32>,
    },
}

/// Canonical optimization context sources. The `chapter` alias is accepted
/// for older prompts; responses always use `current_chapter`.
pub const OPTIMIZATION_CONTEXT_SOURCES: &[&str] = &[
    "current_chapter",
    "current_chapter_outline",
    "project_detail",
    "blueprint",
    "part_outlines",
    "chapter_outlines",
    "character_profiles",
    "foreshadowing_index",
    "previous_chapters",
    "next_chapters",
];

pub fn canonical_optimization_context_source(source: &str) -> Option<&'static str> {
    match source.trim() {
        "chapter" | "current_chapter" => Some("current_chapter"),
        "current_chapter_outline" => Some("current_chapter_outline"),
        "project_detail" => Some("project_detail"),
        "blueprint" => Some("blueprint"),
        "part_outlines" => Some("part_outlines"),
        "chapter_outlines" => Some("chapter_outlines"),
        "character_profiles" => Some("character_profiles"),
        "foreshadowing_index" => Some("foreshadowing_index"),
        "previous_chapters" => Some("previous_chapters"),
        "next_chapters" => Some("next_chapters"),
        _ => None,
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InspectOptimizationContextInput {
    #[serde(default)]
    #[schemars(
        description = "Canonical source: current_chapter, current_chapter_outline, project_detail, blueprint, part_outlines, chapter_outlines, character_profiles, foreshadowing_index, previous_chapters, or next_chapters. Legacy alias chapter is accepted."
    )]
    pub source: Option<String>,
    #[serde(default)]
    pub cursor: Option<String>,
    #[serde(default)]
    pub expected_hash: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReadOptimizationContextInput {
    #[schemars(
        description = "Canonical source: current_chapter, current_chapter_outline, project_detail, blueprint, part_outlines, chapter_outlines, character_profiles, foreshadowing_index, previous_chapters, or next_chapters. Legacy alias chapter is accepted."
    )]
    pub source: String,
    #[serde(default)]
    pub selection: OptimizationContextSelection,
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub cursor: Option<String>,
    #[serde(default)]
    pub expected_hash: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SearchOptimizationContextInput {
    pub query: String,
    #[serde(default)]
    #[schemars(
        description = "Canonical source: current_chapter, current_chapter_outline, project_detail, blueprint, part_outlines, chapter_outlines, character_profiles, foreshadowing_index, previous_chapters, or next_chapters. Legacy alias chapter is accepted."
    )]
    pub source: Option<String>,
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub cursor: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{OPTIMIZATION_CONTEXT_SOURCES, canonical_optimization_context_source};

    #[test]
    fn context_source_aliases_canonicalize_without_accepting_unknown_sources() {
        assert_eq!(
            canonical_optimization_context_source("chapter"),
            Some("current_chapter")
        );
        assert_eq!(
            canonical_optimization_context_source(" current_chapter "),
            Some("current_chapter")
        );
        assert!(canonical_optimization_context_source("unknown").is_none());
        assert!(OPTIMIZATION_CONTEXT_SOURCES.contains(&"current_chapter"));
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OptimizationArticlePatchToolInput {
    pub summary: String,
    pub patches: Vec<OptimizationPatch>,
}

/// Host-facing read boundary for content-optimization sessions.
///
/// Session creation, mutation and generation remain with the optimize owner.
#[async_trait::async_trait]
pub trait OptimizeSessionReadApplicationPort: Send + Sync {
    async fn list_sessions(
        &self,
        user_id: i64,
        input: ListOptimizeSessionsInput,
    ) -> Result<Vec<OptimizationSessionSummary>, a4n_error::AppError>;

    async fn get_session(
        &self,
        user_id: i64,
        input: OptimizeSessionInput,
    ) -> Result<OptimizationSessionSummary, a4n_error::AppError>;

    async fn list_turns(
        &self,
        user_id: i64,
        input: OptimizeSessionInput,
    ) -> Result<Vec<OptimizationTurnSummary>, a4n_error::AppError>;

    async fn list_patch_sets(
        &self,
        user_id: i64,
        input: OptimizeSessionInput,
    ) -> Result<Vec<OptimizationPatchSetSummary>, a4n_error::AppError>;
}

/// Start one Optimize chat turn through the canonical generation task path.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StartOptimizeChatGenerationInput {
    pub meta: GenerationStartRequestMeta,
    pub request: SendOptimizationMessageInput,
}

/// Start one prepared Optimize analysis through the canonical generation task path.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StartOptimizeAnalysisGenerationInput {
    pub meta: GenerationStartRequestMeta,
    #[schemars(range(min = 1))]
    pub session_id: i64,
}

/// Start one Optimize patch proposal through the canonical generation task path.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StartOptimizePatchGenerationInput {
    pub meta: GenerationStartRequestMeta,
    pub request: ProposeOptimizationPatchesInput,
}
