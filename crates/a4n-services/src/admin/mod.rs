//! 管理后台服务：用量统计、任务队列管理
//!
//! 对应文档 17-管理后台系统

use sea_orm::*;
use std::sync::Arc;

use a4n_core::error::AppError;
use a4n_core::models::task::task_queue;
use a4n_core::models::user::llm_usage_log;

#[derive(Clone)]
pub struct AdminService {
    db: Arc<DatabaseConnection>,
}

impl AdminService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 查询用户用量（最近 N 条）
    pub async fn usage_by_user(
        &self,
        user_id: i64,
        limit: u64,
    ) -> Result<Vec<llm_usage_log::Model>, AppError> {
        llm_usage_log::Entity::find()
            .filter(llm_usage_log::Column::UserId.eq(user_id))
            .order_by_desc(llm_usage_log::Column::CreatedAt)
            .limit(limit)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    /// Token 用量汇总
    pub async fn token_summary(&self, user_id: i64) -> Result<serde_json::Value, AppError> {
        let logs = llm_usage_log::Entity::find()
            .filter(llm_usage_log::Column::UserId.eq(user_id))
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;

        let total_prompt: i64 = logs.iter().map(|l| l.prompt_tokens as i64).sum();
        let total_completion: i64 = logs.iter().map(|l| l.completion_tokens as i64).sum();
        let total_tokens: i64 = total_prompt + total_completion;
        let success_count = logs.iter().filter(|l| l.is_success).count();
        let fail_count = logs.len() - success_count;

        Ok(serde_json::json!({
            "total_calls": logs.len(),
            "total_prompt_tokens": total_prompt,
            "total_completion_tokens": total_completion,
            "total_tokens": total_tokens,
            "success_count": success_count,
            "fail_count": fail_count,
        }))
    }

    // ===== 任务队列 =====

    /// 列出待处理任务
    pub async fn pending_tasks(&self) -> Result<Vec<task_queue::Model>, AppError> {
        let mut tasks = task_queue::Entity::find()
            .filter(task_queue::Column::Status.eq("pending"))
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        tasks.sort_by(|a, b| {
            priority_rank(&a.priority)
                .cmp(&priority_rank(&b.priority))
                .then(a.created_at.cmp(&b.created_at))
                .then(a.id.cmp(&b.id))
        });
        Ok(tasks)
    }

    /// 健康检查：数据库连接。
    pub async fn health_check(&self) -> Result<serde_json::Value, AppError> {
        let db_ok = self
            .db
            .execute(sea_orm::Statement::from_string(
                self.db.get_database_backend(),
                "SELECT 1",
            ))
            .await
            .is_ok();

        Ok(serde_json::json!({
            "database": db_ok,
            "backend": format!("{:?}", self.db.get_database_backend()),
        }))
    }
}

fn priority_rank(priority: &str) -> u8 {
    match priority {
        "high" => 0,
        "medium" => 1,
        "low" => 2,
        _ => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use sea_orm::{DatabaseConnection, EntityTrait, Set};

    async fn setup_db() -> Arc<DatabaseConnection> {
        Arc::new(a4n_migration::test_support::connect_test_database().await)
    }

    async fn insert_task(
        db: &DatabaseConnection,
        id: &str,
        priority: &str,
        status: &str,
        created_offset: i64,
    ) {
        let now = Utc::now().naive_utc() + Duration::seconds(created_offset);
        task_queue::Entity::insert(task_queue::ActiveModel {
            id: Set(id.to_string()),
            task_type: Set("llm_call".to_string()),
            priority: Set(priority.to_string()),
            status: Set(status.to_string()),
            project_id: Set(Some(1)),
            root_project_id: Set(None),
            source_snapshot_id: Set(None),
            task_scope: Set(None),
            workflow_run_id: Set(None),
            payload: Set(None),
            progress: Set(None),
            error_message: Set(None),
            dag_id: Set(None),
            created_at: Set(now),
            started_at: Set(None),
            completed_at: Set(
                matches!(status, "completed" | "failed" | "cancelled").then_some(now),
            ),
        })
        .exec(db)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn pending_tasks_orders_by_semantic_priority_not_lexicographic_text() {
        let db = setup_db().await;
        insert_task(db.as_ref(), "low-first", "low", "pending", 0).await;
        insert_task(db.as_ref(), "medium-second", "medium", "pending", 1).await;
        insert_task(db.as_ref(), "high-third", "high", "pending", 2).await;
        insert_task(db.as_ref(), "unknown-last", "urgent", "pending", 3).await;
        let service = AdminService::new(db);

        let task_ids = service
            .pending_tasks()
            .await
            .unwrap()
            .into_iter()
            .map(|task| task.id)
            .collect::<Vec<_>>();

        assert_eq!(
            task_ids,
            vec!["high-third", "medium-second", "low-first", "unknown-last"]
        );
    }
}
