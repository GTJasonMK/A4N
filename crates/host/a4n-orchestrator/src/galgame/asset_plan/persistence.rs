use a4n_db::galgame::GalgameGenerationSpecUpsertOutcome;
use a4n_domain::generation::GenerationSpecRecord;
use a4n_error::AppError;

use crate::galgame::{OrchestratorContext, galgame_asset_plan_spec_record_error};

use super::contracts::{GalgameAssetPlanGroup, GalgameAssetPlanItemDraft};
use super::drafts::push_plan_item;
use super::{GalgameAssetPlan, GalgameAssetPlanOutcome, GalgameAssetPlanSummary};

pub(super) async fn persist_galgame_asset_plan(
    service: &OrchestratorContext,
    project_id: i64,
    item_drafts: Vec<GalgameAssetPlanItemDraft>,
) -> Result<GalgameAssetPlan, AppError> {
    let record_inputs = item_drafts
        .iter()
        .map(|draft| draft.record_input.clone())
        .collect::<Vec<_>>();
    let upserts = a4n_db::galgame::upsert_galgame_asset_plan_spec_records_transaction(
        service.database.executor(),
        project_id,
        record_inputs,
    )
    .await
    .map_err(galgame_asset_plan_spec_record_error)?;
    if upserts.len() != item_drafts.len() {
        return Err(AppError::InvalidState {
            current: format!(
                "galgame_asset_plan_upsert_count_mismatch: drafts={}, upserts={}",
                item_drafts.len(),
                upserts.len()
            ),
            operation: "plan_galgame_assets".to_string(),
        });
    }

    assemble_galgame_asset_plan(item_drafts.into_iter().zip(upserts).map(|(draft, upsert)| {
        let outcome = match upsert.outcome {
            GalgameGenerationSpecUpsertOutcome::Created => GalgameAssetPlanOutcome::Created,
            GalgameGenerationSpecUpsertOutcome::Updated => GalgameAssetPlanOutcome::Updated,
            GalgameGenerationSpecUpsertOutcome::Reused => GalgameAssetPlanOutcome::Reused,
        };
        (draft, upsert.record, Some(outcome))
    }))
}

pub(super) fn assemble_galgame_asset_plan(
    items: impl IntoIterator<
        Item = (
            GalgameAssetPlanItemDraft,
            GenerationSpecRecord,
            Option<GalgameAssetPlanOutcome>,
        ),
    >,
) -> Result<GalgameAssetPlan, AppError> {
    let mut summary = GalgameAssetPlanSummary::default();
    let mut backgrounds = Vec::new();
    let mut portrait_expressions = Vec::new();
    let mut event_cgs = Vec::new();
    for (draft, spec, outcome) in items {
        let items = match draft.group {
            GalgameAssetPlanGroup::Background => &mut backgrounds,
            GalgameAssetPlanGroup::PortraitExpression => &mut portrait_expressions,
            GalgameAssetPlanGroup::EventCg => &mut event_cgs,
        };
        push_plan_item(items, &mut summary, spec, draft, outcome)?;
    }
    summary.background_specs = backgrounds.len();
    summary.portrait_expression_specs = portrait_expressions.len();
    summary.event_cg_specs = event_cgs.len();
    Ok(GalgameAssetPlan {
        summary,
        backgrounds,
        portrait_expressions,
        event_cgs,
    })
}
