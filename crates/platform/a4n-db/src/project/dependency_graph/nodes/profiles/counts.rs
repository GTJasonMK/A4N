use std::collections::BTreeMap;

use crate::project::dependency_graph::graph::GeneratedImageGraphRow;
use crate::project::dependency_graph::graph::PortraitGraphRow;

pub(super) fn portrait_counts_by_profile(portraits: &[PortraitGraphRow]) -> BTreeMap<i64, u64> {
    let mut counts = BTreeMap::new();
    for portrait in portraits {
        *counts.entry(portrait.profile_id).or_insert(0) += 1;
    }
    counts
}

pub(super) fn material_image_counts_by_profile(
    generated_images: &[GeneratedImageGraphRow],
    profile_kind: &str,
) -> BTreeMap<i64, u64> {
    let mut counts = BTreeMap::new();
    for image in generated_images {
        let Some((kind, profile_id)) = crate::profile_material::profile_material_identity(
            &image.image_type,
            image.metadata_json.as_ref(),
        ) else {
            continue;
        };
        if kind == profile_kind {
            *counts.entry(profile_id).or_insert(0) += 1;
        }
    }
    counts
}
