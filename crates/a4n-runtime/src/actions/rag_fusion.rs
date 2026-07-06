use a4n_core::error::AppError;
use a4n_core::rag::RagDataType;
use a4n_engine::rag::vector_store::SearchResult;
use std::collections::HashMap;

const RAG_RRF_K: f32 = 60.0;

struct FusionEntry {
    result: SearchResult,
    fused_score: f32,
    best_source_score: f32,
}

pub(crate) fn ensure_rag_scores_finite(
    batches: &[(f32, Vec<SearchResult>)],
) -> Result<(), AppError> {
    for (_, results) in batches {
        for result in results {
            if !result.score.is_finite() {
                return Err(AppError::InvalidArgument(format!(
                    "RAG 结果 {} score 必须是有限数",
                    result.id
                )));
            }
        }
    }
    Ok(())
}

pub(crate) fn fuse_weighted_rag_results<F>(
    batches: Vec<(f32, Vec<SearchResult>)>,
    temporal_multiplier: F,
) -> Vec<SearchResult>
where
    F: Fn(&SearchResult) -> f32,
{
    let mut fused: HashMap<String, FusionEntry> = HashMap::new();
    for (query_weight, results) in batches {
        for (rank, result) in results.into_iter().enumerate() {
            let contribution =
                query_weight * temporal_multiplier(&result) * rag_data_type_weight(&result)
                    / (RAG_RRF_K + rank as f32 + 1.0);
            let source_score = if result.score.is_finite() {
                result.score
            } else {
                0.0
            };
            let entry = fused
                .entry(result.id.clone())
                .or_insert_with(|| FusionEntry {
                    result: result.clone(),
                    fused_score: 0.0,
                    best_source_score: source_score,
                });
            entry.fused_score += contribution;
            if source_score > entry.best_source_score {
                entry.result = result;
                entry.best_source_score = source_score;
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
