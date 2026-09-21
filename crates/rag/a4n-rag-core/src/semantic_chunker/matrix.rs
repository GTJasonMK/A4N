pub(super) fn build_similarity_matrix(embeddings: &[Vec<f32>]) -> Vec<f64> {
    let norms = embeddings
        .iter()
        .map(|embedding| {
            embedding
                .iter()
                .map(|value| f64::from(*value) * f64::from(*value))
                .sum::<f64>()
                .sqrt()
        })
        .collect::<Vec<_>>();

    let n = embeddings.len();
    let mut matrix = vec![0.0; n * n];
    for i in 0..n {
        for j in i..n {
            let score = if norms[i] == 0.0 || norms[j] == 0.0 {
                0.0
            } else {
                embeddings[i]
                    .iter()
                    .zip(&embeddings[j])
                    .map(|(a, b)| f64::from(*a) * f64::from(*b))
                    .sum::<f64>()
                    / (norms[i] * norms[j])
            };
            matrix[matrix_index(n, i, j)] = score;
            matrix[matrix_index(n, j, i)] = score;
        }
    }
    matrix
}

pub(super) fn apply_structure_enhancement(
    mut sim_matrix: Vec<f64>,
    n: usize,
    gate_threshold: f64,
    alpha: f64,
) -> Vec<f64> {
    for i in 0..n {
        for j in (i + 1)..n {
            let score = sim_matrix[matrix_index(n, i, j)];
            if score > gate_threshold {
                let distance = (j - i) as f64;
                let adjusted = score + score * alpha * distance.ln_1p();
                sim_matrix[matrix_index(n, i, j)] = adjusted;
                sim_matrix[matrix_index(n, j, i)] = adjusted;
            }
        }
    }
    sim_matrix
}

pub(super) fn compute_prefix_sum(matrix: &[f64], n: usize) -> Vec<f64> {
    let width = n + 1;
    let mut prefix = vec![0.0; width * width];
    for i in 1..=n {
        for j in 1..=n {
            prefix[matrix_index(width, i, j)] = matrix[matrix_index(n, i - 1, j - 1)]
                + prefix[matrix_index(width, i - 1, j)]
                + prefix[matrix_index(width, i, j - 1)]
                - prefix[matrix_index(width, i - 1, j - 1)];
        }
    }
    prefix
}

pub(super) fn block_score(prefix: &[f64], n: usize, start: usize, end: usize, gamma: f64) -> f64 {
    let length = end - start;
    let region_sum = sum_region(prefix, n + 1, start, start, end - 1, end - 1);
    region_sum / (length as f64).powf(gamma)
}

fn sum_region(prefix: &[f64], width: usize, r1: usize, c1: usize, r2: usize, c2: usize) -> f64 {
    prefix[matrix_index(width, r2 + 1, c2 + 1)]
        - prefix[matrix_index(width, r1, c2 + 1)]
        - prefix[matrix_index(width, r2 + 1, c1)]
        + prefix[matrix_index(width, r1, c1)]
}

fn matrix_index(width: usize, row: usize, col: usize) -> usize {
    row * width + col
}
