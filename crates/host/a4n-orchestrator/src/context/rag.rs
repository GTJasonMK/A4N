//! RAG 领域私有 owner。
//!
//! 持有 `DbHandle`，封装 ingest 读模型与 foreshadowing 查询的数据库行为。
//! 向量库读写仍经 `context/rag_store`；搜索/同步编排仍在 `rag/` 模块。

use a4n_db::DbHandle;
use a4n_db_models::models::{
    chapter::{chapter, part_outline},
    character::{attribute_event, character_state_index, foreshadowing_index, protagonist_profile},
    project::{blueprint_character, blueprint_relationship, novel_blueprint},
};
use a4n_error::AppError;

use super::OrchestratorContext;

/// RAG 领域 application owner（DB 读模型）。
#[derive(Clone)]
pub(super) struct RagApplication {
    database: DbHandle,
}

impl RagApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    async fn chapters_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<chapter::Model>, AppError> {
        a4n_db::rag::chapters_for_rag_ingest(self.database.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    async fn chapter_outlines_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<a4n_db::rag::RagChapterOutlineRow>, AppError> {
        a4n_db::rag::chapter_outlines_for_rag_ingest(self.database.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    async fn part_outlines_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<part_outline::Model>, AppError> {
        a4n_db::rag::part_outlines_for_rag_ingest(self.database.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    async fn novel_blueprints_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<novel_blueprint::Model>, AppError> {
        a4n_db::rag::novel_blueprints_for_rag_ingest(self.database.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    async fn blueprint_characters_for_rag_search(
        &self,
        project_id: i64,
    ) -> Result<Vec<String>, AppError> {
        a4n_db::rag::blueprint_characters_for_rag_search(self.database.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    async fn blueprint_characters_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<blueprint_character::Model>, AppError> {
        a4n_db::rag::blueprint_characters_for_rag_ingest(self.database.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    async fn blueprint_relationships_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<blueprint_relationship::Model>, AppError> {
        a4n_db::rag::blueprint_relationships_for_rag_ingest(self.database.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    async fn protagonist_profiles_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<protagonist_profile::Model>, AppError> {
        a4n_db::rag::protagonist_profiles_for_rag_ingest(self.database.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    async fn character_states_for_rag_ingest(
        &self,
        profile_ids: &[i64],
    ) -> Result<Vec<character_state_index::Model>, AppError> {
        a4n_db::rag::character_states_for_rag_ingest(self.database.connection(), profile_ids)
            .await
            .map_err(AppError::Database)
    }

    async fn character_states_for_rag_ingest_before_chapter(
        &self,
        profile_ids: &[i64],
        chapter_number: i32,
    ) -> Result<Vec<character_state_index::Model>, AppError> {
        a4n_db::rag::character_states_for_rag_ingest_before_chapter(
            self.database.connection(),
            profile_ids,
            chapter_number,
        )
        .await
        .map_err(AppError::Database)
    }

    async fn attribute_events_for_rag_ingest(
        &self,
        profile_ids: &[i64],
    ) -> Result<Vec<attribute_event::Model>, AppError> {
        a4n_db::rag::attribute_events_for_rag_ingest(self.database.connection(), profile_ids)
            .await
            .map_err(AppError::Database)
    }

    async fn attribute_events_for_rag_ingest_before_chapter(
        &self,
        profile_ids: &[i64],
        chapter_number: i32,
    ) -> Result<Vec<attribute_event::Model>, AppError> {
        a4n_db::rag::attribute_events_for_rag_ingest_before_chapter(
            self.database.connection(),
            profile_ids,
            chapter_number,
        )
        .await
        .map_err(AppError::Database)
    }

    async fn scene_profiles_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<a4n_db::rag::RagSceneProfileRow>, AppError> {
        a4n_db::rag::scene_profiles_for_rag_ingest(self.database.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    async fn scene_occurrences_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<a4n_db::rag::RagSceneOccurrenceRow>, AppError> {
        a4n_db::rag::scene_occurrences_for_rag_ingest(self.database.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    async fn scene_transitions_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<a4n_db::rag::RagSceneTransitionRow>, AppError> {
        a4n_db::rag::scene_transitions_for_rag_ingest(self.database.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    async fn foreshadowing_items_by_ids(
        &self,
        project_id: i64,
        foreshadowing_ids: &[i64],
    ) -> Result<Vec<foreshadowing_index::Model>, AppError> {
        a4n_db::rag::foreshadowing_items_by_ids(
            self.database.connection(),
            project_id,
            foreshadowing_ids,
        )
        .await
        .map_err(AppError::Database)
    }

    async fn foreshadowing_items_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<a4n_db::rag::RagForeshadowingRow>, AppError> {
        a4n_db::rag::foreshadowing_items_for_rag_ingest(self.database.connection(), project_id)
            .await
            .map_err(AppError::Database)
    }

    async fn pending_foreshadowing_before_chapter(
        &self,
        project_id: i64,
        chapter_number: i32,
        limit: u64,
    ) -> Result<Vec<foreshadowing_index::Model>, AppError> {
        a4n_db::rag::pending_foreshadowing_before_chapter(
            self.database.connection(),
            project_id,
            chapter_number,
            limit,
        )
        .await
        .map_err(AppError::Database)
    }

    async fn pending_foreshadowing_for_project(
        &self,
        project_id: i64,
        limit: u64,
    ) -> Result<Vec<foreshadowing_index::Model>, AppError> {
        a4n_db::rag::pending_foreshadowing_for_project(
            self.database.connection(),
            project_id,
            limit,
        )
        .await
        .map_err(AppError::Database)
    }

    async fn pending_foreshadowing_search_rows(
        &self,
        project_id: i64,
        chapter_number: Option<i32>,
        limit: u64,
    ) -> Result<Vec<a4n_db::rag::PendingForeshadowingSearchRow>, AppError> {
        a4n_db::rag::pending_foreshadowing_search_rows(
            self.database.connection(),
            project_id,
            chapter_number,
            limit,
        )
        .await
        .map_err(AppError::Database)
    }
}

impl OrchestratorContext {
    pub(crate) async fn chapters_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<chapter::Model>, AppError> {
        self.rag.chapters_for_rag_ingest(project_id).await
    }

    pub(crate) async fn chapter_outlines_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<a4n_db::rag::RagChapterOutlineRow>, AppError> {
        self.rag.chapter_outlines_for_rag_ingest(project_id).await
    }

    pub(crate) async fn part_outlines_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<part_outline::Model>, AppError> {
        self.rag.part_outlines_for_rag_ingest(project_id).await
    }

    pub(crate) async fn novel_blueprints_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<novel_blueprint::Model>, AppError> {
        self.rag.novel_blueprints_for_rag_ingest(project_id).await
    }

    pub(crate) async fn blueprint_characters_for_rag_search(
        &self,
        project_id: i64,
    ) -> Result<Vec<String>, AppError> {
        self.rag
            .blueprint_characters_for_rag_search(project_id)
            .await
    }

    pub(crate) async fn blueprint_characters_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<blueprint_character::Model>, AppError> {
        self.rag
            .blueprint_characters_for_rag_ingest(project_id)
            .await
    }

    pub(crate) async fn blueprint_relationships_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<blueprint_relationship::Model>, AppError> {
        self.rag
            .blueprint_relationships_for_rag_ingest(project_id)
            .await
    }

    pub(crate) async fn protagonist_profiles_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<protagonist_profile::Model>, AppError> {
        self.rag
            .protagonist_profiles_for_rag_ingest(project_id)
            .await
    }

    pub(crate) async fn character_states_for_rag_ingest(
        &self,
        profile_ids: &[i64],
    ) -> Result<Vec<character_state_index::Model>, AppError> {
        self.rag.character_states_for_rag_ingest(profile_ids).await
    }

    pub(crate) async fn character_states_for_rag_ingest_before_chapter(
        &self,
        profile_ids: &[i64],
        chapter_number: i32,
    ) -> Result<Vec<character_state_index::Model>, AppError> {
        self.rag
            .character_states_for_rag_ingest_before_chapter(profile_ids, chapter_number)
            .await
    }

    pub(crate) async fn attribute_events_for_rag_ingest(
        &self,
        profile_ids: &[i64],
    ) -> Result<Vec<attribute_event::Model>, AppError> {
        self.rag.attribute_events_for_rag_ingest(profile_ids).await
    }

    pub(crate) async fn attribute_events_for_rag_ingest_before_chapter(
        &self,
        profile_ids: &[i64],
        chapter_number: i32,
    ) -> Result<Vec<attribute_event::Model>, AppError> {
        self.rag
            .attribute_events_for_rag_ingest_before_chapter(profile_ids, chapter_number)
            .await
    }

    pub(crate) async fn scene_profiles_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<a4n_db::rag::RagSceneProfileRow>, AppError> {
        self.rag.scene_profiles_for_rag_ingest(project_id).await
    }

    pub(crate) async fn scene_occurrences_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<a4n_db::rag::RagSceneOccurrenceRow>, AppError> {
        self.rag.scene_occurrences_for_rag_ingest(project_id).await
    }

    pub(crate) async fn scene_transitions_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<a4n_db::rag::RagSceneTransitionRow>, AppError> {
        self.rag.scene_transitions_for_rag_ingest(project_id).await
    }

    pub(crate) async fn foreshadowing_items_by_ids(
        &self,
        project_id: i64,
        foreshadowing_ids: &[i64],
    ) -> Result<Vec<foreshadowing_index::Model>, AppError> {
        self.rag
            .foreshadowing_items_by_ids(project_id, foreshadowing_ids)
            .await
    }

    pub(crate) async fn foreshadowing_items_for_rag_ingest(
        &self,
        project_id: i64,
    ) -> Result<Vec<a4n_db::rag::RagForeshadowingRow>, AppError> {
        self.rag
            .foreshadowing_items_for_rag_ingest(project_id)
            .await
    }

    pub(crate) async fn pending_foreshadowing_before_chapter(
        &self,
        project_id: i64,
        chapter_number: i32,
        limit: u64,
    ) -> Result<Vec<foreshadowing_index::Model>, AppError> {
        self.rag
            .pending_foreshadowing_before_chapter(project_id, chapter_number, limit)
            .await
    }

    pub(crate) async fn pending_foreshadowing_for_project(
        &self,
        project_id: i64,
        limit: u64,
    ) -> Result<Vec<foreshadowing_index::Model>, AppError> {
        self.rag
            .pending_foreshadowing_for_project(project_id, limit)
            .await
    }

    pub(crate) async fn pending_foreshadowing_search_rows(
        &self,
        project_id: i64,
        chapter_number: Option<i32>,
        limit: u64,
    ) -> Result<Vec<a4n_db::rag::PendingForeshadowingSearchRow>, AppError> {
        self.rag
            .pending_foreshadowing_search_rows(project_id, chapter_number, limit)
            .await
    }
}
