use a4n_core::{
    error::AppError,
    event::bridge::InternalEvent,
    models::project::novel_project,
    state::{rollback::rollback_actions, validator::parse_project_status},
};
use a4n_db::{DbHandle, project_reset::ProjectStatusRollbackOutcome};
use a4n_domain::project::{
    ProjectRollbackPreview, ProjectStatus, ProjectStatusArtifacts, ProjectStatusMachineSummary,
};
use chrono::{NaiveDateTime, Utc};

use super::OrchestratorContext;
use crate::{
    collaboration::ProjectAccessRole,
    project::{
        PROJECT_KIND_TEXT, project_mutation_error_to_app_error, project_reset_error_to_app_error,
        project_rollback_impact_read_error_to_app_error, project_status_read_error_to_app_error,
        project_status_write_error_to_app_error,
    },
};

#[derive(Clone)]
pub(super) struct ProjectStatusApplication {
    database: DbHandle,
}

struct ProjectStatusTransition {
    project: novel_project::Model,
    from: ProjectStatus,
    to: ProjectStatus,
    changed_at: NaiveDateTime,
}

struct ProjectStatusRollback {
    outcome: ProjectStatusRollbackOutcome,
    from: ProjectStatus,
    to: ProjectStatus,
    changed_at: NaiveDateTime,
}

impl ProjectStatusApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    async fn project_with_status(
        &self,
        project_id: i64,
        operation: &'static str,
    ) -> Result<(novel_project::Model, ProjectStatus), AppError> {
        let project = a4n_db::project::project_by_id_record(self.database.connection(), project_id)
            .await
            .map_err(project_mutation_error_to_app_error)?
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
        if project.project_kind != PROJECT_KIND_TEXT {
            return Err(AppError::InvalidArgument(
                "非文本项目不能使用文本写作状态机".to_string(),
            ));
        }
        let status = parse_project_status(&project.status).map_err(|_| AppError::InvalidState {
            current: project.status.clone(),
            operation: operation.to_string(),
        })?;
        Ok((project, status))
    }

    async fn artifacts(&self, project_id: i64) -> Result<ProjectStatusArtifacts, AppError> {
        a4n_db::project::project_status_artifacts(self.database.connection(), project_id)
            .await
            .map_err(project_status_read_error_to_app_error)
    }

    async fn machine(&self, project_id: i64) -> Result<ProjectStatusMachineSummary, AppError> {
        let (project_result, artifacts_result, history_result) = futures::join!(
            self.project_with_status(project_id, "读取项目状态机"),
            self.artifacts(project_id),
            async {
                a4n_db::project::project_status_history(self.database.connection(), project_id)
                    .await
                    .map_err(project_status_read_error_to_app_error)
            },
        );
        let (_, current_status) = project_result?;
        let artifacts = artifacts_result?;
        let forward_targets = ProjectStatus::ALL
            .into_iter()
            .filter(|target| current_status < *target && artifacts.supports(*target))
            .collect();
        let rollback_targets = ProjectStatus::ALL
            .into_iter()
            .filter(|target| current_status > *target)
            .collect();
        let history = history_result?;

        Ok(ProjectStatusMachineSummary {
            current_status,
            forward_targets,
            rollback_targets,
            history,
        })
    }

    async fn preview_rollback(
        &self,
        project_id: i64,
        target: ProjectStatus,
    ) -> Result<ProjectRollbackPreview, AppError> {
        let (_, current_status) = self
            .project_with_status(project_id, "预览项目状态回退")
            .await?;
        if current_status <= target {
            return Err(AppError::InvalidState {
                current: current_status.as_str().to_string(),
                operation: format!("回退到状态 {}", target.as_str()),
            });
        }
        let actions = rollback_actions(current_status, target);
        let impacts = a4n_db::project::project_rollback_impacts(
            self.database.connection(),
            project_id,
            &actions,
        )
        .await
        .map_err(project_rollback_impact_read_error_to_app_error)?;

        Ok(ProjectRollbackPreview {
            path: ProjectStatus::ALL
                .into_iter()
                .rev()
                .filter(|status| *status <= current_status && *status >= target)
                .collect(),
            impacts,
        })
    }

    async fn forward(
        &self,
        project_id: i64,
        target: ProjectStatus,
    ) -> Result<ProjectStatusTransition, AppError> {
        let (project_result, artifacts_result) = futures::join!(
            self.project_with_status(project_id, "推进项目状态"),
            self.artifacts(project_id),
        );
        let (project, from) = project_result?;
        let artifacts = artifacts_result?;
        self.forward_loaded(project, from, target, artifacts).await
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
        let project = a4n_db::project::update_project_status_record(
            self.database.connection(),
            project,
            target,
            changed_at,
        )
        .await
        .map_err(project_status_write_error_to_app_error)?;

        Ok(ProjectStatusTransition {
            project,
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

    async fn rollback(
        &self,
        project_id: i64,
        target: ProjectStatus,
    ) -> Result<ProjectStatusRollback, AppError> {
        let (_, from) = self.project_with_status(project_id, "回退项目状态").await?;
        if from <= target {
            return Err(AppError::InvalidState {
                current: from.as_str().to_string(),
                operation: format!("回退到状态 {}", target.as_str()),
            });
        }
        let changed_at = Utc::now().naive_utc();
        let outcome = a4n_db::project_reset::rollback_project_status_record(
            self.database.connection(),
            project_id,
            from,
            target,
            changed_at,
        )
        .await
        .map_err(project_reset_error_to_app_error)?;

        Ok(ProjectStatusRollback {
            outcome,
            from,
            to: target,
            changed_at,
        })
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

    pub async fn project_status_machine_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<ProjectStatusMachineSummary, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Viewer)
            .await?;
        self.project_status.machine(project_id).await
    }

    pub async fn preview_project_status_rollback_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        target: ProjectStatus,
    ) -> Result<ProjectRollbackPreview, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Viewer)
            .await?;
        self.project_status
            .preview_rollback(project_id, target)
            .await
    }

    pub async fn forward_project_status_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        target: ProjectStatus,
    ) -> Result<novel_project::Model, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        self.forward_project_status_record(project_id, target).await
    }

    pub async fn rollback_project_status_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        target: ProjectStatus,
    ) -> Result<novel_project::Model, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Owner)
            .await?;
        let rollback = self.project_status.rollback(project_id, target).await?;
        self.publish_internal_event(InternalEvent::ProjectStatusChanged {
            project_id,
            from: rollback.from,
            to: rollback.to,
            timestamp: rollback.changed_at,
        })?;
        if rollback.outcome.needs_rag_clear {
            self.clear_project_rag(project_id)
                .await
                .map_err(|error| AppError::VectorStoreError(error.to_string()))?;
        }
        Ok(rollback.outcome.project)
    }

    pub(crate) async fn forward_project_status_record(
        &self,
        project_id: i64,
        target: ProjectStatus,
    ) -> Result<novel_project::Model, AppError> {
        let transition = self.project_status.forward(project_id, target).await?;
        self.publish_project_status_transition(project_id, &transition)?;
        Ok(transition.project)
    }

    pub async fn advance_project_status_if_allowed(
        &self,
        project_id: i64,
        target: ProjectStatus,
    ) -> Result<(), AppError> {
        if let Some(transition) = self.project_status.advance(project_id, target).await? {
            self.publish_project_status_transition(project_id, &transition)?;
        }
        Ok(())
    }

    pub async fn sync_project_status_from_artifacts(
        &self,
        project_id: i64,
    ) -> Result<(), AppError> {
        if let Some(transition) = self.project_status.sync_from_artifacts(project_id).await? {
            self.publish_project_status_transition(project_id, &transition)?;
        }
        Ok(())
    }
}
