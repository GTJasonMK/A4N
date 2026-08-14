use a4n_orchestrator::rag::RagRecordSyncOutcome;

#[derive(Default)]
pub struct ExpectedSyncStats {
    pub chunk_count: u32,
    pub added_count: u32,
    pub updated_count: u32,
    pub skipped_count: u32,
    pub deleted_count: u32,
}

impl ExpectedSyncStats {
    pub(crate) fn record_outcome(&mut self, outcome: RagRecordSyncOutcome) -> Result<(), String> {
        match outcome {
            RagRecordSyncOutcome::Added => {
                let added_count = checked_rag_count_add(self.added_count, 1, "新增记录数")?;
                let chunk_count = checked_rag_count_add(self.chunk_count, 1, "RAG 分块数")?;
                self.added_count = added_count;
                self.chunk_count = chunk_count;
            }
            RagRecordSyncOutcome::Updated => {
                let updated_count = checked_rag_count_add(self.updated_count, 1, "更新记录数")?;
                let chunk_count = checked_rag_count_add(self.chunk_count, 1, "RAG 分块数")?;
                self.updated_count = updated_count;
                self.chunk_count = chunk_count;
            }
            RagRecordSyncOutcome::Skipped => {
                self.skipped_count = checked_rag_count_add(self.skipped_count, 1, "跳过记录数")?;
            }
        }
        Ok(())
    }

    pub(crate) fn add_deleted(&mut self, count: u32) -> Result<(), String> {
        self.deleted_count = checked_rag_count_add(self.deleted_count, count, "删除记录数")?;
        Ok(())
    }

    pub(crate) fn into_result(self) -> a4n_api_contract::rag::RagIngestResult {
        a4n_api_contract::rag::RagIngestResult {
            chunk_count: self.chunk_count,
            added_count: self.added_count,
            updated_count: self.updated_count,
            skipped_count: self.skipped_count,
            deleted_count: self.deleted_count,
        }
    }
}

pub(super) fn checked_rag_count_add(left: u32, right: u32, field: &str) -> Result<u32, String> {
    left.checked_add(right)
        .ok_or_else(|| format!("{field}超过 u32 上限：{left} + {right}"))
}
