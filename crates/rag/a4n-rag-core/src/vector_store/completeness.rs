use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashSet};

/// 完整性报告
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompletenessReport {
    pub collection: String,
    pub expected: usize,
    pub actual_vector: usize,
    pub actual_bm25: usize,
    pub is_complete: bool,
    pub missing_vector_ids: Vec<String>,
    pub missing_bm25_ids: Vec<String>,
    pub missing_ids: Vec<String>,
}

impl CompletenessReport {
    pub fn by_expected_ids(
        collection: impl Into<String>,
        expected_ids: &[String],
        actual_vector_ids: &HashSet<String>,
        actual_bm25_ids: Option<&HashSet<String>>,
    ) -> Self {
        let missing_vector_ids = missing_expected_ids(expected_ids, actual_vector_ids);
        let missing_bm25_ids = actual_bm25_ids
            .map(|ids| missing_expected_ids(expected_ids, ids))
            .unwrap_or_default();
        let missing_ids = merged_missing_ids(&missing_vector_ids, &missing_bm25_ids);
        Self {
            collection: collection.into(),
            expected: expected_ids.len(),
            actual_vector: actual_vector_ids.len(),
            actual_bm25: actual_bm25_ids.map(HashSet::len).unwrap_or(0),
            is_complete: missing_vector_ids.is_empty() && missing_bm25_ids.is_empty(),
            missing_vector_ids,
            missing_bm25_ids,
            missing_ids,
        }
    }
}

fn missing_expected_ids(expected_ids: &[String], actual_ids: &HashSet<String>) -> Vec<String> {
    expected_ids
        .iter()
        .filter(|id| !actual_ids.contains(*id))
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn merged_missing_ids(vector_ids: &[String], bm25_ids: &[String]) -> Vec<String> {
    vector_ids
        .iter()
        .chain(bm25_ids)
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
