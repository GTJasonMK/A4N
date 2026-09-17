//! 回补 generation checkpoint 与 task_queue 之间缺失的 workflow 绑定。

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;
use serde_json::Value;
use sha2::{Digest, Sha256};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N generation_task_workflow_binding_repair migration only supports PostgreSQL"
                    .to_string(),
            ));
        }
        let connection = manager.get_connection();
        let rows = connection
            .query_all(Statement::from_string(
                DbBackend::Postgres,
                r#"
SELECT checkpoint.task_id, task.workflow_run_id, checkpoint.public_snapshot_json
FROM generation_task_checkpoint AS checkpoint
INNER JOIN task_queue AS task ON task.id = checkpoint.task_id
WHERE checkpoint.workflow_run_id IS NULL
  AND task.workflow_run_id IS NOT NULL
ORDER BY checkpoint.task_id
"#
                .to_string(),
            ))
            .await?;

        for row in rows {
            let task_id: String = row.try_get("", "task_id")?;
            let workflow_run_id: i64 = row.try_get("", "workflow_run_id")?;
            let snapshot: Option<Value> = row.try_get("", "public_snapshot_json")?;
            if let Some(mut snapshot) = snapshot {
                bind_snapshot_workflow(&mut snapshot, &task_id, workflow_run_id)?;
                let canonical = serde_json_canonicalizer::to_vec(&snapshot).map_err(|error| {
                    DbErr::Custom(format!(
                        "generation task {task_id} public snapshot canonical JSON 失败: {error}"
                    ))
                })?;
                let sha256 = format!("{:x}", Sha256::digest(canonical));
                connection
                    .execute(Statement::from_sql_and_values(
                        DbBackend::Postgres,
                        r#"
UPDATE generation_task_checkpoint AS checkpoint
SET workflow_run_id = $2,
    public_snapshot_json = $3,
    public_snapshot_sha256 = $4,
    checkpoint_revision = checkpoint_revision + 1,
    updated_at = NOW()
FROM task_queue AS task
WHERE checkpoint.task_id = $1
  AND task.id = checkpoint.task_id
  AND checkpoint.workflow_run_id IS NULL
  AND task.workflow_run_id = $2
"#,
                        [
                            task_id.into(),
                            workflow_run_id.into(),
                            snapshot.into(),
                            sha256.into(),
                        ],
                    ))
                    .await?;
            } else {
                connection
                    .execute(Statement::from_sql_and_values(
                        DbBackend::Postgres,
                        r#"
UPDATE generation_task_checkpoint AS checkpoint
SET workflow_run_id = $2,
    checkpoint_revision = checkpoint_revision + 1,
    updated_at = NOW()
FROM task_queue AS task
WHERE checkpoint.task_id = $1
  AND task.id = checkpoint.task_id
  AND checkpoint.workflow_run_id IS NULL
  AND task.workflow_run_id = $2
"#,
                        [task_id.into(), workflow_run_id.into()],
                    ))
                    .await?;
            }
        }
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "generation_task_workflow_binding_repair 是前向数据修复，不能安全回滚".to_string(),
        ))
    }
}

fn bind_snapshot_workflow(
    snapshot: &mut Value,
    task_id: &str,
    workflow_run_id: i64,
) -> Result<(), DbErr> {
    let started = snapshot
        .get_mut("started")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| {
            DbErr::Custom(format!(
                "generation task {task_id} public snapshot 缺少 started"
            ))
        })?;
    if let Some(current) = started.get("workflow_run_id").and_then(Value::as_i64)
        && current != workflow_run_id
    {
        return Err(DbErr::Custom(format!(
            "generation task {task_id} public snapshot 已绑定 workflow {current}，不能回补 {workflow_run_id}"
        )));
    }
    started.insert(
        "workflow_run_id".to_string(),
        Value::Number(workflow_run_id.into()),
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::bind_snapshot_workflow;
    use serde_json::json;

    #[test]
    fn repair_binds_workflow_in_public_snapshot() {
        let mut snapshot = json!({"started": {"workflow_run_id": null}});
        bind_snapshot_workflow(&mut snapshot, "task", 170).unwrap();
        assert_eq!(snapshot["started"]["workflow_run_id"], 170);
    }
}
