use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProjectKind {
    Text,
    Comic,
    Video,
    Galgame,
}

impl ProjectKind {
    pub const ALL: [Self; 4] = [Self::Text, Self::Comic, Self::Video, Self::Galgame];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Comic => "comic",
            Self::Video => "video",
            Self::Galgame => "galgame",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == value)
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatus {
    Draft,
    BlueprintReady,
    PartOutlinesReady,
    ChapterOutlinesReady,
    Writing,
    Completed,
}

impl ProjectStatus {
    pub const ALL: [Self; 6] = [
        Self::Draft,
        Self::BlueprintReady,
        Self::PartOutlinesReady,
        Self::ChapterOutlinesReady,
        Self::Writing,
        Self::Completed,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::BlueprintReady => "blueprint_ready",
            Self::PartOutlinesReady => "part_outlines_ready",
            Self::ChapterOutlinesReady => "chapter_outlines_ready",
            Self::Writing => "writing",
            Self::Completed => "completed",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|status| status.as_str() == value)
    }
}

impl std::fmt::Display for ProjectStatus {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Draft => "草稿",
            Self::BlueprintReady => "蓝图就绪",
            Self::PartOutlinesReady => "分部大纲就绪",
            Self::ChapterOutlinesReady => "章节大纲就绪",
            Self::Writing => "写作中",
            Self::Completed => "已完成",
        })
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProjectStatusArtifacts {
    pub has_blueprint: bool,
    pub has_part_outline: bool,
    pub has_chapter_outline: bool,
    pub has_chapter_content: bool,
}

impl ProjectStatusArtifacts {
    pub const fn supports(self, status: ProjectStatus) -> bool {
        match status {
            ProjectStatus::Draft => true,
            ProjectStatus::BlueprintReady => self.has_blueprint,
            ProjectStatus::PartOutlinesReady => self.has_part_outline,
            ProjectStatus::ChapterOutlinesReady => self.has_chapter_outline,
            ProjectStatus::Writing | ProjectStatus::Completed => self.has_chapter_content,
        }
    }

    pub const fn furthest_status(self) -> ProjectStatus {
        if self.has_chapter_content {
            ProjectStatus::Writing
        } else if self.has_chapter_outline {
            ProjectStatus::ChapterOutlinesReady
        } else if self.has_part_outline {
            ProjectStatus::PartOutlinesReady
        } else if self.has_blueprint {
            ProjectStatus::BlueprintReady
        } else {
            ProjectStatus::Draft
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct ProjectStatusHistorySummary {
    pub id: i64,
    pub from_status: ProjectStatus,
    pub to_status: ProjectStatus,
    pub reason: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct ProjectStatusMachineSummary {
    pub current_status: ProjectStatus,
    pub forward_targets: Vec<ProjectStatus>,
    pub rollback_targets: Vec<ProjectStatus>,
    pub history: Vec<ProjectStatusHistorySummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct ProjectRollbackImpactSummary {
    pub title: String,
    pub count: i32,
    pub chapter_start: Option<i32>,
    pub chapter_end: Option<i32>,
    pub part_start: Option<i32>,
    pub part_end: Option<i32>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct ProjectRollbackPreview {
    pub path: Vec<ProjectStatus>,
    pub impacts: Vec<ProjectRollbackImpactSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
pub struct ProjectProgressSummary {
    pub label: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AssetDependencyGraph {
    pub project_id: i64,
    pub generated_at: String,
    pub nodes: Vec<AssetGraphNode>,
    pub edges: Vec<AssetGraphEdge>,
    pub layers: Vec<AssetGraphLayer>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AssetGraphNode {
    pub id: String,
    pub kind: String,
    pub asset_type: String,
    pub label: String,
    pub count: u64,
    pub status: Option<String>,
    pub layer: i32,
    pub chapter_id: Option<i64>,
    pub profile_id: Option<i64>,
    pub record_id: Option<i64>,
    pub editable: bool,
    pub mutable: bool,
    pub direct_downstream_count: u64,
    pub downstream_count: u64,
    pub mutation_policy: String,
    pub needs_visualization: bool,
    pub visualized: bool,
    pub summary: Option<String>,
    pub details: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AssetGraphEdge {
    pub from: String,
    pub to: String,
    pub relation: String,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AssetGraphLayer {
    pub index: i32,
    pub label: String,
    pub node_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AssetGraphNodeDetail {
    pub node_id: String,
    pub title: String,
    pub kind: String,
    pub asset_type: String,
    pub summary: Option<String>,
    pub records: Vec<AssetLineageRecord>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AssetLineageRecord {
    pub id: String,
    pub label: String,
    pub asset_kind: String,
    pub asset_id: Option<i64>,
    pub status: Option<String>,
    pub origin: String,
    pub source_refs: Vec<AssetLineageRef>,
    pub prompt_sources: Vec<PromptSource>,
    pub media_sources: Vec<AssetLineageRef>,
    pub request_snapshot: Option<Value>,
    pub input_status: String,
    pub generation_runs: Vec<GenerationRunSnapshot>,
    pub generation_inputs: Vec<GenerationInputSource>,
    pub result: Value,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AssetLineageRef {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub asset_type: String,
    pub record_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PromptSource {
    pub id: String,
    pub kind: String,
    pub text: String,
    pub source_record_id: Option<i64>,
    pub edited_by_user: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GenerationRunSnapshot {
    pub id: Option<i64>,
    pub workflow: String,
    pub step_key: String,
    pub title: String,
    pub model_kind: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub status: String,
    pub created_at: Option<String>,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GenerationInputSource {
    pub id: String,
    pub run_id: Option<i64>,
    pub group: String,
    pub kind: String,
    pub label: String,
    pub content_type: String,
    pub text: Option<String>,
    pub value: Option<Value>,
    pub source_record_id: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::{ProjectStatus, ProjectStatusArtifacts};

    #[test]
    fn project_status_order_matches_all_and_artifact_progress() {
        assert!(ProjectStatus::ALL.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(
            ProjectStatusArtifacts {
                has_blueprint: true,
                has_part_outline: true,
                has_chapter_outline: true,
                has_chapter_content: true,
            }
            .furthest_status(),
            ProjectStatus::Writing
        );
        assert!(
            ProjectStatusArtifacts {
                has_chapter_content: true,
                ..Default::default()
            }
            .supports(ProjectStatus::Completed)
        );
    }
}
