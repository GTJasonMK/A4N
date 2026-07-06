use sea_orm::*;
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

use a4n_core::error::AppError;
use a4n_core::models::ai::generated_image;
use a4n_core::models::asset::{object_profile, profile_primary_material};
use a4n_core::models::scene::scene_profile;

use crate::profile_material::{
    PROFILE_KIND_OBJECT, PROFILE_KIND_SCENE, generated_image_profile_material_identity,
};

use super::{ChapterVideoSourceContext, VideoService, is_project_reusable_video_material_image};

pub(super) const MAX_VIDEO_LIBRARY_IMAGE_IDS: usize = 32;
pub(super) const MAX_REFERENCE_IMAGE_IDS: usize = 16;
const MAX_AUTO_PROFILE_ASSETS: usize = 24;
const MAX_AVAILABLE_VIDEO_ASSETS: usize = 48;
const SELECTION_REASON_REFERENCE: &str = "reference";
const SELECTION_REASON_MANUAL_LIBRARY: &str = "manual_library";
const SELECTION_REASON_CHAPTER_MATERIAL: &str = "chapter_material";
const SELECTION_REASON_PROFILE_PRIMARY: &str = "profile_primary";
const SELECTION_REASON_PROFILE_FAVORITE: &str = "profile_favorite";
const SELECTION_REASON_PROFILE_LATEST: &str = "profile_latest";

pub(super) fn available_video_image_ids(
    available_assets: &Value,
    extra_ids: &[i64],
) -> Result<HashSet<i64>, AppError> {
    let mut ids = HashSet::new();
    let mut seen_extra_ids = HashSet::new();
    for (index, id) in extra_ids.iter().copied().enumerate() {
        if id <= 0 {
            return Err(AppError::InvalidArgument(format!(
                "reference_image_ids[{index}] 必须为正整数"
            )));
        }
        if !seen_extra_ids.insert(id) {
            return Err(AppError::InvalidArgument(format!(
                "reference_image_ids 包含重复 ID：{id}"
            )));
        }
        ids.insert(id);
    }

    let items = available_assets.as_array().ok_or_else(|| {
        AppError::InvalidArgument("available_assets 必须是视频素材数组".to_string())
    })?;
    let mut seen_asset_ids = HashSet::new();
    for (index, item) in items.iter().enumerate() {
        let id = item
            .get("image_id")
            .and_then(Value::as_i64)
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "available_assets[{index}].image_id 必须为正整数"
                ))
            })?;
        if id <= 0 {
            return Err(AppError::InvalidArgument(format!(
                "available_assets[{index}].image_id 必须为正整数"
            )));
        }
        if !seen_asset_ids.insert(id) {
            return Err(AppError::InvalidArgument(format!(
                "available_assets 包含重复图片 ID：{id}"
            )));
        }
        ids.insert(id);
    }
    Ok(ids)
}

pub(super) fn ensure_video_reference_ids_known(
    ids: &[i64],
    allowed_ids: &HashSet<i64>,
    context: &str,
) -> Result<(), AppError> {
    if let Some(id) = ids.iter().find(|id| !allowed_ids.contains(id)) {
        return Err(AppError::InvalidArgument(format!(
            "{context} 引用了不在当前视频素材库中的图片 ID：{id}"
        )));
    }
    Ok(())
}

impl VideoService {
    pub(super) async fn video_library_image_context(
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
        let chapter = self.get_chapter(chapter_id).await?;
        let allowed_project_ids = self.video_asset_project_ids(project_id).await?;
        let mut candidates = VideoAssetCandidates::default();

        let explicit_reference_ids = reference_image_ids
            .iter()
            .copied()
            .chain(explicit_image_ids.iter().copied())
            .collect::<Vec<_>>();
        if !explicit_reference_ids.is_empty() {
            let explicit_rows = generated_image::Entity::find()
                .filter(generated_image::Column::ProjectId.is_in(allowed_project_ids.clone()))
                .filter(generated_image::Column::Id.is_in(explicit_reference_ids.clone()))
                .order_by_asc(generated_image::Column::CreatedAt)
                .order_by_asc(generated_image::Column::Id)
                .all(self.db.as_ref())
                .await
                .map_err(AppError::Database)?;
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

        let chapter_images = generated_image::Entity::find()
            .filter(generated_image::Column::ProjectId.is_in(allowed_project_ids.clone()))
            .filter(generated_image::Column::ChapterId.eq(chapter_id))
            .order_by_asc(generated_image::Column::CreatedAt)
            .order_by_asc(generated_image::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
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

        for candidate in self
            .relevant_profile_material_candidates(
                &allowed_project_ids,
                chapter.chapter_number,
                source,
            )
            .await?
        {
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

        let scenes = scene_profile::Entity::find()
            .filter(scene_profile::Column::ProjectId.is_in(project_ids.to_vec()))
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        for scene in scenes {
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

        let objects = object_profile::Entity::find()
            .filter(object_profile::Column::ProjectId.is_in(project_ids.to_vec()))
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        for object in objects {
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

        let primary_rows = profile_primary_material::Entity::find()
            .filter(profile_primary_material::Column::ProjectId.is_in(project_ids.to_vec()))
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        let primary_by_profile = primary_rows
            .into_iter()
            .map(|row| ((row.profile_kind, row.profile_id), row.primary_image_id))
            .collect::<HashMap<_, _>>();

        let material_images = generated_image::Entity::find()
            .filter(generated_image::Column::ProjectId.is_in(project_ids.to_vec()))
            .order_by_desc(generated_image::Column::CreatedAt)
            .order_by_desc(generated_image::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        let mut images_by_profile: HashMap<(&'static str, i64), Vec<generated_image::Model>> =
            HashMap::new();
        for image in material_images {
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

fn ensure_video_asset_image_usable(
    project_id: i64,
    chapter_id: i64,
    allowed_project_ids: &[i64],
    image: &generated_image::Model,
) -> Result<(), AppError> {
    if !allowed_project_ids.contains(&image.project_id) {
        return Err(AppError::InvalidArgument(format!(
            "视频素材库不能引用不属于视频项目 {project_id} 或其根文本项目的图片 #{}",
            image.id
        )));
    }
    if image.chapter_id.is_some_and(|id| id != chapter_id)
        && !is_project_reusable_video_material_image(
            &image.image_type,
            image.metadata_json.as_ref(),
        )
    {
        return Err(AppError::InvalidArgument(format!(
            "视频素材库不能引用其他章节的图片 #{}",
            image.id
        )));
    }
    Ok(())
}

#[derive(Clone)]
struct VideoAssetCandidate {
    image: generated_image::Model,
    selection_reason: &'static str,
    priority: usize,
    sort_order: usize,
    relevance_rank: usize,
    profile_kind: Option<&'static str>,
    profile_id: Option<i64>,
    is_primary: bool,
}

impl VideoAssetCandidate {
    fn new(
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
struct VideoAssetCandidates {
    items: Vec<VideoAssetCandidate>,
    index_by_id: HashMap<i64, usize>,
}

impl VideoAssetCandidates {
    fn push(&mut self, candidate: VideoAssetCandidate) {
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

    fn into_limited(mut self) -> Vec<VideoAssetCandidate> {
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
struct RelevantProfile {
    kind: &'static str,
    id: i64,
    relevance_rank: usize,
}

fn best_profile_material_image(
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

fn is_chapter_video_material(metadata: Option<&Value>) -> bool {
    let Some(metadata) = metadata.and_then(Value::as_object) else {
        return false;
    };
    if super::is_video_frame_input_metadata(metadata) {
        return false;
    }
    metadata
        .get("chapter_video_material")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn source_relevance_text(source: &ChapterVideoSourceContext<'_>) -> String {
    let mut parts = Vec::new();
    parts.push(source.content.to_string());
    if let Some(summary) = source.summary {
        parts.push(summary.to_string());
    }
    if let Some(analysis) = source.analysis {
        parts.push(analysis.to_string());
    }
    parts.join("\n")
}

fn scene_profile_relevance(
    scene: &scene_profile::Model,
    chapter_number: i32,
    source_text: &str,
) -> Option<usize> {
    profile_relevance(
        &scene.name,
        &scene.aliases_json,
        scene.first_seen_chapter,
        scene.last_seen_chapter,
        chapter_number,
        source_text,
    )
}

fn object_profile_relevance(
    object: &object_profile::Model,
    chapter_number: i32,
    source_text: &str,
) -> Option<usize> {
    profile_relevance(
        &object.name,
        &object.aliases_json,
        object.first_seen_chapter,
        object.last_seen_chapter,
        chapter_number,
        source_text,
    )
}

fn profile_relevance(
    name: &str,
    aliases: &Value,
    first_seen_chapter: i32,
    last_seen_chapter: i32,
    chapter_number: i32,
    source_text: &str,
) -> Option<usize> {
    if profile_name_matches_source(name, aliases, source_text) {
        return Some(0);
    }
    if chapter_number >= first_seen_chapter && chapter_number <= last_seen_chapter {
        return Some(1);
    }
    None
}

fn profile_name_matches_source(name: &str, aliases: &Value, source_text: &str) -> bool {
    let source_text = source_text.trim();
    if source_text.is_empty() {
        return false;
    }
    if text_contains_non_empty(source_text, name) {
        return true;
    }
    aliases
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .any(|alias| text_contains_non_empty(source_text, alias))
}

fn text_contains_non_empty(haystack: &str, needle: &str) -> bool {
    let needle = needle.trim();
    !needle.is_empty() && haystack.contains(needle)
}

fn video_library_image_entry(candidate: &VideoAssetCandidate, display_order: usize) -> Value {
    let image = &candidate.image;
    let metadata = image.metadata_json.as_ref();
    let role = non_empty_text(Some(image.image_type.as_str()));
    let label = metadata_nested_text(metadata, "generation_context", "title")
        .or_else(|| metadata_text(metadata, "profile_name"))
        .or_else(|| metadata_text(metadata, "character_name"))
        .or_else(|| non_empty_text(Some(image.image_type.as_str())));
    let prompt = non_empty_text(Some(image.prompt.as_str()));
    let prompt_excerpt = prompt.map(truncate_asset_text);
    let description = metadata_nested_text(metadata, "generation_context", "scene_summary")
        .or_else(|| metadata_nested_text(metadata, "generation_context", "prompt_summary"))
        .or_else(|| metadata_text(metadata, "description"))
        .map(str::to_string)
        .or_else(|| prompt_excerpt.clone());
    json!({
        "image_id": image.id,
        "display_order": display_order,
        "display_label": format!("参考图 {display_order}"),
        "project_id": image.project_id,
        "role": role,
        "label": label,
        "description": description,
        "prompt_excerpt": prompt_excerpt,
        "selection_reason": candidate.selection_reason,
        "profile_kind": candidate.profile_kind,
        "profile_id": candidate.profile_id,
        "is_primary": candidate.is_primary,
        "is_favorite": image.is_favorite,
    })
}

fn truncate_asset_text(text: &str) -> String {
    const MAX_CHARS: usize = 180;
    let trimmed = text.trim();
    let mut chars = trimmed.chars();
    let excerpt = chars.by_ref().take(MAX_CHARS).collect::<String>();
    if chars.next().is_some() {
        format!("{excerpt}...")
    } else {
        excerpt
    }
}

fn metadata_text<'a>(metadata: Option<&'a Value>, key: &str) -> Option<&'a str> {
    metadata
        .and_then(|metadata| metadata.get(key))
        .and_then(Value::as_str)
        .and_then(|text| non_empty_text(Some(text)))
}

fn metadata_nested_text<'a>(
    metadata: Option<&'a Value>,
    object_key: &str,
    text_key: &str,
) -> Option<&'a str> {
    metadata
        .and_then(|metadata| metadata.get(object_key))
        .and_then(|object| object.get(text_key))
        .and_then(Value::as_str)
        .and_then(|text| non_empty_text(Some(text)))
}

fn non_empty_text(text: Option<&str>) -> Option<&str> {
    text.map(str::trim).filter(|text| !text.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{
        SELECTION_REASON_PROFILE_FAVORITE, SELECTION_REASON_PROFILE_PRIMARY, VideoAssetCandidate,
        available_video_image_ids, best_profile_material_image, is_chapter_video_material,
        profile_relevance, video_library_image_entry,
    };
    use serde_json::json;

    #[test]
    fn chapter_video_material_excludes_generated_frame_inputs() {
        let metadata = json!({
            "chapter_video_material": true,
            "video_frame_role": "start_frame",
            "generation_context": {
                "source": "video_first_frame_prompt"
            }
        });

        assert!(!is_chapter_video_material(Some(&metadata)));
    }

    #[test]
    fn chapter_video_material_accepts_explicit_material_image() {
        let metadata = json!({
            "chapter_video_material": true,
            "generation_context": {
                "source": "portrait_generation"
            }
        });

        assert!(is_chapter_video_material(Some(&metadata)));
    }

    #[test]
    fn available_video_image_ids_rejects_invalid_extra_id_instead_of_filtering() {
        let assets = json!([{"image_id": 9, "role": "portrait"}]);
        let err =
            available_video_image_ids(&assets, &[4, 0]).expect_err("invalid extra id should fail");

        assert!(
            err.to_string()
                .contains("reference_image_ids[1] 必须为正整数")
        );
    }

    #[test]
    fn available_video_image_ids_rejects_invalid_asset_entry_instead_of_filtering() {
        let assets = json!([{"image_id": 9}, {"image_id": -1}]);
        let err = available_video_image_ids(&assets, &[4])
            .expect_err("invalid available asset id should fail");

        assert!(
            err.to_string()
                .contains("available_assets[1].image_id 必须为正整数")
        );
    }

    #[test]
    fn profile_relevance_prioritizes_name_match_over_chapter_range() {
        assert_eq!(
            profile_relevance("玄铁匣", &json!(["旧匣"]), 1, 3, 8, "他取出旧匣。"),
            Some(0)
        );
        assert_eq!(
            profile_relevance("偏殿", &json!([]), 3, 9, 8, "无人提到此地。"),
            Some(1)
        );
        assert_eq!(
            profile_relevance("竹林", &json!([]), 1, 2, 8, "无人提到此地。"),
            None
        );
    }

    #[test]
    fn best_profile_material_prefers_primary_then_favorite() {
        let mut first = fixed_test_image(1, false);
        let mut favorite = fixed_test_image(2, true);
        favorite.created_at = chrono::NaiveDate::from_ymd_opt(2026, 5, 4)
            .unwrap()
            .and_hms_opt(10, 0, 0)
            .unwrap();

        let images = vec![first.clone(), favorite.clone()];
        let (image, reason, _, primary) = best_profile_material_image(&images, Some(1)).unwrap();
        assert_eq!(image.id, 1);
        assert_eq!(reason, SELECTION_REASON_PROFILE_PRIMARY);
        assert!(primary);

        first.is_favorite = false;
        let images = vec![first, favorite];
        let (image, reason, _, primary) = best_profile_material_image(&images, None).unwrap();
        assert_eq!(image.id, 2);
        assert_eq!(reason, SELECTION_REASON_PROFILE_FAVORITE);
        assert!(!primary);
    }

    #[test]
    fn video_library_image_entry_includes_display_order_and_label() {
        let image = fixed_test_image(42, false);
        let candidate = VideoAssetCandidate::new(image, "manual_library", 1, 0, 0);
        let entry = video_library_image_entry(&candidate, 3);

        assert_eq!(entry["image_id"], json!(42));
        assert_eq!(entry["display_order"], json!(3));
        assert_eq!(entry["display_label"], json!("参考图 3"));
    }

    fn fixed_test_image(
        id: i64,
        is_favorite: bool,
    ) -> a4n_core::models::ai::generated_image::Model {
        a4n_core::models::ai::generated_image::Model {
            id,
            project_id: 1,
            image_type: "object_material".to_string(),
            file_path: "images/material.png".to_string(),
            prompt: "material".to_string(),
            model: "image-model".to_string(),
            width: 512,
            height: 512,
            file_size: None,
            ai_score: None,
            is_favorite,
            chapter_id: None,
            panel_id: None,
            metadata_json: Some(json!({"profile_kind": "object", "profile_id": 7})),
            created_at: chrono::NaiveDate::from_ymd_opt(2026, 5, 3)
                .unwrap()
                .and_hms_opt(10, 0, 0)
                .unwrap(),
        }
    }
}
