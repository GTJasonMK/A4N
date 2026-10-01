//! Pure profile-material projection shared by the host and its worker.
//!
//! Database access, authorization and file publication stay outside this crate.

use a4n_api_contract::profile::{
    ListProfileMaterialsInput, ProfileMaterialGroup, ProfileMaterialImageSummary,
    ProfileMaterialProfileSummary,
};
use a4n_db_models::models::{ai::generated_image, asset::profile_primary_material};
use a4n_domain::scene_profile::SceneProfileRecord;
use a4n_domain::{
    image::{IMAGE_TYPE_OBJECT_MATERIAL, IMAGE_TYPE_SCENE_MATERIAL},
    object_profile::ObjectProfileRecord,
    profile_material::{
        ProfileMaterialFilter, ProfileMaterialImageStaleness, ProfileMaterialKind,
        ProfileMaterialKindFilter, ProfileMaterialSort, ProfileMaterialState,
        profile_material_identity,
    },
    source_text_hash as source_content_sha256,
};
use chrono::NaiveDateTime;
use serde_json::Value;
use std::collections::HashMap;

pub struct ProfileMaterialSource {
    pub kind: ProfileMaterialKind,
    pub profile: ProfileMaterialProfileSummary,
    pub content_hash: String,
}

pub struct ProfileMaterialGroupProjection {
    pub group: ProfileMaterialGroup,
    newest_image_sort_key: Option<(NaiveDateTime, i64)>,
    has_favorite_image: bool,
}

pub fn requested_profile_material_kinds(
    kind: ProfileMaterialKindFilter,
) -> Vec<ProfileMaterialKind> {
    match kind {
        ProfileMaterialKindFilter::Scene => vec![ProfileMaterialKind::Scene],
        ProfileMaterialKindFilter::Object => vec![ProfileMaterialKind::Object],
        ProfileMaterialKindFilter::All => {
            vec![ProfileMaterialKind::Scene, ProfileMaterialKind::Object]
        }
    }
}

pub const fn image_type_for_profile_material_kind(kind: ProfileMaterialKind) -> &'static str {
    match kind {
        ProfileMaterialKind::Scene => IMAGE_TYPE_SCENE_MATERIAL,
        ProfileMaterialKind::Object => IMAGE_TYPE_OBJECT_MATERIAL,
    }
}

pub fn scene_profile_content(profile: &SceneProfileRecord) -> String {
    format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        profile.name,
        profile.base_description,
        profile.visual_description,
        profile.spatial_layout,
        profile.atmosphere,
        profile.entry_points_json,
        profile.rules_json,
        profile.image_prompt_seed
    )
}

pub fn object_profile_content(profile: &ObjectProfileRecord) -> String {
    format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        profile.name,
        profile.visual_description,
        profile.material_and_state,
        profile.story_usage,
        profile.visual_usage,
        profile.generation_usage,
        profile.location_context.as_deref().unwrap_or(""),
        profile.image_prompt_seed
    )
}

pub fn scene_source_from_record(profile: SceneProfileRecord) -> ProfileMaterialSource {
    let content_hash = source_content_sha256(&scene_profile_content(&profile));
    ProfileMaterialSource {
        kind: ProfileMaterialKind::Scene,
        profile: ProfileMaterialProfileSummary {
            id: profile.id,
            project_id: profile.project_id,
            name: profile.name,
            category: profile.scene_type,
            importance: profile.importance,
            first_seen_chapter: profile.first_seen_chapter,
            last_seen_chapter: profile.last_seen_chapter,
            primary_text: first_non_empty(&[
                &profile.visual_description,
                &profile.base_description,
                &profile.atmosphere,
            ]),
            secondary_text: first_non_empty(&[&profile.spatial_layout, &profile.atmosphere]),
            updated_at: Some(profile.updated_at.to_string()),
        },
        content_hash,
    }
}

pub fn object_source_from_record(profile: ObjectProfileRecord) -> ProfileMaterialSource {
    let content_hash = source_content_sha256(&object_profile_content(&profile));
    ProfileMaterialSource {
        kind: ProfileMaterialKind::Object,
        profile: ProfileMaterialProfileSummary {
            id: profile.id,
            project_id: profile.project_id,
            name: profile.name,
            category: profile.object_type,
            importance: profile.importance,
            first_seen_chapter: profile.first_seen_chapter,
            last_seen_chapter: profile.last_seen_chapter,
            primary_text: first_non_empty(&[
                &profile.visual_description,
                &profile.generation_usage,
                &profile.visual_usage,
                &profile.story_usage,
                &profile.material_and_state,
            ]),
            secondary_text: first_non_empty_opt(&[
                Some(profile.material_and_state.as_str()),
                Some(profile.visual_usage.as_str()),
                profile.location_context.as_deref(),
            ]),
            updated_at: Some(profile.updated_at.to_string()),
        },
        content_hash,
    }
}

pub fn profile_material_group_projections(
    sources: Vec<ProfileMaterialSource>,
    images: Vec<generated_image::Model>,
    primary_rows: Vec<profile_primary_material::Model>,
    image_limit: usize,
) -> Vec<ProfileMaterialGroupProjection> {
    let mut images_by_profile: HashMap<(String, i64), Vec<generated_image::Model>> = HashMap::new();
    for image in images {
        if let Some((kind, profile_id)) =
            profile_material_identity(&image.image_type, image.metadata_json.as_ref())
        {
            images_by_profile
                .entry((kind.to_string(), profile_id))
                .or_default()
                .push(image);
        }
    }
    let primary_by_profile = primary_rows
        .into_iter()
        .map(|row| ((row.profile_kind, row.profile_id), row.primary_image_id))
        .collect::<HashMap<_, _>>();
    sources
        .into_iter()
        .map(|source| {
            material_group(
                source,
                &mut images_by_profile,
                &primary_by_profile,
                image_limit,
            )
        })
        .collect()
}

pub fn apply_profile_material_query(
    groups: &mut Vec<ProfileMaterialGroupProjection>,
    input: &ListProfileMaterialsInput,
) {
    apply_group_filters(groups, input.query.as_deref(), input.filter);
    apply_group_sort(groups, input.sort);
}

fn material_group(
    source: ProfileMaterialSource,
    images_by_profile: &mut HashMap<(String, i64), Vec<generated_image::Model>>,
    primary_by_profile: &HashMap<(String, i64), i64>,
    image_limit: usize,
) -> ProfileMaterialGroupProjection {
    let key = (source.kind.as_str().to_string(), source.profile.id);
    let images = images_by_profile.remove(&key).unwrap_or_default();
    let primary_image_id = primary_by_profile.get(&key).copied();
    let newest_image_sort_key = images
        .iter()
        .map(|image| (image.created_at, image.id))
        .max();
    let has_favorite_image = images.iter().any(|image| image.is_favorite);
    let mut image_summaries = images
        .into_iter()
        .map(|image| {
            material_image_summary(
                &image,
                primary_image_id == Some(image.id),
                &source.content_hash,
            )
        })
        .collect::<Vec<_>>();
    image_summaries.sort_by(|a, b| {
        b.is_primary
            .cmp(&a.is_primary)
            .then_with(|| b.image.is_favorite.cmp(&a.image.is_favorite))
            .then_with(|| b.image.id.cmp(&a.image.id))
    });
    let primary_image = image_summaries
        .iter()
        .find(|image| image.is_primary)
        .cloned();
    let image_count = image_summaries.len();
    let material_state = material_state(&image_summaries);
    let images = image_summaries.into_iter().take(image_limit).collect();
    ProfileMaterialGroupProjection {
        group: ProfileMaterialGroup {
            kind: source.kind,
            profile: source.profile,
            material_state,
            current_profile_content_sha256: source.content_hash,
            image_count,
            primary_image_id,
            primary_image,
            images,
        },
        newest_image_sort_key,
        has_favorite_image,
    }
}

fn material_image_summary(
    image: &generated_image::Model,
    is_primary: bool,
    current_profile_hash: &str,
) -> ProfileMaterialImageSummary {
    let metadata = image.metadata_json.as_ref();
    let profile_hash = metadata_text(metadata, "profile_content_sha256");
    let staleness = match profile_hash {
        Some(hash) if hash == current_profile_hash => ProfileMaterialImageStaleness::Fresh,
        Some(_) => ProfileMaterialImageStaleness::Stale,
        None => ProfileMaterialImageStaleness::Unknown,
    };
    ProfileMaterialImageSummary {
        image: generated_image_summary(image),
        is_primary,
        staleness,
        style_template_id: metadata_text(metadata, "style_template_id").map(ToString::to_string),
        style_template_name: metadata_text(metadata, "style_template_name")
            .map(ToString::to_string),
    }
}

fn generated_image_summary(
    image: &generated_image::Model,
) -> a4n_api_contract::image::GeneratedImageSummary {
    a4n_api_contract::image::GeneratedImageSummary {
        id: image.id,
        project_id: image.project_id,
        image_type: image.image_type.clone(),
        file_path: image.file_path.clone(),
        prompt: image.prompt.clone(),
        model: image.model.clone(),
        width: image.width,
        height: image.height,
        file_size: image.file_size,
        ai_score: image.ai_score,
        is_favorite: image.is_favorite,
        is_system_managed: a4n_domain::image::is_system_managed_image_type(&image.image_type),
        chapter_id: image.chapter_id,
        panel_id: image.panel_id.clone(),
        metadata_json: image.metadata_json.clone(),
        created_at: image.created_at.to_string(),
    }
}

fn material_state(images: &[ProfileMaterialImageSummary]) -> ProfileMaterialState {
    if images.is_empty() {
        return ProfileMaterialState::Missing;
    }
    if images
        .iter()
        .any(|image| image.staleness == ProfileMaterialImageStaleness::Stale)
    {
        return ProfileMaterialState::Stale;
    }
    if images
        .iter()
        .all(|image| image.staleness == ProfileMaterialImageStaleness::Unknown)
    {
        return ProfileMaterialState::Unknown;
    }
    ProfileMaterialState::Ready
}

fn apply_group_filters(
    groups: &mut Vec<ProfileMaterialGroupProjection>,
    query: Option<&str>,
    filter: ProfileMaterialFilter,
) {
    if let Some(query) = query.map(str::trim).filter(|value| !value.is_empty()) {
        let query = query.to_lowercase();
        groups.retain(|projection| {
            projection
                .group
                .profile
                .name
                .to_lowercase()
                .contains(&query)
        });
    }
    match filter {
        ProfileMaterialFilter::All => {}
        ProfileMaterialFilter::Missing => groups
            .retain(|projection| projection.group.material_state == ProfileMaterialState::Missing),
        ProfileMaterialFilter::Ready => groups
            .retain(|projection| projection.group.material_state == ProfileMaterialState::Ready),
        ProfileMaterialFilter::Stale => groups
            .retain(|projection| projection.group.material_state == ProfileMaterialState::Stale),
        ProfileMaterialFilter::Unknown => groups
            .retain(|projection| projection.group.material_state == ProfileMaterialState::Unknown),
        ProfileMaterialFilter::Favorite => {
            groups.retain(|projection| projection.has_favorite_image)
        }
    }
}

fn apply_group_sort(groups: &mut [ProfileMaterialGroupProjection], sort: ProfileMaterialSort) {
    match sort {
        ProfileMaterialSort::StaleFirst => groups.sort_by(|a, b| {
            stale_material_state_rank(a.group.material_state)
                .cmp(&stale_material_state_rank(b.group.material_state))
                .then_with(|| {
                    a.group
                        .profile
                        .first_seen_chapter
                        .cmp(&b.group.profile.first_seen_chapter)
                })
                .then_with(|| a.group.profile.id.cmp(&b.group.profile.id))
        }),
        ProfileMaterialSort::Newest => {
            groups.sort_by_key(|projection| std::cmp::Reverse(projection.newest_image_sort_key))
        }
        ProfileMaterialSort::Importance => groups.sort_by(|a, b| {
            material_importance_rank(&a.group.profile.importance)
                .cmp(&material_importance_rank(&b.group.profile.importance))
                .then_with(|| {
                    a.group
                        .profile
                        .first_seen_chapter
                        .cmp(&b.group.profile.first_seen_chapter)
                })
                .then_with(|| a.group.profile.id.cmp(&b.group.profile.id))
        }),
        ProfileMaterialSort::MissingFirst => groups.sort_by(|a, b| {
            missing_material_state_rank(a.group.material_state)
                .cmp(&missing_material_state_rank(b.group.material_state))
                .then_with(|| {
                    a.group
                        .profile
                        .first_seen_chapter
                        .cmp(&b.group.profile.first_seen_chapter)
                })
                .then_with(|| a.group.profile.id.cmp(&b.group.profile.id))
        }),
    }
}

fn missing_material_state_rank(state: ProfileMaterialState) -> u8 {
    match state {
        ProfileMaterialState::Missing => 0,
        ProfileMaterialState::Stale => 1,
        ProfileMaterialState::Unknown => 2,
        ProfileMaterialState::Ready => 3,
    }
}

fn stale_material_state_rank(state: ProfileMaterialState) -> u8 {
    match state {
        ProfileMaterialState::Stale => 0,
        ProfileMaterialState::Missing => 1,
        ProfileMaterialState::Unknown => 2,
        ProfileMaterialState::Ready => 3,
    }
}

fn material_importance_rank(value: &str) -> i32 {
    match value {
        "primary" | "主要" | "high" => 0,
        "secondary" | "次要" | "medium" => 1,
        _ => 2,
    }
}

fn metadata_text<'a>(metadata: Option<&'a Value>, key: &str) -> Option<&'a str> {
    metadata
        .and_then(|metadata| metadata.get(key))
        .and_then(Value::as_str)
        .and_then(non_empty)
}

fn first_non_empty(values: &[&str]) -> String {
    values
        .iter()
        .map(|value| value.trim())
        .find(|value| !value.is_empty())
        .unwrap_or("")
        .to_string()
}

fn first_non_empty_opt(values: &[Option<&str>]) -> String {
    values
        .iter()
        .flatten()
        .map(|value| value.trim())
        .find(|value| !value.is_empty())
        .unwrap_or("")
        .to_string()
}

fn non_empty(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_domain::profile_material::{
        ProfileMaterialFilter, ProfileMaterialKindFilter, ProfileMaterialSort,
    };
    use chrono::NaiveDate;
    use serde_json::json;

    fn source(id: i64, state_hash: &str) -> ProfileMaterialSource {
        ProfileMaterialSource {
            kind: ProfileMaterialKind::Scene,
            profile: ProfileMaterialProfileSummary {
                id,
                project_id: 1,
                name: format!("场景 {id}"),
                category: "interior".into(),
                importance: "primary".into(),
                first_seen_chapter: 1,
                last_seen_chapter: 1,
                primary_text: "视觉描述".into(),
                secondary_text: "空间布局".into(),
                updated_at: None,
            },
            content_hash: state_hash.into(),
        }
    }

    fn image(
        id: i64,
        profile_id: i64,
        hash: Option<&str>,
        favorite: bool,
    ) -> generated_image::Model {
        generated_image::Model {
            id,
            project_id: 1,
            image_type: IMAGE_TYPE_SCENE_MATERIAL.into(),
            file_path: format!("images/{id}.png"),
            prompt: "prompt".into(),
            model: "model".into(),
            width: 1,
            height: 1,
            file_size: None,
            ai_score: None,
            is_favorite: favorite,
            chapter_id: None,
            panel_id: None,
            metadata_json: Some(json!({
                "profile_kind": "scene",
                "profile_id": profile_id,
                "profile_content_sha256": hash,
            })),
            created_at: NaiveDate::from_ymd_opt(2026, 1, id as u32)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
        }
    }

    #[test]
    fn projection_preserves_primary_order_and_staleness_filtering() {
        let groups = profile_material_group_projections(
            vec![source(1, "current"), source(2, "current")],
            vec![
                image(1, 1, Some("current"), false),
                image(2, 1, Some("old"), true),
            ],
            Vec::new(),
            1,
        );
        assert_eq!(groups[0].group.image_count, 2);
        assert_eq!(groups[0].group.images.len(), 1);
        assert_eq!(groups[0].group.material_state, ProfileMaterialState::Stale);

        let mut filtered = groups;
        let input = ListProfileMaterialsInput {
            project_id: 1,
            kind: ProfileMaterialKindFilter::All,
            query: None,
            filter: ProfileMaterialFilter::Stale,
            sort: ProfileMaterialSort::StaleFirst,
            image_limit_per_profile: 1,
        };
        apply_profile_material_query(&mut filtered, &input);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].group.profile.id, 1);
    }
}
