//! Workflow 领域私有 owner。
//!
//! 持有 `DbHandle`，封装 workflow run / checkpoint / progress / task 历史的读写。
//! 编排侧只通过 `OrchestratorContext` 行为方法或 runtime 窄 facade 访问。

use std::collections::HashSet;

use a4n_api_contract::workflow::{
    ActiveTaskLookupRecord, TaskLookupRecord, WorkflowCancellationPort, WorkflowResumeRecord,
    WorkflowTaskAccessRecord, WorkflowTaskLookupApplicationPort,
};
use a4n_db::DbHandle;
use a4n_db_models::models::generation::{
    workflow_checkpoint, workflow_progress_event, workflow_run,
};
use a4n_domain::media::GeneratedVideoErrorStatusBatchInput;
use a4n_error::AppError;

use super::OrchestratorContext;
use crate::workflow::{
    CancelWorkflowRunPersistenceResult, ClaimExclusiveWorkflowRunInput, CompletedLlmCallCheckpoint,
    CompletedLlmCallCheckpointInput, GetWorkflowRunInput, ListWorkflowRunsInput, PersistedTaskInfo,
    TaskProgressEventSummary, UpsertWorkflowCheckpointInput, UpsertWorkflowRunInput,
    WorkflowProgressEventInput, WorkflowRunAccessRecord, WorkflowRunDetailSummary,
    WorkflowRunRecoveryRecord, WorkflowRunStatusRecord, WorkflowRunSummary,
    is_video_status_final_for_cancel_interrupt, workflow_read_app_error,
    workflow_run_summary_to_api, workflow_write_app_error,
};

/// Workflow 领域 application owner。
#[derive(Clone)]
pub(super) struct WorkflowApplication {
    database: DbHandle,
}

impl WorkflowApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }
}

mod application;
mod facade;
mod helpers;
mod ports;

pub(super) use helpers::*;

#[cfg(test)]
mod task_read_tests;
