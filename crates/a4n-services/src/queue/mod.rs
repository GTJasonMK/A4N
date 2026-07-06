//! 持久化任务队列
//!
//! 基于 MemoryTaskQueue 的数据库持久化包装器。
//! 任务状态变更时自动同步到数据库，启动时恢复未完成任务。

use a4n_core::error::AppError;
use a4n_core::models::{
    generation::{generation_run, workflow_run},
    task::task_queue,
};
use a4n_engine::queue::memory_queue::MemoryTaskQueue;
use a4n_engine::queue::priority::*;
use async_trait::async_trait;
use chrono::Utc;
use sea_orm::sea_query::Expr;
use sea_orm::*;
use std::{future::Future, sync::Arc};
use tokio_util::sync::CancellationToken;

mod concurrency;
mod row_mapping;
mod status_store;
mod submitted_guard;

use concurrency::QueueConcurrencyLimits;
use row_mapping::task_info_from_row;
use status_store::update_task_status_by_db;
pub use submitted_guard::SubmittedTaskGuard;

/// 持久化任务队列
///
/// 包装 MemoryTaskQueue，将任务状态同步到数据库。
pub struct PersistentTaskQueue {
    inner: Arc<MemoryTaskQueue>,
    db: Arc<DatabaseConnection>,
}

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

struct RunTrackedTaskInput {
    task_type: TaskType,
    priority: TaskPriority,
    project_id: Option<i64>,
    context: Option<TaskFamilyContext>,
    workflow: Option<TaskWorkflowContext>,
    cancel_token: CancellationToken,
}

impl TaskWorkflowContext {
    pub fn new(workflow_run_id: i64) -> Result<Self, AppError> {
        if workflow_run_id <= 0 {
            return Err(AppError::InvalidArgument(
                "workflow_run_id 必须大于 0".to_string(),
            ));
        }
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
        if project_id <= 0 {
            return Err(AppError::InvalidArgument(
                "project_id 必须大于 0".to_string(),
            ));
        }
        if root_project_id <= 0 {
            return Err(AppError::InvalidArgument(
                "root_project_id 必须大于 0".to_string(),
            ));
        }
        if source_snapshot_id.is_some_and(|id| id <= 0) {
            return Err(AppError::InvalidArgument(
                "source_snapshot_id 必须大于 0".to_string(),
            ));
        }

        let task_scope = task_scope.into();
        if !is_valid_task_scope(&task_scope) {
            return Err(AppError::InvalidArgument(format!(
                "task_scope 无效: {task_scope}"
            )));
        }

        Ok(Self {
            project_id,
            root_project_id,
            source_snapshot_id,
            task_scope,
        })
    }
}

fn is_valid_task_scope(value: &str) -> bool {
    matches!(
        value,
        "text_analysis" | "source_sync" | "video_prompt" | "media_generation" | "assembly"
    )
}

impl PersistentTaskQueue {
    /// 创建并恢复未完成任务
    pub async fn new(db: Arc<DatabaseConnection>) -> Result<Self, AppError> {
        let limits = QueueConcurrencyLimits::load(db.as_ref()).await?;
        let inner = Arc::new(MemoryTaskQueue::with_limits(
            limits.llm,
            limits.image,
            limits.video,
            limits.embedding,
            limits.background,
        ));
        let queue = Self { inner, db };
        queue.recover_pending().await?;
        Ok(queue)
    }

    /// 启动时中断数据库中 pending/running 状态的历史任务。
    ///
    /// 当前执行器是内存态，应用重启后不能继续占用旧执行器；业务层通过
    /// workflow checkpoint 判断是否允许用户手动恢复。
    async fn recover_pending(&self) -> Result<(), AppError> {
        let active_tasks = task_queue::Entity::find()
            .filter(task_queue::Column::Status.is_in(["pending", "running"]))
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;

        for task in active_tasks {
            let workflow_run_id = task.workflow_run_id;
            update_task_status_by_db(
                self.db.clone(),
                task.id.clone(),
                "interrupted",
                Some("应用重启后中断未绑定执行器的队列任务".to_string()),
            )
            .await?;
            if let Some(workflow_run_id) = workflow_run_id {
                self.mark_workflow_interrupted(workflow_run_id, &task.id)
                    .await?;
            }
        }

        Ok(())
    }

    async fn mark_workflow_interrupted(
        &self,
        workflow_run_id: i64,
        task_id: &str,
    ) -> Result<(), AppError> {
        let Some(run) = workflow_run::Entity::find_by_id(workflow_run_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
        else {
            return Ok(());
        };
        if matches!(
            run.status.as_str(),
            "completed" | "failed" | "cancelled" | "superseded"
        ) {
            return Ok(());
        }
        let now = Utc::now().naive_utc();
        let mut active: workflow_run::ActiveModel = run.into();
        active.status = Set("interrupted".to_string());
        active.active_task_id = Set(None);
        active.can_resume = Set(true);
        active.resume_block_reason = Set(None);
        active.error_message = Set(Some(format!(
            "应用重启后中断任务 {task_id}，可从最近有效检查点继续"
        )));
        active.updated_at = Set(now);
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        generation_run::Entity::update_many()
            .col_expr(
                generation_run::Column::Status,
                Expr::value("interrupted".to_string()),
            )
            .col_expr(
                generation_run::Column::ErrorMessage,
                Expr::value(Some(format!(
                    "应用重启后中断关联队列任务 {task_id}，等待 workflow 恢复"
                ))),
            )
            .col_expr(generation_run::Column::CompletedAt, Expr::value(Some(now)))
            .filter(generation_run::Column::WorkflowRunId.eq(workflow_run_id))
            .filter(generation_run::Column::Status.eq("running"))
            .exec(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        Ok(())
    }

    /// 将任务写入数据库
    async fn persist_task(
        &self,
        task_id: &str,
        task_type: &TaskType,
        priority: &TaskPriority,
        project_id: Option<i64>,
        context: Option<&TaskFamilyContext>,
        workflow: Option<&TaskWorkflowContext>,
    ) -> Result<(), AppError> {
        let now = Utc::now().naive_utc();
        let existing = task_queue::Entity::find_by_id(task_id.to_string())
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;

        if existing.is_none() {
            let persisted_project_id = context.map_or(project_id, |ctx| Some(ctx.project_id));
            task_queue::Entity::insert(task_queue::ActiveModel {
                id: Set(task_id.to_string()),
                task_type: Set(task_type.as_str().to_string()),
                priority: Set(priority.as_str().to_string()),
                status: Set("pending".into()),
                project_id: Set(persisted_project_id),
                root_project_id: Set(context.map(|ctx| ctx.root_project_id)),
                source_snapshot_id: Set(context.and_then(|ctx| ctx.source_snapshot_id)),
                task_scope: Set(context.map(|ctx| ctx.task_scope.clone())),
                workflow_run_id: Set(workflow.map(|ctx| ctx.workflow_run_id)),
                payload: Set(None),
                progress: Set(None),
                error_message: Set(None),
                dag_id: Set(None),
                created_at: Set(now),
                started_at: Set(None),
                completed_at: Set(None),
            })
            .exec(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        }

        Ok(())
    }

    async fn submit_internal(
        &self,
        task_type: TaskType,
        priority: TaskPriority,
        project_id: Option<i64>,
        context: Option<&TaskFamilyContext>,
        workflow: Option<&TaskWorkflowContext>,
    ) -> Result<String, AppError> {
        let queue_project_id = context.map_or(project_id, |ctx| Some(ctx.project_id));
        let task_id = self
            .inner
            .submit(task_type, priority, queue_project_id)
            .await?;
        self.persist_task(
            &task_id, &task_type, &priority, project_id, context, workflow,
        )
        .await?;
        Ok(task_id)
    }

    pub async fn submit_with_family_context(
        &self,
        task_type: TaskType,
        priority: TaskPriority,
        context: TaskFamilyContext,
    ) -> Result<String, AppError> {
        self.submit_internal(
            task_type,
            priority,
            Some(context.project_id),
            Some(&context),
            None,
        )
        .await
    }

    pub async fn submit_with_workflow_context(
        &self,
        task_type: TaskType,
        priority: TaskPriority,
        project_id: Option<i64>,
        workflow: TaskWorkflowContext,
    ) -> Result<String, AppError> {
        self.submit_internal(task_type, priority, project_id, None, Some(&workflow))
            .await
    }

    /// 更新任务状态到数据库
    async fn update_status(
        &self,
        task_id: &str,
        status: &str,
        error: Option<&str>,
    ) -> Result<(), AppError> {
        update_task_status_by_db(
            self.db.clone(),
            task_id.to_string(),
            status,
            error.map(str::to_string),
        )
        .await?;
        self.sync_workflow_active_task(task_id, status).await
    }

    async fn sync_workflow_active_task(&self, task_id: &str, status: &str) -> Result<(), AppError> {
        let Some(task) = task_queue::Entity::find_by_id(task_id.to_string())
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
        else {
            return Ok(());
        };
        let Some(workflow_run_id) = task.workflow_run_id else {
            return Ok(());
        };
        let Some(run) = workflow_run::Entity::find_by_id(workflow_run_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
        else {
            return Ok(());
        };
        let next_active_task_id = match status {
            "pending" | "running" => Some(task_id.to_string()),
            _ if run.active_task_id.as_deref() == Some(task_id) => None,
            _ => return Ok(()),
        };
        let now = Utc::now().naive_utc();
        let mut active: workflow_run::ActiveModel = run.into();
        active.active_task_id = Set(next_active_task_id);
        active.updated_at = Set(now);
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        Ok(())
    }

    /// 业务层显式上报任务成功完成
    ///
    /// 用法：业务代码在 `acquire` 拿到 permit 并执行业务逻辑成功后调用，
    /// 把 DB 中的任务状态从 `running` 推到 `completed`。
    /// DB 状态不会被内存队列自动更新——本方法负责该同步。
    pub async fn mark_done(&self, task_id: &str) -> Result<(), AppError> {
        self.update_status(task_id, "completed", None).await
    }

    /// 业务层显式上报任务失败
    pub async fn mark_failed(&self, task_id: &str, error: &str) -> Result<(), AppError> {
        self.update_status(task_id, "failed", Some(error)).await
    }

    /// 提交并执行一个真实业务请求。
    ///
    /// 这是业务代码接入请求队列的首选入口：先把请求登记为 pending，
    /// 再按任务类型等待并发槽位，业务结束后统一释放 permit 并同步 DB 状态。
    pub async fn run_tracked<F, Fut, T>(
        &self,
        task_type: TaskType,
        priority: TaskPriority,
        project_id: Option<i64>,
        operation: F,
    ) -> Result<T, AppError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, AppError>>,
    {
        self.run_tracked_with_cancel(
            task_type,
            priority,
            project_id,
            CancellationToken::new(),
            operation,
        )
        .await
    }

    pub async fn run_tracked_with_family_context<F, Fut, T>(
        &self,
        task_type: TaskType,
        priority: TaskPriority,
        context: TaskFamilyContext,
        operation: F,
    ) -> Result<T, AppError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, AppError>>,
    {
        self.run_tracked_with_family_context_and_cancel(
            task_type,
            priority,
            context,
            CancellationToken::new(),
            operation,
        )
        .await
    }

    pub async fn run_tracked_with_workflow_context<F, Fut, T>(
        &self,
        task_type: TaskType,
        priority: TaskPriority,
        project_id: Option<i64>,
        workflow: TaskWorkflowContext,
        operation: F,
    ) -> Result<T, AppError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, AppError>>,
    {
        self.run_tracked_with_workflow_context_and_cancel(
            task_type,
            priority,
            project_id,
            workflow,
            CancellationToken::new(),
            operation,
        )
        .await
    }

    /// 提交并执行一个支持外部取消令牌的真实业务请求。
    ///
    /// 与 `run_tracked` 一样，这里只会执行自己提交出来的队列任务，避免
    /// 请求 A 占用/完成请求 B 的历史记录。
    pub async fn run_tracked_with_cancel<F, Fut, T>(
        &self,
        task_type: TaskType,
        priority: TaskPriority,
        project_id: Option<i64>,
        cancel_token: CancellationToken,
        operation: F,
    ) -> Result<T, AppError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, AppError>>,
    {
        self.run_tracked_internal(
            RunTrackedTaskInput {
                task_type,
                priority,
                project_id,
                context: None,
                workflow: None,
                cancel_token,
            },
            operation,
        )
        .await
    }

    pub async fn run_tracked_with_family_context_and_cancel<F, Fut, T>(
        &self,
        task_type: TaskType,
        priority: TaskPriority,
        context: TaskFamilyContext,
        cancel_token: CancellationToken,
        operation: F,
    ) -> Result<T, AppError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, AppError>>,
    {
        self.run_tracked_internal(
            RunTrackedTaskInput {
                task_type,
                priority,
                project_id: Some(context.project_id),
                context: Some(context),
                workflow: None,
                cancel_token,
            },
            operation,
        )
        .await
    }

    pub async fn run_tracked_with_workflow_context_and_cancel<F, Fut, T>(
        &self,
        task_type: TaskType,
        priority: TaskPriority,
        project_id: Option<i64>,
        workflow: TaskWorkflowContext,
        cancel_token: CancellationToken,
        operation: F,
    ) -> Result<T, AppError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, AppError>>,
    {
        self.run_tracked_internal(
            RunTrackedTaskInput {
                task_type,
                priority,
                project_id,
                context: None,
                workflow: Some(workflow),
                cancel_token,
            },
            operation,
        )
        .await
    }

    async fn run_tracked_internal<F, Fut, T>(
        &self,
        input: RunTrackedTaskInput,
        operation: F,
    ) -> Result<T, AppError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, AppError>>,
    {
        let RunTrackedTaskInput {
            task_type,
            priority,
            project_id,
            cancel_token,
            context,
            workflow,
        } = input;
        let submitted_id = self
            .submit_internal(
                task_type,
                priority,
                project_id,
                context.as_ref(),
                workflow.as_ref(),
            )
            .await?;
        let mut submitted_guard =
            SubmittedTaskGuard::new(self.inner.clone(), self.db.clone(), submitted_id.clone());
        let Some(permit) = self
            .acquire_task_with_cancel(&submitted_id, cancel_token)
            .await?
        else {
            if let Err(err) = TaskQueue::cancel(self, &submitted_id).await {
                tracing::warn!(
                    task_id = %submitted_id,
                    error = %err,
                    "同步取消队列任务失败"
                );
            }
            submitted_guard.disarm();
            return Err(AppError::Cancelled);
        };
        let running_id = permit.task_id().to_string();

        match operation().await {
            Ok(value) => {
                permit.complete();
                if let Err(err) = self.mark_done(&running_id).await {
                    tracing::warn!(
                        task_id = %running_id,
                        error = %err,
                        "同步队列任务完成状态失败"
                    );
                }
                submitted_guard.disarm();
                Ok(value)
            }
            Err(err) => {
                let message = err.to_string();
                permit.fail(message.clone());
                if let Err(status_err) = self.mark_failed(&running_id, &message).await {
                    tracing::warn!(
                        task_id = %running_id,
                        error = %status_err,
                        "同步队列任务失败状态失败"
                    );
                }
                submitted_guard.disarm();
                Err(err)
            }
        }
    }

    /// 获取指定任务 ID 的执行许可。
    pub async fn acquire_task_with_cancel(
        &self,
        task_id: &str,
        cancel_token: CancellationToken,
    ) -> Result<Option<TaskPermit>, AppError> {
        match self
            .inner
            .acquire_task_with_cancel(task_id, cancel_token)
            .await?
        {
            Some(permit) => {
                let task_id = permit.task_id().to_string();
                self.update_status(&task_id, "running", None).await?;
                Ok(Some(permit))
            }
            None => Ok(None),
        }
    }

    pub fn submitted_task_guard(&self, task_id: String) -> SubmittedTaskGuard {
        SubmittedTaskGuard::new(self.inner.clone(), self.db.clone(), task_id)
    }
}

#[async_trait]
impl TaskQueue for PersistentTaskQueue {
    async fn submit(
        &self,
        task_type: TaskType,
        priority: TaskPriority,
        project_id: Option<i64>,
    ) -> Result<String, AppError> {
        self.submit_internal(task_type, priority, project_id, None, None)
            .await
    }

    async fn acquire(&self, task_type: TaskType) -> Result<TaskPermit, AppError> {
        let permit = self.inner.acquire(task_type).await?;
        self.update_status(permit.task_id(), "running", None)
            .await?;
        Ok(permit)
    }

    async fn acquire_with_cancel(
        &self,
        task_type: TaskType,
        cancel_token: CancellationToken,
    ) -> Result<Option<TaskPermit>, AppError> {
        match self
            .inner
            .acquire_with_cancel(task_type, cancel_token)
            .await?
        {
            Some(permit) => {
                let task_id = permit.task_id().to_string();
                self.update_status(&task_id, "running", None).await?;
                Ok(Some(permit))
            }
            None => Ok(None),
        }
    }

    async fn status(&self, task_id: &str) -> Result<TaskInfo, AppError> {
        let row = task_queue::Entity::find_by_id(task_id.to_string())
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("任务不存在: {task_id}")))?;
        task_info_from_row(row)
    }

    async fn cancel(&self, task_id: &str) -> Result<(), AppError> {
        for cancelled_id in self.inner.cancel_task_now(task_id)? {
            self.update_status(&cancelled_id, "cancelled", None).await?;
        }
        Ok(())
    }

    fn queue_stats(&self) -> QueueStats {
        self.inner.queue_stats()
    }

    async fn history(
        &self,
        project_id: Option<i64>,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<TaskInfo>, AppError> {
        let mut query = task_queue::Entity::find()
            .order_by_desc(task_queue::Column::CreatedAt)
            .offset(offset)
            .limit(limit);
        if let Some(project_id) = project_id {
            query = query.filter(task_queue::Column::ProjectId.eq(project_id));
        }
        query
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .into_iter()
            .map(task_info_from_row)
            .collect()
    }
}

#[cfg(test)]
mod tests;
