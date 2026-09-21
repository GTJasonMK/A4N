use a4n_db_models::models::chapter::chapter;

use crate::{
    OrchestratorContext,
    rag::{RagSyncContext, chapter_rag_chunk_prefix},
};
use a4n_rag_core::ChapterRagChunkingMode;

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
        let batch_size = self.embedding_batch_size();
        if batch_size == 0 {
            return Err("RAG embedding provider batch_size 必须大于 0".to_string());
        }
        let prepared = records
            .into_iter()
            .map(|record| sync.prepare_record(record))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("同步章节 RAG 失败：{error}"))?;
        let mut pending = prepared.into_iter();
        loop {
            let batch = pending.by_ref().take(batch_size).collect::<Vec<_>>();
            if batch.is_empty() {
                break;
            }
            let chunk_ids = batch
                .iter()
                .map(|record| record.chunk_id().to_string())
                .collect::<Vec<_>>();
            self.store_prepared_rag_records(chapter.project_id, batch)
                .await
                .map_err(|error| {
                    format!("同步章节 RAG 失败 [{}]：{error}", chunk_ids.join(", "))
                })?;
        }
        sync.cleanup_stale(self).await.map_err(|error| {
            format!("清理章节过期 RAG 失败：chapter_id={}：{error}", chapter.id)
        })?;
        Ok(processed)
    }
}
