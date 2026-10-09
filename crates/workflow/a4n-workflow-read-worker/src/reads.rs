use super::*;
use a4n_api_contract::task::TaskProgressSummary;
use a4n_api_contract::workflow::{ActiveTaskLookupRecord, WorkflowProgressEvent};

pub(super) fn active_task_lookup_record(
    persisted: a4n_db::workflow::PersistedTaskInfo,
) -> ActiveTaskLookupRecord {
    let task = persisted.task;
    let progress = task.progress.as_ref().map(|progress| TaskProgressSummary {
        current: progress.current,
        total: progress.total,
        message: progress.message.clone(),
        meta: progress.meta.clone(),
    });
    let progress_events = task
        .progress
        .as_ref()
        .map(|progress| WorkflowProgressEvent {
            id: format!("{}-snapshot", task.id),
            current: progress.current,
            total: progress.total,
            message: progress.message.clone(),
            meta: progress.meta.clone(),
            timestamp: progress
                .updated_at
                .clone()
                .unwrap_or_else(|| task.started_at.unwrap_or(task.created_at).to_string()),
        })
        .into_iter()
        .collect();
    ActiveTaskLookupRecord {
        task_id: task.id.clone(),
        project_id: task.project_id,
        root_project_id: task.root_project_id,
        status: task.status.as_str().to_string(),
        workflow: a4n_api_contract::workflow::WorkflowStateSummary {
            task_id: task.id,
            root_project_id: task.root_project_id,
            source_snapshot_id: task.source_snapshot_id,
            task_scope: task.task_scope,
            progress,
            progress_events,
        },
    }
}

pub(super) fn task_info_summary(info: &a4n_queue::priority::TaskInfo) -> TaskInfoSummary {
    TaskInfoSummary {
        id: info.id.clone(),
        task_type: info.task_type.as_str().to_string(),
        priority: info.priority.as_str().to_string(),
        status: info.status.as_str().to_string(),
        root_project_id: info.root_project_id,
        source_snapshot_id: info.source_snapshot_id,
        task_scope: info.task_scope.clone(),
        progress: info.progress.as_ref().map(|progress| {
            a4n_api_contract::task::TaskProgressSummary {
                current: progress.current,
                total: progress.total,
                message: progress.message.clone(),
                meta: progress.meta.clone(),
            }
        }),
        created_at: info.created_at.and_utc().to_rfc3339(),
        started_at: info.started_at.map(|value| value.and_utc().to_rfc3339()),
        completed_at: info.completed_at.map(|value| value.and_utc().to_rfc3339()),
    }
}

impl WorkerState {
    pub(super) async fn list(
        &self,
        request: WorkerEnvelope<ListWorkflowRunsInput>,
    ) -> Result<Vec<a4n_api_contract::workflow::WorkflowRunSummary>, WorkerFailure> {
        validate_request(&request, WORKFLOW_READ_LIST_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        if !(1..=200).contains(&input.limit) {
            return Err(WorkerFailure::invalid_request("limit 必须在 1 到 200 之间"));
        }
        a4n_domain::workflow::validate_optional_project_id(input.project_id)
            .map_err(WorkerFailure::invalid_request)?;
        if let Some(project_id) = input.project_id {
            verify_project_editor(&self.database, project_id, user_id).await?;
        }
        let runs = a4n_db::workflow::list_workflow_runs(
            self.database.executor(),
            a4n_db::workflow::ListWorkflowRunsInput {
                project_id: input.project_id,
                chapter_id: input.chapter_id,
                chapter_ids: None,
                workflow_kind: input.workflow_kind.clone(),
                status: input.status.clone(),
                resumable_only: Some(input.resumable_only),
                limit: Some(input.limit),
            },
        )
        .await
        .map_err(workflow_error)?;
        let visible = if input.project_id.is_some() {
            runs
        } else {
            let project_ids = runs
                .iter()
                .filter_map(|run| run.project_id.or(run.root_project_id))
                .collect::<Vec<_>>();
            let visible_project_ids = a4n_db::collaboration::project_ids_with_permission(
                self.database.executor(),
                &project_ids,
                user_id,
                ProjectAccessRole::Editor,
            )
            .await
            .map_err(collaboration_error)?
            .into_iter()
            .collect::<HashSet<_>>();
            let unscoped_visible = is_active_admin(&self.database, user_id).await?;
            runs.into_iter()
                .filter(|run| match run.project_id.or(run.root_project_id) {
                    Some(project_id) => visible_project_ids.contains(&project_id),
                    None => unscoped_visible,
                })
                .collect()
        };
        Ok(visible
            .into_iter()
            .map(workflow_run_summary_to_api)
            .collect())
    }

    pub(super) async fn detail(
        &self,
        request: WorkerEnvelope<GetWorkflowRunInput>,
    ) -> Result<a4n_api_contract::workflow::WorkflowRunDetailSummary, WorkerFailure> {
        validate_request(&request, WORKFLOW_READ_DETAIL_OPERATION)?;
        if request.payload.workflow_run_id <= 0 {
            return Err(WorkerFailure::invalid_request("workflow_run_id 必须大于 0"));
        }
        let user_id = user_id(&request)?;
        let detail = a4n_db::workflow::workflow_run_detail(
            self.database.executor(),
            a4n_db::workflow::GetWorkflowRunInput {
                workflow_run_id: request.payload.workflow_run_id,
                include_generation_runs: request.payload.include_generation_runs,
            },
        )
        .await
        .map_err(workflow_error)?;
        match detail.run.project_id.or(detail.run.root_project_id) {
            Some(project_id) => verify_project_editor(&self.database, project_id, user_id).await?,
            None if !is_active_admin(&self.database, user_id).await? => {
                return Err(WorkerFailure::unauthorized("无权查看未绑定项目的 workflow"));
            }
            None => {}
        }
        Ok(workflow_run_detail_to_api(detail))
    }

    pub(super) async fn task_history(
        &self,
        request: WorkerEnvelope<GetTaskHistoryInput>,
    ) -> Result<Vec<TaskInfoSummary>, WorkerFailure> {
        validate_request(&request, TASK_READ_HISTORY_OPERATION)?;
        require_service_principal(&request.principal)?;
        let input = request.payload;
        if input.project_id.is_some_and(|id| id <= 0) {
            return Err(WorkerFailure::invalid_request("project_id 必须大于 0"));
        }
        if input.limit == 0 {
            return Err(WorkerFailure::invalid_request("limit 必须大于 0"));
        }
        let tasks = a4n_db::workflow::persisted_task_history(
            self.database.executor(),
            input.project_id,
            input.offset,
            input.limit,
        )
        .await
        .map_err(workflow_error)?;
        Ok(tasks
            .into_iter()
            .map(|task| task_info_summary(&task.task))
            .collect())
    }

    pub(super) async fn task_by_id(
        &self,
        request: WorkerEnvelope<TaskIdInput>,
    ) -> Result<Option<TaskLookupRecord>, WorkerFailure> {
        validate_request(&request, TASK_LOOKUP_BY_ID_OPERATION)?;
        require_service_principal(&request.principal)?;
        let persisted = a4n_db::workflow::persisted_task_by_id(
            self.database.executor(),
            &request.payload.task_id,
        )
        .await
        .map_err(workflow_error)?;
        Ok(persisted.map(|persisted| TaskLookupRecord {
            task_id: persisted.task.id,
            project_id: persisted.task.project_id,
            root_project_id: persisted.task.root_project_id,
            workflow_run_id: persisted.workflow_run_id,
            status: persisted.task.status.as_str().to_string(),
        }))
    }

    pub(super) async fn active_tasks(
        &self,
        request: WorkerEnvelope<EmptyInput>,
    ) -> Result<Vec<ActiveTaskLookupRecord>, WorkerFailure> {
        validate_request(&request, TASK_LOOKUP_ACTIVE_OPERATION)?;
        require_service_principal(&request.principal)?;
        let persisted =
            a4n_db::workflow::persisted_task_history(self.database.executor(), None, 0, 100)
                .await
                .map_err(workflow_error)?;
        Ok(persisted
            .into_iter()
            .map(active_task_lookup_record)
            .collect())
    }

    pub(super) async fn workflow_access_by_active_task(
        &self,
        request: WorkerEnvelope<TaskIdInput>,
    ) -> Result<Option<WorkflowTaskAccessRecord>, WorkerFailure> {
        validate_request(&request, WORKFLOW_ACCESS_ACTIVE_TASK_OPERATION)?;
        require_service_principal(&request.principal)?;
        Ok(
            a4n_db::workflow::workflow_run_access_record_by_active_task_id(
                self.database.executor(),
                &request.payload.task_id,
            )
            .await
            .map_err(workflow_error)?
            .map(|record| WorkflowTaskAccessRecord {
                workflow_run_id: record.id,
                project_id: record.project_id,
                root_project_id: record.root_project_id,
            }),
        )
    }

    pub(super) async fn resume_record(
        &self,
        request: WorkerEnvelope<WorkflowRunIdInput>,
    ) -> Result<WorkflowResumeRecord, WorkerFailure> {
        validate_request(&request, WORKFLOW_READ_RESUME_RECORD_OPERATION)?;
        require_service_principal(&request.principal)?;
        let run = a4n_db::workflow::workflow_run_resume_record_by_id(
            self.database.executor(),
            request.payload.workflow_run_id,
        )
        .await
        .map_err(workflow_error)?;
        Ok(WorkflowResumeRecord {
            id: run.id,
            workflow_kind: run.workflow_kind,
            project_id: run.project_id,
            root_project_id: run.root_project_id,
            chapter_id: run.chapter_id,
            scope_table: run.scope_table,
            scope_id: run.scope_id,
            input_fingerprint_json: run.input_fingerprint_json,
            status: run.status,
            can_resume: run.can_resume,
            resume_block_reason: run.resume_block_reason,
        })
    }

    pub(super) async fn summary(
        &self,
        request: WorkerEnvelope<WorkflowRunIdInput>,
    ) -> Result<WorkflowRunSummary, WorkerFailure> {
        validate_request(&request, WORKFLOW_READ_SUMMARY_OPERATION)?;
        require_service_principal(&request.principal)?;
        a4n_db::workflow::workflow_run_summary_by_id(
            self.database.executor(),
            request.payload.workflow_run_id,
        )
        .await
        .map(workflow_run_summary_to_api)
        .map_err(workflow_error)
    }

    pub(super) async fn progress_events(
        &self,
        request: WorkerEnvelope<WorkflowProgressEventsInput>,
    ) -> Result<Vec<WorkflowProgressEvent>, WorkerFailure> {
        validate_request(&request, WORKFLOW_READ_PROGRESS_EVENTS_OPERATION)?;
        require_service_principal(&request.principal)?;
        let Some(workflow_run_id) = request.payload.workflow_run_id else {
            return Err(WorkerFailure::invalid_request("workflow_run_id 必须存在"));
        };
        if request.payload.task_id.is_some() {
            return Err(WorkerFailure::invalid_request(
                "progress events 不接受 task_id",
            ));
        }
        a4n_db::workflow::workflow_progress_events(
            self.database.executor(),
            workflow_run_id,
            request.payload.limit,
        )
        .await
        .map(|events| {
            events
                .into_iter()
                .map(|event| WorkflowProgressEvent {
                    id: event.id,
                    current: event.current,
                    total: event.total,
                    message: event.message,
                    meta: event.meta,
                    timestamp: event.timestamp,
                })
                .collect()
        })
        .map_err(workflow_error)
    }

    pub(super) fn authorized(&self, headers: &HeaderMap) -> bool {
        a4n_worker_runtime::worker_token_authorized(
            headers,
            WORKER_AUTH_HEADER,
            self.auth_token.as_bytes(),
        )
    }
}
