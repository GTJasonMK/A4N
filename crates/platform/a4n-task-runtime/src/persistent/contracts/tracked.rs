use a4n_queue::{
    TaskFamilyContext, TaskWorkflowContext,
    priority::{TaskKind, TaskPriority},
};
use tokio_util::sync::CancellationToken;

pub(in crate::persistent) struct RunTrackedTaskInput {
    pub(in crate::persistent) task_type: TaskKind,
    pub(in crate::persistent) priority: TaskPriority,
    pub(in crate::persistent) project_id: Option<i64>,
    pub(in crate::persistent) context: Option<TaskFamilyContext>,
    pub(in crate::persistent) workflow: Option<TaskWorkflowContext>,
    pub(in crate::persistent) cancel_token: CancellationToken,
}
