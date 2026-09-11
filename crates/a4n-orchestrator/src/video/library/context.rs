use std::collections::{HashMap, HashSet};

use a4n_db_models::models::ai::generated_image;
use a4n_error::AppError;
use serde_json::Value;

use crate::profile_material::{
    generated_image_profile_material_identity, is_project_reusable_video_material_image,
};
use crate::video::{ChapterVideoSourceContext, OrchestratorContext};
use a4n_db::profile_material::{PROFILE_KIND_OBJECT, PROFILE_KIND_SCENE};

use super::candidates::{
    RelevantProfile, VideoAssetCandidate, VideoAssetCandidates, best_profile_material_image,
};
use super::projection::video_library_image_entry;
use super::relevance::{
    is_chapter_video_material, object_profile_relevance, scene_profile_relevance,
    source_relevance_text,
};
use super::validation::ensure_video_asset_image_usable;
use super::{
    MAX_AUTO_PROFILE_ASSETS, MAX_REFERENCE_IMAGE_IDS, MAX_VIDEO_LIBRARY_IMAGE_IDS,
    SELECTION_REASON_CHAPTER_MATERIAL, SELECTION_REASON_MANUAL_LIBRARY, SELECTION_REASON_REFERENCE,
};

impl OrchestratorContext {
    pub(in crate::video) async fn video_library_image_context(
        &self,
        project_id: i64,
        chapter_id: i64,
        source: &ChapterVideoSourceContext<'_>,
        reference_image_ids: &[i64],
        explicit_image_ids: &[i64],
    ) -> Result<Value, AppError> {
        if reference_image_ids.len() > MAX_REFERENCE_IMAGE_IDS {
            return Err(AppError::InvalidArgument(format!(
                "reference_image_ids 最多选择 {MAX_REFERENCE_IMAGE_IDS} 张"
            )));
        }
        if explicit_image_ids.len() > MAX_VIDEO_LIBRARY_IMAGE_IDS {
            return Err(AppError::InvalidArgument(format!(
                "video_library_image_ids 最多选择 {MAX_VIDEO_LIBRARY_IMAGE_IDS} 张"
            )));
        }
        let (chapter, allowed_project_ids) = futures::join!(
            self.get_chapter(chapter_id),
            self.video_asset_project_ids(project_id),
        );
        let chapter = chapter?;
        let allowed_project_ids = allowed_project_ids?;
        let mut candidates = VideoAssetCandidates::default();

        let explicit_reference_ids = reference_image_ids
            .iter()
            .copied()
            .chain(explicit_image_ids.iter().copied())
            .collect::<Vec<_>>();
        let (explicit_rows, chapter_images, profile_candidates) = futures::join!(
            async {
                if explicit_reference_ids.is_empty() {
                    return Ok::<Vec<generated_image::Model>, AppError>(Vec::new());
                }
                a4n_db::video::video_library_explicit_image_records(
                    self.database.connection(),
                    &allowed_project_ids,
                    &explicit_reference_ids,
                )
                .await
                .map_err(AppError::Database)
            },
            async {
                a4n_db::video::video_library_chapter_image_records(
                    self.database.connection(),
                    &allowed_project_ids,
                    chapter_id,
                )
                .await
                .map_err(AppError::Database)
            },
            self.relevant_profile_material_candidates(
                &allowed_project_ids,
                chapter.chapter_number,
                source,
            ),
        );
        let explicit_rows = explicit_rows?;
        if !explicit_reference_ids.is_empty() {
            let found_ids = explicit_rows
                .iter()
                .map(|image| image.id)
                .collect::<HashSet<_>>();
            if let Some(missing_id) = explicit_reference_ids
                .iter()
                .find(|id| !found_ids.contains(id))
            {
                return Err(AppError::InvalidArgument(format!(
                    "视频素材库引用的图片 #{} 不存在或不属于当前视频项目/根文本项目",
                    missing_id
                )));
            }
            let reference_order = reference_image_ids
                .iter()
                .copied()
                .enumerate()
                .map(|(index, id)| (id, index))
                .collect::<HashMap<_, _>>();
            let manual_order = explicit_image_ids
                .iter()
                .copied()
                .enumerate()
                .map(|(index, id)| (id, index))
                .collect::<HashMap<_, _>>();
            for image in explicit_rows {
                ensure_video_asset_image_usable(
                    project_id,
                    chapter_id,
                    &allowed_project_ids,
                    &image,
                )?;
                if let Some(order) = reference_order.get(&image.id).copied() {
                    candidates.push(VideoAssetCandidate::new(
                        image.clone(),
                        SELECTION_REASON_REFERENCE,
                        0,
                        order,
                        0,
                    ));
                }
                if let Some(order) = manual_order.get(&image.id).copied() {
                    candidates.push(VideoAssetCandidate::new(
                        image,
                        SELECTION_REASON_MANUAL_LIBRARY,
                        1,
                        order,
                        0,
                    ));
                }
            }
        }

        let chapter_images = chapter_images?;
        for (index, image) in chapter_images.into_iter().enumerate() {
            if is_chapter_video_material(image.metadata_json.as_ref()) {
                candidates.push(VideoAssetCandidate::new(
                    image,
                    SELECTION_REASON_CHAPTER_MATERIAL,
                    2,
                    index,
                    0,
                ));
            }
        }

        for candidate in profile_candidates? {
            candidates.push(candidate);
        }

        Ok(Value::Array(
            candidates
                .into_limited()
                .into_iter()
                .enumerate()
                .map(|(index, candidate)| video_library_image_entry(&candidate, index + 1))
                .collect(),
        ))
    }

    async fn relevant_profile_material_candidates(
        &self,
        project_ids: &[i64],
        chapter_number: i32,
        source: &ChapterVideoSourceContext<'_>,
    ) -> Result<Vec<VideoAssetCandidate>, AppError> {
        let mut relevant_profiles = Vec::new();
        let source_text = source_relevance_text(source);
        let read_model = a4n_db::video::video_profile_material_read_model(
            self.database.connection(),
            project_ids,
        )
        .await
        .map_err(AppError::Database)?;

        for scene in read_model.scenes {
            if let Some(relevance_rank) =
                scene_profile_relevance(&scene, chapter_number, &source_text)
            {
                relevant_profiles.push(RelevantProfile {
                    kind: PROFILE_KIND_SCENE,
                    id: scene.id,
                    relevance_rank,
                });
            }
        }

        for object in read_model.objects {
            if let Some(relevance_rank) =
                object_profile_relevance(&object, chapter_number, &source_text)
            {
                relevant_profiles.push(RelevantProfile {
                    kind: PROFILE_KIND_OBJECT,
                    id: object.id,
                    relevance_rank,
                });
            }
        }
        if relevant_profiles.is_empty() {
            return Ok(Vec::new());
        }

        let primary_by_profile = read_model
            .primary_materials
            .into_iter()
            .map(|row| ((row.profile_kind, row.profile_id), row.primary_image_id))
            .collect::<HashMap<_, _>>();

        let mut images_by_profile: HashMap<(&'static str, i64), Vec<generated_image::Model>> =
            HashMap::new();
        for image in read_model.material_images {
            if !is_project_reusable_video_material_image(
                &image.image_type,
                image.metadata_json.as_ref(),
            ) {
                continue;
            }
            if let Some((kind, profile_id)) = generated_image_profile_material_identity(&image) {
                images_by_profile
                    .entry((kind, profile_id))
                    .or_default()
                    .push(image);
            }
        }

        relevant_profiles.sort_by(|a, b| {
            a.relevance_rank
                .cmp(&b.relevance_rank)
                .then_with(|| a.kind.cmp(b.kind))
                .then_with(|| a.id.cmp(&b.id))
        });
        let mut candidates = Vec::new();
        for profile in relevant_profiles {
            let Some(images) = images_by_profile.get(&(profile.kind, profile.id)) else {
                continue;
            };
            let primary_id = primary_by_profile
                .get(&(profile.kind.to_string(), profile.id))
                .copied();
            let Some((image, reason, priority, is_primary)) =
                best_profile_material_image(images, primary_id)
            else {
                continue;
            };
            let mut candidate = VideoAssetCandidate::new(
                image.clone(),
                reason,
                priority,
                candidates.len(),
                profile.relevance_rank,
            );
            candidate.profile_kind = Some(profile.kind);
            candidate.profile_id = Some(profile.id);
            candidate.is_primary = is_primary;
            candidates.push(candidate);
            if candidates.len() >= MAX_AUTO_PROFILE_ASSETS {
                break;
            }
        }
        Ok(candidates)
    }
}
