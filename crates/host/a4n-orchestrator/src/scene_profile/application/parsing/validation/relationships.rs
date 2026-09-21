use crate::scene_profile::{
    SceneEntityStateDraft, SceneOccurrenceDraft, SceneProfileDraft, SceneSyncInput,
    SceneTransitionDraft,
};
use std::collections::{HashMap, HashSet};

pub(in crate::scene_profile::application::parsing) fn validate_occurrence_references(
    profiles: &[SceneProfileDraft],
    occurrences: &[SceneOccurrenceDraft],
) -> Result<(), String> {
    let scene_refs = profiles
        .iter()
        .map(|profile| profile.scene_ref.as_str())
        .collect::<HashSet<_>>();
    let scenes_with_occurrences = occurrences
        .iter()
        .map(|occurrence| occurrence.scene_ref.as_str())
        .collect::<HashSet<_>>();

    for profile in profiles {
        if !scenes_with_occurrences.contains(profile.scene_ref.as_str()) {
            return Err(format!(
                "scenes.scene_ref={} 没有对应 occurrences；场景档案必须记录本章出场",
                profile.scene_ref
            ));
        }
    }

    for (index, occurrence) in occurrences.iter().enumerate() {
        if !scene_refs.contains(occurrence.scene_ref.as_str()) {
            return Err(format!(
                "occurrences[{index}].scene_ref 引用了不存在的 scenes[].scene_ref: {}",
                occurrence.scene_ref
            ));
        }
    }

    Ok(())
}

pub(in crate::scene_profile::application::parsing) fn validate_transition_references(
    known: &SceneSyncInput,
    transitions: &[SceneTransitionDraft],
) -> Result<(), String> {
    let scene_refs = known
        .profiles
        .iter()
        .map(|profile| profile.scene_ref.as_str())
        .collect::<HashSet<_>>();
    let occurrence_refs = known
        .occurrences
        .iter()
        .map(|occurrence| occurrence.occurrence_ref.as_str())
        .collect::<HashSet<_>>();
    for (index, transition) in transitions.iter().enumerate() {
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
            && !occurrence_refs.contains(occurrence_ref)
        {
            return Err(format!(
                "transitions[{index}].occurrence_ref 引用了不存在的 occurrences[].occurrence_ref: {occurrence_ref}"
            ));
        }
    }

    Ok(())
}

pub(in crate::scene_profile::application::parsing) fn validate_entity_state_references(
    known: &SceneSyncInput,
    entity_states: &[SceneEntityStateDraft],
) -> Result<(), String> {
    let occurrence_scene_refs = known
        .occurrences
        .iter()
        .map(|occurrence| {
            (
                occurrence.occurrence_ref.as_str(),
                occurrence.scene_ref.as_str(),
            )
        })
        .collect::<HashMap<_, _>>();
    for (index, entity_state) in entity_states.iter().enumerate() {
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
