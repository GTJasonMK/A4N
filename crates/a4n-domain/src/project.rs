use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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

    pub const fn ordinal(self) -> u8 {
        match self {
            Self::Draft => 0,
            Self::BlueprintReady => 1,
            Self::PartOutlinesReady => 2,
            Self::ChapterOutlinesReady => 3,
            Self::Writing => 4,
            Self::Completed => 5,
        }
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

#[derive(Serialize, JsonSchema)]
pub struct ProjectStatusHistorySummary {
    pub id: i64,
    pub from_status: ProjectStatus,
    pub to_status: ProjectStatus,
    pub reason: Option<String>,
    pub created_at: String,
}

#[derive(Serialize, JsonSchema)]
pub struct ProjectStatusMachineSummary {
    pub current_status: ProjectStatus,
    pub forward_targets: Vec<ProjectStatus>,
    pub rollback_targets: Vec<ProjectStatus>,
    pub history: Vec<ProjectStatusHistorySummary>,
}

#[derive(Serialize, JsonSchema)]
pub struct ProjectRollbackImpactSummary {
    pub title: String,
    pub count: i32,
    pub chapter_start: Option<i32>,
    pub chapter_end: Option<i32>,
    pub part_start: Option<i32>,
    pub part_end: Option<i32>,
    pub note: Option<String>,
}

#[derive(Serialize, JsonSchema)]
pub struct ProjectRollbackPreview {
    pub path: Vec<ProjectStatus>,
    pub impacts: Vec<ProjectRollbackImpactSummary>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProjectProgressStatus {
    Idle,
    Running,
    Blocked,
    Failed,
    Completed,
}

#[derive(Debug, PartialEq, Eq, Serialize, JsonSchema)]
pub struct ProjectProgressSummary {
    pub kind: String,
    pub stage: String,
    pub label: String,
    pub current: Option<i64>,
    pub total: Option<i64>,
    pub status: ProjectProgressStatus,
    pub detail: Option<String>,
}
