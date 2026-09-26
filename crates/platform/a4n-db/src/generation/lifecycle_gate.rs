use sea_orm::{
    ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbBackend, DbErr, Statement,
    TransactionTrait,
};

use super::tasks::{GenerationTaskPersistenceError, validate_safe_id};
use a4n_domain::generation::{
    GenerationLifecycleTask, LifecycleGateClaimState, PendingLifecycleOperation,
    ProjectLifecycleGateClaim, ProjectLifecycleOperation, UserLifecycleGateClaim,
};

pub const PROJECT_GATE_ACTIVE: &str = "active";
pub(crate) const PROJECT_GATE_DELETING: &str = "deleting";
pub(crate) const PROJECT_GATE_RESETTING: &str = "resetting";
pub(crate) const USER_GATE_ACTIVE: &str = "active";
const USER_GATE_DISABLING: &str = "disabling";
const USER_GATE_DISABLED: &str = "disabled";

const PROJECT_METADATA_SQL: &str =
    "SELECT user_id, root_project_id FROM novel_project WHERE id = $1";
const LOCK_USER_GATE_SQL: &str = "SELECT lifecycle_gate FROM \"user\" WHERE id = $1 FOR UPDATE";
const SHARE_USER_GATE_SQL: &str = "SELECT lifecycle_gate FROM \"user\" WHERE id = $1 FOR SHARE";
const LOCK_PROJECT_GATE_SQL: &str = r#"
SELECT user_id, root_project_id, lifecycle_gate
FROM novel_project
WHERE id = $1
FOR UPDATE
"#;
const SHARE_PROJECT_GATE_SQL: &str = r#"
SELECT user_id, root_project_id, lifecycle_gate
FROM novel_project
WHERE id = $1
FOR SHARE
"#;
const PROJECT_CONFLICT_SQL: &str = r#"
SELECT id, lifecycle_gate
FROM novel_project
WHERE user_id = $1
  AND lifecycle_gate <> 'active'
ORDER BY id
LIMIT 1
"#;
const DERIVED_PROJECT_CONFLICT_SQL: &str = r#"
SELECT id, lifecycle_gate
FROM novel_project
WHERE root_project_id = $1
  AND lifecycle_gate <> 'active'
ORDER BY id
LIMIT 1
"#;
const UPDATE_PROJECT_GATE_SQL: &str = r#"
UPDATE novel_project
SET lifecycle_gate = $2,
    updated_at = NOW()
WHERE id = $1
  AND lifecycle_gate = 'active'
"#;
const UPDATE_USER_GATE_SQL: &str = r#"
UPDATE "user"
SET lifecycle_gate = 'disabling',
    updated_at = NOW()
WHERE id = $1
  AND lifecycle_gate = 'active'
"#;
const COMPLETE_USER_DISABLE_SQL: &str = r#"
UPDATE "user"
SET lifecycle_gate = 'disabled',
    is_active = FALSE,
    updated_at = NOW()
WHERE id = $1
  AND lifecycle_gate = 'disabling'
"#;
const COMPLETE_PROJECT_RESET_SQL: &str = r#"
UPDATE novel_project
SET lifecycle_gate = 'active',
    updated_at = NOW()
WHERE id = $1
  AND lifecycle_gate = 'resetting'
"#;
const RESTORE_PROJECT_GATE_SQL: &str = r#"
UPDATE novel_project
SET lifecycle_gate = 'active',
    updated_at = NOW()
WHERE id = $1
  AND lifecycle_gate = $2
"#;
const RESTORE_USER_GATE_SQL: &str = r#"
UPDATE "user"
SET lifecycle_gate = 'active',
    updated_at = NOW()
WHERE id = $1
  AND lifecycle_gate = 'disabling'
  AND is_active = TRUE
"#;
const PENDING_USER_GATES_SQL: &str = r#"
SELECT id
FROM "user"
WHERE lifecycle_gate = 'disabling'
ORDER BY id
"#;
const PENDING_PROJECT_GATES_SQL: &str = r#"
SELECT id, user_id, root_project_id, lifecycle_gate
FROM novel_project
WHERE lifecycle_gate IN ('deleting', 'resetting')
ORDER BY COALESCE(root_project_id, id), id
"#;
const ACTIVE_TASKS_FOR_USER_SQL: &str = r#"
SELECT task_id, attempt, status
FROM generation_task_checkpoint
WHERE requested_by_user_id = $1
  AND status IN ('reserved', 'running', 'finalizing', 'cancelling')
ORDER BY task_id
"#;
const ACTIVE_TASKS_FOR_PROJECT_SQL: &str = r#"
SELECT task_id, attempt, status
FROM generation_task_checkpoint
WHERE (project_id = $1 OR root_project_id = $1)
  AND status IN ('reserved', 'running', 'finalizing', 'cancelling')
ORDER BY task_id
"#;
const ACTIVE_TASKS_SQL: &str = r#"
SELECT task_id, attempt, status
FROM generation_task_checkpoint
WHERE status IN ('reserved', 'running', 'finalizing', 'cancelling')
ORDER BY task_id
"#;

#[derive(Debug, thiserror::Error)]
pub enum GenerationLifecycleGateError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{0}")]
    NotFound(String),
    #[error("generation lifecycle gate 被占用: {0}")]
    Blocked(String),
    #[error("generation lifecycle gate 数据损坏: {0}")]
    Corrupt(String),
    #[error(transparent)]
    Database(#[from] DbErr),
    #[error(transparent)]
    ApplicationDatabase(a4n_error::DatabaseError),
}

impl GenerationLifecycleGateError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidArgument(_) => "generation_request_invalid",
            Self::NotFound(_) => "generation_task_not_found",
            Self::Blocked(_) => "generation_owner_lifecycle_blocked",
            Self::Corrupt(_) => "generation_checkpoint_corrupt",
            Self::Database(_) => "generation_persist_failed",
            Self::ApplicationDatabase(_) => "generation_persist_failed",
        }
    }
}

pub async fn begin_project_lifecycle_gate(
    db: &DatabaseConnection,
    project_id: i64,
    expected_owner_user_id: i64,
    operation: ProjectLifecycleOperation,
) -> Result<ProjectLifecycleGateClaim, GenerationLifecycleGateError> {
    ensure_postgres(db)?;
    validate_safe_id(project_id, "project_id")?;
    validate_safe_id(expected_owner_user_id, "expected_owner_user_id")?;

    let metadata = project_metadata(db, project_id).await?;
    if metadata.user_id != expected_owner_user_id {
        return Err(GenerationLifecycleGateError::Blocked(format!(
            "project {project_id} owner 已变化"
        )));
    }

    let tx = db.begin().await?;
    require_gate(
        lock_user_gate(&tx, metadata.user_id).await?,
        USER_GATE_ACTIVE,
        format!("user {}", metadata.user_id),
    )?;

    if let Some(root_project_id) = metadata.root_project_id.filter(|id| *id != project_id) {
        let root = lock_project_gate(&tx, root_project_id).await?;
        if root.user_id != metadata.user_id {
            return Err(GenerationLifecycleGateError::Corrupt(format!(
                "project {project_id} 与 root project {root_project_id} owner 不一致"
            )));
        }
        require_gate(
            root.lifecycle_gate,
            PROJECT_GATE_ACTIVE,
            format!("root project {root_project_id}"),
        )?;
    }

    let project = lock_project_gate(&tx, project_id).await?;
    if project.user_id != metadata.user_id || project.root_project_id != metadata.root_project_id {
        return Err(GenerationLifecycleGateError::Blocked(format!(
            "project {project_id} owner/root 在 gate 事务前发生变化"
        )));
    }
    if metadata.root_project_id.is_none() {
        if operation == ProjectLifecycleOperation::Delete {
            let derived_count = crate::project::derived_project_count(&tx, project_id).await?;
            if derived_count > 0 {
                return Err(GenerationLifecycleGateError::InvalidArgument(
                    crate::project::delete::ProjectDeleteDbError::DerivedProjectsExist {
                        project_id,
                        derived_count,
                    }
                    .to_string(),
                ));
            }
        }
        reject_derived_gate_conflict(&tx, project_id).await?;
    }

    let desired_gate = operation.gate();
    let state = match project.lifecycle_gate.as_str() {
        PROJECT_GATE_ACTIVE => {
            let result = tx
                .execute(Statement::from_sql_and_values(
                    DbBackend::Postgres,
                    UPDATE_PROJECT_GATE_SQL,
                    [project_id.into(), desired_gate.into()],
                ))
                .await?;
            if result.rows_affected() != 1 {
                return Err(GenerationLifecycleGateError::Blocked(format!(
                    "project {project_id} gate CAS 失败"
                )));
            }
            LifecycleGateClaimState::Started
        }
        current if current == desired_gate => LifecycleGateClaimState::InProgress,
        current => {
            return Err(GenerationLifecycleGateError::Blocked(format!(
                "project {project_id} 当前 gate={current}，请求 gate={desired_gate}"
            )));
        }
    };
    tx.commit().await?;
    Ok(ProjectLifecycleGateClaim {
        project_id,
        user_id: metadata.user_id,
        root_project_id: metadata.root_project_id,
        operation,
        state,
    })
}

pub async fn begin_user_disable_gate(
    db: &DatabaseConnection,
    caller_user_id: i64,
    user_id: i64,
) -> Result<UserLifecycleGateClaim, GenerationLifecycleGateError> {
    ensure_postgres(db)?;
    validate_safe_id(caller_user_id, "caller_user_id")?;
    validate_safe_id(user_id, "user_id")?;
    if caller_user_id == user_id {
        return Err(GenerationLifecycleGateError::InvalidArgument(
            "不能禁用当前登录用户".to_string(),
        ));
    }

    let tx = db.begin().await?;
    crate::auth::lock_administrator_membership(&tx, user_id, true)
        .await
        .map_err(|error| match error {
            a4n_error::AppError::Database(error) => {
                GenerationLifecycleGateError::ApplicationDatabase(error)
            }
            a4n_error::AppError::NotFound(message) => {
                GenerationLifecycleGateError::NotFound(message)
            }
            error => GenerationLifecycleGateError::InvalidArgument(error.to_string()),
        })?;
    let current = lock_user_gate(&tx, user_id).await?;
    let state = match current.as_str() {
        USER_GATE_ACTIVE => {
            reject_project_gate_conflict(&tx, user_id).await?;
            let result = tx
                .execute(Statement::from_sql_and_values(
                    DbBackend::Postgres,
                    UPDATE_USER_GATE_SQL,
                    [user_id.into()],
                ))
                .await?;
            if result.rows_affected() != 1 {
                return Err(GenerationLifecycleGateError::Blocked(format!(
                    "user {user_id} gate CAS 失败"
                )));
            }
            LifecycleGateClaimState::Started
        }
        USER_GATE_DISABLING => LifecycleGateClaimState::InProgress,
        USER_GATE_DISABLED => LifecycleGateClaimState::Complete,
        current => {
            return Err(GenerationLifecycleGateError::Corrupt(format!(
                "user {user_id} gate={current}"
            )));
        }
    };
    tx.commit().await?;
    Ok(UserLifecycleGateClaim { user_id, state })
}

pub async fn active_generation_tasks_for_user(
    db: &DatabaseConnection,
    user_id: i64,
) -> Result<Vec<GenerationLifecycleTask>, GenerationLifecycleGateError> {
    ensure_postgres(db)?;
    validate_safe_id(user_id, "user_id")?;
    query_lifecycle_tasks(db, ACTIVE_TASKS_FOR_USER_SQL, user_id).await
}

pub async fn active_generation_tasks_for_project(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<Vec<GenerationLifecycleTask>, GenerationLifecycleGateError> {
    ensure_postgres(db)?;
    validate_safe_id(project_id, "project_id")?;
    query_lifecycle_tasks(db, ACTIVE_TASKS_FOR_PROJECT_SQL, project_id).await
}

pub async fn active_generation_tasks(
    db: &DatabaseConnection,
) -> Result<Vec<GenerationLifecycleTask>, GenerationLifecycleGateError> {
    ensure_postgres(db)?;
    query_lifecycle_tasks_without_owner(db, ACTIVE_TASKS_SQL).await
}

pub async fn pending_lifecycle_operations(
    db: &DatabaseConnection,
) -> Result<Vec<PendingLifecycleOperation>, GenerationLifecycleGateError> {
    ensure_postgres(db)?;
    let mut operations = Vec::new();
    for row in db
        .query_all(Statement::from_string(
            DbBackend::Postgres,
            PENDING_USER_GATES_SQL,
        ))
        .await?
    {
        operations.push(PendingLifecycleOperation::DisableUser {
            user_id: row.try_get("", "id")?,
        });
    }
    for row in db
        .query_all(Statement::from_string(
            DbBackend::Postgres,
            PENDING_PROJECT_GATES_SQL,
        ))
        .await?
    {
        let project_id = row.try_get("", "id")?;
        let user_id = row.try_get("", "user_id")?;
        let root_project_id = row.try_get("", "root_project_id")?;
        let gate: String = row.try_get("", "lifecycle_gate")?;
        operations.push(match gate.as_str() {
            PROJECT_GATE_DELETING => PendingLifecycleOperation::DeleteProject {
                project_id,
                user_id,
                root_project_id,
            },
            PROJECT_GATE_RESETTING => PendingLifecycleOperation::ResetProject {
                project_id,
                user_id,
                root_project_id,
            },
            _ => {
                return Err(GenerationLifecycleGateError::Corrupt(format!(
                    "project {project_id} pending gate={gate}"
                )));
            }
        });
    }
    Ok(operations)
}

pub(super) async fn require_generation_owner_gates_active<C>(
    db: &C,
    requested_by_user_id: i64,
    project_id: Option<i64>,
    root_project_id: Option<i64>,
) -> Result<(), GenerationTaskPersistenceError>
where
    C: ConnectionTrait,
{
    validate_safe_id(requested_by_user_id, "requested_by_user_id")?;
    let user = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            SHARE_USER_GATE_SQL,
            [requested_by_user_id.into()],
        ))
        .await?
        .ok_or_else(|| {
            GenerationTaskPersistenceError::NotFound(format!("用户不存在: {requested_by_user_id}"))
        })?;
    require_active_generation_gate(
        user.try_get("", "lifecycle_gate")?,
        "用户",
        requested_by_user_id,
    )?;

    let (Some(project_id), Some(root_project_id)) = (project_id, root_project_id) else {
        if project_id.is_none() && root_project_id.is_none() {
            return Ok(());
        }
        return Err(GenerationTaskPersistenceError::InvalidArgument(
            "generation owner project_id/root_project_id 必须同空或同非空".to_string(),
        ));
    };
    validate_safe_id(project_id, "project_id")?;
    validate_safe_id(root_project_id, "root_project_id")?;

    let root = share_project_gate(db, root_project_id).await?;
    if root.user_id != requested_by_user_id
        || root.root_project_id.unwrap_or(root_project_id) != root_project_id
    {
        return Err(GenerationTaskPersistenceError::InvalidArgument(format!(
            "root project {root_project_id} 与 generation owner 不一致"
        )));
    }
    require_active_generation_gate(root.lifecycle_gate, "项目", root_project_id)?;

    if project_id != root_project_id {
        let project = share_project_gate(db, project_id).await?;
        if project.user_id != requested_by_user_id
            || project.root_project_id != Some(root_project_id)
        {
            return Err(GenerationTaskPersistenceError::InvalidArgument(format!(
                "项目 {project_id} 不属于 user {requested_by_user_id} / root {root_project_id}"
            )));
        }
        require_active_generation_gate(project.lifecycle_gate, "项目", project_id)?;
    }
    Ok(())
}

pub(super) async fn resolve_generation_root_project_id<C>(
    db: &C,
    project_id: i64,
) -> Result<i64, GenerationTaskPersistenceError>
where
    C: ConnectionTrait,
{
    validate_safe_id(project_id, "project_id")?;
    let row = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            PROJECT_METADATA_SQL,
            [project_id.into()],
        ))
        .await?
        .ok_or_else(|| {
            GenerationTaskPersistenceError::NotFound(format!("项目不存在: {project_id}"))
        })?;
    let root_project_id: Option<i64> = row.try_get("", "root_project_id")?;
    Ok(root_project_id.unwrap_or(project_id))
}

pub(crate) async fn require_project_lifecycle_gate<C>(
    db: &C,
    project_id: i64,
    expected_gate: &str,
) -> Result<(), GenerationLifecycleGateError>
where
    C: ConnectionTrait,
{
    validate_project_gate(expected_gate)?;
    let project = lock_project_gate(db, project_id).await?;
    require_gate(
        project.lifecycle_gate,
        expected_gate,
        format!("project {project_id}"),
    )
}

pub async fn complete_project_reset_gate<C>(
    db: &C,
    project_id: i64,
) -> Result<(), GenerationLifecycleGateError>
where
    C: ConnectionTrait,
{
    execute_single_row(db, COMPLETE_PROJECT_RESET_SQL, [project_id.into()], || {
        format!("project {project_id} resetting→active CAS 失败")
    })
    .await
}

pub async fn complete_user_disable_gate<C>(
    db: &C,
    user_id: i64,
) -> Result<(), GenerationLifecycleGateError>
where
    C: ConnectionTrait,
{
    execute_single_row(db, COMPLETE_USER_DISABLE_SQL, [user_id.into()], || {
        format!("user {user_id} disabling→disabled CAS 失败")
    })
    .await
}

pub async fn restore_project_lifecycle_gate(
    db: &DatabaseConnection,
    project_id: i64,
    operation: ProjectLifecycleOperation,
) -> Result<(), GenerationLifecycleGateError> {
    ensure_postgres(db)?;
    execute_single_row(
        db,
        RESTORE_PROJECT_GATE_SQL,
        [project_id.into(), operation.gate().into()],
        || format!("project {project_id} gate 恢复 active 失败"),
    )
    .await
}

pub async fn restore_user_lifecycle_gate(
    db: &DatabaseConnection,
    user_id: i64,
) -> Result<(), GenerationLifecycleGateError> {
    ensure_postgres(db)?;
    execute_single_row(db, RESTORE_USER_GATE_SQL, [user_id.into()], || {
        format!("user {user_id} gate 恢复 active 失败")
    })
    .await
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ProjectMetadata {
    user_id: i64,
    root_project_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LockedProjectGate {
    user_id: i64,
    root_project_id: Option<i64>,
    lifecycle_gate: String,
}

async fn project_metadata<C>(
    db: &C,
    project_id: i64,
) -> Result<ProjectMetadata, GenerationLifecycleGateError>
where
    C: ConnectionTrait,
{
    let row = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            PROJECT_METADATA_SQL,
            [project_id.into()],
        ))
        .await?
        .ok_or_else(|| {
            GenerationLifecycleGateError::NotFound(format!("project 不存在: {project_id}"))
        })?;
    Ok(ProjectMetadata {
        user_id: row.try_get("", "user_id")?,
        root_project_id: row.try_get("", "root_project_id")?,
    })
}

async fn lock_user_gate<C>(db: &C, user_id: i64) -> Result<String, GenerationLifecycleGateError>
where
    C: ConnectionTrait,
{
    db.query_one(Statement::from_sql_and_values(
        DbBackend::Postgres,
        LOCK_USER_GATE_SQL,
        [user_id.into()],
    ))
    .await?
    .ok_or_else(|| GenerationLifecycleGateError::NotFound(format!("user 不存在: {user_id}")))?
    .try_get("", "lifecycle_gate")
    .map_err(Into::into)
}

async fn lock_project_gate<C>(
    db: &C,
    project_id: i64,
) -> Result<LockedProjectGate, GenerationLifecycleGateError>
where
    C: ConnectionTrait,
{
    let row = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            LOCK_PROJECT_GATE_SQL,
            [project_id.into()],
        ))
        .await?
        .ok_or_else(|| {
            GenerationLifecycleGateError::NotFound(format!("project 不存在: {project_id}"))
        })?;
    Ok(LockedProjectGate {
        user_id: row.try_get("", "user_id")?,
        root_project_id: row.try_get("", "root_project_id")?,
        lifecycle_gate: row.try_get("", "lifecycle_gate")?,
    })
}

async fn share_project_gate<C>(
    db: &C,
    project_id: i64,
) -> Result<LockedProjectGate, GenerationTaskPersistenceError>
where
    C: ConnectionTrait,
{
    let row = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            SHARE_PROJECT_GATE_SQL,
            [project_id.into()],
        ))
        .await?
        .ok_or_else(|| {
            GenerationTaskPersistenceError::NotFound(format!("项目不存在: {project_id}"))
        })?;
    Ok(LockedProjectGate {
        user_id: row.try_get("", "user_id")?,
        root_project_id: row.try_get("", "root_project_id")?,
        lifecycle_gate: row.try_get("", "lifecycle_gate")?,
    })
}

fn require_active_generation_gate(
    lifecycle_gate: String,
    owner_kind: &str,
    owner_id: i64,
) -> Result<(), GenerationTaskPersistenceError> {
    if lifecycle_gate == "active" {
        Ok(())
    } else {
        Err(GenerationTaskPersistenceError::Rejected {
            code: "generation_owner_lifecycle_blocked",
            message: format!(
                "{owner_kind} {owner_id} lifecycle gate 为 {lifecycle_gate}，不能推进生成任务"
            ),
        })
    }
}

async fn reject_project_gate_conflict(
    tx: &DatabaseTransaction,
    user_id: i64,
) -> Result<(), GenerationLifecycleGateError> {
    if let Some(row) = tx
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            PROJECT_CONFLICT_SQL,
            [user_id.into()],
        ))
        .await?
    {
        let project_id: i64 = row.try_get("", "id")?;
        let gate: String = row.try_get("", "lifecycle_gate")?;
        return Err(GenerationLifecycleGateError::Blocked(format!(
            "user {user_id} 的 project {project_id} gate={gate}"
        )));
    }
    Ok(())
}

async fn reject_derived_gate_conflict(
    tx: &DatabaseTransaction,
    root_project_id: i64,
) -> Result<(), GenerationLifecycleGateError> {
    if let Some(row) = tx
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            DERIVED_PROJECT_CONFLICT_SQL,
            [root_project_id.into()],
        ))
        .await?
    {
        let project_id: i64 = row.try_get("", "id")?;
        let gate: String = row.try_get("", "lifecycle_gate")?;
        return Err(GenerationLifecycleGateError::Blocked(format!(
            "root project {root_project_id} 的 derived project {project_id} gate={gate}"
        )));
    }
    Ok(())
}

async fn query_lifecycle_tasks(
    db: &DatabaseConnection,
    sql: &str,
    owner_id: i64,
) -> Result<Vec<GenerationLifecycleTask>, GenerationLifecycleGateError> {
    db.query_all(Statement::from_sql_and_values(
        DbBackend::Postgres,
        sql,
        [owner_id.into()],
    ))
    .await?
    .into_iter()
    .map(lifecycle_task_from_row)
    .collect()
}

async fn query_lifecycle_tasks_without_owner(
    db: &DatabaseConnection,
    sql: &str,
) -> Result<Vec<GenerationLifecycleTask>, GenerationLifecycleGateError> {
    db.query_all(Statement::from_string(DbBackend::Postgres, sql))
        .await?
        .into_iter()
        .map(lifecycle_task_from_row)
        .collect()
}

fn lifecycle_task_from_row(
    row: sea_orm::QueryResult,
) -> Result<GenerationLifecycleTask, GenerationLifecycleGateError> {
    let attempt: i32 = row.try_get("", "attempt")?;
    Ok(GenerationLifecycleTask {
        task_id: row.try_get("", "task_id")?,
        attempt: u32::try_from(attempt).map_err(|_| {
            GenerationLifecycleGateError::Corrupt(format!("generation task attempt={attempt}"))
        })?,
        status: row.try_get("", "status")?,
    })
}

async fn execute_single_row<C, const N: usize>(
    db: &C,
    sql: &str,
    values: [sea_orm::Value; N],
    conflict: impl FnOnce() -> String,
) -> Result<(), GenerationLifecycleGateError>
where
    C: ConnectionTrait,
{
    let result = db
        .execute(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            values,
        ))
        .await?;
    if result.rows_affected() == 1 {
        Ok(())
    } else {
        Err(GenerationLifecycleGateError::Blocked(conflict()))
    }
}

fn require_gate(
    current: String,
    expected: &str,
    owner: String,
) -> Result<(), GenerationLifecycleGateError> {
    if current == expected {
        Ok(())
    } else {
        Err(GenerationLifecycleGateError::Blocked(format!(
            "{owner} 当前 gate={current}，要求 gate={expected}"
        )))
    }
}

fn validate_project_gate(gate: &str) -> Result<(), GenerationLifecycleGateError> {
    if matches!(
        gate,
        PROJECT_GATE_ACTIVE | PROJECT_GATE_DELETING | PROJECT_GATE_RESETTING
    ) {
        Ok(())
    } else {
        Err(GenerationLifecycleGateError::InvalidArgument(format!(
            "未知 project lifecycle gate: {gate}"
        )))
    }
}

fn ensure_postgres(db: &DatabaseConnection) -> Result<(), GenerationLifecycleGateError> {
    if db.get_database_backend() == DbBackend::Postgres {
        Ok(())
    } else {
        Err(GenerationLifecycleGateError::InvalidArgument(
            "generation lifecycle gate 仅支持 PostgreSQL".to_string(),
        ))
    }
}

impl From<GenerationTaskPersistenceError> for GenerationLifecycleGateError {
    fn from(error: GenerationTaskPersistenceError) -> Self {
        match error {
            GenerationTaskPersistenceError::InvalidArgument(message) => {
                Self::InvalidArgument(message)
            }
            other => Self::Corrupt(other.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_gate_sql_keeps_short_transaction_and_owner_order_contract() {
        assert!(LOCK_USER_GATE_SQL.contains("FOR UPDATE"));
        assert!(LOCK_PROJECT_GATE_SQL.contains("FOR UPDATE"));
        assert!(SHARE_USER_GATE_SQL.contains("FOR SHARE"));
        assert!(SHARE_PROJECT_GATE_SQL.contains("FOR SHARE"));
        assert!(UPDATE_PROJECT_GATE_SQL.contains("lifecycle_gate = 'active'"));
        assert!(UPDATE_USER_GATE_SQL.contains("lifecycle_gate = 'active'"));
        assert!(COMPLETE_USER_DISABLE_SQL.contains("is_active = FALSE"));
        assert!(PENDING_PROJECT_GATES_SQL.contains("COALESCE(root_project_id, id), id"));
        assert!(ACTIVE_TASKS_FOR_PROJECT_SQL.contains("ORDER BY task_id"));
        assert_eq!(ProjectLifecycleOperation::Delete.gate(), "deleting");
        assert_eq!(ProjectLifecycleOperation::Reset.gate(), "resetting");
    }
}
