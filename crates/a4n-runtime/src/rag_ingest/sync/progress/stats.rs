use a4n_orchestrator::rag::RagRecordSyncOutcome;

#[derive(Default)]
pub struct ExpectedSyncStats {
    pub item_count: u32,
    pub chunk_count: u32,
    pub added_count: u32,
    pub updated_count: u32,
    pub skipped_count: u32,
    pub deleted_count: u32,
    pub errors: Vec<String>,
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

    pub(crate) fn add_item_count(&mut self, count: u32) -> Result<(), String> {
        self.item_count = checked_rag_count_add(self.item_count, count, "RAG 业务记录数")?;
        Ok(())
    }

    pub fn merge(&mut self, other: ExpectedSyncStats) -> Result<(), String> {
        let item_count =
            checked_rag_count_add(self.item_count, other.item_count, "RAG 业务记录数")?;
        let chunk_count = checked_rag_count_add(self.chunk_count, other.chunk_count, "RAG 分块数")?;
        let added_count = checked_rag_count_add(self.added_count, other.added_count, "新增记录数")?;
        let updated_count =
            checked_rag_count_add(self.updated_count, other.updated_count, "更新记录数")?;
        let skipped_count =
            checked_rag_count_add(self.skipped_count, other.skipped_count, "跳过记录数")?;
        let deleted_count =
            checked_rag_count_add(self.deleted_count, other.deleted_count, "删除记录数")?;
        self.item_count = item_count;
        self.chunk_count = chunk_count;
        self.added_count = added_count;
        self.updated_count = updated_count;
        self.skipped_count = skipped_count;
        self.deleted_count = deleted_count;
        self.errors.extend(other.errors);
        Ok(())
    }
}

pub(super) fn checked_rag_count_add(left: u32, right: u32, field: &str) -> Result<u32, String> {
    left.checked_add(right)
        .ok_or_else(|| format!("{field}超过 u32 上限：{left} + {right}"))
}
