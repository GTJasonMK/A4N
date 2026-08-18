use a4n_core::models::chapter::chapter;

use super::contracts::ChapterRagChunkingMode;
use crate::{
    OrchestratorContext,
    rag::{RagSyncContext, chapter_rag_chunk_prefix},
};

impl OrchestratorContext {
    pub async fn sync_chapter_rag_records(
        &self,
        chapter: &chapter::Model,
        mode: ChapterRagChunkingMode,
    ) -> Result<usize, String> {
        let records = self.chapter_rag_records(chapter, mode).await?;
        let mut sync = RagSyncContext::for_id_prefixes(
            self,
            chapter.project_id,
            vec![chapter_rag_chunk_prefix(chapter.project_id, chapter.id)],
        )
        .await
        .map_err(|error| error.to_string())?;

        let processed = records.len();
        for record in records {
            let chunk_id = record.chunk_id.clone();
            sync.sync_record(self, record)
                .await
                .map_err(|error| format!("同步章节 RAG 失败：{chunk_id}：{error}"))?;
        }
        sync.cleanup_stale(self).await.map_err(|error| {
            format!("清理章节过期 RAG 失败：chapter_id={}：{error}", chapter.id)
        })?;
        Ok(processed)
    }
}
