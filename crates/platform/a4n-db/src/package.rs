use a4n_db_models::models::{
    agent::run as agent_run,
    generation::generation_task_checkpoint,
    package::{host_package_operation, host_package_state, user_package_preference},
    task::task_queue,
};
use a4n_error::AppError;
use chrono::Utc;
use sea_orm::sea_query::{LockType, OnConflict};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, TransactionTrait,
};
use serde_json::Value;

use a4n_domain::generation::GenerationLifecycleTask;

const HOST_SCOPE: &str = "a4n-server";
pub const INSTALLATION_INSTALLED: &str = "installed";
pub const INSTALLATION_MISSING: &str = "missing";
pub const INSTALLATION_INCOMPATIBLE: &str = "incompatible";
pub const HOST_STATE_ENABLED: &str = "enabled";
pub const HOST_STATE_DISABLED: &str = "disabled";
pub const HOST_STATE_ENABLING: &str = "enabling";
pub const HOST_STATE_DISABLING: &str = "disabling";
pub const OPERATION_PENDING: &str = "pending";
pub const OPERATION_DRAINING: &str = "draining";
pub const OPERATION_WAITING_EXTERNAL: &str = "waiting_external";
pub const OPERATION_FAILED: &str = "failed";
pub const OPERATION_COMPLETED: &str = "completed";

const ACTIVE_TASK_STATUSES: [&str; 2] = ["pending", "running"];
const ACTIVE_GENERATION_STATUSES: [&str; 4] = ["reserved", "running", "finalizing", "cancelling"];

/// 由现有 task_queue、generation checkpoint 和 Agent run 汇总的停用快照。
///
/// 这里故意不落库：这些表才是各自执行事实的 owner，快照只用于一次推进决策。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PackageWorkloadSnapshot {
    pub queue_task_ids: Vec<String>,
    pub generation_tasks: Vec<GenerationLifecycleTask>,
    pub provider_job_ids: Vec<String>,
    pub agent_run_ids: Vec<i64>,
    /// 无法从旧记录可靠确定包归属时，宁可阻塞停用，也不误放行。
    pub unknown_task_ids: Vec<String>,
}

impl PackageWorkloadSnapshot {
    pub fn is_empty(&self) -> bool {
        self.queue_task_ids.is_empty()
            && self.generation_tasks.is_empty()
            && self.provider_job_ids.is_empty()
            && self.agent_run_ids.is_empty()
            && self.unknown_task_ids.is_empty()
    }

    pub fn has_external_work(&self) -> bool {
        !self.provider_job_ids.is_empty()
    }
}

pub async fn ensure_host_package_states(
    db: &DatabaseConnection,
    package_ids: &[String],
) -> Result<(), AppError> {
    if package_ids.is_empty() {
        return Ok(());
    }
    let now = Utc::now().naive_utc();
    host_package_state::Entity::insert_many(package_ids.iter().map(|package_id| {
        host_package_state::ActiveModel {
            host_scope: Set(HOST_SCOPE.to_string()),
            package_id: Set(package_id.clone()),
            installation_state: Set(INSTALLATION_INSTALLED.to_string()),
            host_state: Set(HOST_STATE_ENABLED.to_string()),
            installed_version: Set(None),
            status_version: Set(1),
            updated_at: Set(now),
        }
    }))
    .on_conflict(
        OnConflict::columns([
            host_package_state::Column::HostScope,
            host_package_state::Column::PackageId,
        ])
        .do_nothing()
        .to_owned(),
    )
    .do_nothing()
    .exec_without_returning(db)
    .await
    .map(|_| ())
    .map_err(AppError::Database)
}

pub async fn reconcile_package_installation_state(
    db: &DatabaseConnection,
    package_id: &str,
    installation_state: &str,
) -> Result<(), AppError> {
    if !matches!(
        installation_state,
        INSTALLATION_INSTALLED | INSTALLATION_MISSING | INSTALLATION_INCOMPATIBLE
    ) {
        return Err(AppError::InvalidArgument(format!(
            "未知功能包安装状态: {installation_state}"
        )));
    }
    let tx = db.begin().await.map_err(AppError::Database)?;
    let current = host_package_state::Entity::find()
        .filter(host_package_state::Column::HostScope.eq(HOST_SCOPE))
        .filter(host_package_state::Column::PackageId.eq(package_id))
        .lock(LockType::Update)
        .one(&tx)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("功能包状态不存在: {package_id}")))?;
    if current.installation_state == installation_state {
        tx.commit().await.map_err(AppError::Database)?;
        return Ok(());
    }
    let next_status_version =
        current
            .status_version
            .checked_add(1)
            .ok_or_else(|| AppError::InvalidState {
                current: format!("功能包 {package_id} 状态版本已达到上限"),
                operation: "同步功能包安装状态".to_string(),
            })?;
    let mut active: host_package_state::ActiveModel = current.clone().into();
    active.installation_state = Set(installation_state.to_string());
    if installation_state != INSTALLATION_INSTALLED
        && !matches!(
            current.host_state.as_str(),
            HOST_STATE_ENABLING | HOST_STATE_DISABLING
        )
    {
        active.host_state = Set(HOST_STATE_DISABLED.to_string());
    }
    active.status_version = Set(next_status_version);
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(&tx).await.map_err(AppError::Database)?;
    tx.commit().await.map_err(AppError::Database)
}

pub async fn list_host_package_states(
    db: &DatabaseConnection,
) -> Result<Vec<host_package_state::Model>, AppError> {
    host_package_state::Entity::find()
        .filter(host_package_state::Column::HostScope.eq(HOST_SCOPE))
        .order_by_asc(host_package_state::Column::PackageId)
        .all(db)
        .await
        .map_err(AppError::Database)
}

pub async fn host_package_state(
    db: &DatabaseConnection,
    package_id: &str,
) -> Result<Option<host_package_state::Model>, AppError> {
    host_package_state::Entity::find()
        .filter(host_package_state::Column::HostScope.eq(HOST_SCOPE))
        .filter(host_package_state::Column::PackageId.eq(package_id))
        .one(db)
        .await
        .map_err(AppError::Database)
}

pub async fn latest_package_operation(
    db: &DatabaseConnection,
    package_id: &str,
) -> Result<Option<host_package_operation::Model>, AppError> {
    host_package_operation::Entity::find()
        .filter(host_package_operation::Column::HostScope.eq(HOST_SCOPE))
        .filter(host_package_operation::Column::PackageId.eq(package_id))
        .order_by_desc(host_package_operation::Column::UpdatedAt)
        .one(db)
        .await
        .map_err(AppError::Database)
}

pub async fn package_operation(
    db: &DatabaseConnection,
    operation_id: &str,
) -> Result<Option<host_package_operation::Model>, AppError> {
    host_package_operation::Entity::find_by_id(operation_id.to_string())
        .filter(host_package_operation::Column::HostScope.eq(HOST_SCOPE))
        .one(db)
        .await
        .map_err(AppError::Database)
}

pub async fn pending_package_operations(
    db: &DatabaseConnection,
) -> Result<Vec<host_package_operation::Model>, AppError> {
    host_package_operation::Entity::find()
        .filter(host_package_operation::Column::HostScope.eq(HOST_SCOPE))
        .filter(host_package_operation::Column::State.is_in([
            OPERATION_PENDING,
            OPERATION_DRAINING,
            OPERATION_WAITING_EXTERNAL,
        ]))
        .order_by_asc(host_package_operation::Column::UpdatedAt)
        .all(db)
        .await
        .map_err(AppError::Database)
}

/// 读取一个功能包当前需要等待/取消的工作。
///
/// 旧版本记录没有显式 package owner，因此只对能由 task type/scope 确定的工作
/// 归类；其余工作作为 unknown 返回，调用方必须保持停用中的关闭准入状态。
pub async fn package_workload_snapshot(
    db: &DatabaseConnection,
    package_id: &str,
) -> Result<PackageWorkloadSnapshot, AppError> {
    let checkpoints = generation_task_checkpoint::Entity::find()
        .filter(generation_task_checkpoint::Column::Status.is_in(ACTIVE_GENERATION_STATUSES))
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let generation_ids = checkpoints
        .iter()
        .map(|row| row.task_id.clone())
        .collect::<std::collections::HashSet<_>>();
    let mut snapshot = PackageWorkloadSnapshot::default();

    for checkpoint in checkpoints {
        match workload_package_owner(
            &checkpoint.task_kind,
            None,
            checkpoint.target_kind.as_deref(),
        ) {
            Some(owner) if owner == package_id => {
                snapshot.generation_tasks.push(GenerationLifecycleTask {
                    task_id: checkpoint.task_id,
                    attempt: u32::try_from(checkpoint.attempt).map_err(|_| {
                        AppError::InvalidState {
                            current: "generation checkpoint attempt 超出 u32".to_string(),
                            operation: "读取功能包排空快照".to_string(),
                        }
                    })?,
                    status: checkpoint.status,
                });
                if let Some(provider_job_id) = checkpoint.provider_job_id {
                    snapshot.provider_job_ids.push(provider_job_id);
                }
            }
            Some(_) => {}
            None => snapshot.unknown_task_ids.push(checkpoint.task_id),
        }
    }

    let tasks = task_queue::Entity::find()
        .filter(task_queue::Column::Status.is_in(ACTIVE_TASK_STATUSES))
        .all(db)
        .await
        .map_err(AppError::Database)?;
    for task in tasks {
        if generation_ids.contains(&task.id) {
            continue;
        }
        match workload_package_owner(task.task_type.as_str(), task.task_scope.as_deref(), None) {
            Some(owner) if owner == package_id => snapshot.queue_task_ids.push(task.id),
            Some(_) => {}
            None => snapshot.unknown_task_ids.push(task.id),
        }
    }

    let provider_cleanups = crate::cleanup_outbox::pending_generation_provider_job_cleanups(db)
        .await
        .map_err(AppError::Database)?;
    if !provider_cleanups.is_empty() {
        let cleanup_task_ids = provider_cleanups
            .iter()
            .map(|cleanup| cleanup.task_id.clone())
            .collect::<Vec<_>>();
        let cleanup_checkpoints = generation_task_checkpoint::Entity::find()
            .filter(generation_task_checkpoint::Column::TaskId.is_in(cleanup_task_ids))
            .all(db)
            .await
            .map_err(AppError::Database)?;
        let cleanup_checkpoints = cleanup_checkpoints
            .into_iter()
            .map(|checkpoint| (checkpoint.task_id.clone(), checkpoint))
            .collect::<std::collections::HashMap<_, _>>();
        for cleanup in provider_cleanups {
            match cleanup_checkpoints.get(&cleanup.task_id) {
                Some(checkpoint)
                    if workload_package_owner(
                        &checkpoint.task_kind,
                        None,
                        checkpoint.target_kind.as_deref(),
                    ) == Some(package_id) =>
                {
                    snapshot.provider_job_ids.push(cleanup.provider_job_id);
                }
                Some(_) => {}
                None => snapshot.unknown_task_ids.push(format!(
                    "provider-cleanup:{}:{}",
                    cleanup.task_id, cleanup.provider_job_id
                )),
            }
        }
    }

    // Agent owns the run record, but its LLM/prompt services are owned by
    // ai-workflow. Disabling that dependency must drain the same active runs
    // before its services can be stopped.
    if matches!(package_id, "agent" | "ai-workflow") {
        snapshot.agent_run_ids = agent_run::Entity::find()
            .filter(agent_run::Column::IsActive.eq(true))
            .select_only()
            .column(agent_run::Column::Id)
            .order_by_asc(agent_run::Column::Id)
            .into_tuple::<i64>()
            .all(db)
            .await
            .map_err(AppError::Database)?;
    }
    snapshot.provider_job_ids.sort();
    snapshot.provider_job_ids.dedup();
    snapshot.unknown_task_ids.sort();
    snapshot.unknown_task_ids.dedup();
    Ok(snapshot)
}

fn workload_package_owner(
    task_kind: &str,
    task_scope: Option<&str>,
    target_kind: Option<&str>,
) -> Option<&'static str> {
    let value = format!(
        "{}:{}:{}",
        task_kind,
        task_scope.unwrap_or_default(),
        target_kind.unwrap_or_default()
    )
    .to_ascii_lowercase();
    if value.contains("video") || task_kind == "assembly" {
        Some("video")
    } else if value.contains("manga") {
        Some("manga")
    } else if value.contains("galgame") {
        Some("galgame")
    } else if value.contains("export") {
        Some("export")
    } else if value.contains("import") {
        Some("external-import")
    } else if matches!(
        task_kind,
        "embedding_call" | "embedding_download" | "rag_ingestion"
    ) {
        Some("vector-rag")
    } else if matches!(
        task_kind,
        "image_generation" | "llm_call" | "text_generation"
    ) {
        Some("ai-workflow")
    } else {
        None
    }
}

/// 在创建新的持久任务前锁定其功能包状态。
///
/// 返回 `None` 表示没有可识别的可选包归属。已识别包缺少状态行时按
/// `not_installed` 拒绝，避免初始化窗口绕过停用准入。
pub async fn package_admission_block<C: ConnectionTrait>(
    db: &C,
    task_kind: &str,
    task_scope: Option<&str>,
    target_kind: Option<&str>,
) -> Result<Option<(String, String)>, sea_orm::DbErr> {
    let Some(package_id) = workload_package_owner(task_kind, task_scope, target_kind) else {
        return Ok(None);
    };
    let Some(state) = host_package_state::Entity::find()
        .filter(host_package_state::Column::HostScope.eq(HOST_SCOPE))
        .filter(host_package_state::Column::PackageId.eq(package_id))
        .lock(LockType::Update)
        .one(db)
        .await?
    else {
        return Ok(Some((package_id.to_string(), "not_installed".to_string())));
    };
    let reason = if state.installation_state != INSTALLATION_INSTALLED {
        Some("not_installed")
    } else {
        match state.host_state.as_str() {
            HOST_STATE_ENABLED => None,
            HOST_STATE_ENABLING | HOST_STATE_DISABLING => Some("operation_in_progress"),
            _ => Some("disabled"),
        }
    };
    Ok(reason.map(|reason| (package_id.to_string(), reason.to_string())))
}

/// Atomically closes or opens package admission and records its idempotent
/// lifecycle operation. The business owner validates package dependencies and
/// permissions before calling this boundary; the transaction only owns row
/// locks, status versions, and the operation identity.
pub async fn start_package_lifecycle(
    db: &DatabaseConnection,
    operation_id: &str,
    package_id: &str,
    action: &str,
    requested_version: Option<String>,
    requested_by_user_id: i64,
    expected_status_version: i64,
) -> Result<host_package_operation::Model, AppError> {
    if operation_id.trim().is_empty() {
        return Err(AppError::InvalidArgument(
            "功能包操作 ID 不能为空".to_string(),
        ));
    }
    let transitional_state = match action {
        "enable" => HOST_STATE_ENABLING,
        "disable" => HOST_STATE_DISABLING,
        _ => {
            return Err(AppError::InvalidArgument(format!(
                "未知功能包生命周期动作: {action}"
            )));
        }
    };
    let tx = db.begin().await.map_err(AppError::Database)?;

    if let Some(existing) = host_package_operation::Entity::find_by_id(operation_id.to_string())
        .one(&tx)
        .await
        .map_err(AppError::Database)?
    {
        validate_idempotent_package_operation(
            &existing,
            package_id,
            action,
            requested_version.as_deref(),
        )?;
        tx.commit().await.map_err(AppError::Database)?;
        return Ok(existing);
    }

    let current = host_package_state::Entity::find()
        .filter(host_package_state::Column::HostScope.eq(HOST_SCOPE))
        .filter(host_package_state::Column::PackageId.eq(package_id))
        .lock(LockType::Update)
        .one(&tx)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("功能包状态不存在: {package_id}")))?;
    // Every lifecycle path locks the package row before the operation row. The
    // second lookup closes the race where two retries observed a missing
    // operation before one of them acquired the package lock.
    if let Some(existing) = host_package_operation::Entity::find_by_id(operation_id.to_string())
        .lock(LockType::Update)
        .one(&tx)
        .await
        .map_err(AppError::Database)?
    {
        validate_idempotent_package_operation(
            &existing,
            package_id,
            action,
            requested_version.as_deref(),
        )?;
        tx.commit().await.map_err(AppError::Database)?;
        return Ok(existing);
    }
    if current.status_version != expected_status_version {
        return Err(AppError::InvalidState {
            current: format!("功能包 {package_id} 状态版本为 {}", current.status_version),
            operation: format!("按版本 {expected_status_version} 修改功能包状态"),
        });
    }
    if matches!(
        current.host_state.as_str(),
        HOST_STATE_ENABLING | HOST_STATE_DISABLING
    ) {
        return Err(AppError::InvalidState {
            current: format!("功能包 {package_id} 正在进行生命周期操作"),
            operation: "等待现有功能包生命周期操作完成".to_string(),
        });
    }

    let target_host_state = if action == "enable" {
        HOST_STATE_ENABLED
    } else {
        HOST_STATE_DISABLED
    };
    let already_at_target = current.host_state == target_host_state;
    let now = Utc::now().naive_utc();
    if !already_at_target {
        let next_status_version =
            current
                .status_version
                .checked_add(1)
                .ok_or_else(|| AppError::InvalidState {
                    current: format!("功能包 {package_id} 状态版本已达到上限"),
                    operation: "创建功能包生命周期操作".to_string(),
                })?;
        let mut state: host_package_state::ActiveModel = current.into();
        state.host_state = Set(transitional_state.to_string());
        state.status_version = Set(next_status_version);
        state.updated_at = Set(now);
        state.update(&tx).await.map_err(AppError::Database)?;
    }

    host_package_operation::Entity::insert(host_package_operation::ActiveModel {
        operation_id: Set(operation_id.to_string()),
        host_scope: Set(HOST_SCOPE.to_string()),
        package_id: Set(package_id.to_string()),
        action: Set(action.to_string()),
        state: Set(if already_at_target {
            OPERATION_COMPLETED.to_string()
        } else if action == "disable" {
            OPERATION_DRAINING.to_string()
        } else {
            OPERATION_PENDING.to_string()
        }),
        requested_version: Set(requested_version),
        requested_by_user_id: Set(requested_by_user_id),
        status_version: Set(1),
        reason_code: Set(None),
        error: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    })
    .exec_without_returning(&tx)
    .await
    .map_err(AppError::Database)?;
    tx.commit().await.map_err(AppError::Database)?;
    package_operation(db, operation_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("功能包操作不存在: {operation_id}")))
}

pub async fn update_package_lifecycle_progress(
    db: &DatabaseConnection,
    operation_id: &str,
    expected_operation_status_version: i64,
    operation_state: &str,
    reason_code: Option<&str>,
    error: Option<&str>,
) -> Result<host_package_operation::Model, AppError> {
    if !matches!(
        operation_state,
        OPERATION_PENDING | OPERATION_DRAINING | OPERATION_WAITING_EXTERNAL
    ) {
        return Err(AppError::InvalidArgument(format!(
            "未知功能包非终态: {operation_state}"
        )));
    }
    let tx = db.begin().await.map_err(AppError::Database)?;
    let operation = host_package_operation::Entity::find_by_id(operation_id.to_string())
        .filter(host_package_operation::Column::HostScope.eq(HOST_SCOPE))
        .lock(LockType::Update)
        .one(&tx)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("功能包操作不存在: {operation_id}")))?;
    if matches!(
        operation.state.as_str(),
        OPERATION_COMPLETED | OPERATION_FAILED
    ) {
        tx.commit().await.map_err(AppError::Database)?;
        return Ok(operation);
    }
    if operation.status_version != expected_operation_status_version {
        return Err(AppError::InvalidState {
            current: format!(
                "功能包操作 {} 版本为 {}",
                operation_id, operation.status_version
            ),
            operation: format!("按版本 {expected_operation_status_version} 推进功能包生命周期操作"),
        });
    }
    let next_status_version = expected_operation_status_version
        .checked_add(1)
        .ok_or_else(|| AppError::InvalidState {
            current: format!("功能包操作 {operation_id} 状态版本已达到上限"),
            operation: "推进功能包生命周期操作".to_string(),
        })?;
    let mut active: host_package_operation::ActiveModel = operation.into();
    active.state = Set(operation_state.to_string());
    active.status_version = Set(next_status_version);
    active.reason_code = Set(reason_code.map(str::to_string));
    active.error = Set(error.map(str::to_string));
    active.updated_at = Set(Utc::now().naive_utc());
    let updated = active.update(&tx).await.map_err(AppError::Database)?;
    tx.commit().await.map_err(AppError::Database)?;
    Ok(updated)
}

pub async fn finish_package_lifecycle(
    db: &DatabaseConnection,
    operation_id: &str,
    expected_operation_status_version: i64,
    operation_state: &str,
    final_host_state: &str,
    reason_code: Option<&str>,
    error: Option<&str>,
) -> Result<host_package_operation::Model, AppError> {
    if !matches!(operation_state, OPERATION_COMPLETED | OPERATION_FAILED) {
        return Err(AppError::InvalidArgument(format!(
            "未知功能包终态: {operation_state}"
        )));
    }
    if !matches!(final_host_state, HOST_STATE_ENABLED | HOST_STATE_DISABLED) {
        return Err(AppError::InvalidArgument(format!(
            "未知功能包 Host 终态: {final_host_state}"
        )));
    }
    let tx = db.begin().await.map_err(AppError::Database)?;
    let operation = host_package_operation::Entity::find_by_id(operation_id.to_string())
        .filter(host_package_operation::Column::HostScope.eq(HOST_SCOPE))
        .one(&tx)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("功能包操作不存在: {operation_id}")))?;
    if matches!(
        operation.state.as_str(),
        OPERATION_COMPLETED | OPERATION_FAILED
    ) {
        tx.commit().await.map_err(AppError::Database)?;
        return Ok(operation);
    }

    let current = host_package_state::Entity::find()
        .filter(host_package_state::Column::HostScope.eq(HOST_SCOPE))
        .filter(host_package_state::Column::PackageId.eq(&operation.package_id))
        .lock(LockType::Update)
        .one(&tx)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("功能包状态不存在: {}", operation.package_id)))?;
    let operation = host_package_operation::Entity::find_by_id(operation_id.to_string())
        .filter(host_package_operation::Column::HostScope.eq(HOST_SCOPE))
        .lock(LockType::Update)
        .one(&tx)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("功能包操作不存在: {operation_id}")))?;
    if operation.status_version != expected_operation_status_version {
        return Err(AppError::InvalidState {
            current: format!(
                "功能包操作 {} 版本为 {}",
                operation_id, operation.status_version
            ),
            operation: format!("按版本 {expected_operation_status_version} 完成功能包生命周期操作"),
        });
    }
    if matches!(
        operation.state.as_str(),
        OPERATION_COMPLETED | OPERATION_FAILED
    ) {
        tx.commit().await.map_err(AppError::Database)?;
        return Ok(operation);
    }
    let expected_host_state = match operation.action.as_str() {
        "enable" => HOST_STATE_ENABLING,
        "disable" => HOST_STATE_DISABLING,
        action => {
            return Err(AppError::InvalidState {
                current: format!("未知功能包生命周期动作: {action}"),
                operation: "完成功能包生命周期操作".to_string(),
            });
        }
    };
    if current.host_state != expected_host_state {
        return Err(AppError::InvalidState {
            current: format!(
                "功能包 {} 当前状态为 {}，不是操作要求的 {}",
                operation.package_id, current.host_state, expected_host_state
            ),
            operation: "拒绝覆盖并发功能包生命周期状态".to_string(),
        });
    }
    let next_host_version =
        current
            .status_version
            .checked_add(1)
            .ok_or_else(|| AppError::InvalidState {
                current: format!("功能包 {} 状态版本已达到上限", operation.package_id),
                operation: "完成功能包生命周期操作".to_string(),
            })?;
    let next_operation_version = expected_operation_status_version
        .checked_add(1)
        .ok_or_else(|| AppError::InvalidState {
            current: format!("功能包操作 {operation_id} 状态版本已达到上限"),
            operation: "完成功能包生命周期操作".to_string(),
        })?;
    let mut state: host_package_state::ActiveModel = current.into();
    state.host_state = Set(final_host_state.to_string());
    state.status_version = Set(next_host_version);
    state.updated_at = Set(Utc::now().naive_utc());
    state.update(&tx).await.map_err(AppError::Database)?;

    let mut active: host_package_operation::ActiveModel = operation.into();
    active.state = Set(operation_state.to_string());
    active.status_version = Set(next_operation_version);
    active.reason_code = Set(reason_code.map(str::to_string));
    active.error = Set(error.map(str::to_string));
    active.updated_at = Set(Utc::now().naive_utc());
    let updated = active.update(&tx).await.map_err(AppError::Database)?;
    tx.commit().await.map_err(AppError::Database)?;
    Ok(updated)
}

fn validate_idempotent_package_operation(
    operation: &host_package_operation::Model,
    package_id: &str,
    action: &str,
    requested_version: Option<&str>,
) -> Result<(), AppError> {
    if operation.host_scope != HOST_SCOPE
        || operation.package_id != package_id
        || operation.action != action
        || operation.requested_version.as_deref() != requested_version
    {
        return Err(AppError::InvalidArgument(
            "operation_id 已绑定到不同的功能包生命周期请求".to_string(),
        ));
    }
    Ok(())
}

pub async fn package_preference(
    db: &DatabaseConnection,
    user_id: i64,
) -> Result<Option<user_package_preference::Model>, AppError> {
    user_package_preference::Entity::find()
        .filter(user_package_preference::Column::UserId.eq(user_id))
        .filter(user_package_preference::Column::HostScope.eq(HOST_SCOPE))
        .one(db)
        .await
        .map_err(AppError::Database)
}

pub async fn upsert_package_preference(
    db: &DatabaseConnection,
    user_id: i64,
    hidden_package_ids: Value,
) -> Result<user_package_preference::Model, AppError> {
    user_package_preference::Entity::insert(user_package_preference::ActiveModel {
        user_id: Set(user_id),
        host_scope: Set(HOST_SCOPE.to_string()),
        hidden_package_ids: Set(hidden_package_ids),
        updated_at: Set(Utc::now().naive_utc()),
    })
    .on_conflict(
        OnConflict::columns([
            user_package_preference::Column::UserId,
            user_package_preference::Column::HostScope,
        ])
        .update_columns([
            user_package_preference::Column::HiddenPackageIds,
            user_package_preference::Column::UpdatedAt,
        ])
        .to_owned(),
    )
    .exec_without_returning(db)
    .await
    .map_err(AppError::Database)?;
    package_preference(db, user_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("用户功能包偏好不存在: {user_id}")))
}

#[cfg(test)]
mod workload_tests {
    use super::workload_package_owner;

    #[test]
    fn workload_owner_mapping_is_conservative() {
        assert_eq!(
            workload_package_owner("export_generation", None, None),
            Some("export")
        );
        assert_eq!(
            workload_package_owner("image_generation", Some("manga"), None),
            Some("manga")
        );
        assert_eq!(
            workload_package_owner("image_generation", Some("portrait_generation"), None),
            Some("ai-workflow")
        );
        assert_eq!(
            workload_package_owner("video_generation", None, None),
            Some("video")
        );
        assert_eq!(
            workload_package_owner("llm_call", Some("video_prompt_generation"), None),
            Some("video")
        );
        assert_eq!(
            workload_package_owner("image_generation", None, None),
            Some("ai-workflow")
        );
        assert_eq!(workload_package_owner("unknown", None, None), None);
    }
}
