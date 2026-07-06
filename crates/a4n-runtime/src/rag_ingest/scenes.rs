use crate::state::AppState;
use a4n_core::models::scene::{scene_occurrence, scene_profile, scene_transition};
use a4n_core::rag::RagDataType;
use sea_orm::*;
use std::collections::HashMap;

use super::record_builders::{
    scene_occurrence_rag_record, scene_profile_rag_record, scene_transition_rag_record,
};
use super::sync::{
    ExpectedSyncStats as IngestStats, RagIngestProgress, labeled_rag_record, sync_data_type_records,
};

pub(crate) async fn ingest_scene_profiles_inner(
    state: &AppState,
    project_id: i64,
    mut progress: Option<&mut RagIngestProgress<'_>>,
) -> Result<IngestStats, String> {
    let mut stats = IngestStats::default();
    let profiles = scene_profile::Entity::find()
        .filter(scene_profile::Column::ProjectId.eq(project_id))
        .order_by_asc(scene_profile::Column::FirstSeenChapter)
        .order_by_asc(scene_profile::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    let mut records = Vec::new();
    for profile in &profiles {
        let record = match scene_profile_rag_record(project_id, profile) {
            Ok(record) => record,
            Err(err) => {
                stats.errors.push(err);
                continue;
            }
        };
        records.push(labeled_rag_record(
            record,
            format!("场景档案：{}", profile.name),
            format!("场景档案 {}", profile.id),
        ));
    }
    let processed = sync_data_type_records(
        state,
        project_id,
        RagDataType::SceneProfile,
        records,
        &mut stats,
        &mut progress,
        "场景档案",
    )
    .await?;
    stats.item_count = stats.item_count.saturating_add(processed);
    Ok(stats)
}

pub(crate) async fn ingest_scene_occurrences_inner(
    state: &AppState,
    project_id: i64,
    mut progress: Option<&mut RagIngestProgress<'_>>,
) -> Result<IngestStats, String> {
    let mut stats = IngestStats::default();
    let profile_names = scene_profile_names(state, project_id).await?;
    let occurrences = scene_occurrence::Entity::find()
        .filter(scene_occurrence::Column::ProjectId.eq(project_id))
        .order_by_asc(scene_occurrence::Column::ChapterNumber)
        .order_by_asc(scene_occurrence::Column::OccurrenceOrder)
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    let mut records = Vec::new();
    for occurrence in &occurrences {
        let Some(scene_name) = profile_names
            .get(&occurrence.scene_profile_id)
            .map(String::as_str)
        else {
            stats.errors.push(format!(
                "场景出场 {} 找不到 scene_profile_id={} 对应场景，跳过 RAG 入库",
                occurrence.id, occurrence.scene_profile_id
            ));
            continue;
        };
        let record = match scene_occurrence_rag_record(project_id, occurrence, scene_name) {
            Ok(record) => record,
            Err(err) => {
                stats.errors.push(err);
                continue;
            }
        };
        records.push(labeled_rag_record(
            record,
            format!("场景出场：{} 第{}章", scene_name, occurrence.chapter_number),
            format!("场景出场 {}", occurrence.id),
        ));
    }
    let processed = sync_data_type_records(
        state,
        project_id,
        RagDataType::SceneOccurrence,
        records,
        &mut stats,
        &mut progress,
        "场景出场",
    )
    .await?;
    stats.item_count = stats.item_count.saturating_add(processed);
    Ok(stats)
}

pub(crate) async fn ingest_scene_transitions_inner(
    state: &AppState,
    project_id: i64,
    mut progress: Option<&mut RagIngestProgress<'_>>,
) -> Result<IngestStats, String> {
    let mut stats = IngestStats::default();
    let profile_names = scene_profile_names(state, project_id).await?;
    let transitions = scene_transition::Entity::find()
        .filter(scene_transition::Column::ProjectId.eq(project_id))
        .order_by_asc(scene_transition::Column::ChapterNumber)
        .order_by_asc(scene_transition::Column::TransitionOrder)
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    let mut records = Vec::new();
    for transition in &transitions {
        let Some(to_scene_name) = profile_names
            .get(&transition.to_scene_profile_id)
            .map(String::as_str)
        else {
            stats.errors.push(format!(
                "场景位移 {} 找不到 to_scene_profile_id={} 对应场景，跳过 RAG 入库",
                transition.id, transition.to_scene_profile_id
            ));
            continue;
        };
        let from_scene_name = transition
            .from_scene_profile_id
            .and_then(|id| profile_names.get(&id))
            .map(String::as_str);
        let record = match scene_transition_rag_record(
            project_id,
            transition,
            from_scene_name,
            to_scene_name,
        ) {
            Ok(record) => record,
            Err(err) => {
                stats.errors.push(err);
                continue;
            }
        };
        records.push(labeled_rag_record(
            record,
            format!(
                "场景位移：第{}章 -> {}",
                transition.chapter_number, to_scene_name
            ),
            format!("场景位移 {}", transition.id),
        ));
    }
    let processed = sync_data_type_records(
        state,
        project_id,
        RagDataType::SceneTransition,
        records,
        &mut stats,
        &mut progress,
        "场景位移",
    )
    .await?;
    stats.item_count = stats.item_count.saturating_add(processed);
    Ok(stats)
}

async fn scene_profile_names(
    state: &AppState,
    project_id: i64,
) -> Result<HashMap<i64, String>, String> {
    let profiles = scene_profile::Entity::find()
        .filter(scene_profile::Column::ProjectId.eq(project_id))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    Ok(profiles
        .into_iter()
        .map(|profile| (profile.id, profile.name))
        .collect())
}
