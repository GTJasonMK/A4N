use a4n_orchestrator::scene_profile::SceneSyncInput;
use std::collections::{BTreeMap, BTreeSet};

pub(in crate::actions::scene_profile_sync::parsing) fn validate_scene_sync_references(
    input: &SceneSyncInput,
) -> Result<(), String> {
    let scene_refs = input
        .profiles
        .iter()
        .map(|profile| profile.scene_ref.as_str())
        .collect::<BTreeSet<_>>();
    let occurrence_scene_refs = input
        .occurrences
        .iter()
        .map(|occurrence| {
            (
                occurrence.occurrence_ref.as_str(),
                occurrence.scene_ref.as_str(),
            )
        })
        .collect::<BTreeMap<_, _>>();

    for profile in &input.profiles {
        if !input
            .occurrences
            .iter()
            .any(|occurrence| occurrence.scene_ref == profile.scene_ref)
        {
            return Err(format!(
                "scenes.scene_ref={} 没有对应 occurrences；场景档案必须记录本章出场",
                profile.scene_ref
            ));
        }
    }

    for (index, occurrence) in input.occurrences.iter().enumerate() {
        if !scene_refs.contains(occurrence.scene_ref.as_str()) {
            return Err(format!(
                "occurrences[{index}].scene_ref 引用了不存在的 scenes[].scene_ref: {}",
                occurrence.scene_ref
            ));
        }
    }

    for (index, transition) in input.transitions.iter().enumerate() {
        if let Some(from_scene_ref) = transition.from_scene_ref.as_deref()
            && !scene_refs.contains(from_scene_ref)
        {
            return Err(format!(
                "transitions[{index}].from_scene_ref 引用了不存在的 scenes[].scene_ref: {from_scene_ref}"
            ));
        }
        if !scene_refs.contains(transition.to_scene_ref.as_str()) {
            return Err(format!(
                "transitions[{index}].to_scene_ref 引用了不存在的 scenes[].scene_ref: {}",
                transition.to_scene_ref
            ));
        }
        if let Some(occurrence_ref) = transition.occurrence_ref.as_deref()
            && !occurrence_scene_refs.contains_key(occurrence_ref)
        {
            return Err(format!(
                "transitions[{index}].occurrence_ref 引用了不存在的 occurrences[].occurrence_ref: {occurrence_ref}"
            ));
        }
    }

    for (index, entity_state) in input.entity_states.iter().enumerate() {
        if !scene_refs.contains(entity_state.scene_ref.as_str()) {
            return Err(format!(
                "entity_states[{index}].scene_ref 引用了不存在的 scenes[].scene_ref: {}",
                entity_state.scene_ref
            ));
        }
        let Some(occurrence_scene_ref) =
            occurrence_scene_refs.get(entity_state.occurrence_ref.as_str())
        else {
            return Err(format!(
                "entity_states[{index}].occurrence_ref 引用了不存在的 occurrences[].occurrence_ref: {}",
                entity_state.occurrence_ref
            ));
        };
        if *occurrence_scene_ref != entity_state.scene_ref {
            return Err(format!(
                "entity_states[{index}].occurrence_ref 必须指向 scene_ref 对应的出场"
            ));
        }
    }

    Ok(())
}
