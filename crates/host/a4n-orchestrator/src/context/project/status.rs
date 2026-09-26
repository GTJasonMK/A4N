use a4n_db::DbHandle;
use a4n_db_models::models::project::novel_project;
use a4n_domain::project::{ProjectStatus, ProjectStatusArtifacts};
use a4n_error::AppError;
use a4n_event::InternalEvent;
use chrono::{NaiveDateTime, Utc};

use super::super::OrchestratorContext;
use crate::project::{
    PROJECT_KIND_TEXT, project_mutation_error_to_app_error, project_status_read_error_to_app_error,
    project_status_write_error_to_app_error,
};

#[derive(Clone)]
pub(in crate::context) struct ProjectStatusApplication {
    database: DbHandle,
}

struct ProjectStatusTransition {
    from: ProjectStatus,
    to: ProjectStatus,
    changed_at: NaiveDateTime,
}

impl ProjectStatusApplication {
    pub(in crate::context) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    async fn project_with_status(
        &self,
        project_id: i64,
        operation: &'static str,
    ) -> Result<(novel_project::Model, ProjectStatus), AppError> {
        let project = a4n_db::project::project_by_id_record(&self.database, project_id)
            .await
            .map_err(project_mutation_error_to_app_error)?
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
        if project.project_kind != PROJECT_KIND_TEXT {
            return Err(AppError::InvalidArgument(
                "非文本项目不能使用文本写作状态机".to_string(),
            ));
        }
        let status =
            ProjectStatus::parse_storage(&project.status).map_err(|_| AppError::InvalidState {
                current: project.status.clone(),
                operation: operation.to_string(),
            })?;
        Ok((project, status))
    }

    async fn artifacts(&self, project_id: i64) -> Result<ProjectStatusArtifacts, AppError> {
        a4n_db::project::project_status_artifacts(&self.database, project_id)
            .await
            .map_err(project_status_read_error_to_app_error)
    }

    async fn forward_loaded(
        &self,
        project: novel_project::Model,
        from: ProjectStatus,
        target: ProjectStatus,
        artifacts: ProjectStatusArtifacts,
    ) -> Result<ProjectStatusTransition, AppError> {
        if from >= target {
            return Err(AppError::InvalidState {
                current: from.as_str().to_string(),
                operation: format!("前进到状态 {}", target.as_str()),
            });
        }
        if !artifacts.supports(target) {
            let operation = match target {
                ProjectStatus::Draft => unreachable!("前进目标不能是草稿状态"),
                ProjectStatus::BlueprintReady => "前进到蓝图后续阶段（缺少故事蓝图）",
                ProjectStatus::PartOutlinesReady => "前进到分部大纲后续阶段（缺少分部大纲）",
                ProjectStatus::ChapterOutlinesReady => "前进到章节大纲后续阶段（缺少章节大纲）",
                ProjectStatus::Writing => "前进到 writing（至少需要一个章节已有正文）",
                ProjectStatus::Completed => "前进到 completed（至少需要一个章节已有正文）",
            };
            return Err(AppError::InvalidState {
                current: project.status.clone(),
                operation: operation.to_string(),
            });
        }

        let changed_at = Utc::now().naive_utc();
        let _ = a4n_db::project::update_project_status_record(
            &self.database,
            project,
            target,
            changed_at,
        )
        .await
        .map_err(project_status_write_error_to_app_error)?;

        Ok(ProjectStatusTransition {
            from,
            to: target,
            changed_at,
        })
    }

    async fn advance(
        &self,
        project_id: i64,
        target: ProjectStatus,
    ) -> Result<Option<ProjectStatusTransition>, AppError> {
        let (project, current) = self
            .project_with_status(project_id, "自动推进项目状态")
            .await?;
        if current >= target {
            return Ok(None);
        }
        let artifacts = self.artifacts(project.id).await?;
        self.forward_loaded(project, current, target, artifacts)
            .await
            .map(Some)
    }

    async fn sync_from_artifacts(
        &self,
        project_id: i64,
    ) -> Result<Option<ProjectStatusTransition>, AppError> {
        let (project_result, artifacts_result) = futures::join!(
            self.project_with_status(project_id, "同步项目状态"),
            self.artifacts(project_id),
        );
        let (project, current) = project_result?;
        let artifacts = artifacts_result?;
        let target = artifacts.furthest_status();
        if current >= target {
            return Ok(None);
        }
        self.forward_loaded(project, current, target, artifacts)
            .await
            .map(Some)
    }
}

impl OrchestratorContext {
    fn publish_project_status_transition(
        &self,
        project_id: i64,
        transition: &ProjectStatusTransition,
    ) -> Result<(), AppError> {
        self.publish_internal_event(InternalEvent::ProjectStatusChanged {
            project_id,
            from: transition.from,
            to: transition.to,
            timestamp: transition.changed_at,
        })?;
        Ok(())
    }

    pub(crate) async fn advance_project_status_if_allowed(
        &self,
        project_id: i64,
        target: ProjectStatus,
    ) -> Result<(), AppError> {
        if let Some(transition) = self.project_status.advance(project_id, target).await? {
            self.publish_project_status_transition(project_id, &transition)?;
        }
        Ok(())
    }

    pub(crate) async fn sync_project_status_from_artifacts(
        &self,
        project_id: i64,
    ) -> Result<(), AppError> {
        if let Some(transition) = self.project_status.sync_from_artifacts(project_id).await? {
            self.publish_project_status_transition(project_id, &transition)?;
        }
        Ok(())
    }
}
