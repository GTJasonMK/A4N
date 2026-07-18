use a4n_core::error::AppError;
use a4n_rag_core::fusion::{ensure_rag_scores_finite, fuse_weighted_rag_results};
use a4n_rag_core::vector_store::SearchResult;

pub fn fuse_enhanced_rag_results(
    batches: Vec<(f32, Vec<SearchResult>)>,
    chapter_number: Option<i32>,
) -> Result<Vec<SearchResult>, AppError> {
    ensure_rag_scores_finite(&batches)?;
    Ok(fuse_weighted_rag_results(batches, |result| {
        enhanced_temporal_multiplier(result, chapter_number)
    }))
}

fn enhanced_temporal_multiplier(result: &SearchResult, chapter_number: Option<i32>) -> f32 {
    let Some(chapter_number) = chapter_number.filter(|value| *value > 0) else {
        return 1.0;
    };
    let Some(source_chapter) = metadata_i64(&result.metadata, "chapter_number")
        .and_then(|source| i32::try_from(source).ok())
    else {
        return 1.0;
    };
    let distance = (i64::from(chapter_number) - i64::from(source_chapter)).abs();
    if distance == 0 {
        return 1.3;
    }
    if distance <= 5 {
        1.0 + 0.25 * (1.0 - ((distance - 1) as f32 / 5.0))
    } else {
        1.0
    }
}

fn metadata_i64(metadata: &serde_json::Value, key: &str) -> Option<i64> {
    metadata.get(key).and_then(|value| {
        value
            .as_i64()
            .or_else(|| value.as_u64().and_then(|value| i64::try_from(value).ok()))
    })
}
