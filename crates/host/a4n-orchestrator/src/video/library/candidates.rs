use std::collections::HashMap;

use a4n_db::profile_material::generated_image_profile_material_identity;
use a4n_db_models::models::ai::generated_image;

use super::{
    MAX_AVAILABLE_VIDEO_ASSETS, SELECTION_REASON_PROFILE_FAVORITE, SELECTION_REASON_PROFILE_LATEST,
    SELECTION_REASON_PROFILE_PRIMARY,
};

#[derive(Clone)]
pub(super) struct VideoAssetCandidate {
    pub(super) image: generated_image::Model,
    pub(super) selection_reason: &'static str,
    priority: usize,
    sort_order: usize,
    relevance_rank: usize,
    pub(super) profile_kind: Option<&'static str>,
    pub(super) profile_id: Option<i64>,
    pub(super) is_primary: bool,
}

impl VideoAssetCandidate {
    pub(super) fn new(
        image: generated_image::Model,
        selection_reason: &'static str,
        priority: usize,
        sort_order: usize,
        relevance_rank: usize,
    ) -> Self {
        let identity = generated_image_profile_material_identity(&image);
        Self {
            image,
            selection_reason,
            priority,
            sort_order,
            relevance_rank,
            profile_kind: identity.map(|(kind, _)| kind),
            profile_id: identity.map(|(_, id)| id),
            is_primary: selection_reason == SELECTION_REASON_PROFILE_PRIMARY,
        }
    }
}

#[derive(Default)]
pub(super) struct VideoAssetCandidates {
    items: Vec<VideoAssetCandidate>,
    index_by_id: HashMap<i64, usize>,
}

impl VideoAssetCandidates {
    pub(super) fn push(&mut self, candidate: VideoAssetCandidate) {
        if let Some(index) = self.index_by_id.get(&candidate.image.id).copied() {
            let current = &self.items[index];
            let replace = candidate.priority < current.priority
                || (candidate.priority == current.priority
                    && candidate.relevance_rank < current.relevance_rank)
                || (candidate.priority == current.priority
                    && candidate.relevance_rank == current.relevance_rank
                    && candidate.is_primary
                    && !current.is_primary);
            if replace {
                self.items[index] = candidate;
            }
            return;
        }
        self.index_by_id
            .insert(candidate.image.id, self.items.len());
        self.items.push(candidate);
    }

    pub(super) fn into_limited(mut self) -> Vec<VideoAssetCandidate> {
        self.items.sort_by(|a, b| {
            a.priority
                .cmp(&b.priority)
                .then_with(|| a.relevance_rank.cmp(&b.relevance_rank))
                .then_with(|| a.sort_order.cmp(&b.sort_order))
                .then_with(|| b.image.is_favorite.cmp(&a.image.is_favorite))
                .then_with(|| b.image.created_at.cmp(&a.image.created_at))
                .then_with(|| b.image.id.cmp(&a.image.id))
        });
        self.items.truncate(MAX_AVAILABLE_VIDEO_ASSETS);
        self.items
    }
}

#[derive(Debug)]
pub(super) struct RelevantProfile {
    pub(super) kind: &'static str,
    pub(super) id: i64,
    pub(super) relevance_rank: usize,
}

pub(super) fn best_profile_material_image(
    images: &[generated_image::Model],
    primary_id: Option<i64>,
) -> Option<(&generated_image::Model, &'static str, usize, bool)> {
    if let Some(primary_id) = primary_id
        && let Some(image) = images.iter().find(|image| image.id == primary_id)
    {
        return Some((image, SELECTION_REASON_PROFILE_PRIMARY, 3, true));
    }
    if let Some(image) = images
        .iter()
        .filter(|image| image.is_favorite)
        .max_by(|a, b| {
            a.created_at
                .cmp(&b.created_at)
                .then_with(|| a.id.cmp(&b.id))
        })
    {
        return Some((image, SELECTION_REASON_PROFILE_FAVORITE, 4, false));
    }
    images
        .iter()
        .max_by(|a, b| {
            a.created_at
                .cmp(&b.created_at)
                .then_with(|| a.id.cmp(&b.id))
        })
        .map(|image| (image, SELECTION_REASON_PROFILE_LATEST, 5, false))
}
