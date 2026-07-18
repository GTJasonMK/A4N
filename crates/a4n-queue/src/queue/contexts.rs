use a4n_core::error::AppError;

mod id;
mod scope;

use id::required_positive_id;
use scope::validate_task_scope;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskFamilyContext {
    pub project_id: i64,
    pub root_project_id: i64,
    pub source_snapshot_id: Option<i64>,
    pub task_scope: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskWorkflowContext {
    pub workflow_run_id: i64,
}

impl TaskWorkflowContext {
    pub fn new(workflow_run_id: i64) -> Result<Self, AppError> {
        let workflow_run_id = required_positive_id(workflow_run_id, "workflow_run_id")?;
        Ok(Self { workflow_run_id })
    }
}

impl TaskFamilyContext {
    pub fn new(
        project_id: i64,
        root_project_id: i64,
        source_snapshot_id: Option<i64>,
        task_scope: impl Into<String>,
    ) -> Result<Self, AppError> {
        let project_id = required_positive_id(project_id, "project_id")?;
        let root_project_id = required_positive_id(root_project_id, "root_project_id")?;
        let source_snapshot_id = source_snapshot_id
            .map(|id| required_positive_id(id, "source_snapshot_id"))
            .transpose()?;
        let task_scope = validate_task_scope(task_scope.into())?;
        Ok(Self {
            project_id,
            root_project_id,
            source_snapshot_id,
            task_scope,
        })
    }
}

#[cfg(test)]
mod tests;
