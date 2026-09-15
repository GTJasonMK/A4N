use std::collections::{HashMap, HashSet};

use a4n_domain::rag::RagDataType;
use a4n_embedding_provider_contract::EmbeddingModelIdentity;
use a4n_error::AppError;
use a4n_rag_contract::{RagIngestPort, RagIngestPreparation};
use a4n_rag_core::{
    ChapterRagChunkingMode, DeleteRagRecordsInput, PreparedRagRecordSync, RagEmbeddedInput,
    RagRecord, RagSyncError,
};
use serde_json::Value;

use super::OrchestratorContext;

fn add_record(
    records: &mut Vec<RagRecord>,
    ids: &mut HashSet<String>,
    record: RagRecord,
) -> Result<(), AppError> {
    if !ids.insert(record.chunk_id.clone()) {
        return Err(AppError::InvalidState {
            current: format!("RAG 预期记录 chunk_id 重复: {}", record.chunk_id),
            operation: "构造 RAG 预期记录".into(),
        });
    }
    records.push(record);
    Ok(())
}

async fn append_chapter_records(
    context: &OrchestratorContext,
    project_id: i64,
    chapter_mode: ChapterRagChunkingMode,
    records: &mut Vec<RagRecord>,
    ids: &mut HashSet<String>,
) -> Result<(), AppError> {
    let chapters = context.chapters_for_rag_ingest(project_id).await?;
    for chapter in chapters {
        if !chapter.has_available_content() {
            continue;
        }
        for record in context
            .chapter_rag_records(&chapter, chapter_mode)
            .await
            .map_err(|error| AppError::InvalidArgument(format!("构造章节 RAG 记录失败: {error}")))?
        {
            add_record(records, ids, record)?;
        }
    }
    Ok(())
}

async fn append_outline_records(
    context: &OrchestratorContext,
    project_id: i64,
    records: &mut Vec<RagRecord>,
    ids: &mut HashSet<String>,
) -> Result<(), AppError> {
    for outline in context.chapter_outlines_for_rag_ingest(project_id).await? {
        add_record(
            records,
            ids,
            crate::rag::outline_rag_record_from_row(project_id, &outline)
                .map_err(AppError::InvalidArgument)?,
        )?;
    }
    for part in context.part_outlines_for_rag_ingest(project_id).await? {
        add_record(
            records,
            ids,
            crate::rag::part_outline_rag_record(project_id, &part)
                .map_err(AppError::InvalidArgument)?,
        )?;
    }
    Ok(())
}

async fn append_blueprint_records(
    context: &OrchestratorContext,
    project_id: i64,
    records: &mut Vec<RagRecord>,
    ids: &mut HashSet<String>,
) -> Result<(), AppError> {
    for blueprint in context.novel_blueprints_for_rag_ingest(project_id).await? {
        add_record(
            records,
            ids,
            crate::rag::blueprint_rag_record(project_id, &blueprint)
                .map_err(AppError::InvalidArgument)?,
        )?;
        add_record(
            records,
            ids,
            crate::rag::blueprint_metadata_rag_record(project_id, &blueprint)
                .map_err(AppError::InvalidArgument)?,
        )?;
        for record in crate::rag::blueprint_world_setting_rag_records(project_id, &blueprint)
            .map_err(AppError::InvalidArgument)?
        {
            add_record(records, ids, record)?;
        }
    }
    Ok(())
}

async fn append_blueprint_character_records(
    context: &OrchestratorContext,
    project_id: i64,
    records: &mut Vec<RagRecord>,
    ids: &mut HashSet<String>,
) -> Result<(), AppError> {
    for character in context
        .blueprint_characters_for_rag_ingest(project_id)
        .await?
    {
        for record in crate::rag::blueprint_character_rag_records(project_id, &character)
            .map_err(AppError::InvalidArgument)?
        {
            add_record(records, ids, record)?;
        }
    }
    Ok(())
}

async fn append_protagonist_records(
    context: &OrchestratorContext,
    project_id: i64,
    records: &mut Vec<RagRecord>,
    ids: &mut HashSet<String>,
) -> Result<(), AppError> {
    for profile in context
        .protagonist_profiles_for_rag_ingest(project_id)
        .await?
    {
        for record in crate::rag::protagonist_rag_records(project_id, &profile)
            .map_err(AppError::InvalidArgument)?
        {
            add_record(records, ids, record)?;
        }
    }
    Ok(())
}

async fn append_character_records(
    context: &OrchestratorContext,
    project_id: i64,
    records: &mut Vec<RagRecord>,
    ids: &mut HashSet<String>,
) -> Result<(), AppError> {
    append_blueprint_character_records(context, project_id, records, ids).await?;
    let profiles = context
        .protagonist_profiles_for_rag_ingest(project_id)
        .await?;
    let profile_names = profiles
        .iter()
        .map(|profile| (profile.id, profile.name.as_str()))
        .collect::<HashMap<_, _>>();
    for profile in &profiles {
        for record in crate::rag::protagonist_rag_records(project_id, profile)
            .map_err(AppError::InvalidArgument)?
        {
            add_record(records, ids, record)?;
        }
    }
    let profile_ids = profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<Vec<_>>();
    if !profile_ids.is_empty() {
        for state in context
            .character_states_for_rag_ingest(&profile_ids)
            .await?
        {
            let name = profile_names
                .get(&state.profile_id)
                .copied()
                .ok_or_else(|| AppError::InvalidState {
                    current: format!("角色状态 {} 缺少 profile {}", state.id, state.profile_id),
                    operation: "构造角色状态 RAG 记录".into(),
                })?;
            add_record(
                records,
                ids,
                crate::rag::character_state_rag_record(project_id, &state, name)
                    .map_err(AppError::InvalidArgument)?,
            )?;
        }
        for event in context
            .attribute_events_for_rag_ingest(&profile_ids)
            .await?
        {
            let name = profile_names
                .get(&event.profile_id)
                .copied()
                .ok_or_else(|| AppError::InvalidState {
                    current: format!(
                        "角色属性事件 {} 缺少 profile {}",
                        event.id, event.profile_id
                    ),
                    operation: "构造角色属性 RAG 记录".into(),
                })?;
            add_record(
                records,
                ids,
                crate::rag::character_attribute_event_rag_record(project_id, &event, name)
                    .map_err(AppError::InvalidArgument)?,
            )?;
        }
    }
    for item in context
        .foreshadowing_items_for_rag_ingest(project_id)
        .await?
    {
        add_record(
            records,
            ids,
            crate::rag::foreshadowing_rag_record_from_row(project_id, &item)
                .map_err(AppError::InvalidArgument)?,
        )?;
    }
    for relationship in context
        .blueprint_relationships_for_rag_ingest(project_id)
        .await?
    {
        add_record(
            records,
            ids,
            crate::rag::relationship_rag_record(project_id, &relationship)
                .map_err(AppError::InvalidArgument)?,
        )?;
    }
    Ok(())
}

async fn append_scene_records(
    context: &OrchestratorContext,
    project_id: i64,
    records: &mut Vec<RagRecord>,
    ids: &mut HashSet<String>,
) -> Result<(), AppError> {
    let profiles = context.scene_profiles_for_rag_ingest(project_id).await?;
    let scene_names = profiles
        .iter()
        .map(|profile| (profile.id, profile.name.as_str()))
        .collect::<HashMap<_, _>>();
    for profile in &profiles {
        add_record(
            records,
            ids,
            crate::rag::scene_profile_rag_record(project_id, profile)
                .map_err(AppError::InvalidArgument)?,
        )?;
    }
    for occurrence in context.scene_occurrences_for_rag_ingest(project_id).await? {
        let name = scene_names
            .get(&occurrence.scene_profile_id)
            .copied()
            .ok_or_else(|| AppError::InvalidState {
                current: format!(
                    "场景出场 {} 缺少 scene profile {}",
                    occurrence.id, occurrence.scene_profile_id
                ),
                operation: "构造场景出场 RAG 记录".into(),
            })?;
        add_record(
            records,
            ids,
            crate::rag::scene_occurrence_rag_record(project_id, &occurrence, name)
                .map_err(AppError::InvalidArgument)?,
        )?;
    }
    for transition in context.scene_transitions_for_rag_ingest(project_id).await? {
        let to_name = scene_names
            .get(&transition.to_scene_profile_id)
            .copied()
            .ok_or_else(|| AppError::InvalidState {
                current: format!(
                    "场景位移 {} 缺少目标 scene profile {}",
                    transition.id, transition.to_scene_profile_id
                ),
                operation: "构造场景位移 RAG 记录".into(),
            })?;
        let from_name = transition
            .from_scene_profile_id
            .map(|id| {
                scene_names
                    .get(&id)
                    .copied()
                    .ok_or_else(|| AppError::InvalidState {
                        current: format!(
                            "场景位移 {} 缺少来源 scene profile {}",
                            transition.id, id
                        ),
                        operation: "构造场景位移 RAG 记录".into(),
                    })
            })
            .transpose()?;
        add_record(
            records,
            ids,
            crate::rag::scene_transition_rag_record(project_id, &transition, from_name, to_name)
                .map_err(AppError::InvalidArgument)?,
        )?;
    }
    Ok(())
}

pub(crate) fn map_sync_error(error: RagSyncError) -> AppError {
    match error {
        RagSyncError::InvalidRecord { message }
        | RagSyncError::EmbeddingIdentityMismatch { message } => AppError::InvalidArgument(message),
        RagSyncError::ReadStoredMetadata { message }
        | RagSyncError::ReadEmbeddingIdentity { message }
        | RagSyncError::Store { message }
        | RagSyncError::Delete { message }
        | RagSyncError::DeleteStale { message } => AppError::VectorStoreError(message),
        RagSyncError::DeletedCountOverflow { deleted } => AppError::InvalidState {
            current: format!("清理过期 RAG 记录数量超出 u32: {deleted}"),
            operation: "清理过期 RAG 记录".into(),
        },
    }
}

#[async_trait::async_trait]
impl RagIngestPort for OrchestratorContext {
    async fn sync_chapter_records(
        &self,
        project_id: i64,
        chapter_id: i64,
        chapter_mode: ChapterRagChunkingMode,
    ) -> Result<u32, AppError> {
        let chapter = self
            .chapter_by_id(chapter_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("章节不存在: chapter_id={chapter_id}")))?;
        if chapter.project_id != project_id {
            return Err(AppError::NotFound(format!(
                "项目 {project_id} 不存在章节 {chapter_id}"
            )));
        }
        self.sync_chapter_rag_records(&chapter, chapter_mode)
            .await
            .map_err(AppError::VectorStoreError)
            .and_then(|count| {
                u32::try_from(count).map_err(|_| AppError::InvalidState {
                    current: format!("章节 RAG 记录数量超出 u32: {count}"),
                    operation: "同步章节 RAG 记录".to_string(),
                })
            })
    }

    async fn expected_records(
        &self,
        project_id: i64,
        set: a4n_rag_contract::RagRecordSet,
        chapter_mode: ChapterRagChunkingMode,
    ) -> Result<Vec<RagRecord>, AppError> {
        let mut records = Vec::new();
        let mut ids = HashSet::new();
        match set {
            a4n_rag_contract::RagRecordSet::All => {
                append_chapter_records(self, project_id, chapter_mode, &mut records, &mut ids)
                    .await?;
                append_outline_records(self, project_id, &mut records, &mut ids).await?;
                append_blueprint_records(self, project_id, &mut records, &mut ids).await?;
                append_character_records(self, project_id, &mut records, &mut ids).await?;
                append_scene_records(self, project_id, &mut records, &mut ids).await?;
            }
            a4n_rag_contract::RagRecordSet::Outline => {
                append_outline_records(self, project_id, &mut records, &mut ids).await?;
            }
            a4n_rag_contract::RagRecordSet::BlueprintMaterial => {
                append_blueprint_records(self, project_id, &mut records, &mut ids).await?;
                append_blueprint_character_records(self, project_id, &mut records, &mut ids)
                    .await?;
                append_protagonist_records(self, project_id, &mut records, &mut ids).await?;
                for relationship in self
                    .blueprint_relationships_for_rag_ingest(project_id)
                    .await?
                {
                    add_record(
                        &mut records,
                        &mut ids,
                        crate::rag::relationship_rag_record(project_id, &relationship)
                            .map_err(AppError::InvalidArgument)?,
                    )?;
                }
            }
            a4n_rag_contract::RagRecordSet::Protagonist => {
                append_protagonist_records(self, project_id, &mut records, &mut ids).await?;
            }
            a4n_rag_contract::RagRecordSet::Scene => {
                append_scene_records(self, project_id, &mut records, &mut ids).await?;
            }
        }
        Ok(records)
    }

    async fn prepare_rag_sync(
        &self,
        project_id: i64,
        data_types: Option<Vec<RagDataType>>,
    ) -> Result<RagIngestPreparation, AppError> {
        let sync = match data_types {
            Some(data_types) => {
                crate::rag::RagSyncContext::for_data_types(self, project_id, &data_types).await
            }
            None => crate::rag::RagSyncContext::for_project(self, project_id).await,
        }
        .map_err(map_sync_error)?;
        let (embedding_identity, stored_metadata, stale_ids) = sync.into_port_snapshot();
        Ok(RagIngestPreparation {
            embedding_identity,
            stored_metadata,
            stale_ids,
        })
    }

    async fn project_rag_metadata(
        &self,
        project_id: i64,
    ) -> Result<HashMap<String, Value>, AppError> {
        OrchestratorContext::project_rag_metadata(self, project_id)
            .await
            .map_err(map_sync_error)
    }

    async fn commit_embedded_rag_records(
        &self,
        project_id: i64,
        rebuild: bool,
        inputs: Vec<RagEmbeddedInput>,
        embedding_identity: EmbeddingModelIdentity,
        stale_ids: Vec<String>,
    ) -> Result<u32, AppError> {
        if rebuild {
            OrchestratorContext::clear_project_rag(self, project_id)
                .await
                .map_err(map_sync_error)?;
        }
        OrchestratorContext::store_embedded_rag_records(
            self,
            project_id,
            inputs,
            embedding_identity,
        )
        .await?;
        if stale_ids.is_empty() {
            return Ok(0);
        }
        let deleted = OrchestratorContext::delete_rag_records(
            self,
            DeleteRagRecordsInput {
                project_id,
                ids: stale_ids,
                id_prefixes: Vec::new(),
            },
        )
        .await
        .map_err(map_sync_error)?;
        u32::try_from(deleted).map_err(|_| AppError::InvalidState {
            current: format!("清理过期 RAG 记录数量超出 u32: {deleted}"),
            operation: "清理过期 RAG 记录".into(),
        })
    }

    async fn clear_project_rag(&self, project_id: i64) -> Result<(), AppError> {
        OrchestratorContext::clear_project_rag(self, project_id)
            .await
            .map_err(map_sync_error)
    }

    async fn store_prepared_rag_records(
        &self,
        project_id: i64,
        records: Vec<PreparedRagRecordSync>,
    ) -> Result<(), AppError> {
        OrchestratorContext::store_prepared_rag_records(self, project_id, records)
            .await
            .map_err(map_sync_error)
    }

    async fn cleanup_stale_rag_records(
        &self,
        project_id: i64,
        stale_ids: Vec<String>,
    ) -> Result<u32, AppError> {
        if stale_ids.is_empty() {
            return Ok(0);
        }
        let deleted = OrchestratorContext::delete_rag_records(
            self,
            DeleteRagRecordsInput {
                project_id,
                ids: stale_ids,
                id_prefixes: Vec::new(),
            },
        )
        .await
        .map_err(map_sync_error)?;
        u32::try_from(deleted).map_err(|_| AppError::InvalidState {
            current: format!("清理过期 RAG 记录数量超出 u32: {deleted}"),
            operation: "清理过期 RAG 记录".into(),
        })
    }

    fn embedding_batch_size(&self) -> usize {
        OrchestratorContext::embedding_batch_size(self)
    }
}
