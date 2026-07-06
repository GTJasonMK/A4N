use crate::AppState;
use crate::actions::access;
use crate::actions::image_generation::{GeneratedImageSummary, to_summary};
use crate::actions::image_records;
use crate::actions::uploaded_image::decode_uploaded_image_data;
use a4n_core::models::ai::generated_image;
use a4n_core::models::asset::{object_profile, profile_primary_material};
use a4n_core::models::scene::scene_profile;
use a4n_services::image::CreateImageRecord;
use a4n_services::profile_material::{
    IMAGE_TYPE_OBJECT_MATERIAL, IMAGE_TYPE_SCENE_MATERIAL, PROFILE_KIND_OBJECT, PROFILE_KIND_SCENE,
    generated_image_profile_material_identity,
};
use a4n_services::source::source_content_sha256;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Path, PathBuf};

const KIND_SCENE: &str = PROFILE_KIND_SCENE;
const KIND_OBJECT: &str = PROFILE_KIND_OBJECT;
const IMAGE_TYPE_SCENE: &str = IMAGE_TYPE_SCENE_MATERIAL;
const IMAGE_TYPE_OBJECT: &str = IMAGE_TYPE_OBJECT_MATERIAL;
const MANUAL_UPLOAD_MODEL: &str = "manual_upload";
const MAX_UPLOAD_IMAGE_BYTES: usize = 20 * 1024 * 1024;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListProfileMaterialsInput {
    pub project_id: i64,
    pub kind: Option<String>,
    pub query: Option<String>,
    pub filter: Option<String>,
    pub sort: Option<String>,
    pub image_limit_per_profile: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetProfilePrimaryMaterialInput {
    pub project_id: i64,
    pub profile_kind: String,
    pub profile_id: i64,
    pub image_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UploadProfileMaterialImageInput {
    pub project_id: i64,
    pub profile_kind: String,
    pub profile_id: i64,
    pub file_name: String,
    pub mime_type: String,
    pub data_base64: String,
    pub set_primary: bool,
    pub prompt_snapshot: Option<String>,
    pub style_template_id: Option<String>,
    pub style_template_name: Option<String>,
    pub style_template_version: Option<String>,
    pub custom_style: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProfileMaterialGroup {
    pub kind: String,
    pub profile: ProfileMaterialProfileSummary,
    pub material_state: String,
    pub current_profile_content_sha256: String,
    pub image_count: usize,
    pub primary_image_id: Option<i64>,
    pub primary_image: Option<ProfileMaterialImageSummary>,
    pub images: Vec<ProfileMaterialImageSummary>,
    #[serde(skip_serializing)]
    newest_image_sort_key: Option<(chrono::NaiveDateTime, i64)>,
    #[serde(skip_serializing)]
    has_favorite_image: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProfileMaterialProfileSummary {
    pub id: i64,
    pub project_id: i64,
    pub name: String,
    pub category: String,
    pub importance: String,
    pub first_seen_chapter: i32,
    pub last_seen_chapter: i32,
    pub primary_text: String,
    pub secondary_text: String,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProfileMaterialImageSummary {
    pub image: GeneratedImageSummary,
    pub is_primary: bool,
    pub staleness: String,
    pub style_template_id: Option<String>,
    pub style_template_name: Option<String>,
}

#[derive(Debug, Clone)]
struct ProfileMaterialSource {
    kind: &'static str,
    profile: ProfileMaterialProfileSummary,
    content_hash: String,
}

#[derive(Debug)]
struct UploadedProfileMaterialImage {
    bytes: Vec<u8>,
    mime_type: &'static str,
    extension: &'static str,
    width: i32,
    height: i32,
    file_size: i64,
    source_file_name: String,
}

pub async fn list_profile_materials(
    state: &AppState,
    user_id: i64,
    input: ListProfileMaterialsInput,
) -> Result<Vec<ProfileMaterialGroup>, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let kinds = requested_kinds(input.kind.as_deref())?;
    let image_limit = input.image_limit_per_profile.unwrap_or(8).clamp(1, 32);

    let mut profiles = Vec::new();
    if kinds.contains(&KIND_SCENE) {
        profiles.extend(scene_sources(state, input.project_id).await?);
    }
    if kinds.contains(&KIND_OBJECT) {
        profiles.extend(object_sources(state, input.project_id).await?);
    }

    let images = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.eq(input.project_id))
        .filter(generated_image::Column::ImageType.is_in([IMAGE_TYPE_SCENE, IMAGE_TYPE_OBJECT]))
        .order_by_desc(generated_image::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?;
    let mut images_by_profile: BTreeMap<(String, i64), Vec<generated_image::Model>> =
        BTreeMap::new();
    for image in images {
        if let Some((kind, profile_id)) = material_identity(&image) {
            images_by_profile
                .entry((kind.to_string(), profile_id))
                .or_default()
                .push(image);
        }
    }

    let primary_rows = profile_primary_material::Entity::find()
        .filter(profile_primary_material::Column::ProjectId.eq(input.project_id))
        .all(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?;
    let primary_by_profile: BTreeMap<(String, i64), i64> = primary_rows
        .into_iter()
        .map(|row| ((row.profile_kind, row.profile_id), row.primary_image_id))
        .collect();

    let mut groups = profiles
        .into_iter()
        .map(|source| {
            material_group(
                source,
                &mut images_by_profile,
                &primary_by_profile,
                image_limit,
            )
        })
        .collect::<Vec<_>>();

    apply_group_filters(&mut groups, input.query.as_deref(), input.filter.as_deref());
    apply_group_sort(&mut groups, input.sort.as_deref());
    Ok(groups)
}

pub async fn set_profile_primary_material(
    state: &AppState,
    user_id: i64,
    input: SetProfilePrimaryMaterialInput,
) -> Result<ProfileMaterialGroup, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let kind = normalize_kind(&input.profile_kind)?;
    ensure_profile_exists(state, input.project_id, kind, input.profile_id).await?;

    if let Some(image_id) = input.image_id {
        let image = state
            .image
            .get_image(image_id)
            .await
            .map_err(|err| err.to_string())?;
        if image.project_id != input.project_id {
            return Err(format!("图片 #{image_id} 不属于项目 {}", input.project_id));
        }
        if material_identity(&image) != Some((kind, input.profile_id)) {
            return Err(format!(
                "图片 #{image_id} 不是 {} 档案 #{} 的素材图",
                kind, input.profile_id
            ));
        }
        upsert_primary_material(state, input.project_id, kind, input.profile_id, image_id).await?;
    } else {
        profile_primary_material::Entity::delete_many()
            .filter(profile_primary_material::Column::ProjectId.eq(input.project_id))
            .filter(profile_primary_material::Column::ProfileKind.eq(kind))
            .filter(profile_primary_material::Column::ProfileId.eq(input.profile_id))
            .exec(state.db.as_ref())
            .await
            .map_err(|err| err.to_string())?;
    }

    let groups = list_profile_materials(
        state,
        user_id,
        ListProfileMaterialsInput {
            project_id: input.project_id,
            kind: Some(kind.to_string()),
            query: None,
            filter: None,
            sort: None,
            image_limit_per_profile: Some(32),
        },
    )
    .await?;
    groups
        .into_iter()
        .find(|group| group.profile.id == input.profile_id)
        .ok_or_else(|| "主素材已更新，但无法重新读取档案素材".to_string())
}

pub async fn upload_profile_material_image(
    state: &AppState,
    user_id: i64,
    input: UploadProfileMaterialImageInput,
) -> Result<ProfileMaterialGroup, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let kind = normalize_kind(&input.profile_kind)?;
    let uploaded = decode_uploaded_profile_material_image(&input)?;
    let style = ProfileMaterialStyleMetadata {
        style_template_id: input.style_template_id.clone(),
        style_template_name: input.style_template_name.clone(),
        style_template_version: input.style_template_version.clone(),
        custom_style: input.custom_style.clone(),
    };

    let (source, image_type, prompt) = match kind {
        KIND_SCENE => {
            let profile = state
                .scene_profile
                .get(input.profile_id)
                .await
                .map_err(|err| err.to_string())?;
            if profile.project_id != input.project_id {
                return Err(format!(
                    "场景档案 #{} 不属于项目 {}",
                    input.profile_id, input.project_id
                ));
            }
            let prompt = input
                .prompt_snapshot
                .as_deref()
                .and_then(non_empty)
                .map(ToString::to_string)
                .unwrap_or_else(|| scene_profile_image_prompt(&profile, None));
            (scene_source_from_model(profile), IMAGE_TYPE_SCENE, prompt)
        }
        KIND_OBJECT => {
            let profile = state
                .object_profile
                .get(input.profile_id)
                .await
                .map_err(|err| err.to_string())?;
            if profile.project_id != input.project_id {
                return Err(format!(
                    "物体档案 #{} 不属于项目 {}",
                    input.profile_id, input.project_id
                ));
            }
            let prompt = input
                .prompt_snapshot
                .as_deref()
                .and_then(non_empty)
                .map(ToString::to_string)
                .unwrap_or_else(|| object_profile_image_prompt(&profile, None));
            (object_source_from_model(profile), IMAGE_TYPE_OBJECT, prompt)
        }
        _ => return Err(format!("不支持的素材档案类型: {kind}")),
    };

    let path = save_uploaded_profile_material_image(
        state,
        input.project_id,
        kind,
        input.profile_id,
        uploaded.extension,
        &uploaded.bytes,
    )?;
    let path_str = crate::file_utils::data_dir_relative_file_path(
        &state.config.data_dir,
        &path,
        "上传素材图",
    )?;

    let mut metadata = profile_material_metadata(
        kind,
        source.profile.id,
        &source.profile.name,
        &source.content_hash,
        &style,
    );
    if let Some(map) = metadata.as_object_mut() {
        map.insert("source".to_string(), json!("manual_upload"));
        map.insert("manual_upload".to_string(), json!(true));
        map.insert("chapter_video_material".to_string(), json!(true));
        map.insert("prompt_snapshot".to_string(), json!(prompt));
        map.insert(
            "upload".to_string(),
            json!({
                "original_file_name": uploaded.source_file_name,
                "mime_type": uploaded.mime_type,
                "file_size": uploaded.file_size,
            }),
        );
    }

    let now = Utc::now().naive_utc();
    let image = match state
        .image
        .create_record(CreateImageRecord {
            project_id: input.project_id,
            image_type: image_type.to_string(),
            file_path: path_str,
            prompt,
            model: MANUAL_UPLOAD_MODEL.to_string(),
            width: uploaded.width,
            height: uploaded.height,
            file_size: Some(uploaded.file_size),
            chapter_id: None,
            panel_id: Some(format!("{kind}_profile_{}", input.profile_id)),
            metadata_json: Some(metadata),
            created_at: Some(now),
        })
        .await
    {
        Ok(image) => image,
        Err(err) => {
            let cleanup_error = remove_uploaded_profile_material_file(&path).err();
            return Err(append_cleanup_error(
                format!("素材图记录创建失败：{err}"),
                cleanup_error,
            ));
        }
    };

    if input.set_primary
        && let Err(err) =
            upsert_primary_material(state, input.project_id, kind, input.profile_id, image.id).await
    {
        let cleanup_error = cleanup_uploaded_profile_material_image(state, user_id, image.id)
            .await
            .err();
        return Err(append_cleanup_error(err, cleanup_error));
    }

    read_single_profile_material_group(state, user_id, input.project_id, kind, input.profile_id)
        .await
}

fn material_group(
    source: ProfileMaterialSource,
    images_by_profile: &mut BTreeMap<(String, i64), Vec<generated_image::Model>>,
    primary_by_profile: &BTreeMap<(String, i64), i64>,
    image_limit: usize,
) -> ProfileMaterialGroup {
    let key = (source.kind.to_string(), source.profile.id);
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
    ProfileMaterialGroup {
        kind: source.kind.to_string(),
        profile: source.profile,
        material_state,
        current_profile_content_sha256: source.content_hash,
        image_count,
        primary_image_id,
        primary_image,
        images,
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
        Some(hash) if hash == current_profile_hash => "fresh",
        Some(_) => "stale",
        None => "unknown",
    }
    .to_string();
    ProfileMaterialImageSummary {
        image: to_summary(image),
        is_primary,
        staleness,
        style_template_id: metadata_text(metadata, "style_template_id").map(ToString::to_string),
        style_template_name: metadata_text(metadata, "style_template_name")
            .map(ToString::to_string),
    }
}

fn material_state(images: &[ProfileMaterialImageSummary]) -> String {
    if images.is_empty() {
        return "missing".to_string();
    }
    if images.iter().any(|image| image.staleness == "stale") {
        return "stale".to_string();
    }
    if images.iter().all(|image| image.staleness == "unknown") {
        return "unknown".to_string();
    }
    "ready".to_string()
}

async fn scene_sources(
    state: &AppState,
    project_id: i64,
) -> Result<Vec<ProfileMaterialSource>, String> {
    let profiles = state
        .scene_profile
        .list_by_project(project_id)
        .await
        .map_err(|err| err.to_string())?;
    Ok(profiles.into_iter().map(scene_source_from_model).collect())
}

async fn object_sources(
    state: &AppState,
    project_id: i64,
) -> Result<Vec<ProfileMaterialSource>, String> {
    let profiles = state
        .object_profile
        .list_by_project(project_id)
        .await
        .map_err(|err| err.to_string())?;
    Ok(profiles.into_iter().map(object_source_from_model).collect())
}

fn scene_source_from_model(profile: scene_profile::Model) -> ProfileMaterialSource {
    let content_hash = source_content_sha256(&scene_profile_content(&profile));
    ProfileMaterialSource {
        kind: KIND_SCENE,
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

fn object_source_from_model(profile: object_profile::Model) -> ProfileMaterialSource {
    let content_hash = source_content_sha256(&object_profile_content(&profile));
    ProfileMaterialSource {
        kind: KIND_OBJECT,
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

async fn read_single_profile_material_group(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    kind: &str,
    profile_id: i64,
) -> Result<ProfileMaterialGroup, String> {
    let groups = list_profile_materials(
        state,
        user_id,
        ListProfileMaterialsInput {
            project_id,
            kind: Some(kind.to_string()),
            query: None,
            filter: None,
            sort: None,
            image_limit_per_profile: Some(32),
        },
    )
    .await?;
    groups
        .into_iter()
        .find(|group| group.profile.id == profile_id)
        .ok_or_else(|| "素材图已上传，但无法重新读取档案素材".to_string())
}

async fn ensure_profile_exists(
    state: &AppState,
    project_id: i64,
    kind: &str,
    profile_id: i64,
) -> Result<(), String> {
    match kind {
        KIND_SCENE => {
            let profile = state
                .scene_profile
                .get(profile_id)
                .await
                .map_err(|err| err.to_string())?;
            if profile.project_id != project_id {
                return Err(format!("场景档案 #{profile_id} 不属于项目 {project_id}"));
            }
        }
        KIND_OBJECT => {
            let profile = state
                .object_profile
                .get(profile_id)
                .await
                .map_err(|err| err.to_string())?;
            if profile.project_id != project_id {
                return Err(format!("物体档案 #{profile_id} 不属于项目 {project_id}"));
            }
        }
        _ => return Err(format!("不支持的素材档案类型: {kind}")),
    }
    Ok(())
}

async fn upsert_primary_material(
    state: &AppState,
    project_id: i64,
    kind: &str,
    profile_id: i64,
    image_id: i64,
) -> Result<(), String> {
    let existing = profile_primary_material::Entity::find()
        .filter(profile_primary_material::Column::ProjectId.eq(project_id))
        .filter(profile_primary_material::Column::ProfileKind.eq(kind))
        .filter(profile_primary_material::Column::ProfileId.eq(profile_id))
        .one(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?;
    let now = Utc::now().naive_utc();
    match existing {
        Some(row) => {
            let mut active: profile_primary_material::ActiveModel = row.into();
            active.primary_image_id = Set(image_id);
            active.updated_at = Set(now);
            active
                .update(state.db.as_ref())
                .await
                .map_err(|err| err.to_string())?;
        }
        None => {
            profile_primary_material::ActiveModel {
                project_id: Set(project_id),
                profile_kind: Set(kind.to_string()),
                profile_id: Set(profile_id),
                primary_image_id: Set(image_id),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(state.db.as_ref())
            .await
            .map_err(|err| err.to_string())?;
        }
    }
    Ok(())
}

fn apply_group_filters(
    groups: &mut Vec<ProfileMaterialGroup>,
    query: Option<&str>,
    filter: Option<&str>,
) {
    if let Some(query) = query.map(str::trim).filter(|value| !value.is_empty()) {
        let query = query.to_lowercase();
        groups.retain(|group| group.profile.name.to_lowercase().contains(&query));
    }
    match filter.map(str::trim).filter(|value| !value.is_empty()) {
        Some("missing") => groups.retain(|group| group.material_state == "missing"),
        Some("ready") => groups.retain(|group| group.material_state == "ready"),
        Some("stale") => groups.retain(|group| group.material_state == "stale"),
        Some("unknown") => groups.retain(|group| group.material_state == "unknown"),
        Some("favorite") => groups.retain(|group| group.has_favorite_image),
        _ => {}
    }
}

fn apply_group_sort(groups: &mut [ProfileMaterialGroup], sort: Option<&str>) {
    match sort.map(str::trim).filter(|value| !value.is_empty()) {
        Some("stale_first") => groups.sort_by(|a, b| {
            state_rank(&a.material_state)
                .cmp(&state_rank(&b.material_state))
                .then_with(|| {
                    a.profile
                        .first_seen_chapter
                        .cmp(&b.profile.first_seen_chapter)
                })
        }),
        Some("newest") => groups.sort_by_key(|group| std::cmp::Reverse(newest_image_id(group))),
        Some("importance") => groups.sort_by(|a, b| {
            importance_rank(&a.profile.importance)
                .cmp(&importance_rank(&b.profile.importance))
                .then_with(|| {
                    a.profile
                        .first_seen_chapter
                        .cmp(&b.profile.first_seen_chapter)
                })
        }),
        _ => groups.sort_by(|a, b| {
            state_rank(&a.material_state)
                .cmp(&state_rank(&b.material_state))
                .then_with(|| {
                    a.profile
                        .first_seen_chapter
                        .cmp(&b.profile.first_seen_chapter)
                })
                .then_with(|| a.profile.id.cmp(&b.profile.id))
        }),
    }
}

fn newest_image_id(group: &ProfileMaterialGroup) -> Option<(chrono::NaiveDateTime, i64)> {
    group.newest_image_sort_key
}

fn state_rank(state: &str) -> i32 {
    match state {
        "missing" => 0,
        "stale" => 1,
        "unknown" => 2,
        _ => 3,
    }
}

fn importance_rank(value: &str) -> i32 {
    match value {
        "primary" | "主要" | "high" => 0,
        "secondary" | "次要" | "medium" => 1,
        _ => 2,
    }
}

fn requested_kinds(kind: Option<&str>) -> Result<Vec<&'static str>, String> {
    match kind.map(str::trim).filter(|value| !value.is_empty()) {
        Some("scene") => Ok(vec![KIND_SCENE]),
        Some("object") => Ok(vec![KIND_OBJECT]),
        Some("all") | None => Ok(vec![KIND_SCENE, KIND_OBJECT]),
        Some(other) => Err(format!("不支持的素材档案类型: {other}")),
    }
}

fn normalize_kind(kind: &str) -> Result<&'static str, String> {
    match kind.trim() {
        KIND_SCENE => Ok(KIND_SCENE),
        KIND_OBJECT => Ok(KIND_OBJECT),
        other => Err(format!("不支持的素材档案类型: {other}")),
    }
}

fn decode_uploaded_profile_material_image(
    input: &UploadProfileMaterialImageInput,
) -> Result<UploadedProfileMaterialImage, String> {
    let decoded = decode_uploaded_image_data(
        &input.file_name,
        &input.mime_type,
        &input.data_base64,
        MAX_UPLOAD_IMAGE_BYTES,
    )?;

    Ok(UploadedProfileMaterialImage {
        bytes: decoded.bytes,
        mime_type: decoded.mime_type,
        extension: decoded.extension,
        width: decoded.width,
        height: decoded.height,
        file_size: decoded.file_size,
        source_file_name: decoded.source_file_name,
    })
}

fn save_uploaded_profile_material_image(
    state: &AppState,
    project_id: i64,
    kind: &str,
    profile_id: i64,
    extension: &str,
    bytes: &[u8],
) -> Result<PathBuf, String> {
    let dir = state
        .config
        .data_dir
        .join("projects")
        .join(project_id.to_string())
        .join("images")
        .join("profile_materials");
    std::fs::create_dir_all(&dir).map_err(|err| format!("无法创建素材图目录：{err}"))?;
    let stamp = Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_else(|| Utc::now().timestamp_micros() * 1000);
    for attempt in 0..20 {
        let suffix = if attempt == 0 {
            String::new()
        } else {
            format!("_{attempt}")
        };
        let file_name = format!("{kind}_{profile_id}_{stamp}{suffix}.{extension}");
        let path = dir.join(file_name);
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => {
                file.write_all(bytes)
                    .map_err(|err| format!("写入上传素材图失败：{err}"))?;
                return Ok(path);
            }
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(format!("创建上传素材图失败：{err}")),
        }
    }
    Err("创建上传素材图失败：文件名冲突过多".to_string())
}

async fn cleanup_uploaded_profile_material_image(
    state: &AppState,
    user_id: i64,
    image_id: i64,
) -> Result<(), String> {
    image_records::delete_image(state, user_id, image_id)
        .await
        .map_err(|err| format!("素材图记录回滚失败：{err}"))
}

fn remove_uploaded_profile_material_file(path: &Path) -> Result<(), String> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("素材图文件清理失败：{}：{err}", path.display())),
    }
}

fn append_cleanup_error(primary: String, cleanup_error: Option<String>) -> String {
    match cleanup_error {
        Some(cleanup_error) if !cleanup_error.trim().is_empty() => {
            format!("{primary}；后续清理也失败：{cleanup_error}")
        }
        _ => primary,
    }
}

fn material_identity(image: &generated_image::Model) -> Option<(&'static str, i64)> {
    generated_image_profile_material_identity(image)
}

pub(crate) async fn count_profile_material_images(
    state: &AppState,
    project_id: i64,
    kind: &str,
    profile_id: i64,
) -> Result<i64, String> {
    let kind = normalize_kind(kind)?;
    let image_type = if kind == KIND_SCENE {
        IMAGE_TYPE_SCENE
    } else {
        IMAGE_TYPE_OBJECT
    };
    let images = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.eq(project_id))
        .filter(generated_image::Column::ImageType.eq(image_type))
        .all(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?;
    Ok(images
        .into_iter()
        .filter(|image| material_identity(image) == Some((kind, profile_id)))
        .count() as i64)
}

pub(crate) async fn count_profile_material_images_by_profile(
    state: &AppState,
    project_id: i64,
    kind: &str,
    profile_ids: &[i64],
) -> Result<BTreeMap<i64, i64>, String> {
    let kind = normalize_kind(kind)?;
    if profile_ids.is_empty() {
        return Ok(BTreeMap::new());
    }
    let image_type = if kind == KIND_SCENE {
        IMAGE_TYPE_SCENE
    } else {
        IMAGE_TYPE_OBJECT
    };
    let requested_ids = profile_ids.iter().copied().collect::<BTreeSet<_>>();
    let images = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.eq(project_id))
        .filter(generated_image::Column::ImageType.eq(image_type))
        .all(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?;
    let mut counts = BTreeMap::new();
    for image in images {
        let Some((image_kind, profile_id)) = material_identity(&image) else {
            continue;
        };
        if image_kind == kind && requested_ids.contains(&profile_id) {
            *counts.entry(profile_id).or_insert(0) += 1;
        }
    }
    Ok(counts)
}

pub(crate) async fn list_profile_material_image_ids(
    state: &AppState,
    project_id: i64,
    kind: &str,
    profile_id: i64,
) -> Result<Vec<i64>, String> {
    let kind = normalize_kind(kind)?;
    let image_type = if kind == KIND_SCENE {
        IMAGE_TYPE_SCENE
    } else {
        IMAGE_TYPE_OBJECT
    };
    let images = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.eq(project_id))
        .filter(generated_image::Column::ImageType.eq(image_type))
        .order_by_asc(generated_image::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?;
    Ok(images
        .into_iter()
        .filter(|image| material_identity(image) == Some((kind, profile_id)))
        .map(|image| image.id)
        .collect())
}

pub(crate) fn profile_material_metadata(
    kind: &str,
    profile_id: i64,
    profile_name: &str,
    profile_content_sha256: &str,
    style: &ProfileMaterialStyleMetadata,
) -> Value {
    let mut metadata = serde_json::Map::new();
    metadata.insert("material_kind".to_string(), json!("profile_material"));
    metadata.insert("profile_kind".to_string(), json!(kind));
    metadata.insert("profile_id".to_string(), json!(profile_id));
    metadata.insert("profile_name".to_string(), json!(profile_name));
    metadata.insert(
        "profile_content_sha256".to_string(),
        json!(profile_content_sha256),
    );
    if let Some(value) = style.style_template_id.as_deref().and_then(non_empty) {
        metadata.insert("style_template_id".to_string(), json!(value));
    }
    if let Some(value) = style.style_template_name.as_deref().and_then(non_empty) {
        metadata.insert("style_template_name".to_string(), json!(value));
    }
    if let Some(value) = style.style_template_version.as_deref().and_then(non_empty) {
        metadata.insert("style_template_version".to_string(), json!(value));
    }
    if let Some(value) = style.custom_style.as_deref().and_then(non_empty) {
        metadata.insert("custom_style".to_string(), json!(value));
    }
    Value::Object(metadata)
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct ProfileMaterialStyleMetadata {
    pub style_template_id: Option<String>,
    pub style_template_name: Option<String>,
    pub style_template_version: Option<String>,
    pub custom_style: Option<String>,
}

pub(crate) fn scene_profile_image_prompt(
    profile: &scene_profile::Model,
    style: Option<&str>,
) -> String {
    let style = style
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("清晰写实的场景概念设定图，空间明确，适合作为视频场景参考");
    let seed = profile.image_prompt_seed.trim();
    let seed = if seed.is_empty() {
        "根据场景档案生成可复用的地点与空间设定图"
    } else {
        seed
    };
    let entry_points = compact_json_hint(&profile.entry_points_json, "未记录明确出入口");
    let rules = compact_json_hint(&profile.rules_json, "未记录特殊规则");
    let current_state = compact_json_hint(&profile.current_state_json, "未记录当前状态变化");
    format!(
        "主体：{name}\n\
         素材定位：可复用的场景/地点设定图，只展示空间环境本身，不表现具体剧情动作。\n\
         参考画面：{seed}\n\
         场景基础：{base}\n\
         视觉特征：{visual}\n\
         空间布局：{layout}\n\
         氛围与光线：{atmosphere}\n\
         出入口与连接：{entry_points}\n\
         场景规则：{rules}\n\
         当前状态：{current_state}\n\
         构图与比例：以空间关系、可行动线、入口出口、主体区域和关键环境物件为重点；不要让人物成为主体；背景不能空泛，适合作为后续视频参考素材。\n\
         画面风格：{style}\n\
         风格执行：如果画面风格中包含“风格模板”或“风格关键词”，必须保留其中的中文风格名和关键英文风格名，并体现对应光影、色彩、构图、镜头、材质、氛围、细节和避免项，不要只概括成统一风格。\n\
         避免项：不要文字、不要水印、不要边框、不要纯抽象氛围图、不要把场景画成单个物体特写。",
        name = profile.name,
        seed = seed,
        base = profile.base_description,
        visual = profile.visual_description,
        layout = profile.spatial_layout,
        atmosphere = profile.atmosphere,
        entry_points = entry_points,
        rules = rules,
        current_state = current_state,
        style = style,
    )
}

pub(crate) fn object_profile_image_prompt(
    profile: &object_profile::Model,
    style: Option<&str>,
) -> String {
    let style = style
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("清晰写实的概念设定图，主体明确，背景简洁");
    let location = profile
        .location_context
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("不强调具体地点");
    let seed = profile.image_prompt_seed.trim();
    let seed = if seed.is_empty() {
        "根据外观细节生成清晰的单物体素材图"
    } else {
        seed
    };
    format!(
        "主体：{name}\n\
         素材定位：单个物体的静物道具素材图，只展示物体外观，不表现使用过程。\n\
         参考画面：{seed}\n\
         外观细节：{visual}\n\
         材质与当前状态：{material}\n\
         环境语境：{location}\n\
         构图与比例：以单个物体为画面中心，完整展示轮廓和关键细节，背景简洁，不出现人物持握、攻击、伤害或威胁行为，适合作为后续视频参考素材。\n\
         画面风格：{style}\n\
         风格执行：如果画面风格中包含“风格模板”或“风格关键词”，必须保留其中的中文风格名和关键英文风格名，并体现对应光影、色彩、构图、镜头、材质、氛围、细节和避免项，不要只概括成统一风格。\n\
         避免项：不要文字、不要水印、不要边框、不要多余人物、不要把它画成其他物体。",
        name = profile.name,
        seed = seed,
        visual = profile.visual_description,
        material = profile.material_and_state,
        location = location,
        style = style,
    )
}

pub(crate) fn scene_profile_content(profile: &scene_profile::Model) -> String {
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

pub(crate) fn object_profile_content(profile: &object_profile::Model) -> String {
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
    if value.is_empty() { None } else { Some(value) }
}

fn compact_json_hint(value: &Value, empty_text: &str) -> String {
    match value {
        Value::Null => empty_text.to_string(),
        Value::Array(items) if items.is_empty() => empty_text.to_string(),
        Value::Object(map) if map.is_empty() => empty_text.to_string(),
        _ => serde_json::to_string(value).unwrap_or_else(|_| empty_text.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use std::collections::BTreeMap;

    #[test]
    fn material_identity_requires_current_profile_metadata() {
        let scene = image(
            1,
            IMAGE_TYPE_SCENE,
            json!({ "profile_kind": KIND_SCENE, "profile_id": 42 }),
        );
        let object = image(
            2,
            IMAGE_TYPE_OBJECT,
            json!({ "profile_kind": KIND_OBJECT, "profile_id": 7 }),
        );
        let old_metadata = image(3, IMAGE_TYPE_SCENE, json!({ "scene_profile_id": "42" }));

        assert_eq!(material_identity(&scene), Some((KIND_SCENE, 42)));
        assert_eq!(material_identity(&object), Some((KIND_OBJECT, 7)));
        assert_eq!(material_identity(&old_metadata), None);
    }

    #[test]
    fn profile_material_metadata_uses_current_profile_fields() {
        let metadata = profile_material_metadata(
            KIND_SCENE,
            9,
            "东市",
            "hash",
            &ProfileMaterialStyleMetadata {
                style_template_id: Some("kyoani-everyday-anime".to_string()),
                style_template_name: Some("京阿尼动画风格".to_string()),
                style_template_version: Some("1".to_string()),
                custom_style: None,
            },
        );

        assert!(metadata.get("scene_profile_id").is_none());
        assert_eq!(metadata["profile_kind"], json!("scene"));
        assert_eq!(metadata["profile_id"], json!(9));
        assert_eq!(metadata["profile_content_sha256"], json!("hash"));
        assert_eq!(
            metadata["style_template_id"],
            json!("kyoani-everyday-anime")
        );
    }

    #[test]
    fn material_state_prefers_missing_and_stale_over_ready() {
        assert_eq!(material_state(&[]), "missing");

        let fresh = image_summary(1, "fresh");
        assert_eq!(material_state(&[fresh]), "ready");

        let stale = image_summary(2, "stale");
        assert_eq!(material_state(&[image_summary(1, "fresh"), stale]), "stale");

        let unknown = image_summary(3, "unknown");
        assert_eq!(material_state(&[unknown]), "unknown");
    }

    #[test]
    fn favorite_filter_uses_all_profile_images_not_only_visible_limit() {
        let mut images_by_profile = BTreeMap::from([(
            (KIND_SCENE.to_string(), 1),
            vec![
                image_at(
                    1,
                    IMAGE_TYPE_SCENE,
                    json!({ "profile_kind": KIND_SCENE, "profile_id": 1 }),
                    1,
                    false,
                ),
                image_at(
                    2,
                    IMAGE_TYPE_SCENE,
                    json!({ "profile_kind": KIND_SCENE, "profile_id": 1 }),
                    2,
                    true,
                ),
            ],
        )]);
        let primary_by_profile = BTreeMap::from([((KIND_SCENE.to_string(), 1), 1)]);
        let group = material_group(
            material_source(1, "档案大厅"),
            &mut images_by_profile,
            &primary_by_profile,
            1,
        );
        assert_eq!(group.images.len(), 1);
        assert!(!group.images[0].image.is_favorite);

        let mut groups = vec![group];
        apply_group_filters(&mut groups, None, Some("favorite"));

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].profile.id, 1);
    }

    #[test]
    fn newest_sort_uses_all_profile_images_not_only_visible_limit() {
        let mut first_images = BTreeMap::from([(
            (KIND_SCENE.to_string(), 1),
            vec![
                image_at(
                    1,
                    IMAGE_TYPE_SCENE,
                    json!({ "profile_kind": KIND_SCENE, "profile_id": 1 }),
                    1,
                    false,
                ),
                image_at(
                    10,
                    IMAGE_TYPE_SCENE,
                    json!({ "profile_kind": KIND_SCENE, "profile_id": 1 }),
                    3,
                    false,
                ),
            ],
        )]);
        let first_primary = BTreeMap::from([((KIND_SCENE.to_string(), 1), 1)]);
        let first = material_group(
            material_source(1, "档案大厅"),
            &mut first_images,
            &first_primary,
            1,
        );
        assert_eq!(first.images[0].image.id, 1);

        let mut second_images = BTreeMap::from([(
            (KIND_SCENE.to_string(), 2),
            vec![image_at(
                5,
                IMAGE_TYPE_SCENE,
                json!({ "profile_kind": KIND_SCENE, "profile_id": 2 }),
                2,
                false,
            )],
        )]);
        let second = material_group(
            material_source(2, "旧书房"),
            &mut second_images,
            &BTreeMap::new(),
            1,
        );

        let mut groups = vec![second, first];
        apply_group_sort(&mut groups, Some("newest"));

        assert_eq!(groups[0].profile.id, 1);
        assert_eq!(groups[1].profile.id, 2);
    }

    #[test]
    fn decode_uploaded_profile_material_image_accepts_png_base64_and_data_url() {
        for data in [
            "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=",
            "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=",
        ] {
            let input = upload_input("image/png", data);
            let decoded = decode_uploaded_profile_material_image(&input).expect("valid png upload");

            assert_eq!(decoded.mime_type, "image/png");
            assert_eq!(decoded.extension, "png");
            assert_eq!((decoded.width, decoded.height), (1, 1));
        }
    }

    #[test]
    fn decode_uploaded_profile_material_image_rejects_unsupported_mime() {
        let input = upload_input(
            "image/gif",
            "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=",
        );

        let err = decode_uploaded_profile_material_image(&input)
            .expect_err("unsupported mime should fail");

        assert_eq!(err, "只支持 PNG/JPEG/WebP 图片");
    }

    #[test]
    fn decode_uploaded_profile_material_image_rejects_mime_format_mismatch() {
        let input = upload_input(
            "image/jpeg",
            "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=",
        );

        let err =
            decode_uploaded_profile_material_image(&input).expect_err("mime mismatch should fail");

        assert_eq!(err, "上传图片 MIME 与真实图片格式不一致");
    }

    #[test]
    fn append_cleanup_error_keeps_secondary_failure_visible() {
        assert_eq!(
            append_cleanup_error("主错误".to_string(), Some("文件清理失败".to_string())),
            "主错误；后续清理也失败：文件清理失败"
        );
        assert_eq!(append_cleanup_error("主错误".to_string(), None), "主错误");
        assert_eq!(
            append_cleanup_error("主错误".to_string(), Some("  ".to_string())),
            "主错误"
        );
    }

    fn material_source(id: i64, name: &str) -> ProfileMaterialSource {
        ProfileMaterialSource {
            kind: KIND_SCENE,
            profile: ProfileMaterialProfileSummary {
                id,
                project_id: 1,
                name: name.to_string(),
                category: "地点".to_string(),
                importance: "primary".to_string(),
                first_seen_chapter: 1,
                last_seen_chapter: 10,
                primary_text: "空间设定".to_string(),
                secondary_text: "氛围".to_string(),
                updated_at: None,
            },
            content_hash: "hash".to_string(),
        }
    }

    fn image(id: i64, image_type: &str, metadata_json: Value) -> generated_image::Model {
        image_at(id, image_type, metadata_json, 1, false)
    }

    fn image_at(
        id: i64,
        image_type: &str,
        metadata_json: Value,
        day: u32,
        is_favorite: bool,
    ) -> generated_image::Model {
        generated_image::Model {
            id,
            project_id: 1,
            image_type: image_type.to_string(),
            file_path: "images/test.png".to_string(),
            prompt: "prompt".to_string(),
            model: "model".to_string(),
            width: 1024,
            height: 768,
            file_size: None,
            ai_score: None,
            is_favorite,
            chapter_id: None,
            panel_id: None,
            metadata_json: Some(metadata_json),
            created_at: NaiveDate::from_ymd_opt(2026, 1, day)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
        }
    }

    fn image_summary(id: i64, staleness: &str) -> ProfileMaterialImageSummary {
        ProfileMaterialImageSummary {
            image: to_summary(&image(
                id,
                IMAGE_TYPE_SCENE,
                json!({ "profile_kind": KIND_SCENE, "profile_id": 1 }),
            )),
            is_primary: false,
            staleness: staleness.to_string(),
            style_template_id: None,
            style_template_name: None,
        }
    }

    fn upload_input(mime_type: &str, data_base64: &str) -> UploadProfileMaterialImageInput {
        UploadProfileMaterialImageInput {
            project_id: 1,
            profile_kind: KIND_SCENE.to_string(),
            profile_id: 1,
            file_name: "ref.png".to_string(),
            mime_type: mime_type.to_string(),
            data_base64: data_base64.to_string(),
            set_primary: true,
            prompt_snapshot: Some("prompt".to_string()),
            style_template_id: None,
            style_template_name: None,
            style_template_version: None,
            custom_style: None,
        }
    }
}
