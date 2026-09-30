use super::*;

impl WorkerState {
    pub(super) async fn status_machine(
        &self,
        request: WorkerEnvelope<ProjectStatusProjectInput>,
    ) -> Result<ProjectStatusMachineSummary, WorkerFailure> {
        validate_request(&request, PROJECT_STATUS_MACHINE_OPERATION)?;
        let project_id = scoped_project_id(&request, request.payload.project_id)?;
        let user_id = user_id(&request)?;
        verify_viewer(&self.database, project_id, user_id).await?;
        let (_, current_status) = self
            .project_with_status(project_id, "读取项目状态机")
            .await?;
        let (artifacts, history) = tokio::join!(
            async {
                a4n_db::project::project_status_artifacts(self.database.connection(), project_id)
                    .await
                    .map_err(status_read_error)
            },
            async {
                a4n_db::project::project_status_history(self.database.connection(), project_id)
                    .await
                    .map_err(status_read_error)
            },
        );
        let artifacts = artifacts?;
        Ok(ProjectStatusMachineSummary {
            current_status,
            forward_targets: ProjectStatus::ALL
                .into_iter()
                .filter(|target| current_status < *target && artifacts.supports(*target))
                .collect(),
            rollback_targets: ProjectStatus::ALL
                .into_iter()
                .filter(|target| current_status > *target)
                .collect(),
            history: history?,
        })
    }

    pub(super) async fn preview_rollback(
        &self,
        request: WorkerEnvelope<ProjectStatusRollbackInput>,
    ) -> Result<ProjectRollbackPreview, WorkerFailure> {
        validate_request(&request, PROJECT_STATUS_PREVIEW_ROLLBACK_OPERATION)?;
        scoped_project_id(&request, request.payload.project_id)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        let project_id = input.project_id;
        verify_viewer(&self.database, project_id, user_id).await?;
        let (_, current_status) = self
            .project_with_status(project_id, "预览项目状态回退")
            .await?;
        if current_status <= input.target {
            return Err(WorkerFailure::conflict(format!(
                "当前状态 {} 不允许回退到 {}",
                current_status.as_str(),
                input.target.as_str()
            )));
        }
        let actions = rollback_actions(current_status, input.target);
        let impacts = a4n_db::project::project_rollback_impacts(
            self.database.connection(),
            project_id,
            &actions,
        )
        .await
        .map_err(rollback_impact_error)?;
        Ok(ProjectRollbackPreview {
            path: ProjectStatus::ALL
                .into_iter()
                .rev()
                .filter(|status| *status <= current_status && *status >= input.target)
                .collect(),
            impacts,
        })
    }

    pub(super) async fn forward(
        &self,
        request: WorkerEnvelope<ProjectStatusRollbackInput>,
    ) -> Result<ProjectStatusMutationResult, WorkerFailure> {
        validate_request(&request, PROJECT_STATUS_FORWARD_OPERATION)?;
        let project_id = scoped_project_id(&request, request.payload.project_id)?;
        let user_id = user_id(&request)?;
        verify_permission(
            &self.database,
            project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        let (project, from) = self.project_with_status(project_id, "推进项目状态").await?;
        let artifacts =
            a4n_db::project::project_status_artifacts(self.database.connection(), project_id)
                .await
                .map_err(status_read_error)?;
        let target = request.payload.target;
        if from >= target {
            return Err(WorkerFailure::conflict(format!(
                "当前状态 {} 不允许前进到 {}",
                from.as_str(),
                target.as_str()
            )));
        }
        if !artifacts.supports(target) {
            return Err(WorkerFailure::conflict(format!(
                "当前状态 {} 缺少前进到 {} 所需产物",
                from.as_str(),
                target.as_str()
            )));
        }
        let changed_at = chrono::Utc::now().naive_utc();
        let project = a4n_db::project::update_project_status_record(
            self.database.connection(),
            project,
            target,
            changed_at,
        )
        .await
        .map_err(status_write_error)?;
        let summary = self.summary_for_user(user_id, &project).await?;
        Ok(ProjectStatusMutationResult {
            project: summary,
            from,
            to: target,
            changed_at,
            needs_rag_clear: false,
        })
    }

    pub(super) async fn rollback(
        &self,
        request: WorkerEnvelope<ProjectStatusRollbackInput>,
    ) -> Result<ProjectStatusMutationResult, WorkerFailure> {
        validate_request(&request, PROJECT_STATUS_ROLLBACK_OPERATION)?;
        let project_id = scoped_project_id(&request, request.payload.project_id)?;
        let user_id = user_id(&request)?;
        verify_permission(
            &self.database,
            project_id,
            user_id,
            ProjectAccessRole::Owner,
        )
        .await?;
        let (_, from) = self.project_with_status(project_id, "回退项目状态").await?;
        let target = request.payload.target;
        if from <= target {
            return Err(WorkerFailure::conflict(format!(
                "当前状态 {} 不允许回退到 {}",
                from.as_str(),
                target.as_str()
            )));
        }
        let changed_at = chrono::Utc::now().naive_utc();
        let outcome = a4n_db::project::reset::rollback_project_status_record(
            self.database.connection(),
            project_id,
            from,
            target,
            changed_at,
        )
        .await
        .map_err(project_reset_error)?;
        let summary = self.summary_for_user(user_id, &outcome.project).await?;
        Ok(ProjectStatusMutationResult {
            project: summary,
            from,
            to: target,
            changed_at,
            needs_rag_clear: outcome.needs_rag_clear,
        })
    }

    async fn summary_for_user(
        &self,
        user_id: i64,
        project: &a4n_db_models::models::project::novel_project::Model,
    ) -> Result<a4n_api_contract::project::ProjectSummary, WorkerFailure> {
        a4n_project_read_core::project_summary_for_user(
            self.database.connection(),
            user_id,
            project,
        )
        .await
        .map_err(summary_error)
    }

    async fn project_with_status(
        &self,
        project_id: i64,
        operation: &'static str,
    ) -> Result<
        (
            a4n_db_models::models::project::novel_project::Model,
            ProjectStatus,
        ),
        WorkerFailure,
    > {
        let project = a4n_db::project::project_by_id_record(self.database.connection(), project_id)
            .await
            .map_err(project_mutation_error)?
            .ok_or_else(|| WorkerFailure::not_found(format!("项目不存在: {project_id}")))?;
        if project.project_kind != a4n_db::project::PROJECT_KIND_TEXT {
            return Err(WorkerFailure::invalid_request(
                "非文本项目不能使用文本写作状态机",
            ));
        }
        let status = ProjectStatus::parse_storage(&project.status).map_err(|_| {
            WorkerFailure::conflict(format!(
                "当前状态 {} 不允许执行 {operation}",
                project.status
            ))
        })?;
        Ok((project, status))
    }

    pub(super) fn authorized(&self, headers: &HeaderMap) -> bool {
        headers
            .get(WORKER_AUTH_HEADER)
            .is_some_and(|value| bool::from(value.as_bytes().ct_eq(self.auth_token.as_bytes())))
    }
}
