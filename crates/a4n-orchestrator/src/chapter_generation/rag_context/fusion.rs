use super::rules::metadata_i64;
use a4n_rag_core::fusion::fuse_weighted_rag_results;
use a4n_rag_core::vector_store::SearchResult;
use serde_json::Value;

pub fn fuse_generation_rag_results(
    batches: Vec<(f32, Vec<SearchResult>)>,
    chapter_number: i32,
) -> Vec<SearchResult> {
    fuse_weighted_rag_results(batches, |result| {
        generation_temporal_multiplier(&result.metadata, chapter_number)
    })
}

fn generation_temporal_multiplier(metadata: &Value, chapter_number: i32) -> f32 {
    let Some(source_chapter) =
        metadata_i64(metadata, "chapter_number").and_then(|value| i32::try_from(value).ok())
    else {
        return 1.0;
    };
    let distance = i64::from(chapter_number) - i64::from(source_chapter);
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
