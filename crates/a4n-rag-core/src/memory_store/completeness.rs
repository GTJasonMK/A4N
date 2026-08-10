use std::collections::HashSet;

use crate::vector_store::CompletenessReport;

use super::MemoryVectorStore;

impl MemoryVectorStore {
    pub(super) fn memory_completeness_report_by_ids(
        &self,
        collection: &str,
        expected_ids: &[String],
    ) -> CompletenessReport {
        let actual_ids = self
            .collections
            .get(collection)
            .map(|collection| {
                collection
                    .iter()
                    .map(|entry| entry.key().clone())
                    .collect::<HashSet<_>>()
            })
            .unwrap_or_default();
        CompletenessReport::by_expected_ids(
            collection.to_string(),
            expected_ids,
            &actual_ids,
            Some(&actual_ids),
        )
    }
}
