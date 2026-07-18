use std::collections::HashMap;

use super::ExpectedRagRecords;
use crate::state::AppState;
use a4n_orchestrator::rag::{
    scene_occurrence_rag_record, scene_profile_rag_record, scene_transition_rag_record,
};

pub(super) async fn append_scene_records(
    state: &AppState,
    project_id: i64,
    expected: &mut ExpectedRagRecords,
) -> Result<(), String> {
    let scene_profiles = state
        .orchestrator
        .scene_profiles_for_rag_ingest(project_id)
        .await
        .map_err(|e| e.to_string())?;
    let scene_names = scene_profiles
        .iter()
        .map(|profile| (profile.id, profile.name.clone()))
        .collect::<HashMap<_, _>>();
    for profile in &scene_profiles {
        let record = scene_profile_rag_record(project_id, profile)?;
        expected.counts.scene_profiles += 1;
        expected.add_rag_record(record)?;
    }

    let scene_occurrences = state
        .orchestrator
        .scene_occurrences_for_rag_ingest(project_id)
        .await
        .map_err(|e| e.to_string())?;
    for occurrence in &scene_occurrences {
        let scene_name = scene_names
            .get(&occurrence.scene_profile_id)
            .ok_or_else(|| {
                format!(
                    "场景出场 {} 找不到 scene_profile_id={} 对应场景，无法构造 RAG 预期记录",
                    occurrence.id, occurrence.scene_profile_id
                )
            })?;
        let record = scene_occurrence_rag_record(project_id, occurrence, scene_name)?;
        expected.counts.scene_occurrences += 1;
        expected.add_rag_record(record)?;
    }

    let scene_transitions = state
        .orchestrator
        .scene_transitions_for_rag_ingest(project_id)
        .await
        .map_err(|e| e.to_string())?;
    for transition in &scene_transitions {
        let to_scene_name = scene_names
            .get(&transition.to_scene_profile_id)
            .ok_or_else(|| {
                format!(
                    "场景位移 {} 找不到 to_scene_profile_id={} 对应场景，无法构造 RAG 预期记录",
                    transition.id, transition.to_scene_profile_id
                )
            })?;
        let from_scene_name = match transition.from_scene_profile_id {
            Some(profile_id) => Some(
                scene_names
                    .get(&profile_id)
                    .ok_or_else(|| {
                        format!(
                            "场景位移 {} 找不到 from_scene_profile_id={} 对应场景，无法构造 RAG 预期记录",
                            transition.id, profile_id
                        )
                    })?
                    .as_str(),
            ),
            None => None,
        };
        let record =
            scene_transition_rag_record(project_id, transition, from_scene_name, to_scene_name)?;
        expected.counts.scene_transitions += 1;
        expected.add_rag_record(record)?;
    }
    Ok(())
}
