use a4n_db_models::models::generation::model_usage_record;
use a4n_domain::cost::{
    CostConfidence, ModelCapability, ModelUsageRecordCompletion, ModelUsageRecordStart,
    PricingPlan, SYSTEM_DEFAULT_PROVIDER_CONFIG_ID, estimate_cost,
};
use a4n_error::AppError;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, FromQueryResult, QueryFilter,
    QueryOrder, QuerySelect, Set, sea_query::Expr,
};
use serde_json::json;

pub async fn start_model_usage_record<C>(
    db: &C,
    input: ModelUsageRecordStart,
) -> Result<model_usage_record::Model, AppError>
where
    C: ConnectionTrait,
{
    let mut input = input;
    input.provider_config_id = normalize_provider_config_id(input.provider_config_id);
    validate_start(&input)?;
    let pricing = input
        .pricing
        .clone()
        .normalized_snapshot()
        .map_err(AppError::InvalidArgument)?;
    let now = Utc::now().naive_utc();
    model_usage_record::ActiveModel {
        request_id: Set(input.request_id.trim().to_string()),
        user_id: Set(input.user_id),
        project_id: Set(input.project_id),
        root_project_id: Set(input.root_project_id),
        workflow_run_id: Set(input.workflow_run_id),
        generation_run_id: Set(input.generation_run_id),
        workflow: Set(input.workflow.trim().to_string()),
        step_key: Set(input.step_key.trim().to_string()),
        call_key: Set(normalize_optional(input.call_key)),
        source: Set(normalize_optional(input.source)),
        capability: Set(input.capability.as_str().to_string()),
        provider_config_id: Set(input.provider_config_id),
        provider: Set(normalize_optional(input.provider)),
        model: Set(normalize_optional(input.model)),
        status: Set("started".to_string()),
        confidence: Set(CostConfidence::Unknown.as_str().to_string()),
        currency: Set(pricing.currency.clone()),
        amount_micros: Set(None),
        meter_json: Set(json!({})),
        pricing_snapshot_json: Set(serde_json::to_value(pricing).map_err(AppError::Serialization)?),
        error_kind: Set(None),
        error_message: Set(None),
        started_at: Set(now),
        completed_at: Set(None),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AppError::database)
}

pub async fn finish_model_usage_record<C>(
    db: &C,
    record_id: i64,
    input: ModelUsageRecordCompletion,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    if record_id <= 0 {
        return Err(AppError::InvalidArgument(
            "model usage record id 必须为正整数".to_string(),
        ));
    }
    let record = model_usage_record::Entity::find_by_id(record_id)
        .one(db)
        .await
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::NotFound(format!("模型成本记录不存在: {record_id}")))?;
    let capability = parse_capability(&record.capability)?;
    let pricing: PricingPlan = serde_json::from_value(record.pricing_snapshot_json.clone())
        .map_err(AppError::Serialization)?;
    let estimate =
        estimate_cost(capability, &pricing, &input.meter).map_err(AppError::InvalidArgument)?;
    let status = normalize_status(&input.status)?;
    let mut active: model_usage_record::ActiveModel = record.into();
    active.status = Set(status);
    active.confidence = Set(estimate.confidence.as_str().to_string());
    active.currency = Set(pricing.currency);
    active.amount_micros = Set(estimate.amount_micros);
    active.meter_json = Set(serde_json::to_value(input.meter).map_err(AppError::Serialization)?);
    active.error_kind = Set(normalize_optional(input.error_kind));
    active.error_message = Set(normalize_optional(input.error_message));
    active.completed_at = Set(Some(Utc::now().naive_utc()));
    active.update(db).await.map_err(AppError::database)?;
    Ok(())
}

pub async fn list_model_usage_records<C>(
    db: &C,
    project_id: Option<i64>,
    since: chrono::NaiveDateTime,
    limit: u64,
) -> Result<Vec<model_usage_record::Model>, AppError>
where
    C: ConnectionTrait,
{
    if limit == 0 || limit > 10_000 {
        return Err(AppError::InvalidArgument(
            "模型成本报表 limit 必须在 1 到 10000 之间".to_string(),
        ));
    }
    let mut query = model_usage_record::Entity::find()
        .filter(model_usage_record::Column::StartedAt.gte(since))
        .order_by_desc(model_usage_record::Column::StartedAt)
        .limit(limit);
    if let Some(project_id) = project_id {
        if project_id <= 0 {
            return Err(AppError::InvalidArgument(
                "模型成本报表 project_id 必须为正整数".to_string(),
            ));
        }
        query = query.filter(
            model_usage_record::Column::ProjectId
                .eq(project_id)
                .or(model_usage_record::Column::RootProjectId.eq(project_id)),
        );
    }
    query.all(db).await.map_err(AppError::database)
}

#[derive(Debug, FromQueryResult)]
pub struct ModelUsageAggregate {
    pub currency: String,
    pub capability: String,
    pub calls: i64,
    pub amount_micros: i64,
    pub priced_calls: i64,
    pub partial_calls: i64,
    pub unpriced_calls: i64,
    pub unsettled_calls: i64,
}

pub async fn aggregate_model_usage_records<C>(
    db: &C,
    project_id: Option<i64>,
    since: chrono::NaiveDateTime,
) -> Result<Vec<ModelUsageAggregate>, AppError>
where
    C: ConnectionTrait,
{
    let mut query = model_usage_record::Entity::find()
        .select_only()
        .column(model_usage_record::Column::Currency)
        .column(model_usage_record::Column::Capability)
        .column_as(Expr::cust("COUNT(*)"), "calls")
        .column_as(
            Expr::cust("COALESCE(SUM(amount_micros), 0)::bigint"),
            "amount_micros",
        )
        .column_as(
            Expr::cust("COUNT(*) FILTER (WHERE confidence IN ('exact', 'estimated'))"),
            "priced_calls",
        )
        .column_as(
            Expr::cust("COUNT(*) FILTER (WHERE confidence = 'partial')"),
            "partial_calls",
        )
        .column_as(
            Expr::cust(
                "COUNT(*) FILTER (WHERE confidence IN ('unpriced', 'unknown') OR amount_micros IS NULL)",
            ),
            "unpriced_calls",
        )
        .column_as(
            Expr::cust("COUNT(*) FILTER (WHERE status = 'started')"),
            "unsettled_calls",
        )
        .filter(model_usage_record::Column::StartedAt.gte(since))
        .group_by(model_usage_record::Column::Currency)
        .group_by(model_usage_record::Column::Capability);
    if let Some(project_id) = project_id {
        if project_id <= 0 {
            return Err(AppError::InvalidArgument(
                "模型成本报表 project_id 必须为正整数".to_string(),
            ));
        }
        query = query.filter(
            model_usage_record::Column::ProjectId
                .eq(project_id)
                .or(model_usage_record::Column::RootProjectId.eq(project_id)),
        );
    }
    query
        .into_model::<ModelUsageAggregate>()
        .all(db)
        .await
        .map_err(AppError::database)
}

fn validate_start(input: &ModelUsageRecordStart) -> Result<(), AppError> {
    if input.request_id.trim().is_empty() {
        return Err(AppError::InvalidArgument(
            "模型成本 request_id 不能为空".to_string(),
        ));
    }
    for (name, value) in [
        ("project_id", input.project_id),
        ("root_project_id", input.root_project_id),
        ("workflow_run_id", input.workflow_run_id),
        ("generation_run_id", input.generation_run_id),
        ("provider_config_id", input.provider_config_id),
        ("user_id", input.user_id),
    ] {
        if value.is_some_and(|value| value <= 0) {
            return Err(AppError::InvalidArgument(format!(
                "模型成本 {name} 必须为正整数"
            )));
        }
    }
    if input.workflow.trim().is_empty() || input.step_key.trim().is_empty() {
        return Err(AppError::InvalidArgument(
            "模型成本 workflow 和 step_key 不能为空".to_string(),
        ));
    }
    Ok(())
}

fn normalize_provider_config_id(value: Option<i64>) -> Option<i64> {
    if value == Some(SYSTEM_DEFAULT_PROVIDER_CONFIG_ID) {
        None
    } else {
        value
    }
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim().to_string();
        (!value.is_empty()).then_some(value)
    })
}

fn normalize_status(value: &str) -> Result<String, AppError> {
    let value = value.trim();
    if matches!(value, "completed" | "failed" | "cancelled" | "unknown") {
        Ok(value.to_string())
    } else {
        Err(AppError::InvalidArgument(format!(
            "模型成本状态不支持: {value}"
        )))
    }
}

fn parse_capability(value: &str) -> Result<ModelCapability, AppError> {
    match value {
        "llm" => Ok(ModelCapability::Llm),
        "embedding" => Ok(ModelCapability::Embedding),
        "image" => Ok(ModelCapability::Image),
        "video" => Ok(ModelCapability::Video),
        _ => Err(AppError::InvalidArgument(format!(
            "模型成本能力不支持: {value}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_domain::cost::ModelUsageMeter;

    #[test]
    fn status_and_optional_text_are_bounded() {
        assert_eq!(normalize_optional(Some("  ".into())), None);
        assert!(normalize_status("running").is_err());
        assert_eq!(normalize_status("failed").unwrap(), "failed");
    }

    #[test]
    fn meter_serializes_as_object() {
        let meter = ModelUsageMeter {
            input_tokens: Some(10),
            ..Default::default()
        };
        assert!(serde_json::to_value(meter).unwrap().is_object());
    }

    #[test]
    fn system_default_provider_id_is_not_persisted_as_a_config_id() {
        assert_eq!(
            normalize_provider_config_id(Some(SYSTEM_DEFAULT_PROVIDER_CONFIG_ID)),
            None
        );
        assert_eq!(normalize_provider_config_id(Some(7)), Some(7));
        assert_eq!(normalize_provider_config_id(None), None);
    }
}
