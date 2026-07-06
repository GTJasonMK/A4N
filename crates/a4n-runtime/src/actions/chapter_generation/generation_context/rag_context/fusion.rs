use super::metadata_i64;
use crate::actions::rag_fusion::fuse_weighted_rag_results;
use a4n_engine::rag::vector_store::SearchResult;
use serde_json::Value;

pub(crate) fn fuse_generation_rag_results(
    batches: Vec<(f32, Vec<SearchResult>)>,
    chapter_number: i32,
) -> Vec<SearchResult> {
    fuse_weighted_rag_results(batches, |result| {
        generation_temporal_multiplier(&result.metadata, chapter_number)
    })
}

fn generation_temporal_multiplier(metadata: &Value, chapter_number: i32) -> f32 {
    let Some(source_chapter) = metadata_i64(metadata, "chapter_number").map(|value| value as i32)
    else {
        return 1.0;
    };
    let distance = chapter_number - source_chapter;
    if distance <= 0 {
        return 0.0;
    }
    if distance <= 5 {
        let bonus = 0.25 * (1.0 - ((distance - 1) as f32 / 5.0));
        1.0 + bonus
    } else {
        1.0
    }
}
