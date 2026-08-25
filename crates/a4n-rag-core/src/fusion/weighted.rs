use std::collections::HashMap;

use a4n_core::rag::RagDataType;

use crate::vector_store::SearchResult;

const RAG_RRF_K: f32 = 60.0;

struct FusionEntry {
    result: SearchResult,
    fused_score: f32,
    representative_multiplier: f32,
}

pub fn fuse_weighted_rag_results<F>(
    batches: Vec<(f32, Vec<SearchResult>)>,
    temporal_multiplier: F,
) -> Vec<SearchResult>
where
    F: Fn(&SearchResult) -> f32,
{
    let mut fused: HashMap<String, FusionEntry> = HashMap::new();
    for (query_weight, results) in batches {
        for (rank, result) in results.into_iter().enumerate() {
            let temporal_multiplier_value = temporal_multiplier(&result);
            let contribution =
                query_weight * temporal_multiplier_value * rag_data_type_weight(&result)
                    / (RAG_RRF_K + rank as f32 + 1.0);
            match fused.entry(result.id.clone()) {
                std::collections::hash_map::Entry::Occupied(mut entry) => {
                    let entry = entry.get_mut();
                    if temporal_multiplier_value > entry.representative_multiplier {
                        entry.result = result;
                        entry.representative_multiplier = temporal_multiplier_value;
                    }
                    entry.fused_score += contribution;
                }
                std::collections::hash_map::Entry::Vacant(entry) => {
                    entry.insert(FusionEntry {
                        result,
                        fused_score: contribution,
                        representative_multiplier: temporal_multiplier_value,
                    });
                }
            }
        }
    }

    let mut results = fused
        .into_values()
        .map(|mut entry| {
            entry.result.score = entry.fused_score;
            entry.result
        })
        .collect::<Vec<_>>();
    results.sort_by(|a, b| b.score.total_cmp(&a.score));
    results
}

fn rag_data_type_weight(result: &SearchResult) -> f32 {
    result
        .metadata
        .get("data_type")
        .and_then(serde_json::Value::as_str)
        .and_then(RagDataType::parse)
        .map(RagDataType::retrieval_weight)
        .unwrap_or(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn result(id: &str, score: f32, data_type: Option<&str>) -> SearchResult {
        SearchResult {
            id: id.to_string(),
            content: format!("content-{id}"),
            score,
            metadata: data_type.map_or_else(|| json!({}), |value| json!({"data_type": value})),
        }
    }

    #[test]
    fn fuses_duplicate_ids() {
        let fused = fuse_weighted_rag_results(
            vec![
                (1.0, vec![result("same", 0.2, None)]),
                (1.0, vec![result("same", 0.8, None)]),
            ],
            |_| 1.0,
        );
        assert_eq!(fused.len(), 1);
        assert_eq!(fused[0].content, "content-same");
        assert!(fused[0].score > 0.0);
    }

    #[test]
    fn applies_temporal_and_data_type_weights_before_sorting() {
        let fused = fuse_weighted_rag_results(
            vec![(
                1.0,
                vec![
                    result("core", 1.0, Some("core_story")),
                    result("other", 1.0, None),
                ],
            )],
            |item| if item.id == "core" { 2.0 } else { 1.0 },
        );
        assert_eq!(fused[0].id, "core");
    }

    #[test]
    fn empty_batches_produce_empty_results() {
        assert!(fuse_weighted_rag_results(Vec::new(), |_| 1.0).is_empty());
    }
}
