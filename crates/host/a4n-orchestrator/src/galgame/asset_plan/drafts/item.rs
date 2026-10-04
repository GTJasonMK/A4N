use std::collections::BTreeSet;

use a4n_domain::galgame::GalgameAssetPlanSpecRecordInput;
use a4n_domain::generation::GenerationSpecRecord;
use a4n_error::AppError;

use super::super::contracts::{GalgameAssetPlanGroup, GalgameAssetPlanItemDraft, GalgameSpecInput};
use super::super::{GalgameAssetPlanItem, GalgameAssetPlanOutcome, GalgameAssetPlanSummary};

pub(in crate::galgame::asset_plan) fn galgame_asset_plan_item_draft(
    group: GalgameAssetPlanGroup,
    input: GalgameSpecInput,
    text_anchor_ids: &BTreeSet<i64>,
    replace_entity_refs: bool,
    entity_profile_id: Option<i64>,
    label: String,
    source_label: String,
) -> Result<GalgameAssetPlanItemDraft, AppError> {
    if text_anchor_ids.is_empty() {
        return Err(AppError::InvalidArgument(
            "Galgame 资产生成意图必须至少绑定一个文本锚点".to_string(),
        ));
    }
    let canonical_key = input.canonical_key.clone();
    Ok(GalgameAssetPlanItemDraft {
        group,
        canonical_key,
        label,
        source_label,
        record_input: GalgameAssetPlanSpecRecordInput {
            spec: input.into_record_input(),
            text_anchor_ids: text_anchor_ids.iter().copied().collect(),
            replace_entity_refs,
            entity_profile_id,
        },
    })
}

pub(in crate::galgame::asset_plan) fn push_plan_item(
    items: &mut Vec<GalgameAssetPlanItem>,
    summary: &mut GalgameAssetPlanSummary,
    spec: GenerationSpecRecord,
    draft: GalgameAssetPlanItemDraft,
    outcome: Option<GalgameAssetPlanOutcome>,
) -> Result<(), AppError> {
    match outcome {
        Some(GalgameAssetPlanOutcome::Created) => summary.created_specs += 1,
        Some(GalgameAssetPlanOutcome::Updated) => summary.updated_specs += 1,
        Some(GalgameAssetPlanOutcome::Reused) => summary.reused_specs += 1,
        None => {}
    }
    let target_entity_id = spec
        .target_entity_id
        .ok_or_else(|| AppError::InvalidState {
            current: format!(
                "galgame_generation_spec_{}_missing_target_entity_id",
                spec.id
            ),
            operation: "plan_galgame_assets".to_string(),
        })?;
    items.push(GalgameAssetPlanItem {
        canonical_key: draft.canonical_key,
        label: draft.label,
        source_snapshot_id: draft.record_input.spec.source_snapshot_id,
        source_label: draft.source_label,
        target_kind: a4n_domain::galgame::GalgameGenerationTargetKind::parse(&spec.target_kind)
            .ok_or_else(|| AppError::InvalidState {
                current: format!(
                    "galgame_generation_spec_{}_invalid_target_kind:{}",
                    spec.id, spec.target_kind
                ),
                operation: "plan_galgame_assets".to_string(),
            })?,
        target_entity_kind: a4n_domain::galgame::GalgameTargetEntityKind::parse(
            &spec.target_entity_kind,
        )
        .ok_or_else(|| AppError::InvalidState {
            current: format!(
                "galgame_generation_spec_{}_invalid_target_entity_kind:{}",
                spec.id, spec.target_entity_kind
            ),
            operation: "plan_galgame_assets".to_string(),
        })?,
        target_entity_id,
        generation_spec_id: spec.id,
        status: spec.status,
        outcome,
        params_json: spec.params_json,
    });
    Ok(())
}
