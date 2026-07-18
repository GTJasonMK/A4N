use std::collections::HashSet;

use a4n_core::error::AppError;

use crate::vector_store::CompletenessReport;

use super::MemoryVectorStore;
use super::records::vector_store_error;

impl MemoryVectorStore {
    pub(super) async fn memory_completeness_report(
        &self,
        collection: &str,
        expected_count: usize,
    ) -> Result<CompletenessReport, AppError> {
        let actual = self
            .collections
            .get(collection)
            .map(|collection| collection.len())
            .ok_or_else(|| vector_store_error(format!("集合不存在：{collection}")))?;
        let missing = if actual < expected_count {
            (actual..expected_count)
                .map(|index| format!("missing-{index}"))
                .collect()
        } else {
            Vec::new()
        };
        Ok(CompletenessReport::count_only(
            collection.to_string(),
            expected_count,
            actual,
            actual,
            missing.clone(),
            missing,
        ))
    }

    pub(super) async fn memory_completeness_report_by_ids(
        &self,
        collection: &str,
        expected_ids: &[String],
    ) -> CompletenessReport {
        let Some(actual_ids) = self.collections.get(collection).map(|collection| {
            collection
                .iter()
                .map(|entry| entry.key().clone())
                .collect::<HashSet<_>>()
        }) else {
            return CompletenessReport::from_missing_ids(
                collection.to_string(),
                expected_ids.len(),
                0,
                0,
                expected_ids.to_vec(),
                expected_ids.to_vec(),
            );
        };
        CompletenessReport::by_expected_ids(
            collection.to_string(),
            expected_ids,
            &actual_ids,
            Some(&actual_ids),
        )
    }
}
