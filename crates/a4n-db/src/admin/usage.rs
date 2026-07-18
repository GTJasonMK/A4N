use std::collections::BTreeMap;

use a4n_core::models::user::llm_usage_log;
use chrono::{NaiveDateTime, Utc};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use serde_json::{Value, json};

use super::{AdminDbError, DailyUsagePoint, UsageLogSummary, UsageTrendsInput};

pub async fn usage_trends(
    db: &DatabaseConnection,
    input: UsageTrendsInput,
) -> Result<Vec<DailyUsagePoint>, AdminDbError> {
    let since = usage_trends_since(Utc::now().naive_utc(), input.days)?;
    let logs = llm_usage_log::Entity::find()
        .filter(llm_usage_log::Column::CreatedAt.gte(since))
        .order_by_asc(llm_usage_log::Column::CreatedAt)
        .all(db)
        .await?;

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

pub async fn usage_by_user(
    db: &DatabaseConnection,
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

pub async fn token_summary(db: &DatabaseConnection, user_id: i64) -> Result<Value, AdminDbError> {
    validate_positive_i64(user_id, "user_id")?;
    let logs = llm_usage_log::Entity::find()
        .filter(llm_usage_log::Column::UserId.eq(user_id))
        .all(db)
        .await?;

    let total_prompt: i64 = logs.iter().map(|log| i64::from(log.prompt_tokens)).sum();
    let total_completion: i64 = logs
        .iter()
        .map(|log| i64::from(log.completion_tokens))
        .sum();
    let total_tokens = total_prompt + total_completion;
    let success_count = logs.iter().filter(|log| log.is_success).count();
    let fail_count = logs.len() - success_count;

    Ok(json!({
        "total_calls": logs.len(),
        "total_prompt_tokens": total_prompt,
        "total_completion_tokens": total_completion,
        "total_tokens": total_tokens,
        "success_count": success_count,
        "fail_count": fail_count,
    }))
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
        created_at: Some(log.created_at.to_string()),
    }
}
