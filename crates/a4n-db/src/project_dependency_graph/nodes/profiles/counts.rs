use std::collections::BTreeMap;

use a4n_core::models::{ai::generated_image, character::character_portrait};

pub(super) fn portrait_counts_by_profile(
    portraits: &[character_portrait::Model],
) -> BTreeMap<i64, u64> {
    let mut counts = BTreeMap::new();
    for portrait in portraits {
        *counts.entry(portrait.profile_id).or_insert(0) += 1;
    }
    counts
}

pub(super) fn material_image_counts_by_profile(
    generated_images: &[generated_image::Model],
    profile_kind: &str,
) -> BTreeMap<i64, u64> {
    let mut counts = BTreeMap::new();
    for image in generated_images {
        let Some((kind, profile_id)) =
            crate::profile_material::generated_image_profile_material_identity(image)
        else {
            continue;
        };
        if kind == profile_kind {
            *counts.entry(profile_id).or_insert(0) += 1;
        }
    }
    counts
}
