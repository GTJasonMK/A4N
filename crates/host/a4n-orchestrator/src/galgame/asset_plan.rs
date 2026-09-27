mod collection;
mod contracts;
mod drafts;
mod persistence;
mod planning;
mod reading;

pub(crate) use a4n_api_contract::galgame::{
    GalgameAssetPlan, GalgameAssetPlanItem, GalgameAssetPlanOutcome, GalgameAssetPlanSummary,
};
#[cfg(test)]
pub(in crate::galgame) use drafts::{GalgamePortraitAssetDraft, portrait_variant};
