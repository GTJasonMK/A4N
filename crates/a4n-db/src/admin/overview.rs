use std::collections::HashSet;

use a4n_core::models::ai::{embedding_config, image_gen_config, llm_config};
use a4n_core::models::chapter::chapter;
use a4n_core::models::project::novel_project;
use a4n_core::models::task::task_queue;
use a4n_core::models::user::{llm_usage_log, theme_config, user};
use chrono::Utc;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
};
use serde_json::{Value, json};

use super::{AdminDbError, AdminOverviewSummary, ThemeInfo};

pub async fn database_health(db: &DatabaseConnection) -> Result<Value, AdminDbError> {
    db.execute(sea_orm::Statement::from_string(
        db.get_database_backend(),
        "SELECT 1",
    ))
    .await?;

    Ok(json!({
        "database": true,
        "backend": format!("{:?}", db.get_database_backend()),
    }))
}

pub async fn admin_overview_summary(
    db: &DatabaseConnection,
) -> Result<AdminOverviewSummary, AdminDbError> {
    macro_rules! count_rows {
        ($query:expr) => {
            $query.count(db).await?
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

async fn daily_active_users(db: &DatabaseConnection) -> Result<u64, AdminDbError> {
    let since = Utc::now().naive_utc() - chrono::Duration::hours(24);
    let recent_logs = llm_usage_log::Entity::find()
        .filter(llm_usage_log::Column::CreatedAt.gte(since))
        .all(db)
        .await?;
    let active_ids: HashSet<i64> = recent_logs.into_iter().map(|log| log.user_id).collect();
    Ok(active_ids.len() as u64)
}

pub(super) async fn find_active_global_theme(
    db: &DatabaseConnection,
) -> Result<Option<ThemeInfo>, AdminDbError> {
    let theme = theme_config::Entity::find()
        .filter(theme_config::Column::UserId.is_null())
        .filter(theme_config::Column::IsActive.eq(true))
        .one(db)
        .await?;
    theme
        .map(|theme| {
            let mode = theme.mode.parse().map_err(|error| {
                AdminDbError::InvalidArgument(format!(
                    "主题 {} 的持久化 mode 无效：{error}",
                    theme.id
                ))
            })?;
            Ok(ThemeInfo {
                id: theme.id,
                name: theme.name,
                mode,
            })
        })
        .transpose()
}
