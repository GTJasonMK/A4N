use super::{QueueStats, TaskInfo, TaskKind, TaskPermit, TaskPriority};
use a4n_core::error::AppError;
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

/// TaskQueue trait
///
/// 统一的任务调度接口，支持单任务提交、取消和状态查询。
#[async_trait]
pub trait TaskQueue: Send + Sync {
    /// 提交单个任务，返回任务 ID
    async fn submit(
        &self,
        task_type: TaskKind,
        priority: TaskPriority,
        project_id: Option<i64>,
    ) -> Result<String, AppError>;

    /// 获取执行许可（阻塞直到有可用槽位，高优先级优先）
    async fn acquire(&self, task_type: TaskKind) -> Result<TaskPermit, AppError>;

    /// 获取执行许可（支持取消令牌）
    async fn acquire_with_cancel(
        &self,
        task_type: TaskKind,
        cancel_token: CancellationToken,
    ) -> Result<Option<TaskPermit>, AppError>;

    /// 查询任务状态
    async fn status(&self, task_id: &str) -> Result<TaskInfo, AppError>;

    /// 取消任务（级联取消依赖它的后继任务）
    async fn cancel(&self, task_id: &str) -> Result<(), AppError>;

    /// 获取队列状态概览
    fn queue_stats(&self) -> QueueStats;

    /// 获取任务历史（按项目过滤）
    async fn history(
        &self,
        project_id: Option<i64>,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<TaskInfo>, AppError>;
}
