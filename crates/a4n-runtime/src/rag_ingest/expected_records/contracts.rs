use std::collections::HashMap;

use a4n_rag_core::metadata::content_hash;

use super::super::sync::RagRecord;

#[derive(Debug, Clone, Default)]
pub(crate) struct ExpectedRagRecordCounts {
    pub(crate) chapters: usize,
    pub(crate) outlines: usize,
    pub(crate) part_outlines: usize,
    pub(crate) blueprints: usize,
    pub(crate) blueprint_metadata: usize,
    pub(crate) world_settings: usize,
    pub(crate) characters: usize,
    pub(crate) protagonists: usize,
    pub(crate) character_states: usize,
    pub(crate) character_attribute_events: usize,
    pub(crate) foreshadowing: usize,
    pub(crate) relationships: usize,
    pub(crate) scene_profiles: usize,
    pub(crate) scene_occurrences: usize,
    pub(crate) scene_transitions: usize,
}

impl ExpectedRagRecordCounts {
    pub(crate) fn total(&self) -> usize {
        self.chapters
            + self.outlines
            + self.part_outlines
            + self.blueprints
            + self.blueprint_metadata
            + self.world_settings
            + self.characters
            + self.protagonists
            + self.character_states
            + self.character_attribute_events
            + self.foreshadowing
            + self.relationships
            + self.scene_profiles
            + self.scene_occurrences
            + self.scene_transitions
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ExpectedRagRecords {
    pub(crate) counts: ExpectedRagRecordCounts,
    pub(crate) ids: Vec<String>,
    pub(crate) hashes: HashMap<String, String>,
    pub(crate) records: Vec<RagRecord>,
}

impl ExpectedRagRecords {
    pub(crate) fn total(&self) -> usize {
        self.counts.total()
    }

    fn add_record(&mut self, chunk_id: String, text: &str) -> Result<(), String> {
        if self.hashes.contains_key(&chunk_id) {
            return Err(format!("RAG 预期记录 chunk_id 重复: {chunk_id}"));
        }
        let content_hash = content_hash(text).map_err(|e| e.to_string())?;
        self.hashes.insert(chunk_id.clone(), content_hash);
        self.ids.push(chunk_id);
        Ok(())
    }

    pub(super) fn add_rag_record(&mut self, record: RagRecord) -> Result<(), String> {
        let RagRecord { chunk_id, text, .. } = &record;
        self.add_record(chunk_id.clone(), text)
            .map(|_| self.records.push(record))
    }

    pub(super) fn add_rag_records(&mut self, records: Vec<RagRecord>) -> Result<usize, String> {
        let count = records.len();
        for record in records {
            self.add_rag_record(record)?;
        }
        Ok(count)
    }
}

#[cfg(test)]
mod tests;
