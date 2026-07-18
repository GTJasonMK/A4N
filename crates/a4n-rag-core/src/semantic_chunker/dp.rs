use super::matrix::block_score;
use super::{SemanticChunk, SemanticChunkConfig};
use a4n_core::error::AppError;

pub(super) fn dynamic_programming(
    prefix: &[f64],
    sentences: &[String],
    config: &SemanticChunkConfig,
) -> Result<Vec<isize>, AppError> {
    let n = sentences.len();
    let mut dp = vec![0.0; n + 1];
    let mut path = vec![0isize; n + 1];
    let mut char_lengths = Vec::with_capacity(n + 1);
    char_lengths.push(0usize);
    for sentence in sentences {
        let previous = char_lengths
            .last()
            .copied()
            .expect("character prefix lengths always contain the zero origin");
        let sentence_chars = sentence.chars().count();
        let next = previous.checked_add(sentence_chars).ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "语义分块累计字符数超过 usize 上限：{previous} + {sentence_chars}"
            ))
        })?;
        char_lengths.push(next);
    }

    for i in 1..=n {
        let mut best_score = f64::NEG_INFINITY;
        let mut best_j: isize = -1;
        let j_min = i.saturating_sub(config.max_chunk_sentences);
        let j_max = i.checked_sub(config.min_chunk_sentences);

        if let Some(j_max) = j_max {
            for (j, prev_score) in dp.iter().enumerate().take(j_max + 1).skip(j_min) {
                let chunk_chars =
                    char_lengths[i]
                        .checked_sub(char_lengths[j])
                        .ok_or_else(|| {
                            AppError::InvalidArgument(format!(
                                "语义分块字符前缀不单调：end_index={i}, start_index={j}"
                            ))
                        })?;
                if chunk_chars < config.min_chunk_chars && i < n {
                    continue;
                }
                if chunk_chars > config.max_chunk_chars {
                    continue;
                }
                let score = *prev_score + block_score(prefix, n, j, i, config.gamma);
                if score > best_score {
                    best_score = score;
                    best_j = j as isize;
                }
            }

            if best_j < 0 {
                for (j, prev_score) in dp.iter().enumerate().take(j_max + 1).skip(j_min) {
                    let score = *prev_score + block_score(prefix, n, j, i, config.gamma);
                    if score > best_score {
                        best_score = score;
                        best_j = j as isize;
                    }
                }
            }
        }

        if best_j < 0 {
            dp[i] = dp[i - 1];
            path[i] = -1;
        } else {
            dp[i] = best_score;
            path[i] = best_j;
        }
    }

    Ok(path)
}

pub(super) fn backtrack_cut_points(n: usize, path: &[isize]) -> Vec<usize> {
    let mut cut_points = vec![n];
    let mut cursor = n;

    while cursor > 0 {
        match path[cursor] {
            -1 => cursor -= 1,
            value if value >= 0 => {
                let value = usize::try_from(value)
                    .expect("non-negative semantic chunk path index always fits usize");
                if value > 0 {
                    cut_points.push(value);
                }
                cursor = value;
            }
            _ => cursor -= 1,
        }
    }

    cut_points.push(0);
    cut_points.sort_unstable();
    cut_points.dedup();
    cut_points
}

pub(super) fn generate_chunks(
    sentences: &[String],
    cut_points: &[usize],
    prefix: &[f64],
    config: &SemanticChunkConfig,
) -> Vec<SemanticChunk> {
    let n = sentences.len();
    let mut chunks = Vec::new();

    for (index, window) in cut_points.windows(2).enumerate() {
        let start = window[0];
        let end = window[1];
        if start >= end {
            continue;
        }

        let mut content = String::new();
        if config.with_overlap && index > 0 && config.overlap_sentences > 0 {
            let overlap_start = start.saturating_sub(config.overlap_sentences);
            let overlap = sentences[overlap_start..start].join("");
            if !overlap.trim().is_empty() {
                content.push_str("[...] ");
                content.push_str(&overlap);
                content.push_str("\n\n");
            }
        }
        content.push_str(&sentences[start..end].join(""));
        if !content
            .chars()
            .last()
            .is_some_and(|ch| matches!(ch, '。' | '！' | '？' | '.' | '!' | '?'))
        {
            content.push('。');
        }

        chunks.push(SemanticChunk {
            content,
            start_sentence_idx: start,
            end_sentence_idx: end,
            sentence_count: end - start,
            density_score: block_score(prefix, n, start, end, config.gamma),
        });
    }

    chunks
}
