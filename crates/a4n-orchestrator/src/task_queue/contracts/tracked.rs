use a4n_queue::{
    priority::{TaskPriority, TaskType},
    queue::{TaskFamilyContext, TaskWorkflowContext},
};
use tokio_util::sync::CancellationToken;

pub(in crate::task_queue) struct RunTrackedTaskInput {
    pub(in crate::task_queue) task_type: TaskType,
    pub(in crate::task_queue) priority: TaskPriority,
    pub(in crate::task_queue) project_id: Option<i64>,
    pub(in crate::task_queue) context: Option<TaskFamilyContext>,
    pub(in crate::task_queue) workflow: Option<TaskWorkflowContext>,
    pub(in crate::task_queue) cancel_token: CancellationToken,
}
