//! workflow read port。

use super::*;

#[async_trait::async_trait]
impl a4n_api_contract::workflow::WorkflowReadApplicationPort for OrchestratorContext {
    async fn list_for_user(
        &self,
        user_id: i64,
        input: a4n_api_contract::workflow::ListWorkflowRunsInput,
    ) -> Result<Vec<a4n_api_contract::workflow::WorkflowRunSummary>, AppError> {
        crate::workflow::validate_optional_project_id(input.project_id)
            .map_err(AppError::InvalidArgument)?;
        if !(1..=200).contains(&input.limit) {
            return Err(AppError::InvalidArgument(
                "limit 必须在 1 到 200 之间".to_string(),
            ));
        }
        let project_scoped = input.project_id.is_some();
        if let Some(project_id) = input.project_id {
            self.verify_project_permission(
                project_id,
                user_id,
                crate::collaboration::ProjectAccessRole::Editor,
            )
            .await?;
        }
        let runs = self
            .workflow_runs(crate::workflow::ListWorkflowRunsInput {
                project_id: input.project_id,
                chapter_id: input.chapter_id,
                chapter_ids: None,
                workflow_kind: input.workflow_kind,
                status: input.status,
                resumable_only: Some(input.resumable_only),
                limit: Some(input.limit),
            })
            .await?;
        if project_scoped {
            return Ok(runs.into_iter().map(workflow_run_summary_to_api).collect());
        }

        let project_ids = runs
            .iter()
            .filter_map(|run| run.project_id.or(run.root_project_id))
            .collect::<Vec<_>>();
        let visible_project_ids = if project_ids.is_empty() {
            HashSet::new()
        } else {
            self.project_ids_with_permission(
                &project_ids,
                user_id,
                crate::collaboration::ProjectAccessRole::Editor,
            )
            .await?
            .into_iter()
            .collect()
        };
        let unscoped_visible = runs
            .iter()
            .any(|run| run.project_id.is_none() && run.root_project_id.is_none())
            && self.is_active_admin_user(user_id).await?;
        Ok(runs
            .into_iter()
            .filter(|run| match run.project_id.or(run.root_project_id) {
                Some(project_id) => visible_project_ids.contains(&project_id),
                None => unscoped_visible,
            })
            .map(workflow_run_summary_to_api)
            .collect())
    }

    async fn detail_for_user(
        &self,
        user_id: i64,
        input: a4n_api_contract::workflow::GetWorkflowRunInput,
    ) -> Result<a4n_api_contract::workflow::WorkflowRunDetailSummary, AppError> {
        crate::workflow::require_positive_workflow_run_id(input.workflow_run_id)
            .map_err(AppError::InvalidArgument)?;
        let detail = self
            .workflow_run_detail(crate::workflow::GetWorkflowRunInput {
                workflow_run_id: input.workflow_run_id,
                include_generation_runs: input.include_generation_runs,
            })
            .await?;
        match detail.run.project_id.or(detail.run.root_project_id) {
            Some(project_id) => {
                self.verify_project_permission(
                    project_id,
                    user_id,
                    crate::collaboration::ProjectAccessRole::Editor,
                )
                .await?;
            }
            None if !self.is_active_admin_user(user_id).await? => {
                return Err(AppError::Forbidden(
                    "无权查看未绑定项目的 workflow".to_string(),
                ));
            }
            None => {}
        }
        Ok(workflow_run_detail_to_api(detail))
    }
}
