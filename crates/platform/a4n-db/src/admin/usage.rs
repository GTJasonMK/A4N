use a4n_db_models::models::user::llm_usage_log;
use chrono::{NaiveDateTime, Utc};
use sea_orm::{
    ColumnTrait, EntityTrait, FromQueryResult, QueryFilter, QueryOrder, QuerySelect,
    sea_query::Expr,
};
use serde_json::{Value, json};

use super::{AdminDbError, DailyUsagePoint, UsageLogSummary, UsageTrendsInput};

#[derive(Debug, FromQueryResult)]
struct DailyUsageRow {
    day: NaiveDateTime,
    total_calls: i64,
    total_tokens: i64,
    success_count: i64,
    fail_count: i64,
}

#[derive(Debug, FromQueryResult)]
struct TokenSummaryRow {
    total_prompt_tokens: i64,
    total_completion_tokens: i64,
    total_tokens: i64,
    total_calls: i64,
    success_count: i64,
    fail_count: i64,
}

pub async fn usage_trends(
    db: &impl crate::DbExecutor,
    input: UsageTrendsInput,
) -> Result<Vec<DailyUsagePoint>, AdminDbError> {
    let since = usage_trends_since(Utc::now().naive_utc(), input.days)?;
    let day_expr = Expr::cust("DATE_TRUNC('day', created_at)");
    let rows = llm_usage_log::Entity::find()
        .select_only()
        .column_as(day_expr.clone(), "day")
        .column_as(Expr::cust("COUNT(*)"), "total_calls")
        .column_as(
            Expr::cust("COALESCE(SUM(total_tokens), 0)::bigint"),
            "total_tokens",
        )
        .column_as(
            Expr::cust("COUNT(*) FILTER (WHERE is_success)"),
            "success_count",
        )
        .column_as(
            Expr::cust("COUNT(*) FILTER (WHERE NOT is_success)"),
            "fail_count",
        )
        .filter(llm_usage_log::Column::CreatedAt.gte(since))
        .group_by(day_expr)
        .order_by_asc(Expr::cust("day"))
        .into_model::<DailyUsageRow>()
        .all(db)
        .await?;
    rows.into_iter()
        .map(|row| {
            Ok(DailyUsagePoint {
                date: row.day.format("%Y-%m-%d").to_string(),
                total_calls: count_to_usize(row.total_calls, "total_calls")?,
                total_tokens: row.total_tokens,
                success_count: count_to_usize(row.success_count, "success_count")?,
                fail_count: count_to_usize(row.fail_count, "fail_count")?,
            })
        })
        .collect()
}

pub async fn usage_by_user(
    db: &impl crate::DbExecutor,
    user_id: i64,
    limit: u64,
) -> Result<Vec<UsageLogSummary>, AdminDbError> {
    validate_positive_i64(user_id, "user_id")?;
    validate_positive_u64(limit, "limit")?;
    let logs = llm_usage_log::Entity::find()
        .filter(llm_usage_log::Column::UserId.eq(user_id))
        .order_by_desc(llm_usage_log::Column::CreatedAt)
        .limit(limit)
        .all(db)
        .await?;
    Ok(logs.iter().map(usage_log_summary).collect())
}

pub async fn token_summary(
    db: &impl crate::DbExecutor,
    user_id: i64,
) -> Result<Value, AdminDbError> {
    validate_positive_i64(user_id, "user_id")?;
    let row = llm_usage_log::Entity::find()
        .select_only()
        .column_as(
            Expr::cust("COALESCE(SUM(prompt_tokens), 0)::bigint"),
            "total_prompt_tokens",
        )
        .column_as(
            Expr::cust("COALESCE(SUM(completion_tokens), 0)::bigint"),
            "total_completion_tokens",
        )
        .column_as(
            Expr::cust("COALESCE(SUM(total_tokens), 0)::bigint"),
            "total_tokens",
        )
        .column_as(Expr::cust("COUNT(*)"), "total_calls")
        .column_as(
            Expr::cust("COUNT(*) FILTER (WHERE is_success)"),
            "success_count",
        )
        .column_as(
            Expr::cust("COUNT(*) FILTER (WHERE NOT is_success)"),
            "fail_count",
        )
        .filter(llm_usage_log::Column::UserId.eq(user_id))
        .into_model::<TokenSummaryRow>()
        .one(db)
        .await?
        .ok_or_else(|| AdminDbError::InvalidArgument("token 汇总查询未返回结果".to_string()))?;

    Ok(json!({
        "total_calls": count_to_usize(row.total_calls, "total_calls")?,
        "total_prompt_tokens": row.total_prompt_tokens,
        "total_completion_tokens": row.total_completion_tokens,
        "total_tokens": row.total_tokens,
        "success_count": count_to_usize(row.success_count, "success_count")?,
        "fail_count": count_to_usize(row.fail_count, "fail_count")?,
    }))
}

fn count_to_usize(value: i64, field: &str) -> Result<usize, AdminDbError> {
    usize::try_from(value)
        .map_err(|_| AdminDbError::InvalidArgument(format!("{field} 超出当前平台 usize 上限")))
}

pub(super) fn usage_trends_since(
    now: NaiveDateTime,
    days: u64,
) -> Result<NaiveDateTime, AdminDbError> {
    validate_positive_u64(days, "days")?;
    let days = i64::try_from(days)
        .map_err(|_| AdminDbError::InvalidArgument("days 超出可支持范围".to_string()))?;
    let duration = chrono::Duration::try_days(days)
        .ok_or_else(|| AdminDbError::InvalidArgument("days 超出可支持范围".to_string()))?;
    now.checked_sub_signed(duration)
        .ok_or_else(|| AdminDbError::InvalidArgument("days 超出可支持范围".to_string()))
}

fn validate_positive_u64(value: u64, field: &str) -> Result<(), AdminDbError> {
    if value == 0 {
        Err(AdminDbError::InvalidArgument(format!("{field} 必须大于 0")))
    } else {
        Ok(())
    }
}

pub(super) fn validate_positive_i64(value: i64, field: &str) -> Result<(), AdminDbError> {
    if value <= 0 {
        Err(AdminDbError::InvalidArgument(format!("{field} 必须大于 0")))
    } else {
        Ok(())
    }
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
        created_at: Some(log.created_at.and_utc().to_rfc3339()),
    }
}
