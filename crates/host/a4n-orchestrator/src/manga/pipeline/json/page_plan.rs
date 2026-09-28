use std::collections::HashSet;

use crate::media_authoring::PreparedAdaptation;
use a4n_domain::{
    manga::MangaPagePlanV4,
    media_authoring::{AdaptationBeatRef, AdaptationImportance},
};
use a4n_error::AppError;
use serde_json::Value;

pub(crate) fn parse_manga_page_plan(
    json: &Value,
    adaptation: &PreparedAdaptation,
    context: &str,
) -> Result<MangaPagePlanV4, AppError> {
    let plan: MangaPagePlanV4 = serde_json::from_value(json.clone()).map_err(|error| {
        AppError::InvalidArgument(format!("{context} 不符合 manga_page_plan_v4 合同: {error}"))
    })?;
    plan.validate()
        .map_err(|error| AppError::InvalidArgument(format!("{context}: {error}")))?;
    if plan.adaptation_plan_id != adaptation.record.id
        || plan.adaptation_plan_revision != adaptation.record.revision
    {
        return Err(AppError::InvalidArgument(format!(
            "{context} 改编计划身份不一致: expected id={} revision={}, actual id={} revision={}",
            adaptation.record.id,
            adaptation.record.revision,
            plan.adaptation_plan_id,
            plan.adaptation_plan_revision
        )));
    }
    validate_adaptation_beat_coverage(&plan, adaptation, context)?;
    Ok(plan)
}

fn validate_adaptation_beat_coverage(
    plan: &MangaPagePlanV4,
    adaptation: &PreparedAdaptation,
    context: &str,
) -> Result<(), AppError> {
    let available = adaptation
        .plan
        .units
        .iter()
        .flat_map(|unit| {
            unit.beats.iter().map(|beat| {
                (
                    AdaptationBeatRef {
                        unit_id: unit.id.clone(),
                        beat_id: beat.id.clone(),
                    },
                    beat.importance,
                )
            })
        })
        .collect::<Vec<_>>();
    let available_refs = available
        .iter()
        .map(|(beat_ref, _)| beat_ref.clone())
        .collect::<HashSet<_>>();
    let used_refs = plan
        .pages
        .iter()
        .flat_map(|page| page.beat_refs.iter().cloned())
        .collect::<HashSet<_>>();
    for beat_ref in &used_refs {
        if !available_refs.contains(beat_ref) {
            return Err(AppError::InvalidArgument(format!(
                "{context} 引用了不存在的改编节拍: unit_id={}, beat_id={}",
                beat_ref.unit_id, beat_ref.beat_id
            )));
        }
    }
    for (beat_ref, importance) in available {
        if importance == AdaptationImportance::Required && !used_refs.contains(&beat_ref) {
            return Err(AppError::InvalidArgument(format!(
                "{context} 遗漏关键改编节拍: unit_id={}, beat_id={}",
                beat_ref.unit_id, beat_ref.beat_id
            )));
        }
    }
    Ok(())
}
