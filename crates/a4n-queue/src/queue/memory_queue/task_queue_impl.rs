use a4n_core::error::AppError;
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use super::{MemoryTaskQueue, waiting};
use crate::queue::priority::{
    QueueSlotStats, QueueStats, TaskInfo, TaskKind, TaskPermit, TaskPriority, TaskQueue, TaskStatus,
};

#[async_trait]
impl TaskQueue for MemoryTaskQueue {
    async fn submit(
        &self,
        task_type: TaskKind,
        priority: TaskPriority,
        project_id: Option<i64>,
    ) -> Result<String, AppError> {
        let task_id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().naive_utc();

        self.submit_persisted(TaskInfo {
            id: task_id.clone(),
            task_type,
            priority,
            status: TaskStatus::Pending,
            project_id,
            root_project_id: None,
            source_snapshot_id: None,
            task_scope: None,
            progress: None,
            created_at: now,
            started_at: None,
            completed_at: None,
        })?;

        Ok(task_id)
    }

    async fn acquire(&self, task_type: TaskKind) -> Result<TaskPermit, AppError> {
        self.acquire_with_cancel(task_type, CancellationToken::new())
            .await
            .map(|opt| opt.unwrap())
    }

    async fn acquire_with_cancel(
        &self,
        task_type: TaskKind,
        cancel_token: CancellationToken,
    ) -> Result<Option<TaskPermit>, AppError> {
        self.acquire_next_queued_task(task_type, cancel_token).await
    }

    async fn status(&self, task_id: &str) -> Result<TaskInfo, AppError> {
        self.tasks
            .get(task_id)
            .map(|t| t.clone())
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("任务不存在：{task_id}")))
    }

    async fn cancel(&self, task_id: &str) -> Result<(), AppError> {
        self.cancel_task_now(task_id)?;
        Ok(())
    }

    fn queue_stats(&self) -> QueueStats {
        let counts = waiting::counts(&self.waiting, &self.tasks);

        let llm_running = self.max_llm_concurrent - self.llm_semaphore.available_permits();
        let image_running = self.max_image_concurrent - self.image_semaphore.available_permits();
        let video_running = self.max_video_concurrent - self.video_semaphore.available_permits();
        let embedding_running =
            self.max_embedding_concurrent - self.embedding_semaphore.available_permits();
        let background_running =
            self.max_background_concurrent - self.background_semaphore.available_permits();

        QueueStats {
            llm_queue: QueueSlotStats {
                max_concurrent: self.max_llm_concurrent,
                running: llm_running,
                waiting: counts.llm_waiting,
            },
            image_queue: QueueSlotStats {
                max_concurrent: self.max_image_concurrent,
                running: image_running,
                waiting: counts.image_waiting,
            },
            video_queue: QueueSlotStats {
                max_concurrent: self.max_video_concurrent,
                running: video_running,
                waiting: counts.video_waiting,
            },
            embedding_queue: QueueSlotStats {
                max_concurrent: self.max_embedding_concurrent,
                running: embedding_running,
                waiting: counts.embedding_waiting,
            },
            background_queue: QueueSlotStats {
                max_concurrent: self.max_background_concurrent,
                running: background_running,
                waiting: counts.background_waiting,
            },
            high_priority_waiting: counts.high_priority_waiting,
            medium_priority_waiting: counts.medium_priority_waiting,
            low_priority_waiting: counts.low_priority_waiting,
        }
    }

    async fn history(
        &self,
        project_id: Option<i64>,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<TaskInfo>, AppError> {
        let mut results: Vec<TaskInfo> = self
            .tasks
            .iter()
            .filter(|task| project_id.is_none_or(|pid| task.project_id == Some(pid)))
            .map(|task| task.value().clone())
            .collect();

        results.sort_by_key(|task| std::cmp::Reverse(task.created_at));
        let offset = offset as usize;
        let limit = limit as usize;
        Ok(results.into_iter().skip(offset).take(limit).collect())
    }
}
