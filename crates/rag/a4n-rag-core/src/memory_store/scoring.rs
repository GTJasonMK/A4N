use std::collections::HashMap;

use crate::vector_store::SearchResult;

const RRF_K: f32 = 60.0;

/// Calculate cosine similarity.
pub(super) fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

/// Tokenize mixed Chinese and English text.
pub(super) fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() || c.is_ascii_punctuation() {
            i += 1;
            continue;
        }

        if u32::from(c) > 0x2E80 {
            tokens.push(c.to_string());
            if i + 1 < chars.len() && u32::from(chars[i + 1]) > 0x2E80 {
                tokens.push(format!("{}{}", c, chars[i + 1]));
            }
            i += 1;
        } else {
            let start = i;
            while i < chars.len()
                && !chars[i].is_whitespace()
                && !chars[i].is_ascii_punctuation()
                && u32::from(chars[i]) <= 0x2E80
            {
                i += 1;
            }
            let token: String = chars[start..i].iter().collect();
            if !token.is_empty() {
                tokens.push(token.to_lowercase());
            }
        }
    }

    tokens
}

pub(super) fn sort_scores_desc<T>(items: &mut [(f32, T)]) {
    items.sort_by(|a, b| b.0.total_cmp(&a.0));
}

pub(super) fn sort_doc_scores_desc(items: &mut [(String, f32)]) {
    items.sort_by(|a, b| b.1.total_cmp(&a.1));
}

/// Return RRF scores by id. The caller is responsible for rehydrating content
/// and metadata from the authoritative collection.
pub(super) fn rrf_fuse_scores(
    vector_results: &[SearchResult],
    bm25_results: &[SearchResult],
    top_k: usize,
) -> Vec<(String, f32)> {
    let mut scores: HashMap<String, f32> = HashMap::new();

    for (rank, r) in vector_results.iter().enumerate() {
        let score = 1.0 / (RRF_K + rank as f32 + 1.0);
        *scores.entry(r.id.clone()).or_insert(0.0) += score;
    }

    for (rank, r) in bm25_results.iter().enumerate() {
        let score = 1.0 / (RRF_K + rank as f32 + 1.0);
        *scores.entry(r.id.clone()).or_insert(0.0) += score;
    }

    let mut fused = scores.into_iter().collect::<Vec<_>>();
    fused.sort_by(|a, b| b.1.total_cmp(&a.1));
    fused.truncate(top_k);
    fused
}
