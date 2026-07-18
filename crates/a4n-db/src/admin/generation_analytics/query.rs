use super::*;

use super::aggregate::build_generation_call_analytics;

const DEFAULT_DAYS: u32 = 30;
pub(super) const MAX_DAYS: u32 = 180;
const DEFAULT_RECENT_LIMIT: u64 = 40;
const MAX_RECENT_LIMIT: u64 = 200;

pub async fn generation_call_analytics(
    db: &DatabaseConnection,
    input: GenerationCallAnalyticsInput,
) -> Result<GenerationCallAnalytics, AdminDbError> {
    let (days, recent_limit) = validate_generation_call_analytics_input(input)?;

    let now = Utc::now().naive_utc();
    let since = now - Duration::days(i64::from(days));

    let llm_calls = llm_call_trace::Entity::find()
        .filter(llm_call_trace::Column::CreatedAt.gte(since))
        .order_by_desc(llm_call_trace::Column::CreatedAt)
        .all(db)
        .await
        .map_err(|source| AdminDbError::DatabaseOperation {
            operation: "读取 LLM 调用分析失败",
            source,
        })?;

    let media_runs = generation_run::Entity::find()
        .filter(generation_run::Column::CreatedAt.gte(since))
        .filter(generation_run::Column::ModelKind.is_in(["image", "video"]))
        .order_by_desc(generation_run::Column::CreatedAt)
        .all(db)
        .await
        .map_err(|source| AdminDbError::DatabaseOperation {
            operation: "读取媒体生成分析失败",
            source,
        })?;

    Ok(build_generation_call_analytics(
        now,
        since,
        days,
        recent_limit,
        llm_calls,
        media_runs,
    ))
}

pub(super) fn validate_generation_call_analytics_input(
    input: GenerationCallAnalyticsInput,
) -> Result<(u32, u64), AdminDbError> {
    let days = input.days.unwrap_or(DEFAULT_DAYS);
    if days == 0 || days > MAX_DAYS {
        return Err(AdminDbError::InvalidArgument(format!(
            "days 必须在 1..={MAX_DAYS} 之间"
        )));
    }
    let recent_limit = input.recent_limit.unwrap_or(DEFAULT_RECENT_LIMIT);
    if recent_limit == 0 || recent_limit > MAX_RECENT_LIMIT {
        return Err(AdminDbError::InvalidArgument(format!(
            "recent_limit 必须在 1..={MAX_RECENT_LIMIT} 之间"
        )));
    }
    Ok((days, recent_limit))
}
