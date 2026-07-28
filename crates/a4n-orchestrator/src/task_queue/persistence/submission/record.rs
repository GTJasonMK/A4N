use a4n_db::queue::{PersistTaskFamilyContext, PersistTaskQueueRecordInput};
use a4n_queue::{
    priority::{TaskKind, TaskPriority},
    queue::{TaskFamilyContext, TaskWorkflowContext},
};

pub(super) fn task_queue_record_input(
    task_id: &str,
    task_type: &TaskKind,
    priority: &TaskPriority,
    project_id: Option<i64>,
    context: Option<&TaskFamilyContext>,
    workflow: Option<&TaskWorkflowContext>,
) -> PersistTaskQueueRecordInput {
    PersistTaskQueueRecordInput {
        task_id: task_id.to_string(),
        task_type: task_type.as_str().to_string(),
        priority: priority.as_str().to_string(),
        project_id,
        family: context.map(|context| PersistTaskFamilyContext {
            project_id: context.project_id,
            root_project_id: context.root_project_id,
            source_snapshot_id: context.source_snapshot_id,
            task_scope: context.task_scope.clone(),
        }),
        workflow_run_id: workflow.map(|context| context.workflow_run_id),
    }
}

#[cfg(test)]
mod tests {
    use a4n_queue::{
        priority::{TaskKind, TaskPriority},
        queue::{TaskFamilyContext, TaskWorkflowContext},
    };

    use super::task_queue_record_input;

    #[test]
    fn projects_queue_contexts_into_persistence_record() {
        let family =
            TaskFamilyContext::new(21, 7, Some(3), "source_sync").expect("valid family context");
        let workflow = TaskWorkflowContext::new(99).expect("valid workflow context");

        let record = task_queue_record_input(
            "task-1",
            &TaskKind::SourceSync,
            &TaskPriority::Medium,
            Some(5),
            Some(&family),
            Some(&workflow),
        );

        assert_eq!(record.task_id, "task-1");
        assert_eq!(record.task_type, TaskKind::SourceSync.as_str());
        assert_eq!(record.priority, TaskPriority::Medium.as_str());
        assert_eq!(record.project_id, Some(5));
        assert_eq!(record.workflow_run_id, Some(99));
        let family = record.family.expect("family persistence context");
        assert_eq!(family.project_id, 21);
        assert_eq!(family.root_project_id, 7);
        assert_eq!(family.source_snapshot_id, Some(3));
        assert_eq!(family.task_scope, "source_sync");
    }
}
