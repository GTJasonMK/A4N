use std::collections::{BTreeMap, BTreeSet};

use a4n_api_contract::cost::{
    ModelUsageCapabilityTotal, ModelUsageCurrencyTotal, ModelUsageEntry, ModelUsageReport,
    ModelUsageReportInput, ModelUsageRequest, ModelUsageRequestInput,
};
use a4n_error::AppError;
use chrono::{Duration, Utc};

use super::OrchestratorContext;
use crate::admin::AdminPermission;

impl OrchestratorContext {
    async fn model_usage_report_validated(
        &self,
        caller_user_id: i64,
        input: ModelUsageReportInput,
    ) -> Result<ModelUsageReport, AppError> {
        if input.days == 0 || input.days > 3650 {
            return Err(AppError::InvalidArgument(
                "模型成本报表 days 必须在 1 到 3650 之间".to_string(),
            ));
        }
        if input.limit == 0 || input.limit > 10_000 {
            return Err(AppError::InvalidArgument(
                "模型成本报表 limit 必须在 1 到10000 之间".to_string(),
            ));
        }
        if let Some(project_id) = input.project_id {
            self.verify_project_permission(
                project_id,
                caller_user_id,
                a4n_domain::collaboration::ProjectAccessRole::Viewer,
            )
            .await?;
        } else {
            self.require_admin_permission(caller_user_id, AdminPermission::ManageSettings)
                .await?;
        }
        let until = Utc::now().naive_utc();
        let since = until
            .checked_sub_signed(Duration::days(i64::from(input.days)))
            .ok_or_else(|| AppError::InvalidArgument("模型成本报表时间范围无效".to_string()))?;
        let aggregates = a4n_db::model_usage::aggregate_model_usage_records(
            self.database.connection(),
            input.project_id,
            since,
        )
        .await?;
        let records = a4n_db::model_usage::list_model_usage_summaries(
            self.database.connection(),
            input.project_id,
            since,
            u64::from(input.limit),
        )
        .await?;

        let mut currency_totals: BTreeMap<String, (i64, u64, u64, u64)> = BTreeMap::new();
        let mut capability_totals: BTreeMap<(String, String), (u64, i64)> = BTreeMap::new();
        let mut partial_calls = 0u64;
        let mut unpriced_calls = 0u64;
        let mut unsettled_calls = 0u64;
        for aggregate in aggregates {
            let currency = currency_totals
                .entry(aggregate.currency.clone())
                .or_insert((0, 0, 0, 0));
            currency.0 = currency.0.saturating_add(aggregate.amount_micros);
            currency.1 = currency
                .1
                .saturating_add(count_to_u64(aggregate.priced_calls)?);
            currency.2 = currency
                .2
                .saturating_add(count_to_u64(aggregate.partial_calls)?);
            currency.3 = currency
                .3
                .saturating_add(count_to_u64(aggregate.unpriced_calls)?);
            let capability = capability_totals
                .entry((aggregate.currency.clone(), aggregate.capability))
                .or_insert((0, 0));
            capability.0 = capability.0.saturating_add(count_to_u64(aggregate.calls)?);
            capability.1 = capability.1.saturating_add(aggregate.amount_micros);
            partial_calls = partial_calls.saturating_add(count_to_u64(aggregate.partial_calls)?);
            unpriced_calls = unpriced_calls.saturating_add(count_to_u64(aggregate.unpriced_calls)?);
            unsettled_calls =
                unsettled_calls.saturating_add(count_to_u64(aggregate.unsettled_calls)?);
        }
        let mut recent = Vec::with_capacity(records.len());
        for record in records {
            recent.push(ModelUsageEntry {
                id: record.id,
                request_id: record.request_id,
                capability: record.capability,
                workflow: record.workflow,
                step_key: record.step_key,
                provider: record.provider,
                model: record.model,
                request_available: record.request_available,
                currency: record.currency,
                amount_micros: record.amount_micros.map(|value| value.to_string()),
                confidence: record.confidence,
                status: record.status,
                meter: record.meter_json,
                source: record.source,
                error_kind: record.error_kind,
                error_message: record.error_message,
                started_at: record.started_at.and_utc().to_rfc3339(),
                completed_at: record
                    .completed_at
                    .map(|value| value.and_utc().to_rfc3339()),
            });
        }
        Ok(ModelUsageReport {
            project_id: input.project_id,
            since: since.and_utc().to_rfc3339(),
            until: until.and_utc().to_rfc3339(),
            totals: currency_totals
                .into_iter()
                .map(
                    |(currency, (amount, priced, partial, unpriced))| ModelUsageCurrencyTotal {
                        currency,
                        amount_micros: amount.to_string(),
                        priced_calls: priced,
                        partial_calls: partial,
                        unpriced_calls: unpriced,
                    },
                )
                .collect(),
            capabilities: capability_totals
                .into_iter()
                .map(
                    |((currency, capability), (calls, amount))| ModelUsageCapabilityTotal {
                        currency,
                        capability,
                        calls,
                        amount_micros: amount.to_string(),
                    },
                )
                .collect(),
            recent,
            partial_calls,
            unpriced_calls,
            unsettled_calls,
        })
    }
}

fn count_to_u64(value: i64) -> Result<u64, AppError> {
    u64::try_from(value).map_err(|_| AppError::InvalidState {
        current: value.to_string(),
        operation: "模型成本聚合计数无效".to_string(),
    })
}

#[async_trait::async_trait]
impl a4n_api_contract::cost::ModelUsageReadApplicationPort for OrchestratorContext {
    async fn model_usage_report(
        &self,
        caller_user_id: i64,
        input: ModelUsageReportInput,
    ) -> Result<ModelUsageReport, AppError> {
        self.model_usage_report_validated(caller_user_id, input)
            .await
    }

    async fn model_usage_request(
        &self,
        caller_user_id: i64,
        input: ModelUsageRequestInput,
    ) -> Result<ModelUsageRequest, AppError> {
        let record =
            a4n_db::model_usage::model_usage_record_by_id(self.database.connection(), input.id)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("模型成本记录不存在: {}", input.id)))?;
        let project_ids = [record.project_id, record.root_project_id]
            .into_iter()
            .flatten()
            .collect::<BTreeSet<_>>();
        let has_project_scope = !project_ids.is_empty();
        let mut access_error = None;
        for project_id in project_ids {
            match self
                .verify_project_permission(
                    project_id,
                    caller_user_id,
                    a4n_domain::collaboration::ProjectAccessRole::Viewer,
                )
                .await
            {
                Ok(_) => {
                    access_error = None;
                    break;
                }
                Err(error @ (AppError::Forbidden(_) | AppError::NotFound(_))) => {
                    access_error = Some(error);
                }
                Err(error) => return Err(error),
            }
        }
        if let Some(project_error) = access_error {
            match self
                .require_admin_permission(caller_user_id, AdminPermission::ManageSettings)
                .await
            {
                Ok(()) => {}
                Err(AppError::Forbidden(_)) => return Err(project_error),
                Err(error) => return Err(error),
            }
        }
        if !has_project_scope {
            self.require_admin_permission(caller_user_id, AdminPermission::ManageSettings)
                .await?;
        }
        let request_json = record
            .request_json
            .ok_or_else(|| AppError::NotFound("该模型调用未保存请求快照".to_string()))?;
        Ok(ModelUsageRequest {
            id: record.id,
            request_id: record.request_id,
            capability: record.capability,
            workflow: record.workflow,
            step_key: record.step_key,
            provider: record.provider,
            model: record.model,
            status: record.status,
            started_at: record.started_at.and_utc().to_rfc3339(),
            request_json,
        })
    }
}
