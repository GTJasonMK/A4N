use a4n_core::models::project::novel_project;
pub use a4n_domain::project::ProjectProgressSummary;
use a4n_domain::project::{ProjectKind, ProjectStatus};
use sea_orm::DbErr;

use crate::project::{SOURCE_MODE_DERIVED, SOURCE_MODE_FREEFORM, SOURCE_MODE_SELF};

#[derive(Debug, PartialEq, Eq)]
pub struct ProjectProgressQuery {
    pub project_id: i64,
    pub project_kind: ProjectKind,
    pub source_mode: String,
    pub status: ProjectStatus,
    pub current_chapter: i32,
    pub target_chapters: i32,
}

impl ProjectProgressQuery {
    pub fn from_project(project: &novel_project::Model) -> Result<Self, ProjectProgressReadError> {
        let project_kind = ProjectKind::parse(&project.project_kind).ok_or_else(|| {
            ProjectProgressReadError::InvalidProjectKind {
                project_id: project.id,
                value: project.project_kind.clone(),
            }
        })?;
        let status = ProjectStatus::parse(&project.status).ok_or_else(|| {
            ProjectProgressReadError::InvalidProjectStatus {
                project_id: project.id,
                value: project.status.clone(),
            }
        })?;
        if !matches!(
            project.source_mode.as_str(),
            SOURCE_MODE_SELF | SOURCE_MODE_DERIVED | SOURCE_MODE_FREEFORM
        ) {
            return Err(ProjectProgressReadError::InvalidSourceMode {
                project_id: project.id,
                value: project.source_mode.clone(),
            });
        }
        let source_mode_valid = match project_kind {
            ProjectKind::Text => project.source_mode == SOURCE_MODE_SELF,
            ProjectKind::Comic | ProjectKind::Galgame => project.source_mode == SOURCE_MODE_DERIVED,
            ProjectKind::Video => matches!(
                project.source_mode.as_str(),
                SOURCE_MODE_DERIVED | SOURCE_MODE_FREEFORM
            ),
        };
        if !source_mode_valid {
            return Err(ProjectProgressReadError::InvalidProjectSource {
                project_id: project.id,
                project_kind: project_kind.as_str().to_string(),
                source_mode: project.source_mode.clone(),
            });
        }
        if project.current_chapter < 0 || project.target_chapters < 0 {
            return Err(ProjectProgressReadError::InvalidChapterProgress {
                project_id: project.id,
                current: project.current_chapter,
                target: project.target_chapters,
            });
        }
        Ok(Self {
            project_id: project.id,
            project_kind,
            source_mode: project.source_mode.clone(),
            status,
            current_chapter: project.current_chapter,
            target_chapters: project.target_chapters,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectProgressReadError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("项目 #{project_id} 的项目族无效: {value}")]
    InvalidProjectKind { project_id: i64, value: String },
    #[error("项目 #{project_id} 的状态无效: {value}")]
    InvalidProjectStatus { project_id: i64, value: String },
    #[error("项目 #{project_id} 的来源模式无效: {value}")]
    InvalidSourceMode { project_id: i64, value: String },
    #[error("项目 #{project_id} 的项目族与来源模式不匹配: {project_kind}/{source_mode}")]
    InvalidProjectSource {
        project_id: i64,
        project_kind: String,
        source_mode: String,
    },
    #[error("项目 #{project_id} 的章节进度无效: {current}/{target}")]
    InvalidChapterProgress {
        project_id: i64,
        current: i32,
        target: i32,
    },
    #[error("{table} #{record_id} 的状态无效: {value}")]
    InvalidPersistedStatus {
        table: &'static str,
        record_id: i64,
        value: String,
    },
    #[error("{table} #{record_id} 缺少必需引用: {field}")]
    InvalidPersistedReference {
        table: &'static str,
        record_id: i64,
        field: &'static str,
    },
    #[error("项目缺少进度摘要: {project_id}")]
    SummaryMissing { project_id: i64 },
    #[error("{operation}: {source}")]
    Database {
        operation: &'static str,
        #[source]
        source: DbErr,
    },
}

impl ProjectProgressReadError {
    pub(super) fn database(operation: &'static str, source: DbErr) -> Self {
        Self::Database { operation, source }
    }

    pub(super) fn invalid_persisted_status(
        table: &'static str,
        record_id: i64,
        value: String,
    ) -> Self {
        Self::InvalidPersistedStatus {
            table,
            record_id,
            value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::error::Error;

    fn project() -> novel_project::Model {
        let now = Utc::now().naive_utc();
        novel_project::Model {
            id: 17,
            user_id: 1,
            title: "测试项目".to_string(),
            description: String::new(),
            project_type: "novel_long".to_string(),
            project_kind: "text".to_string(),
            source_mode: "self".to_string(),
            root_project_id: None,
            parent_project_id: None,
            source_snapshot_id: None,
            source_sync_status: "current".to_string(),
            derived_project_status: None,
            status: "writing".to_string(),
            lifecycle_gate: "active".to_string(),
            target_chapters: 10,
            current_chapter: 3,
            total_chapters: Some(10),
            chapters_per_part: None,
            avatar_svg: None,
            avatar_animal: None,
            raw_blueprint: None,
            import_analysis_progress: None,
            export_config: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn project_progress_query_parses_project_contract_once() {
        let query = ProjectProgressQuery::from_project(&project()).unwrap();
        assert_eq!(query.project_kind, ProjectKind::Text);
        assert_eq!(query.status, ProjectStatus::Writing);

        let mut invalid = project();
        invalid.project_kind = "legacy".to_string();
        assert!(matches!(
            ProjectProgressQuery::from_project(&invalid),
            Err(ProjectProgressReadError::InvalidProjectKind { project_id: 17, value })
                if value == "legacy"
        ));

        let mut invalid = project();
        invalid.source_mode = "legacy".to_string();
        assert!(matches!(
            ProjectProgressQuery::from_project(&invalid),
            Err(ProjectProgressReadError::InvalidSourceMode { project_id: 17, value })
                if value == "legacy"
        ));

        let mut invalid = project();
        invalid.project_kind = "comic".to_string();
        assert!(matches!(
            ProjectProgressQuery::from_project(&invalid),
            Err(ProjectProgressReadError::InvalidProjectSource {
                project_id: 17,
                project_kind,
                source_mode,
            }) if project_kind == "comic" && source_mode == "self"
        ));

        let mut invalid = project();
        invalid.current_chapter = -1;
        assert!(matches!(
            ProjectProgressQuery::from_project(&invalid),
            Err(ProjectProgressReadError::InvalidChapterProgress {
                project_id: 17,
                current: -1,
                target: 10,
            })
        ));
    }

    #[test]
    fn project_progress_read_error_preserves_operation_and_invariant_fields() {
        let missing = ProjectProgressReadError::SummaryMissing { project_id: 17 };
        assert_eq!(missing.to_string(), "项目缺少进度摘要: 17");
        assert!(missing.source().is_none());

        let database = ProjectProgressReadError::database(
            "查询视频提示词失败",
            DbErr::Custom("query failed".to_string()),
        );
        assert!(matches!(
            database,
            ProjectProgressReadError::Database {
                operation: "查询视频提示词失败",
                source: DbErr::Custom(ref message),
            } if message == "query failed"
        ));
        assert!(database.source().is_some());
    }
}
