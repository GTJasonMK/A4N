use a4n_db_models::models::ai::{embedding_config, image_gen_config, llm_config};
use a4n_db_models::models::chapter::chapter;
use a4n_db_models::models::project::novel_project;
use a4n_db_models::models::task::task_queue;
use a4n_db_models::models::user::{llm_usage_log, theme_config, user};
use chrono::Utc;
use sea_orm::sea_query::Expr;
use sea_orm::{
    ColumnTrait, EntityTrait, FromQueryResult, PaginatorTrait, QueryFilter, QuerySelect,
};

use super::{AdminDbError, AdminOverviewSummary, ThemeInfo};

pub async fn check_database_health(db: &impl crate::DbExecutor) -> Result<(), AdminDbError> {
    db.execute(sea_orm::Statement::from_string(
        db.get_database_backend(),
        "SELECT 1",
    ))
    .await?;

    Ok(())
}

pub async fn admin_overview_summary(
    db: &impl crate::DbExecutor,
) -> Result<AdminOverviewSummary, AdminDbError> {
    macro_rules! count_rows {
        ($query:expr) => {
            async { $query.count(db).await.map_err(AdminDbError::from) }
        };
    }

    let (
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
        active_theme,
        daily_active_users,
    ) = futures::try_join!(
        count_rows!(user::Entity::find()),
        count_rows!(novel_project::Entity::find()),
        count_rows!(chapter::Entity::find_current()),
        count_rows!(task_queue::Entity::find()),
        count_rows!(task_queue::Entity::find().filter(task_queue::Column::Status.eq("pending"))),
        count_rows!(task_queue::Entity::find().filter(task_queue::Column::Status.eq("running"))),
        count_rows!(llm_usage_log::Entity::find()),
        count_rows!(llm_config::Entity::find()),
        count_rows!(embedding_config::Entity::find()),
        count_rows!(image_gen_config::Entity::find()),
        find_active_global_theme(db),
        daily_active_users(db),
    )?;

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
        active_theme,
        daily_active_users,
    })
}

async fn daily_active_users(db: &impl crate::DbExecutor) -> Result<u64, AdminDbError> {
    let since = Utc::now().naive_utc() - chrono::Duration::hours(24);
    #[derive(Debug, FromQueryResult)]
    struct ActiveUserCount {
        count: i64,
    }
    let count = llm_usage_log::Entity::find()
        .select_only()
        .column_as(Expr::cust("COUNT(DISTINCT user_id)"), "count")
        .filter(llm_usage_log::Column::CreatedAt.gte(since))
        .into_model::<ActiveUserCount>()
        .one(db)
        .await?
        .ok_or_else(|| AdminDbError::InvalidArgument("活跃用户统计查询未返回结果".to_string()))?
        .count;
    u64::try_from(count)
        .map_err(|_| AdminDbError::InvalidArgument("活跃用户数超出 u64 上限".to_string()))
}

pub(super) async fn find_active_global_theme(
    db: &impl crate::DbExecutor,
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
