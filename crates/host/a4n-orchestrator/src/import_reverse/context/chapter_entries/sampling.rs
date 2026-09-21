use std::collections::BTreeSet;

use a4n_db_models::models::chapter::chapter;

fn select_sample_indices(total: usize, limit: usize) -> BTreeSet<usize> {
    let mut indices = BTreeSet::new();
    if total == 0 || limit == 0 {
        return indices;
    }
    if total <= limit {
        indices.extend(0..total);
        return indices;
    }
    if limit == 1 {
        indices.insert(0);
        return indices;
    }
    for i in 0..limit {
        indices.insert(i * (total - 1) / (limit - 1));
    }
    indices
}

pub(in crate::import_reverse::context) fn sampled_chapters(
    chapters: &[chapter::Model],
    limit: usize,
) -> Vec<&chapter::Model> {
    let indices = select_sample_indices(chapters.len(), limit);
    chapters
        .iter()
        .enumerate()
        .filter_map(|(idx, chapter)| indices.contains(&idx).then_some(chapter))
        .collect()
}
