//! RAG 领域 runtime 窄 facade。

use std::collections::HashMap;

use a4n_core::{
    error::AppError,
    models::{
        chapter::{chapter, chapter_outline, part_outline},
        character::{
            attribute_event, character_state_index, foreshadowing_index, protagonist_profile,
        },
        project::{blueprint_character, blueprint_relationship, novel_blueprint},
        scene::{scene_occurrence, scene_profile, scene_transition},
    },
    rag::RagDataType,
};
use a4n_embedding::provider::EmbeddingModelIdentity;
use a4n_orchestrator::{
    OrchestratorContext,
    rag::{
        ChapterRagChunkingMode, DeleteRagRecordsInput, EnhancedRagSearchResult,
        PreparedEnhancedRagSearch, PreparedRagRecordSync, RagEmbeddedInput, RagRecord,
        RagSyncContext, RagSyncError,
    },
};
use a4n_rag_core::vector_store::SearchFilter;
use serde_json::Value;

pub(crate) struct RagRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> RagRuntimeFacade<'a> {
    pub(crate) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn chapters_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<chapter::Model>, AppError> {
        self.orchestrator.chapters_for_rag_ingest(project_id).await
    }

    pub(crate) async fn chapter_outlines_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<chapter_outline::Model>, AppError> {
        self.orchestrator
            .chapter_outlines_for_rag_ingest(project_id)
            .await
    }

    pub(crate) async fn part_outlines_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<part_outline::Model>, AppError> {
        self.orchestrator
            .part_outlines_for_rag_ingest(project_id)
            .await
    }

    pub(crate) async fn novel_blueprints_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<novel_blueprint::Model>, AppError> {
        self.orchestrator
            .novel_blueprints_for_rag_ingest(project_id)
            .await
    }

    pub(crate) async fn blueprint_characters_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<blueprint_character::Model>, AppError> {
        self.orchestrator
            .blueprint_characters_for_rag_ingest(project_id)
            .await
    }

    pub(crate) async fn blueprint_relationships_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<blueprint_relationship::Model>, AppError> {
        self.orchestrator
            .blueprint_relationships_for_rag_ingest(project_id)
            .await
    }

    pub(crate) async fn protagonist_profiles_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<protagonist_profile::Model>, AppError> {
        self.orchestrator
            .protagonist_profiles_for_rag_ingest(project_id)
            .await
    }

    pub(crate) async fn character_states_for_rag_ingest(
        &self,
        profile_ids: &[i64],
    ) -> Result<Vec<character_state_index::Model>, AppError> {
        self.orchestrator
            .character_states_for_rag_ingest(profile_ids)
            .await
    }

    pub(crate) async fn attribute_events_for_rag_ingest(
        &self,
        profile_ids: &[i64],
    ) -> Result<Vec<attribute_event::Model>, AppError> {
        self.orchestrator
            .attribute_events_for_rag_ingest(profile_ids)
            .await
    }

    pub(crate) async fn scene_profiles_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<scene_profile::Model>, AppError> {
        self.orchestrator
            .scene_profiles_for_rag_ingest(project_id)
            .await
    }

    pub(crate) async fn scene_occurrences_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<scene_occurrence::Model>, AppError> {
        self.orchestrator
            .scene_occurrences_for_rag_ingest(project_id)
            .await
    }

    pub(crate) async fn scene_transitions_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<scene_transition::Model>, AppError> {
        self.orchestrator
            .scene_transitions_for_rag_ingest(project_id)
            .await
    }

    pub(crate) async fn foreshadowing_items_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<foreshadowing_index::Model>, AppError> {
        self.orchestrator
            .foreshadowing_items_for_rag_ingest(project_id)
            .await
    }

    pub(crate) async fn chapter_rag_records(
        &self,
        chapter: &chapter::Model,
        mode: ChapterRagChunkingMode,
    ) -> Result<Vec<RagRecord>, String> {
        self.orchestrator.chapter_rag_records(chapter, mode).await
    }

    pub(crate) fn embedding_batch_size(&self) -> usize {
        self.orchestrator.embedding_batch_size()
    }

    pub(crate) async fn embedding_model_identity(
        &self,
    ) -> Result<EmbeddingModelIdentity, AppError> {
        self.orchestrator.embedding_model_identity().await
    }

    pub(crate) async fn project_rag_metadata(
        &self,
        project_id: i64,
    ) -> Result<HashMap<String, Value>, RagSyncError> {
        self.orchestrator.project_rag_metadata(project_id).await
    }

    /// 构建按 data_type 前缀的 RAG 同步上下文（Host 不得直取完整 OrchestratorContext）。
    pub(crate) async fn rag_sync_context_for_data_type(
        &self,
        project_id: i64,
        data_type: RagDataType,
    ) -> Result<RagSyncContext, RagSyncError> {
        RagSyncContext::for_data_type(self.orchestrator, project_id, data_type).await
    }

    /// 构建按 id 前缀的 RAG 同步上下文。
    pub(crate) async fn rag_sync_context_for_id_prefixes(
        &self,
        project_id: i64,
        prefixes: Vec<String>,
    ) -> Result<RagSyncContext, RagSyncError> {
        RagSyncContext::for_id_prefixes(self.orchestrator, project_id, prefixes).await
    }

    /// 清理同步上下文中的过期记录。
    pub(crate) async fn cleanup_rag_sync_stale(
        &self,
        sync: &mut RagSyncContext,
    ) -> Result<u32, RagSyncError> {
        sync.cleanup_stale(self.orchestrator).await
    }

    pub(crate) async fn store_prepared_rag_records<'b, I>(
        &self,
        project_id: i64,
        records: I,
    ) -> Result<(), RagSyncError>
    where
        I: IntoIterator<Item = &'b PreparedRagRecordSync>,
    {
        self.orchestrator
            .store_prepared_rag_records(project_id, records)
            .await
    }

    pub(crate) async fn store_embedded_rag_records(
        &self,
        project_id: i64,
        inputs: Vec<RagEmbeddedInput>,
        embedding_identity: EmbeddingModelIdentity,
    ) -> Result<usize, AppError> {
        self.orchestrator
            .store_embedded_rag_records(project_id, inputs, embedding_identity)
            .await
    }

    pub(crate) async fn delete_rag_records(
        &self,
        input: DeleteRagRecordsInput,
    ) -> Result<usize, RagSyncError> {
        self.orchestrator.delete_rag_records(input).await
    }

    pub(crate) async fn clear_project_rag(&self, project_id: i64) -> Result<(), RagSyncError> {
        self.orchestrator.clear_project_rag(project_id).await
    }

    pub(crate) async fn sync_chapter_rag_records(
        &self,
        chapter: &chapter::Model,
        mode: ChapterRagChunkingMode,
    ) -> Result<usize, String> {
        self.orchestrator
            .sync_chapter_rag_records(chapter, mode)
            .await
    }

    pub(crate) async fn prepare_enhanced_rag_queries(
        &self,
        project_id: i64,
        base_query: &str,
        chapter_number: Option<i32>,
    ) -> Result<Vec<a4n_orchestrator::rag::EnhancedRagQuery>, AppError> {
        self.orchestrator
            .prepare_enhanced_rag_queries(project_id, base_query, chapter_number)
            .await
    }

    pub(crate) async fn enhanced_rag_search_filter(
        &self,
        project_id: i64,
        embedding_identity: &EmbeddingModelIdentity,
        data_types: Option<Vec<String>>,
    ) -> Result<SearchFilter, AppError> {
        self.orchestrator
            .enhanced_rag_search_filter(project_id, embedding_identity, data_types)
            .await
    }

    pub(crate) async fn execute_prepared_enhanced_rag_search(
        &self,
        project_id: i64,
        prepared: &PreparedEnhancedRagSearch,
    ) -> Result<EnhancedRagSearchResult, AppError> {
        self.orchestrator
            .execute_prepared_enhanced_rag_search(project_id, prepared)
            .await
    }

    pub(crate) async fn execute_prepared_enhanced_rag_search_with_vectors(
        &self,
        project_id: i64,
        prepared: &PreparedEnhancedRagSearch,
        embedding_identity: &EmbeddingModelIdentity,
        query_vectors: Vec<Vec<f32>>,
    ) -> Result<EnhancedRagSearchResult, AppError> {
        self.orchestrator
            .execute_prepared_enhanced_rag_search_with_vectors(
                project_id,
                prepared,
                embedding_identity,
                query_vectors,
            )
            .await
    }

    pub(crate) async fn cleanup_chapter_derived_rag(
        &self,
        project_id: i64,
        chapter_id: i64,
        chapter_number: i32,
        artifacts: &a4n_orchestrator::project::ChapterDerivedCleanupArtifacts,
    ) -> Result<(), String> {
        self.orchestrator
            .cleanup_chapter_derived_rag(project_id, chapter_id, chapter_number, artifacts)
            .await
    }
}
