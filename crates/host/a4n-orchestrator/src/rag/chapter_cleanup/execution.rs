use a4n_domain::project::ChapterDerivedCleanupArtifacts;

use crate::OrchestratorContext;

impl OrchestratorContext {
    pub async fn cleanup_chapter_derived_rag(
        &self,
        project_id: i64,
        chapter_id: i64,
        chapter_number: i32,
        artifacts: &ChapterDerivedCleanupArtifacts,
    ) -> Result<(), String> {
        self.delete_chapter_rag_by_id(project_id, chapter_id)
            .await
            .map_err(|error| format!("清理章节 RAG 数据失败：chapter_id={chapter_id}：{error}"))?;
        self.delete_character_state_rag_by_ids(project_id, &artifacts.stale_character_state_ids)
            .await
            .map_err(|error| {
                format!(
                    "清理角色状态 RAG 数据失败：chapter_number={chapter_number}, state_ids={:?}：{error}",
                    artifacts.stale_character_state_ids
                )
            })?;
        self.delete_character_attribute_event_rag_by_ids(
            project_id,
            &artifacts.stale_attribute_event_ids,
        )
        .await
        .map_err(|error| {
            format!(
                "清理角色属性变更 RAG 数据失败：chapter_number={chapter_number}, attribute_event_ids={:?}：{error}",
                artifacts.stale_attribute_event_ids
            )
        })?;
        self.reingest_foreshadowing_rag_by_ids(project_id, &artifacts.changed_foreshadowing_ids)
            .await
            .map_err(|error| {
                format!(
                    "更新伏笔 RAG 数据失败：chapter_number={chapter_number}, foreshadowing_ids={:?}：{error}",
                    artifacts.changed_foreshadowing_ids
                )
            })?;
        Ok(())
    }
}
