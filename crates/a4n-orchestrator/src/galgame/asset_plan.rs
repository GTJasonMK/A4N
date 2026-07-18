mod collection;
mod contracts;
mod drafts;
mod persistence;
mod planning;
mod presentation;

pub use contracts::{
    GalgameAssetPlan, GalgameAssetPlanItem, GalgameAssetPlanOutcome, GalgameAssetPlanSummary,
};
#[cfg(test)]
pub(in crate::galgame) use drafts::{GalgamePortraitAssetDraft, portrait_variant};
#[cfg(test)]
pub(in crate::galgame) use presentation::event_cg_directive;
pub(crate) use presentation::validate_presentation_json;
