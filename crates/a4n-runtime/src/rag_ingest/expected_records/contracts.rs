use std::collections::HashSet;

use super::super::sync::RagRecord;

#[derive(Debug, Clone, Default)]
pub(crate) struct ExpectedRagRecords {
    pub(crate) ids: HashSet<String>,
    pub(crate) records: Vec<RagRecord>,
}

impl ExpectedRagRecords {
    pub(crate) fn total(&self) -> usize {
        self.records.len()
    }

    pub(super) fn add_rag_record(&mut self, record: RagRecord) -> Result<(), String> {
        let chunk_id = record.chunk_id.clone();
        if !self.ids.insert(chunk_id.clone()) {
            return Err(format!("RAG 预期记录 chunk_id 重复: {chunk_id}"));
        }
        self.records.push(record);
        Ok(())
    }

    pub(super) fn add_rag_records(&mut self, records: Vec<RagRecord>) -> Result<(), String> {
        for record in records {
            self.add_rag_record(record)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
