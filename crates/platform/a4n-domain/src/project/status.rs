use chrono::NaiveDateTime;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("未知项目状态：{0}")]
pub struct ProjectStatusParseError(pub String);

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

    pub fn parse_storage(value: &str) -> Result<Self, ProjectStatusParseError> {
        Self::parse(value).ok_or_else(|| ProjectStatusParseError(value.to_string()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectStatusHistoryInput {
    pub project_id: i64,
    pub from_status: ProjectStatus,
    pub to_status: ProjectStatus,
    pub reason: String,
    pub changed_by: String,
    pub changed_at: NaiveDateTime,
}

/// Data categories that must be removed when a project status rolls back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupAction {
    DeleteChapters,
    DeleteChapterOutlines,
    DeletePartOutlines,
    DeleteBlueprintVersions,
}

pub fn rollback_actions(from: ProjectStatus, to: ProjectStatus) -> Vec<CleanupAction> {
    use CleanupAction::*;

    if from <= to {
        return Vec::new();
    }

    let mut actions = Vec::new();
    if from >= ProjectStatus::Writing && to < ProjectStatus::Writing {
        actions.push(DeleteChapters);
    }
    if from >= ProjectStatus::PartOutlinesReady && to < ProjectStatus::ChapterOutlinesReady {
        actions.push(DeleteChapterOutlines);
    }
    if from >= ProjectStatus::PartOutlinesReady && to < ProjectStatus::PartOutlinesReady {
        actions.push(DeletePartOutlines);
    }
    if to == ProjectStatus::Draft {
        actions.push(DeleteBlueprintVersions);
    }
    actions
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectStatusHistorySummary {
    pub id: i64,
    pub from_status: ProjectStatus,
    pub to_status: ProjectStatus,
    pub reason: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectStatusMachineSummary {
    pub current_status: ProjectStatus,
    pub forward_targets: Vec<ProjectStatus>,
    pub rollback_targets: Vec<ProjectStatus>,
    pub history: Vec<ProjectStatusHistorySummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectRollbackImpactSummary {
    pub title: String,
    pub count: i32,
    pub chapter_start: Option<i32>,
    pub chapter_end: Option<i32>,
    pub part_start: Option<i32>,
    pub part_end: Option<i32>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectRollbackPreview {
    pub path: Vec<ProjectStatus>,
    pub impacts: Vec<ProjectRollbackImpactSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectProgressSummary {
    pub label: String,
    pub detail: Option<String>,
}
