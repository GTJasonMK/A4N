#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistTaskQueueRecordInput {
    pub task_id: String,
    pub task_type: String,
    pub priority: String,
    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
    pub task_scope: Option<String>,
    pub workflow_run_id: Option<i64>,
}
