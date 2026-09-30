use a4n_domain::{
    manga::MangaPageContentV1,
    media_authoring::{AdaptationBeat, AdaptationBeatRef, AdaptationImportance, AdaptationPlanV1},
};
use serde_json::{Value, json};
use std::collections::HashSet;

use crate::media_authoring::PreparedAdaptation;

pub(crate) type MangaBeatIndex = std::collections::HashMap<AdaptationBeatRef, AdaptationBeat>;

pub(super) fn manga_beat_refs(plan: &AdaptationPlanV1) -> MangaBeatIndex {
    plan.units
        .iter()
        .flat_map(|unit| {
            unit.beats.iter().map(move |beat| {
                (
                    AdaptationBeatRef {
                        unit_id: unit.id.clone(),
                        beat_id: beat.id.clone(),
                    },
                    beat.clone(),
                )
            })
        })
        .collect()
}

pub(super) fn manga_events_from_adaptation(plan: &AdaptationPlanV1) -> Value {
    Value::Array(
        plan.units
            .iter()
            .flat_map(|unit| {
                unit.beats.iter().map(move |beat| {
                    json!({
                        "beat_ref": {"unit_id": unit.id, "beat_id": beat.id},
                        "type": beat.kind,
                        "description": match beat.reaction.as_deref() {
                            Some(reaction) => format!("{}；反应：{reaction}", beat.action),
                            None => beat.action.clone(),
                        },
                        "participants": unit.participants,
                        "importance": match beat.importance {
                            AdaptationImportance::Required => "critical",
                            AdaptationImportance::Important => "high",
                            AdaptationImportance::Supporting => "normal",
                        },
                        "is_climax": beat.is_climax,
                        "origin": beat.origin,
                        "dialogue_intent": beat.dialogue_intent,
                        "source_span_ids": beat.source_span_ids,
                    })
                })
            })
            .collect(),
    )
}

pub(super) fn manga_page_slots_source(adaptation: &PreparedAdaptation) -> Value {
    json!({
        "adaptation_plan_id": adaptation.record.id,
        "adaptation_plan_revision": adaptation.record.revision,
        "intention": adaptation.plan.intention,
        "units": adaptation.plan.units,
        "events": manga_events_from_adaptation(&adaptation.plan),
    })
}

pub(super) fn manga_page_adaptation_source(
    adaptation: &PreparedAdaptation,
    content: &MangaPageContentV1,
) -> Value {
    let beat_refs = content.beat_refs.iter().cloned().collect::<HashSet<_>>();
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
