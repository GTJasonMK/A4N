use super::SemanticChunkConfig;
use super::matrix::block_score;
use a4n_domain::ProviderError as AppError;

pub(super) fn dynamic_programming(
    prefix: &[f64],
    sentences: &[String],
    config: &SemanticChunkConfig,
) -> Result<Vec<isize>, AppError> {
    let n = sentences.len();
    let mut dp = vec![0.0; n + 1];
    let mut path = vec![-1isize; n + 1];
    let mut char_lengths = Vec::with_capacity(n + 1);
    char_lengths.push(0usize);
    let mut total_chars = 0usize;
    for sentence in sentences {
        let sentence_chars = sentence.chars().count();
        total_chars = total_chars.checked_add(sentence_chars).ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "语义分块累计字符数超过 usize 上限：{total_chars} + {sentence_chars}"
            ))
        })?;
        char_lengths.push(total_chars);
    }

    for i in 1..=n {
        let mut best_score = f64::NEG_INFINITY;
        let mut best_j: isize = -1;
        let j_min = i.saturating_sub(config.max_chunk_sentences);
        let j_max = i.checked_sub(config.min_chunk_sentences);

        if let Some(j_max) = j_max {
            for (j, prev_score) in dp.iter().enumerate().take(j_max + 1).skip(j_min) {
                let chunk_chars = char_lengths[i] - char_lengths[j];
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
        }

        if best_j < 0 {
            dp[i] = dp[i - 1];
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
        let previous = path[cursor];
        if previous < 0 {
            cursor -= 1;
            continue;
        }
        let previous = previous as usize;
        if previous > 0 {
            cut_points.push(previous);
        }
        cursor = previous;
    }

    cut_points.push(0);
    cut_points.sort_unstable();
    cut_points.dedup();
    cut_points
}

pub(super) fn generate_chunks(sentences: &[String], cut_points: &[usize]) -> Vec<String> {
    let mut chunks = Vec::new();

    for window in cut_points.windows(2) {
        let start = window[0];
        let end = window[1];

        let mut content = sentences[start..end].join("");
        if !content
            .chars()
            .last()
            .is_some_and(|ch| matches!(ch, '。' | '！' | '？' | '.' | '!' | '?'))
        {
            content.push('。');
        }

        chunks.push(content);
    }

    chunks
}
