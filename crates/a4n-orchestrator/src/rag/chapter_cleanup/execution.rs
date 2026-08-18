use crate::{OrchestratorContext, project::ChapterDerivedCleanupArtifacts};

use super::plan::ChapterDerivedRagCleanupPlan;

impl OrchestratorContext {
    pub async fn cleanup_chapter_derived_rag(
        &self,
        project_id: i64,
        chapter_id: i64,
        chapter_number: i32,
        artifacts: &ChapterDerivedCleanupArtifacts,
    ) -> Result<(), String> {
        self.execute_chapter_derived_rag_cleanup(ChapterDerivedRagCleanupPlan::new(
            project_id,
            chapter_id,
            chapter_number,
            artifacts,
        ))
        .await
    }

    async fn execute_chapter_derived_rag_cleanup(
        &self,
        plan: ChapterDerivedRagCleanupPlan<'_>,
    ) -> Result<(), String> {
        self.delete_chapter_rag_by_id(plan.project_id, plan.chapter_id)
            .await
            .map_err(|error| {
                format!(
                    "清理章节 RAG 数据失败：chapter_id={}：{error}",
                    plan.chapter_id
                )
            })?;
        self.delete_protagonist_rag_by_ids(plan.project_id, plan.stale_profile_ids)
            .await
            .map_err(|error| {
                format!(
                    "清理角色档案 RAG 数据失败：chapter_number={}, profile_ids={:?}：{error}",
                    plan.chapter_number, plan.stale_profile_ids
                )
            })?;
        self.delete_character_state_rag_by_ids(plan.project_id, plan.stale_character_state_ids)
            .await
            .map_err(|error| {
                format!(
                    "清理角色状态 RAG 数据失败：chapter_number={}, state_ids={:?}：{error}",
                    plan.chapter_number, plan.stale_character_state_ids
                )
            })?;
        self.delete_character_attribute_event_rag_by_ids(
            plan.project_id,
            plan.stale_attribute_event_ids,
        )
        .await
        .map_err(|error| {
            format!(
                "清理角色属性变更 RAG 数据失败：chapter_number={}, attribute_event_ids={:?}：{error}",
                plan.chapter_number, plan.stale_attribute_event_ids
            )
        })?;
        self.delete_foreshadowing_rag_by_ids(plan.project_id, plan.stale_foreshadowing_ids)
            .await
            .map_err(|error| {
                format!(
                    "清理伏笔 RAG 数据失败：chapter_number={}, foreshadowing_ids={:?}：{error}",
                    plan.chapter_number, plan.stale_foreshadowing_ids
                )
            })?;
        self.reingest_foreshadowing_rag_by_ids(plan.project_id, plan.changed_foreshadowing_ids)
            .await
            .map_err(|error| {
                format!(
                    "更新伏笔 RAG 数据失败：chapter_number={}, foreshadowing_ids={:?}：{error}",
                    plan.chapter_number, plan.changed_foreshadowing_ids
                )
            })?;
        Ok(())
    }
}
