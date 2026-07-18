use super::*;
use dashmap::DashMap;
use std::sync::Arc;

/// 任务管理器
///
/// 全局注册和取消任务，管理活跃任务列表。
/// 线程安全，支持多任务并发注册。
pub struct TaskManager {
    tasks: Arc<DashMap<String, Arc<TaskHandle>>>,
}

impl TaskManager {
    /// 创建任务管理器
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(DashMap::new()),
        }
    }

    /// 注册新任务
    pub fn register(&self, handle: TaskHandle) -> Arc<TaskHandle> {
        let handle = Arc::new(handle);
        {
            let progress = handle.progress.read().clone();
            let mut events = handle.progress_events.write();
            if events.is_empty() {
                let timestamp = progress
                    .updated_at
                    .clone()
                    .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
                events.push_back(TaskProgressEvent {
                    id: format!("{}-{}", handle.task_id, timestamp),
                    task_id: handle.task_id.clone(),
                    current: progress.current,
                    total: progress.total,
                    message: progress.message.clone(),
                    percent: progress.percent() * 100.0,
                    meta: progress.meta.clone(),
                    timestamp,
                });
            }
        }
        self.tasks.insert(handle.task_id.clone(), handle.clone());
        handle
    }

    /// 取消指定任务
    ///
    /// 返回 `true` 表示任务存在且已发送取消信号。
    pub fn cancel(&self, task_id: &str) -> bool {
        if let Some(handle) = self.tasks.get(task_id) {
            handle.cancel();
            true
        } else {
            false
        }
    }

    /// 列出所有活跃任务快照
    pub fn list_active_snapshots(&self) -> Vec<TaskSnapshot> {
        self.tasks
            .iter()
            .filter(|entry| !entry.value().is_cancelled())
            .map(|entry| Self::snapshot_from_handle(entry.value()))
            .collect()
    }

    /// 移除已完成的任务
    pub fn remove(&self, task_id: &str) -> Option<Arc<TaskHandle>> {
        self.tasks.remove(task_id).map(|(_, handle)| handle)
    }

    /// 活跃任务总数
    pub fn active_count(&self) -> usize {
        self.tasks
            .iter()
            .filter(|entry| !entry.value().is_cancelled())
            .count()
    }

    fn snapshot_from_handle(handle: &TaskHandle) -> TaskSnapshot {
        TaskSnapshot {
            task_id: handle.task_id.clone(),
            task_type: handle.task_type.clone(),
            project_id: handle.project_id,
            root_project_id: handle.root_project_id,
            source_snapshot_id: handle.source_snapshot_id,
            task_scope: handle.task_scope.clone(),
            progress: handle.progress.read().clone(),
            progress_events: handle.progress_events.read().iter().cloned().collect(),
            cancelled: handle.is_cancelled(),
        }
    }
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for TaskManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskManager")
            .field("active_count", &self.active_count())
            .finish()
    }
}
