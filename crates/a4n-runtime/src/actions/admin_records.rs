use crate::AppState;
use crate::actions::auth_records::to_user_summary;
use a4n_core::error::AppError;
use a4n_core::models::ai::{embedding_config, image_gen_config, llm_config};
use a4n_core::models::chapter::chapter;
use a4n_core::models::project::novel_project;
use a4n_core::models::task::task_queue;
use a4n_core::models::user::role::{UserRole, parse_user_role};
use a4n_core::models::user::{invite_code, llm_usage_log, theme_config, user};
use a4n_engine::queue::priority::TaskQueue;
use chrono::{NaiveDateTime, Utc};
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminUserSummary {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub role: String,
    pub oauth_provider: Option<String>,
    pub oauth_subject: Option<String>,
    pub avatar_url: Option<String>,
    pub is_active: bool,
    pub display_name: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub last_login_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageLogSummary {
    pub id: i64,
    pub user_id: i64,
    pub project_id: Option<i64>,
    pub model: String,
    pub operation: String,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
    pub latency_ms: i32,
    pub is_success: bool,
    pub error_message: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskQueueSummary {
    pub id: String,
    pub task_type: String,
    pub priority: String,
    pub status: String,
    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
    pub task_scope: Option<String>,
    pub payload: Option<Value>,
    pub progress: Option<Value>,
    pub error_message: Option<String>,
    pub dag_id: Option<String>,
    pub created_at: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub description: String,
    pub project_type: String,
    pub status: String,
    pub target_chapters: i32,
    pub current_chapter: i32,
    pub total_chapters: Option<i32>,
    pub chapters_per_part: Option<i32>,
    pub avatar_animal: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateUserInput {
    pub username: String,
    pub email: String,
    pub password: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InviteCodeSummary {
    pub id: i64,
    pub code: String,
    pub created_by_user_id: Option<i64>,
    pub used_by_user_id: Option<i64>,
    pub created_at: Option<String>,
    pub used_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateInviteCodeInput {
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateUserRoleInput {
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UsageTrendsInput {
    pub days: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyUsagePoint {
    pub date: String,
    pub total_calls: usize,
    pub total_tokens: i64,
    pub success_count: usize,
    pub fail_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeInfo {
    pub id: i64,
    pub name: String,
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminOverviewSummary {
    pub total_users: u64,
    pub total_projects: u64,
    pub total_chapters: u64,
    pub task_queue_count: u64,
    pub pending_tasks: u64,
    pub running_tasks: u64,
    pub total_llm_calls: u64,
    pub llm_config_count: u64,
    pub embedding_config_count: u64,
    pub image_gen_config_count: u64,
    pub active_theme: Option<ThemeInfo>,
    pub daily_active_users: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AdminPermission {
    ManageUsers,
    ManageSettings,
}

pub(crate) async fn require_admin_permission(
    state: &AppState,
    caller_user_id: i64,
    permission: AdminPermission,
) -> Result<(), AppError> {
    let user = state.auth.get(caller_user_id).await?;
    if !user.is_active {
        return Err(AppError::Forbidden(format!("用户 {caller_user_id} 已停用")));
    }
    require_admin_role(&user.role, permission)
}

pub(crate) async fn is_active_admin_user(state: &AppState, user_id: i64) -> Result<bool, AppError> {
    let user = state.auth.get(user_id).await?;
    if !user.is_active {
        return Ok(false);
    }
    Ok(parse_user_role(&user.role)? == UserRole::Admin)
}

fn require_admin_role(role: &str, permission: AdminPermission) -> Result<(), AppError> {
    let role = parse_user_role(role)?;
    if role == UserRole::Admin {
        Ok(())
    } else {
        Err(AppError::Forbidden(format!(
            "角色 {role:?} 无权执行操作 {permission:?}"
        )))
    }
}

macro_rules! admin_record_command {
    (
        $name:ident($($arg:ident: $arg_ty:ty),* $(,)?) -> $output:ty,
        $permission:expr,
        |$state:ident, $caller_user_id:ident| $run:expr $(,)?
    ) => {
        pub async fn $name(
            $state: &AppState,
            $caller_user_id: i64,
            $($arg: $arg_ty,)*
        ) -> Result<$output, AppError> {
            require_admin_permission($state, $caller_user_id, $permission).await?;
            $run.await.map_err(AppError::InvalidArgument)
        }
    };
}

admin_record_command!(
    get_user_usage_for_admin(user_id: i64, limit: u64) -> Vec<UsageLogSummary>,
    AdminPermission::ManageUsers,
    |state, caller_user_id| get_user_usage(state, user_id, limit),
);

pub async fn get_user_usage(
    state: &AppState,
    user_id: i64,
    limit: u64,
) -> Result<Vec<UsageLogSummary>, String> {
    validate_positive_i64(user_id, "user_id")?;
    validate_positive_u64(limit, "limit")?;
    let logs = state
        .admin
        .usage_by_user(user_id, limit)
        .await
        .map_err(|e| e.to_string())?;
    Ok(logs.iter().map(usage_log_summary).collect())
}

admin_record_command!(
    get_token_summary_for_admin(user_id: i64) -> Value,
    AdminPermission::ManageUsers,
    |state, caller_user_id| get_token_summary(state, user_id),
);

pub async fn get_token_summary(state: &AppState, user_id: i64) -> Result<Value, String> {
    validate_positive_i64(user_id, "user_id")?;
    state
        .admin
        .token_summary(user_id)
        .await
        .map_err(|e| e.to_string())
}

admin_record_command!(
    list_pending_tasks_for_admin() -> Vec<TaskQueueSummary>,
    AdminPermission::ManageSettings,
    |state, caller_user_id| list_pending_tasks(state),
);

pub async fn list_pending_tasks(state: &AppState) -> Result<Vec<TaskQueueSummary>, String> {
    let tasks = state
        .admin
        .pending_tasks()
        .await
        .map_err(|e| e.to_string())?;
    Ok(tasks.iter().map(task_summary).collect())
}

admin_record_command!(
    cancel_admin_task_for_admin(task_id: &str) -> (),
    AdminPermission::ManageSettings,
    |state, caller_user_id| cancel_admin_task(state, task_id),
);

pub async fn cancel_admin_task(state: &AppState, task_id: &str) -> Result<(), String> {
    state
        .task_queue
        .cancel(task_id)
        .await
        .map_err(|e| e.to_string())
}

admin_record_command!(
    health_check_for_admin() -> Value,
    AdminPermission::ManageSettings,
    |state, caller_user_id| health_check(state),
);

pub async fn health_check(state: &AppState) -> Result<Value, String> {
    state.admin.health_check().await.map_err(|e| e.to_string())
}

admin_record_command!(
    list_all_users_for_admin() -> Vec<AdminUserSummary>,
    AdminPermission::ManageUsers,
    |state, caller_user_id| list_all_users(state),
);

pub async fn list_all_users(state: &AppState) -> Result<Vec<AdminUserSummary>, String> {
    let users = state.auth.list_users().await.map_err(|e| e.to_string())?;
    Ok(users.iter().map(admin_user_summary).collect())
}

admin_record_command!(
    create_user_for_admin(input: CreateUserInput) -> AdminUserSummary,
    AdminPermission::ManageUsers,
    |state, caller_user_id| create_user(state, input),
);

pub async fn create_user(
    state: &AppState,
    input: CreateUserInput,
) -> Result<AdminUserSummary, String> {
    let user = state
        .auth
        .create_user(&input.username, &input.email, &input.password, &input.role)
        .await
        .map_err(|e| e.to_string())?;
    Ok(admin_user_summary(&user))
}

admin_record_command!(
    list_invite_codes_for_admin() -> Vec<InviteCodeSummary>,
    AdminPermission::ManageUsers,
    |state, caller_user_id| list_invite_codes(state),
);

pub async fn list_invite_codes(state: &AppState) -> Result<Vec<InviteCodeSummary>, String> {
    let codes = state
        .auth
        .list_invite_codes()
        .await
        .map_err(|e| e.to_string())?;
    Ok(codes.iter().map(invite_code_summary).collect())
}

admin_record_command!(
    create_invite_code_for_admin(input: CreateInviteCodeInput) -> InviteCodeSummary,
    AdminPermission::ManageUsers,
    |state, caller_user_id| create_invite_code(state, caller_user_id, input),
);

pub async fn create_invite_code(
    state: &AppState,
    caller_user_id: i64,
    input: CreateInviteCodeInput,
) -> Result<InviteCodeSummary, String> {
    let code = state
        .auth
        .create_invite_code(&input.code, Some(caller_user_id))
        .await
        .map_err(|e| e.to_string())?;
    Ok(invite_code_summary(&code))
}

admin_record_command!(
    update_user_role_for_admin(user_id: i64, input: UpdateUserRoleInput) -> AdminUserSummary,
    AdminPermission::ManageUsers,
    |state, caller_user_id| update_user_role(state, user_id, input),
);

pub async fn update_user_role(
    state: &AppState,
    user_id: i64,
    input: UpdateUserRoleInput,
) -> Result<AdminUserSummary, String> {
    let user = state
        .auth
        .update_user_role(user_id, &input.role)
        .await
        .map_err(|e| e.to_string())?;
    Ok(admin_user_summary(&user))
}

admin_record_command!(
    delete_user_for_admin(user_id: i64) -> (),
    AdminPermission::ManageUsers,
    |state, caller_user_id| delete_user(state, caller_user_id, user_id),
);

pub async fn delete_user(
    state: &AppState,
    caller_user_id: i64,
    user_id: i64,
) -> Result<(), String> {
    state
        .auth
        .disable_user(caller_user_id, user_id)
        .await
        .map_err(|e| e.to_string())
}

admin_record_command!(
    list_all_projects_for_admin() -> Vec<ProjectSummary>,
    AdminPermission::ManageUsers,
    |state, caller_user_id| list_all_projects(state),
);

pub async fn list_all_projects(state: &AppState) -> Result<Vec<ProjectSummary>, String> {
    let projects = novel_project::Entity::find()
        .order_by_desc(novel_project::Column::CreatedAt)
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    Ok(projects.iter().map(project_summary).collect())
}

admin_record_command!(
    get_usage_trends_for_admin(input: UsageTrendsInput) -> Vec<DailyUsagePoint>,
    AdminPermission::ManageUsers,
    |state, caller_user_id| get_usage_trends(state, input),
);

pub async fn get_usage_trends(
    state: &AppState,
    input: UsageTrendsInput,
) -> Result<Vec<DailyUsagePoint>, String> {
    let since = usage_trends_since(Utc::now().naive_utc(), input.days)?;
    let logs = llm_usage_log::Entity::find()
        .filter(llm_usage_log::Column::CreatedAt.gte(since))
        .order_by_asc(llm_usage_log::Column::CreatedAt)
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    let mut daily: BTreeMap<String, DailyUsagePoint> = BTreeMap::new();
    for log in &logs {
        let date = log.created_at.format("%Y-%m-%d").to_string();
        let entry = daily
            .entry(date.clone())
            .or_insert_with(|| DailyUsagePoint {
                date: String::new(),
                total_calls: 0,
                total_tokens: 0,
                success_count: 0,
                fail_count: 0,
            });
        entry.date = date;
        entry.total_calls += 1;
        entry.total_tokens += log.total_tokens as i64;
        if log.is_success {
            entry.success_count += 1;
        } else {
            entry.fail_count += 1;
        }
    }

    Ok(daily.into_values().collect())
}

admin_record_command!(
    get_admin_overview_for_admin() -> AdminOverviewSummary,
    AdminPermission::ManageSettings,
    |state, caller_user_id| get_admin_overview(state),
);

pub async fn get_admin_overview(state: &AppState) -> Result<AdminOverviewSummary, String> {
    admin_overview_summary(state.db.as_ref()).await
}

fn usage_log_summary(log: &llm_usage_log::Model) -> UsageLogSummary {
    UsageLogSummary {
        id: log.id,
        user_id: log.user_id,
        project_id: log.project_id,
        model: log.model.clone(),
        operation: log.operation.clone(),
        prompt_tokens: log.prompt_tokens,
        completion_tokens: log.completion_tokens,
        total_tokens: log.total_tokens,
        latency_ms: log.latency_ms,
        is_success: log.is_success,
        error_message: log.error_message.clone(),
        created_at: Some(log.created_at.to_string()),
    }
}

fn task_summary(task: &task_queue::Model) -> TaskQueueSummary {
    TaskQueueSummary {
        id: task.id.clone(),
        task_type: task.task_type.clone(),
        priority: task.priority.clone(),
        status: task.status.clone(),
        project_id: task.project_id,
        root_project_id: task.root_project_id,
        source_snapshot_id: task.source_snapshot_id,
        task_scope: task.task_scope.clone(),
        payload: task.payload.clone(),
        progress: task.progress.clone(),
        error_message: task.error_message.clone(),
        dag_id: task.dag_id.clone(),
        created_at: Some(task.created_at.to_string()),
        started_at: task.started_at.map(|value| value.to_string()),
        completed_at: task.completed_at.map(|value| value.to_string()),
    }
}

fn project_summary(project: &novel_project::Model) -> ProjectSummary {
    ProjectSummary {
        id: project.id,
        user_id: project.user_id,
        title: project.title.clone(),
        description: project.description.clone(),
        project_type: project.project_type.clone(),
        status: project.status.clone(),
        target_chapters: project.target_chapters,
        current_chapter: project.current_chapter,
        total_chapters: project.total_chapters,
        chapters_per_part: project.chapters_per_part,
        avatar_animal: project.avatar_animal.clone(),
        created_at: Some(project.created_at.to_string()),
        updated_at: Some(project.updated_at.to_string()),
    }
}

fn admin_user_summary(user: &user::Model) -> AdminUserSummary {
    let base = to_user_summary(user).with_display_name();
    AdminUserSummary {
        id: base.id,
        username: base.username,
        email: base.email,
        role: base.role,
        oauth_provider: base.oauth_provider,
        oauth_subject: base.oauth_subject,
        avatar_url: base.avatar_url,
        is_active: base.is_active,
        display_name: base.display_name,
        created_at: base.created_at,
        updated_at: base.updated_at,
        last_login_at: base.last_login_at,
    }
}

fn invite_code_summary(code: &invite_code::Model) -> InviteCodeSummary {
    InviteCodeSummary {
        id: code.id,
        code: code.code.clone(),
        created_by_user_id: code.created_by_user_id,
        used_by_user_id: code.used_by_user_id,
        created_at: Some(code.created_at.to_string()),
        used_at: code.used_at.map(|value| value.to_string()),
    }
}

async fn admin_overview_summary(db: &DatabaseConnection) -> Result<AdminOverviewSummary, String> {
    macro_rules! count_rows {
        ($query:expr) => {
            $query.count(db).await.map_err(|e| e.to_string())?
        };
    }

    let total_users = count_rows!(user::Entity::find());
    let total_projects = count_rows!(novel_project::Entity::find());
    let total_chapters = count_rows!(chapter::Entity::find());
    let task_queue_count = count_rows!(task_queue::Entity::find());
    let pending_tasks =
        count_rows!(task_queue::Entity::find().filter(task_queue::Column::Status.eq("pending")));
    let running_tasks =
        count_rows!(task_queue::Entity::find().filter(task_queue::Column::Status.eq("running")));
    let total_llm_calls = count_rows!(llm_usage_log::Entity::find());
    let llm_config_count = count_rows!(llm_config::Entity::find());
    let embedding_config_count = count_rows!(embedding_config::Entity::find());
    let image_gen_config_count = count_rows!(image_gen_config::Entity::find());

    Ok(AdminOverviewSummary {
        total_users,
        total_projects,
        total_chapters,
        task_queue_count,
        pending_tasks,
        running_tasks,
        total_llm_calls,
        llm_config_count,
        embedding_config_count,
        image_gen_config_count,
        active_theme: find_active_global_theme(db).await?,
        daily_active_users: daily_active_users(db).await?,
    })
}

async fn daily_active_users(db: &DatabaseConnection) -> Result<u64, String> {
    let since = Utc::now().naive_utc() - chrono::Duration::hours(24);
    let recent_logs = llm_usage_log::Entity::find()
        .filter(llm_usage_log::Column::CreatedAt.gte(since))
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    let active_ids: HashSet<i64> = recent_logs.into_iter().map(|log| log.user_id).collect();
    Ok(active_ids.len() as u64)
}

async fn find_active_global_theme(db: &DatabaseConnection) -> Result<Option<ThemeInfo>, String> {
    theme_config::Entity::find()
        .filter(theme_config::Column::UserId.is_null())
        .filter(theme_config::Column::IsActive.eq(true))
        .one(db)
        .await
        .map_err(|e| e.to_string())
        .map(|theme| {
            theme.map(|theme| ThemeInfo {
                id: theme.id,
                name: theme.name,
                mode: theme.mode,
            })
        })
}

fn validate_positive_u64(value: u64, field: &str) -> Result<(), String> {
    if value == 0 {
        Err(format!("{field} 必须大于 0"))
    } else {
        Ok(())
    }
}

fn validate_positive_i64(value: i64, field: &str) -> Result<(), String> {
    if value <= 0 {
        Err(format!("{field} 必须大于 0"))
    } else {
        Ok(())
    }
}

fn usage_trends_since(now: NaiveDateTime, days: u64) -> Result<NaiveDateTime, String> {
    validate_positive_u64(days, "days")?;
    let days = i64::try_from(days).map_err(|_| "days 超出可支持范围".to_string())?;
    let duration =
        chrono::Duration::try_days(days).ok_or_else(|| "days 超出可支持范围".to_string())?;
    now.checked_sub_signed(duration)
        .ok_or_else(|| "days 超出可支持范围".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use sea_orm::{ActiveModelTrait, Set};

    async fn setup_db() -> DatabaseConnection {
        a4n_migration::test_support::connect_test_database().await
    }

    #[test]
    fn positive_u64_validation_rejects_zero() {
        assert!(validate_positive_u64(1, "limit").is_ok());
        let err = validate_positive_u64(0, "limit").expect_err("zero should fail");
        assert_eq!(err, "limit 必须大于 0");
    }

    #[test]
    fn positive_i64_validation_rejects_zero_and_negative_ids() {
        assert!(validate_positive_i64(1, "user_id").is_ok());
        assert_eq!(
            validate_positive_i64(0, "user_id").expect_err("zero id should fail"),
            "user_id 必须大于 0"
        );
        assert_eq!(
            validate_positive_i64(-7, "project_id").expect_err("negative id should fail"),
            "project_id 必须大于 0"
        );
    }

    #[test]
    fn usage_trends_since_rejects_zero_days() {
        let now = NaiveDate::from_ymd_opt(2026, 6, 5)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        let err = usage_trends_since(now, 0).expect_err("zero days must be rejected");
        assert_eq!(err, "days 必须大于 0");
    }

    #[test]
    fn usage_trends_since_rejects_u64_to_i64_overflow() {
        let now = NaiveDate::from_ymd_opt(2026, 6, 5)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        let err = usage_trends_since(now, i64::MAX as u64 + 1)
            .expect_err("u64 days above i64::MAX must not wrap to a negative duration");
        assert_eq!(err, "days 超出可支持范围");
    }

    #[test]
    fn usage_trends_since_subtracts_requested_days() {
        let now = NaiveDate::from_ymd_opt(2026, 6, 5)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        let expected = NaiveDate::from_ymd_opt(2026, 5, 29)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        assert_eq!(usage_trends_since(now, 7).unwrap(), expected);
    }

    async fn insert_theme(
        db: &DatabaseConnection,
        name: &str,
        user_id: Option<i64>,
        is_system_default: bool,
        is_active: bool,
    ) -> theme_config::Model {
        let now = NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        theme_config::ActiveModel {
            user_id: Set(user_id),
            name: Set(name.to_string()),
            mode: Set("dark".to_string()),
            css_variables: Set(serde_json::json!({})),
            is_system_default: Set(is_system_default),
            is_active: Set(is_active),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap()
    }

    async fn insert_task(db: &DatabaseConnection, id: &str) {
        let now = NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        task_queue::Entity::insert(task_queue::ActiveModel {
            id: Set(id.to_string()),
            task_type: Set("llm_call".to_string()),
            priority: Set("medium".to_string()),
            status: Set("pending".to_string()),
            project_id: Set(None),
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
            completed_at: Set(None),
        })
        .exec(db)
        .await
        .unwrap();
    }

    async fn insert_llm_usage_log(db: &DatabaseConnection, id: i64) {
        let now = Utc::now().naive_utc();
        llm_usage_log::Entity::insert(llm_usage_log::ActiveModel {
            id: Set(id),
            user_id: Set(1),
            project_id: Set(None),
            model: Set("gpt-4o".to_string()),
            operation: Set("chat".to_string()),
            prompt_tokens: Set(10),
            completion_tokens: Set(20),
            total_tokens: Set(30),
            latency_ms: Set(100),
            is_success: Set(true),
            error_message: Set(None),
            created_at: Set(now),
        })
        .exec(db)
        .await
        .unwrap();
    }

    #[test]
    fn require_admin_role_allows_admin_for_all_admin_permissions() {
        require_admin_role("admin", AdminPermission::ManageUsers)
            .expect("admin should manage users");
        require_admin_role("admin", AdminPermission::ManageSettings)
            .expect("admin should manage settings");
    }

    #[test]
    fn require_admin_role_rejects_non_admin_roles() {
        let err = require_admin_role("user", AdminPermission::ManageUsers)
            .expect_err("non-admin role should be rejected");

        assert!(matches!(err, AppError::Forbidden(_)));
        assert!(err.to_string().contains("ManageUsers"));
    }

    #[test]
    fn require_admin_role_rejects_unknown_role_values() {
        let err = require_admin_role("owner", AdminPermission::ManageSettings)
            .expect_err("unknown role should be rejected");

        assert!(matches!(err, AppError::InvalidArgument(_)));
    }

    #[tokio::test]
    #[ignore = "requires A4N_TEST_DATABASE_URL PostgreSQL test database"]
    async fn active_theme_summary_uses_active_global_theme_not_system_default() {
        let db = setup_db().await;
        let system_default =
            insert_theme(&db, "system-default-but-inactive", None, true, false).await;
        let active_global = insert_theme(&db, "active-global", None, false, true).await;
        let active_user = insert_theme(&db, "active-user", Some(7), false, true).await;

        let summary = find_active_global_theme(&db)
            .await
            .expect("active theme query should succeed")
            .expect("global active theme should exist");

        assert_eq!(summary.id, active_global.id);
        assert_ne!(summary.id, system_default.id);
        assert_ne!(summary.id, active_user.id);
    }

    #[tokio::test]
    #[ignore = "requires A4N_TEST_DATABASE_URL PostgreSQL test database"]
    async fn admin_overview_reports_task_queue_and_real_llm_usage_counts_separately() {
        let db = setup_db().await;
        insert_task(&db, "task-a").await;
        insert_task(&db, "task-b").await;
        insert_llm_usage_log(&db, 1).await;
        let active_theme = insert_theme(&db, "active-global", None, false, true).await;

        let overview = admin_overview_summary(&db)
            .await
            .expect("admin overview should be computed");

        assert_eq!(overview.task_queue_count, 2);
        assert_eq!(overview.total_llm_calls, 1);
        assert_eq!(
            overview.active_theme.as_ref().map(|theme| theme.id),
            Some(active_theme.id)
        );
        assert_eq!(
            overview
                .active_theme
                .as_ref()
                .map(|theme| theme.name.as_str()),
            Some("active-global")
        );
        assert_eq!(overview.daily_active_users, 1);
    }
}
