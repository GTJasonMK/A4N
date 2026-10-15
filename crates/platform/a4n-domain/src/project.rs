mod asset_graph;
mod blueprint;
mod cleanup;
mod family_transfer;
#[path = "optimization.rs"]
mod optimization;
mod status;

pub use asset_graph::*;
pub use blueprint::*;
pub use cleanup::*;
pub use family_transfer::*;
pub use optimization::*;
pub use status::*;

use chrono::NaiveDateTime;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectSummaryRecord {
    pub id: i64,
    pub content_revision: i64,
    pub user_id: i64,
    pub title: String,
    pub description: String,
    pub project_type: String,
    pub project_kind: String,
    pub root_project_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
    pub source_sync_status: String,
    pub status: String,
    pub lifecycle_gate: String,
    pub target_chapters: i32,
    pub current_chapter: i32,
    pub total_chapters: Option<i32>,
    pub chapters_per_part: Option<i32>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlueprintVersionRecord {
    pub id: i64,
    pub project_id: i64,
    pub version: String,
    pub change_description: Option<String>,
    pub blueprint_data: Value,
    pub created_at: NaiveDateTime,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectRecord {
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub description: String,
    pub project_type: String,
    pub project_kind: String,
    pub root_project_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
    pub source_sync_status: String,
    pub status: String,
    #[serde(skip)]
    pub lifecycle_gate: String,
    pub target_chapters: i32,
    pub current_chapter: i32,
    pub total_chapters: Option<i32>,
    pub chapters_per_part: Option<i32>,
    pub avatar_svg: Option<String>,
    pub avatar_animal: Option<String>,
    pub raw_blueprint: Option<String>,
    pub import_analysis_progress: Option<String>,
    pub export_config: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl ProjectRecord {
    pub fn source_mode(&self) -> Result<ProjectSourceMode, ProjectSourceModeError> {
        ProjectSourceMode::from_identity(
            &self.project_kind,
            self.root_project_id,
            self.source_snapshot_id,
        )
    }
}

/// 一页制分页：page 从 1 起，计算 offset。
pub fn project_list_offset(page: u64, limit: u64) -> Result<u64, String> {
    page.checked_sub(1)
        .and_then(|page_index| page_index.checked_mul(limit))
        .ok_or_else(|| "项目分页参数过大".to_string())
}

/// 必填文本：trim 后非空。
pub fn required_text<'a>(value: &'a str, field: &str) -> Result<&'a str, String> {
    let value = value.trim();
    if value.is_empty() {
        Err(format!("{field} 不能为空"))
    } else {
        Ok(value)
    }
}

/// 可选文本：None 或空白视为空串。
pub fn optional_text(value: Option<&str>) -> String {
    value.map(str::trim).unwrap_or_default().to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BlueprintRecord {
    pub id: i64,
    pub project_id: i64,
    pub title: String,
    pub genre: String,
    pub style: Option<String>,
    pub tone: Option<String>,
    pub target_audience: Option<String>,
    pub one_sentence_summary: Option<String>,
    pub full_synopsis: Option<String>,
    pub world_setting: Option<Value>,
    pub current_version: String,
    pub needs_part_outlines: bool,
    pub total_chapters: Option<i32>,
    pub chapters_per_part: Option<i32>,
    pub avatar_svg: Option<String>,
    pub avatar_animal: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectMediaCopyError {
    #[error("媒体源路径无效：{0}")]
    InvalidSourcePath(String),
    #[error("媒体文件缺少扩展名: {path}")]
    MissingExtension { path: String },
    #[error("无法创建媒体复制目录 {}: {source}", path.display())]
    CreateDirectory {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(
        "复制媒体文件失败: {} -> {}: {source}",
        source_path.display(),
        target_path.display()
    )]
    CopyFile {
        source_path: PathBuf,
        target_path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

pub trait ProjectMediaFileCopier {
    fn copy_media_file(
        &mut self,
        new_project_id: i64,
        source_media_asset_id: i64,
        field: &str,
        relative_path: Option<&str>,
    ) -> Result<Option<String>, ProjectMediaCopyError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProjectKind {
    Text,
    Comic,
    Video,
    Canvas,
    Galgame,
}

impl ProjectKind {
    pub const ALL: [Self; 5] = [
        Self::Text,
        Self::Comic,
        Self::Video,
        Self::Canvas,
        Self::Galgame,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Comic => "comic",
            Self::Video => "video",
            Self::Canvas => "canvas",
            Self::Galgame => "galgame",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProjectSourceMode {
    #[serde(rename = "self")]
    Self_,
    Derived,
    Freeform,
    Standalone,
}

impl ProjectSourceMode {
    pub fn from_identity(
        project_kind: &str,
        root_project_id: Option<i64>,
        source_snapshot_id: Option<i64>,
    ) -> Result<Self, ProjectSourceModeError> {
        let project_kind = ProjectKind::parse(project_kind).ok_or_else(|| {
            ProjectSourceModeError::InvalidProjectKind {
                value: project_kind.to_string(),
            }
        })?;
        match (
            project_kind,
            root_project_id.is_some(),
            source_snapshot_id.is_some(),
        ) {
            (ProjectKind::Text, false, false) => Ok(Self::Self_),
            (ProjectKind::Canvas, false, false) => Ok(Self::Freeform),
            (ProjectKind::Comic, false, false) => Ok(Self::Standalone),
            (ProjectKind::Comic | ProjectKind::Video | ProjectKind::Galgame, true, true) => {
                Ok(Self::Derived)
            }
            _ => Err(ProjectSourceModeError::InvalidIdentity {
                project_kind,
                root_project_id,
                source_snapshot_id,
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProjectSourceModeError {
    #[error("项目族无效: {value}")]
    InvalidProjectKind { value: String },
    #[error(
        "项目来源身份无效: project_kind={project_kind:?}, root_project_id={root_project_id:?}, source_snapshot_id={source_snapshot_id:?}"
    )]
    InvalidIdentity {
        project_kind: ProjectKind,
        root_project_id: Option<i64>,
        source_snapshot_id: Option<i64>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SourceSnapshotChapterSummary {
    pub id: i64,
    pub snapshot_id: i64,
    pub root_project_id: i64,
    pub source_chapter_id: i64,
    pub chapter_number: i32,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub content: String,
    pub analysis_data_json: Option<Value>,
    pub content_hash: String,
    pub analysis_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SourceSnapshotChapterIndex {
    pub id: i64,
    pub snapshot_id: i64,
    pub source_chapter_id: i64,
    pub chapter_number: i32,
    pub title: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{
        BLUEPRINT_MAX_TOTAL_CHAPTERS, BLUEPRINT_MIN_GENERATION_CHAPTERS,
        BLUEPRINT_MIN_TOTAL_CHAPTERS, CleanupAction, PART_OUTLINE_THRESHOLD, ProjectSourceMode,
        ProjectStatus, ProjectStatusArtifacts, build_blueprint_generation_payload,
        increment_blueprint_version, is_valid_blueprint_generation_chapters,
        is_valid_blueprint_total_chapters, needs_part_outlines_for_chapters,
        project_status_after_blueprint_generation, rollback_actions,
        validate_blueprint_chapter_count,
    };
    use serde_json::json;

    #[test]
    fn project_source_mode_is_derived_from_the_identity_shape() {
        assert_eq!(
            ProjectSourceMode::from_identity("text", None, None),
            Ok(ProjectSourceMode::Self_)
        );
        assert_eq!(
            ProjectSourceMode::from_identity("canvas", None, None),
            Ok(ProjectSourceMode::Freeform)
        );
        assert!(ProjectSourceMode::from_identity("video", None, None).is_err());
        assert_eq!(
            ProjectSourceMode::from_identity("comic", None, None),
            Ok(ProjectSourceMode::Standalone)
        );
        assert_eq!(
            ProjectSourceMode::from_identity("video", Some(1), Some(2)),
            Ok(ProjectSourceMode::Derived)
        );
        assert!(ProjectSourceMode::from_identity("text", Some(1), Some(2)).is_err());
    }

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

    #[test]
    fn part_outline_threshold_is_strictly_greater_than_limit() {
        assert!(!needs_part_outlines_for_chapters(PART_OUTLINE_THRESHOLD));
        assert!(needs_part_outlines_for_chapters(PART_OUTLINE_THRESHOLD + 1));
    }

    #[test]
    fn blueprint_chapter_ranges_distinguish_persistence_from_generation() {
        assert!(is_valid_blueprint_total_chapters(
            BLUEPRINT_MIN_TOTAL_CHAPTERS
        ));
        assert!(!is_valid_blueprint_generation_chapters(
            BLUEPRINT_MIN_GENERATION_CHAPTERS - 1
        ));
        assert!(is_valid_blueprint_generation_chapters(
            BLUEPRINT_MIN_GENERATION_CHAPTERS
        ));
        assert!(!is_valid_blueprint_total_chapters(
            BLUEPRINT_MAX_TOTAL_CHAPTERS + 1
        ));
    }

    #[test]
    fn project_status_storage_parser_accepts_only_canonical_values() {
        assert_eq!(
            ProjectStatus::parse_storage("blueprint_ready"),
            Ok(ProjectStatus::BlueprintReady)
        );
        for value in ["Draft", " blueprint_ready ", "unknown"] {
            assert!(ProjectStatus::parse_storage(value).is_err(), "{value}");
        }
    }

    #[test]
    fn rollback_actions_match_the_destructive_status_edges() {
        let all_statuses = &ProjectStatus::ALL;
        let edge_count = all_statuses
            .iter()
            .flat_map(|from| all_statuses.iter().map(move |to| (*from, *to)))
            .filter(|(from, to)| !rollback_actions(*from, *to).is_empty())
            .count();
        assert_eq!(edge_count, 14);
        assert_eq!(
            rollback_actions(ProjectStatus::Writing, ProjectStatus::Draft),
            vec![
                CleanupAction::DeleteChapters,
                CleanupAction::DeleteChapterOutlines,
                CleanupAction::DeletePartOutlines,
                CleanupAction::DeleteBlueprintVersions,
            ]
        );
        assert!(rollback_actions(ProjectStatus::Completed, ProjectStatus::Writing).is_empty());
        assert!(rollback_actions(ProjectStatus::Draft, ProjectStatus::Writing).is_empty());
    }

    #[test]
    fn blueprint_rules_have_one_domain_owner() {
        assert_eq!(increment_blueprint_version("1.9"), Ok("1.10".to_string()));
        assert!(increment_blueprint_version("1.0.0").is_err());
        assert_eq!(
            project_status_after_blueprint_generation(ProjectStatus::Writing),
            ProjectStatus::Writing
        );
        assert!(validate_blueprint_chapter_count(4, "target").is_err());
        let variables = std::collections::HashMap::from([
            ("project".to_string(), json!({"id": 7})),
            ("ignored".to_string(), json!(true)),
        ]);
        let payload = build_blueprint_generation_payload("title", "genre", 5, &variables)
            .expect("valid blueprint payload");
        assert_eq!(payload["project"], json!({"id": 7}));
        assert!(payload.get("ignored").is_none());
    }
}
