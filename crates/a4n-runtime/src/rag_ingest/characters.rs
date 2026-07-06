use crate::state::AppState;
use a4n_core::models::character::{
    attribute_event, character_state_index, foreshadowing_index, protagonist_profile,
};
use a4n_core::models::project::{blueprint_character, blueprint_relationship};
use a4n_core::rag::RagDataType;
use sea_orm::*;

use super::record_builders::{
    blueprint_character_rag_records, character_attribute_event_rag_record,
    character_state_rag_record, foreshadowing_rag_record, protagonist_rag_records,
    relationship_rag_record,
};
use super::sync::{
    ExpectedSyncStats as IngestStats, RagIngestProgress, RagSyncContext, cleanup_stale_records,
    labeled_rag_record, progress_stage, required_record_dimension, sync_data_type_records,
    sync_labeled_records_concurrent,
};

pub(crate) async fn ingest_protagonists_inner(
    state: &AppState,
    project_id: i64,
    mut progress: Option<&mut RagIngestProgress<'_>>,
) -> Result<IngestStats, String> {
    let mut stats = IngestStats::default();
    let mut sync =
        RagSyncContext::for_data_type(state, project_id, RagDataType::Protagonist).await?;
    let profiles = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    for (profile_index, profile) in profiles.iter().enumerate() {
        let records = match protagonist_rag_records(project_id, profile) {
            Ok(records) => records,
            Err(err) => {
                stats.errors.push(err);
                continue;
            }
        };

        let mut labeled_records = Vec::new();
        for record in records {
            let chunk_id = record.chunk_id.clone();
            let dimension = match required_record_dimension(
                &record,
                format!("主角档案 {} RAG 分块 {chunk_id}", profile.id),
            ) {
                Ok(dimension) => dimension,
                Err(err) => {
                    stats.errors.push(err);
                    continue;
                }
            };
            labeled_records.push(labeled_rag_record(
                record,
                format!("主角档案：{} / {}", profile.name, dimension),
                format!("主角档案 {} 维度 {chunk_id}", profile.id),
            ));
        }
        progress_stage(
            &mut progress,
            format!(
                "正在同步主角档案 {}/{}：{}",
                profile_index + 1,
                profiles.len(),
                profile.name
            ),
        )?;
        let processed_chunks = sync_labeled_records_concurrent(
            &mut sync,
            state,
            labeled_records,
            &mut stats,
            &mut progress,
        )
        .await?;
        if processed_chunks > 0 {
            stats.item_count += 1;
        } else {
            stats
                .errors
                .push(format!("主角档案 {} 未写入任何 RAG 分块", profile.id));
        }
    }
    cleanup_stale_records(&mut sync, state, &mut stats, &mut progress, "主角档案").await?;
    Ok(stats)
}

pub(crate) async fn ingest_character_states_inner(
    state: &AppState,
    project_id: i64,
    mut progress: Option<&mut RagIngestProgress<'_>>,
) -> Result<IngestStats, String> {
    let mut stats = IngestStats::default();
    let profiles = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    let profile_ids = profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<Vec<_>>();
    if profile_ids.is_empty() {
        sync_data_type_records(
            state,
            project_id,
            RagDataType::CharacterState,
            Vec::new(),
            &mut stats,
            &mut progress,
            "角色状态",
        )
        .await?;
        return Ok(stats);
    }
    let profile_names = profiles
        .iter()
        .map(|profile| (profile.id, profile.name.clone()))
        .collect::<std::collections::HashMap<_, _>>();

    let states = character_state_index::Entity::find()
        .filter(character_state_index::Column::ProfileId.is_in(profile_ids))
        .order_by_asc(character_state_index::Column::ChapterNumber)
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    let mut records = Vec::new();
    for state_index in &states {
        let Some(name) = profile_names
            .get(&state_index.profile_id)
            .map(String::as_str)
        else {
            stats.errors.push(format!(
                "角色状态 {} 找不到 profile_id={} 对应角色，跳过 RAG 入库",
                state_index.id, state_index.profile_id
            ));
            continue;
        };
        let record = match character_state_rag_record(project_id, state_index, name) {
            Ok(record) => record,
            Err(err) => {
                stats.errors.push(err);
                continue;
            }
        };
        records.push(labeled_rag_record(
            record,
            format!("角色状态：{} 第{}章", name, state_index.chapter_number),
            format!("角色状态 {}", state_index.id),
        ));
    }
    let processed = sync_data_type_records(
        state,
        project_id,
        RagDataType::CharacterState,
        records,
        &mut stats,
        &mut progress,
        "角色状态",
    )
    .await?;
    stats.item_count = stats.item_count.saturating_add(processed);
    Ok(stats)
}

pub(crate) async fn ingest_character_attribute_events_inner(
    state: &AppState,
    project_id: i64,
    mut progress: Option<&mut RagIngestProgress<'_>>,
) -> Result<IngestStats, String> {
    let mut stats = IngestStats::default();
    let profiles = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    let profile_ids = profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<Vec<_>>();
    if profile_ids.is_empty() {
        sync_data_type_records(
            state,
            project_id,
            RagDataType::CharacterAttributeEvent,
            Vec::new(),
            &mut stats,
            &mut progress,
            "角色属性变更",
        )
        .await?;
        return Ok(stats);
    }
    let profile_names = profiles
        .iter()
        .map(|profile| (profile.id, profile.name.clone()))
        .collect::<std::collections::HashMap<_, _>>();

    let events = attribute_event::Entity::find()
        .filter(attribute_event::Column::ProfileId.is_in(profile_ids))
        .order_by_asc(attribute_event::Column::ChapterNumber)
        .order_by_asc(attribute_event::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    let mut records = Vec::new();
    for event in &events {
        let Some(name) = profile_names.get(&event.profile_id).map(String::as_str) else {
            stats.errors.push(format!(
                "角色属性事件 {} 找不到 profile_id={} 对应角色，跳过 RAG 入库",
                event.id, event.profile_id
            ));
            continue;
        };
        let record = match character_attribute_event_rag_record(project_id, event, name) {
            Ok(record) => record,
            Err(err) => {
                stats.errors.push(err);
                continue;
            }
        };
        records.push(labeled_rag_record(
            record,
            format!("角色属性变更：{} 第{}章", name, event.chapter_number),
            format!("角色属性事件 {}", event.id),
        ));
    }
    let processed = sync_data_type_records(
        state,
        project_id,
        RagDataType::CharacterAttributeEvent,
        records,
        &mut stats,
        &mut progress,
        "角色属性变更",
    )
    .await?;
    stats.item_count = stats.item_count.saturating_add(processed);
    Ok(stats)
}

pub(crate) async fn ingest_foreshadowing_inner(
    state: &AppState,
    project_id: i64,
    mut progress: Option<&mut RagIngestProgress<'_>>,
) -> Result<IngestStats, String> {
    let mut stats = IngestStats::default();
    let items = foreshadowing_index::Entity::find()
        .filter(foreshadowing_index::Column::ProjectId.eq(project_id))
        .order_by_asc(foreshadowing_index::Column::SetupChapter)
        .order_by_asc(foreshadowing_index::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    let mut records = Vec::new();
    for item in &items {
        let record = match foreshadowing_rag_record(project_id, item) {
            Ok(record) => record,
            Err(err) => {
                stats.errors.push(err);
                continue;
            }
        };
        records.push(labeled_rag_record(
            record,
            format!("伏笔：第{}章", item.setup_chapter),
            format!("伏笔 {}", item.id),
        ));
    }
    let processed = sync_data_type_records(
        state,
        project_id,
        RagDataType::Foreshadowing,
        records,
        &mut stats,
        &mut progress,
        "伏笔",
    )
    .await?;
    stats.item_count = stats.item_count.saturating_add(processed);
    Ok(stats)
}

pub(crate) async fn ingest_character_records_inner(
    state: &AppState,
    project_id: i64,
    mut progress: Option<&mut RagIngestProgress<'_>>,
) -> Result<IngestStats, String> {
    let mut stats = IngestStats::default();
    let mut sync = RagSyncContext::for_data_type(state, project_id, RagDataType::Character).await?;
    let blueprint_chars = blueprint_character::Entity::find()
        .filter(blueprint_character::Column::ProjectId.eq(project_id))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    for (character_index, character) in blueprint_chars.iter().enumerate() {
        let records = match blueprint_character_rag_records(project_id, character) {
            Ok(records) => records,
            Err(err) => {
                stats.errors.push(err);
                continue;
            }
        };

        let mut labeled_records = Vec::new();
        for record in records {
            let chunk_id = record.chunk_id.clone();
            let dimension = match required_record_dimension(
                &record,
                format!("蓝图角色 {} RAG 分块 {chunk_id}", character.id),
            ) {
                Ok(dimension) => dimension,
                Err(err) => {
                    stats.errors.push(err);
                    continue;
                }
            };
            labeled_records.push(labeled_rag_record(
                record,
                format!("角色设定：{} / {}", character.name, dimension),
                format!("蓝图角色 {} 维度 {chunk_id}", character.id),
            ));
        }
        progress_stage(
            &mut progress,
            format!(
                "正在同步角色设定 {}/{}：{}",
                character_index + 1,
                blueprint_chars.len(),
                character.name
            ),
        )?;
        let processed_chunks = sync_labeled_records_concurrent(
            &mut sync,
            state,
            labeled_records,
            &mut stats,
            &mut progress,
        )
        .await?;
        if processed_chunks > 0 {
            stats.item_count += 1;
        } else {
            stats
                .errors
                .push(format!("蓝图角色 {} 未写入任何 RAG 分块", character.id));
        }
    }
    cleanup_stale_records(&mut sync, state, &mut stats, &mut progress, "角色设定").await?;
    Ok(stats)
}

pub(crate) async fn ingest_relationships_inner(
    state: &AppState,
    project_id: i64,
    mut progress: Option<&mut RagIngestProgress<'_>>,
) -> Result<IngestStats, String> {
    let mut stats = IngestStats::default();
    let relationships = blueprint_relationship::Entity::find()
        .filter(blueprint_relationship::Column::ProjectId.eq(project_id))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;
    let mut records = Vec::new();
    for relationship in &relationships {
        let record = match relationship_rag_record(project_id, relationship) {
            Ok(record) => record,
            Err(err) => {
                stats.errors.push(err);
                continue;
            }
        };
        records.push(labeled_rag_record(
            record,
            format!(
                "角色关系：{} - {}",
                relationship.character_a, relationship.character_b
            ),
            format!("角色关系 {}", relationship.id),
        ));
    }
    let processed = sync_data_type_records(
        state,
        project_id,
        RagDataType::Relationship,
        records,
        &mut stats,
        &mut progress,
        "角色关系",
    )
    .await?;
    stats.item_count = stats.item_count.saturating_add(processed);
    Ok(stats)
}
