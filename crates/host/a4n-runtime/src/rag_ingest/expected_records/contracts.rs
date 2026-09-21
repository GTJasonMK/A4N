use std::collections::HashSet;

use super::super::sync::RagRecord;

#[derive(Default)]
pub(crate) struct ExpectedRagRecords {
    ids: HashSet<String>,
    records: Vec<RagRecord>,
}

impl ExpectedRagRecords {
    pub(crate) fn total(&self) -> usize {
        self.records.len()
    }

    pub(super) fn add_rag_record(&mut self, record: RagRecord) -> Result<(), String> {
        if !self.ids.insert(record.chunk_id.clone()) {
            return Err(format!("RAG 预期记录 chunk_id 重复: {}", record.chunk_id));
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

    pub(in crate::rag_ingest) fn into_records(self) -> Vec<RagRecord> {
        self.records
    }

    pub(in crate::rag_ingest) fn into_parts(self) -> (HashSet<String>, Vec<RagRecord>) {
        (self.ids, self.records)
    }
}

#[cfg(test)]
mod tests;
