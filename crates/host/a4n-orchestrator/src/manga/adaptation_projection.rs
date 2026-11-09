use a4n_domain::media_authoring::AdaptationBeatRef;
use serde_json::{Value, json};
use std::collections::HashSet;

use crate::media_authoring::PreparedAdaptation;

pub(super) fn manga_page_slots_source(adaptation: &PreparedAdaptation) -> Value {
    json!({
        "adaptation_plan_id": adaptation.record.id,
        "adaptation_plan_revision": adaptation.record.revision,
        "page_count": adaptation.plan.page_count,
        "intention": adaptation.plan.intention,
        "units": adaptation.plan.units,
    })
}

pub(in crate::manga) fn manga_page_beats_source(
    adaptation: &PreparedAdaptation,
    refs: &[AdaptationBeatRef],
) -> Value {
    let beat_refs = refs.iter().cloned().collect::<HashSet<_>>();
    let units = adaptation
        .plan
        .units
        .iter()
        .filter_map(|unit| {
            let beats = unit
                .beats
                .iter()
                .filter(|beat| {
                    beat_refs.contains(&AdaptationBeatRef {
                        unit_id: unit.id.clone(),
                        beat_id: beat.id.clone(),
                    })
                })
                .collect::<Vec<_>>();
            (!beats.is_empty()).then(|| {
                json!({
                    "id": unit.id,
                    "title": unit.title,
                    "location": unit.location,
                    "participants": unit.participants,
                    "dramatic_question": unit.dramatic_question,
                    "objective": unit.objective,
                    "obstacle": unit.obstacle,
                    "beats": beats,
                })
            })
        })
        .collect::<Vec<_>>();
    json!({
        "adaptation_plan_id": adaptation.record.id,
        "adaptation_plan_revision": adaptation.record.revision,
        "intention": adaptation.plan.intention,
        "units": units,
    })
}
