pub use a4n_db::profile_material::{
    IMAGE_TYPE_OBJECT_MATERIAL, IMAGE_TYPE_SCENE_MATERIAL, PROFILE_KIND_OBJECT, PROFILE_KIND_SCENE,
    ProfileMaterialDbError, ProfileMaterialListRows, generated_image_profile_material_identity,
    is_project_reusable_video_material_image, is_video_frame_input_metadata,
    profile_material_identity,
};

use std::collections::{BTreeMap, BTreeSet};

use a4n_core::{
    error::AppError,
    models::{ai::generated_image, asset::object_profile, scene::scene_profile},
    source_spans::source_text_hash as source_content_sha256,
};
use a4n_db::video::CreateGeneratedImageRecordDraftInput;
use async_trait::async_trait;
use chrono::{NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    OrchestratorContext,
    collaboration::ProjectAccessRole,
    media::{decode_uploaded_image_data, generated_image_summary},
};

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
    newest_image_sort_key: Option<(NaiveDateTime, i64)>,
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
    pub image: crate::media::GeneratedImageSummary,
    pub is_primary: bool,
    pub staleness: String,
    pub style_template_id: Option<String>,
    pub style_template_name: Option<String>,
}

pub struct PersistProfileMaterialImageInput<'a> {
    pub project_id: i64,
    pub profile_kind: &'a str,
    pub profile_id: i64,
    pub extension: &'a str,
    pub bytes: &'a [u8],
}

pub struct PersistedProfileMaterialImage {
    pub relative_path: String,
    pub file_size: i64,
}

#[async_trait]
pub trait ProfileMaterialUploadHostPort: Send + Sync {
    fn persist_profile_material_image(
        &self,
        input: PersistProfileMaterialImageInput<'_>,
    ) -> Result<PersistedProfileMaterialImage, AppError>;

    async fn remove_uncommitted_profile_material_image(
        &self,
        relative_path: &str,
    ) -> Result<(), AppError>;
}

struct ProfileMaterialSource {
    kind: &'static str,
    profile: ProfileMaterialProfileSummary,
    content_hash: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ProfileMaterialStyleMetadata {
    pub style_template_id: Option<String>,
    pub style_template_name: Option<String>,
    pub style_template_version: Option<String>,
    pub custom_style: Option<String>,
}

pub fn profile_material_metadata(
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
    for (key, value) in [
        ("style_template_id", style.style_template_id.as_deref()),
        ("style_template_name", style.style_template_name.as_deref()),
        (
            "style_template_version",
            style.style_template_version.as_deref(),
        ),
        ("custom_style", style.custom_style.as_deref()),
    ] {
        if let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) {
            metadata.insert(key.to_string(), json!(value));
        }
    }
    Value::Object(metadata)
}

pub fn scene_profile_image_prompt(profile: &scene_profile::Model, style: Option<&str>) -> String {
    let style = style
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("清晰写实的场景概念设定图，空间明确，适合作为视频场景参考");
    let seed = if profile.image_prompt_seed.trim().is_empty() {
        "根据场景档案生成可复用的地点与空间设定图"
    } else {
        profile.image_prompt_seed.trim()
    };
    let entry_points = compact_json_hint(&profile.entry_points_json, "未记录明确出入口");
    let rules = compact_json_hint(&profile.rules_json, "未记录特殊规则");
    let current_state = compact_json_hint(&profile.current_state_json, "未记录当前状态变化");
    format!(
        "主体：{name}\n素材定位：可复用的场景/地点设定图，只展示空间环境本身，不表现具体剧情动作。\n参考画面：{seed}\n场景基础：{base}\n视觉特征：{visual}\n空间布局：{layout}\n氛围与光线：{atmosphere}\n出入口与连接：{entry_points}\n场景规则：{rules}\n当前状态：{current_state}\n构图与比例：以空间关系、可行动线、入口出口、主体区域和关键环境物件为重点；不要让人物成为主体；背景不能空泛，适合作为后续视频参考素材。\n画面风格：{style}\n风格执行：如果画面风格中包含“风格模板”或“风格关键词”，必须保留其中的中文风格名和关键英文风格名，并体现对应光影、色彩、构图、镜头、材质、氛围、细节和避免项，不要只概括成统一风格。\n避免项：不要文字、不要水印、不要边框、不要纯抽象氛围图、不要把场景画成单个物体特写。",
        name = profile.name,
        base = profile.base_description,
        visual = profile.visual_description,
        layout = profile.spatial_layout,
        atmosphere = profile.atmosphere
    )
}

pub fn object_profile_image_prompt(profile: &object_profile::Model, style: Option<&str>) -> String {
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
    let seed = if profile.image_prompt_seed.trim().is_empty() {
        "根据外观细节生成清晰的单物体素材图"
    } else {
        profile.image_prompt_seed.trim()
    };
    format!(
        "主体：{name}\n素材定位：单个物体的静物道具素材图，只展示物体外观，不表现使用过程。\n参考画面：{seed}\n外观细节：{visual}\n材质与当前状态：{material}\n环境语境：{location}\n构图与比例：以单个物体为画面中心，完整展示轮廓和关键细节，背景简洁，不出现人物持握、攻击、伤害或威胁行为，适合作为后续视频参考素材。\n画面风格：{style}\n风格执行：如果画面风格中包含“风格模板”或“风格关键词”，必须保留其中的中文风格名和关键英文风格名，并体现对应光影、色彩、构图、镜头、材质、氛围、细节和避免项，不要只概括成统一风格。\n避免项：不要文字、不要水印、不要边框、不要多余人物、不要把它画成其他物体。",
        name = profile.name,
        visual = profile.visual_description,
        material = profile.material_and_state
    )
}

pub fn scene_profile_content(profile: &scene_profile::Model) -> String {
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

pub fn object_profile_content(profile: &object_profile::Model) -> String {
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

impl OrchestratorContext {
    pub async fn list_profile_materials_validated(
        &self,
        user_id: i64,
        input: ListProfileMaterialsInput,
    ) -> Result<Vec<ProfileMaterialGroup>, AppError> {
        self.require_profile_material_editor(user_id, input.project_id)
            .await?;
        self.profile_material_groups(input).await
    }

    pub async fn set_profile_primary_material_validated(
        &self,
        user_id: i64,
        input: SetProfilePrimaryMaterialInput,
    ) -> Result<ProfileMaterialGroup, AppError> {
        self.require_profile_material_editor(user_id, input.project_id)
            .await?;
        let kind = normalize_profile_material_kind(&input.profile_kind)?;
        self.ensure_profile_material_profile(input.project_id, kind, input.profile_id)
            .await?;

        if let Some(image_id) = input.image_id {
            let image = self
                .generated_image_by_id(image_id)
                .await?
                .ok_or_else(|| AppError::NotFound("图片不存在".to_string()))?;
            if image.project_id != input.project_id {
                return Err(AppError::InvalidArgument(format!(
                    "图片 #{image_id} 不属于项目 {}",
                    input.project_id
                )));
            }
            if generated_image_profile_material_identity(&image) != Some((kind, input.profile_id)) {
                return Err(AppError::InvalidArgument(format!(
                    "图片 #{image_id} 不是 {kind} 档案 #{} 的素材图",
                    input.profile_id
                )));
            }
            self.upsert_profile_primary_material(
                input.project_id,
                kind,
                input.profile_id,
                image_id,
            )
            .await?;
        } else {
            self.delete_profile_primary_material(input.project_id, kind, input.profile_id)
                .await?;
        }

        self.profile_material_group(input.project_id, kind, input.profile_id)
            .await?
            .ok_or_else(|| AppError::InvalidState {
                current: "profile_material_updated".to_string(),
                operation: "主素材已更新，但无法重新读取档案素材".to_string(),
            })
    }

    pub async fn upload_profile_material_image_validated(
        &self,
        host: &dyn ProfileMaterialUploadHostPort,
        user_id: i64,
        input: UploadProfileMaterialImageInput,
    ) -> Result<ProfileMaterialGroup, AppError> {
        self.require_profile_material_editor(user_id, input.project_id)
            .await?;
        let kind = normalize_profile_material_kind(&input.profile_kind)?;
        let source = self
            .profile_material_source(input.project_id, kind, input.profile_id)
            .await?;
        let uploaded = decode_uploaded_image_data(
            &input.file_name,
            &input.mime_type,
            &input.data_base64,
            MAX_UPLOAD_IMAGE_BYTES,
        )
        .map_err(AppError::InvalidArgument)?;
        let prompt = input
            .prompt_snapshot
            .as_deref()
            .and_then(non_empty)
            .map(ToString::to_string)
            .unwrap_or_else(|| match kind {
                PROFILE_KIND_SCENE => source
                    .scene_profile
                    .as_ref()
                    .map(|profile| scene_profile_image_prompt(profile, None))
                    .unwrap_or_default(),
                _ => source
                    .object_profile
                    .as_ref()
                    .map(|profile| object_profile_image_prompt(profile, None))
                    .unwrap_or_default(),
            });
        let persisted = host.persist_profile_material_image(PersistProfileMaterialImageInput {
            project_id: input.project_id,
            profile_kind: kind,
            profile_id: input.profile_id,
            extension: uploaded.extension,
            bytes: &uploaded.bytes,
        })?;
        let style = ProfileMaterialStyleMetadata {
            style_template_id: input.style_template_id,
            style_template_name: input.style_template_name,
            style_template_version: input.style_template_version,
            custom_style: input.custom_style,
        };
        let mut metadata = profile_material_metadata(
            kind,
            source.summary.profile.id,
            &source.summary.profile.name,
            &source.summary.content_hash,
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
                    "file_size": persisted.file_size,
                }),
            );
        }

        let image = match self
            .create_generated_image_record(CreateGeneratedImageRecordDraftInput {
                project_id: input.project_id,
                image_type: image_type_for_profile_material_kind(kind).to_string(),
                file_path: persisted.relative_path.clone(),
                prompt,
                model: MANUAL_UPLOAD_MODEL.to_string(),
                width: uploaded.width,
                height: uploaded.height,
                file_size: Some(persisted.file_size),
                chapter_id: None,
                panel_id: Some(format!("{kind}_profile_{}", input.profile_id)),
                metadata_json: Some(metadata),
                created_at: Some(Utc::now().naive_utc()),
            })
            .await
        {
            Ok(image) => image,
            Err(error) => {
                let cleanup_error = host
                    .remove_uncommitted_profile_material_image(&persisted.relative_path)
                    .await
                    .err()
                    .map(|error| error.to_string());
                return Err(AppError::InvalidState {
                    current: "profile_material_upload".to_string(),
                    operation: append_cleanup_error(
                        format!("素材图记录创建失败：{error}"),
                        cleanup_error,
                    ),
                });
            }
        };

        if input.set_primary
            && let Err(error) = self
                .upsert_profile_primary_material(input.project_id, kind, input.profile_id, image.id)
                .await
        {
            let cleanup_error = self
                .delete_generated_image_record_with_cleanup(image.id)
                .await
                .err()
                .map(|error| format!("素材图记录回滚失败：{error}"));
            return Err(AppError::InvalidState {
                current: "profile_material_upload".to_string(),
                operation: append_cleanup_error(error.to_string(), cleanup_error),
            });
        }

        self.profile_material_group(input.project_id, kind, input.profile_id)
            .await?
            .ok_or_else(|| AppError::InvalidState {
                current: "profile_material_uploaded".to_string(),
                operation: "素材图已上传，但无法重新读取档案素材".to_string(),
            })
    }

    pub async fn count_profile_material_images(
        &self,
        project_id: i64,
        kind: &str,
        profile_id: i64,
    ) -> Result<i64, AppError> {
        let kind = normalize_profile_material_kind(kind)?;
        let count = self
            .profile_material_images_by_type(project_id, image_type_for_profile_material_kind(kind))
            .await?
            .into_iter()
            .filter(|image| {
                generated_image_profile_material_identity(image) == Some((kind, profile_id))
            })
            .count();
        i64::try_from(count).map_err(|_| AppError::InvalidState {
            current: "profile_material_image_count".to_string(),
            operation: "素材图数量超过 i64 上限".to_string(),
        })
    }

    pub async fn count_profile_material_images_by_profile(
        &self,
        project_id: i64,
        kind: &str,
        profile_ids: &[i64],
    ) -> Result<BTreeMap<i64, i64>, AppError> {
        let kind = normalize_profile_material_kind(kind)?;
        if profile_ids.is_empty() {
            return Ok(BTreeMap::new());
        }
        let requested = profile_ids.iter().copied().collect::<BTreeSet<_>>();
        let mut counts = BTreeMap::new();
        for image in self
            .profile_material_images_by_type(project_id, image_type_for_profile_material_kind(kind))
            .await?
        {
            if let Some((image_kind, profile_id)) =
                generated_image_profile_material_identity(&image)
                && image_kind == kind
                && requested.contains(&profile_id)
            {
                *counts.entry(profile_id).or_insert(0) += 1;
            }
        }
        Ok(counts)
    }

    pub async fn profile_material_image_ids(
        &self,
        project_id: i64,
        kind: &str,
        profile_id: i64,
    ) -> Result<Vec<i64>, AppError> {
        let kind = normalize_profile_material_kind(kind)?;
        Ok(self
            .profile_material_images_by_type(project_id, image_type_for_profile_material_kind(kind))
            .await?
            .into_iter()
            .filter(|image| {
                generated_image_profile_material_identity(image) == Some((kind, profile_id))
            })
            .map(|image| image.id)
            .collect())
    }

    async fn require_profile_material_editor(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<(), AppError> {
        if project_id <= 0 {
            return Err(AppError::InvalidArgument(
                "project_id 必须是正整数".to_string(),
            ));
        }
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await
            .map(|_| ())
    }

    async fn ensure_profile_material_profile(
        &self,
        project_id: i64,
        kind: &str,
        profile_id: i64,
    ) -> Result<(), AppError> {
        self.profile_material_source(project_id, kind, profile_id)
            .await
            .map(|_| ())
    }

    async fn profile_material_source(
        &self,
        project_id: i64,
        kind: &str,
        profile_id: i64,
    ) -> Result<ProfileMaterialUploadSource, AppError> {
        match kind {
            PROFILE_KIND_SCENE => {
                let profile = self.scene_profile_by_id(profile_id).await?;
                if profile.project_id != project_id {
                    return Err(AppError::InvalidArgument(format!(
                        "场景档案 #{profile_id} 不属于项目 {project_id}"
                    )));
                }
                Ok(ProfileMaterialUploadSource {
                    summary: scene_source_from_model(profile.clone()),
                    scene_profile: Some(profile),
                    object_profile: None,
                })
            }
            PROFILE_KIND_OBJECT => {
                let profile = self.object_profile_by_id(profile_id).await?;
                if profile.project_id != project_id {
                    return Err(AppError::InvalidArgument(format!(
                        "物体档案 #{profile_id} 不属于项目 {project_id}"
                    )));
                }
                Ok(ProfileMaterialUploadSource {
                    summary: object_source_from_model(profile.clone()),
                    scene_profile: None,
                    object_profile: Some(profile),
                })
            }
            _ => Err(AppError::InvalidArgument(format!(
                "不支持的素材档案类型: {kind}"
            ))),
        }
    }

    async fn profile_material_group(
        &self,
        project_id: i64,
        kind: &str,
        profile_id: i64,
    ) -> Result<Option<ProfileMaterialGroup>, AppError> {
        Ok(self
            .profile_material_groups(ListProfileMaterialsInput {
                project_id,
                kind: Some(kind.to_string()),
                query: None,
                filter: None,
                sort: None,
                image_limit_per_profile: Some(32),
            })
            .await?
            .into_iter()
            .find(|group| group.profile.id == profile_id))
    }

    async fn profile_material_groups(
        &self,
        input: ListProfileMaterialsInput,
    ) -> Result<Vec<ProfileMaterialGroup>, AppError> {
        let kinds = requested_profile_material_kinds(input.kind.as_deref())?;
        let image_limit = input.image_limit_per_profile.unwrap_or(8).clamp(1, 32);
        let mut sources = Vec::new();
        if kinds.contains(&PROFILE_KIND_SCENE) {
            sources.extend(
                self.scene_profiles_by_project(input.project_id)
                    .await?
                    .into_iter()
                    .map(scene_source_from_model),
            );
        }
        if kinds.contains(&PROFILE_KIND_OBJECT) {
            sources.extend(
                self.object_profiles_by_project(input.project_id)
                    .await?
                    .into_iter()
                    .map(object_source_from_model),
            );
        }

        let rows = self
            .profile_material_list_rows(
                input.project_id,
                &[IMAGE_TYPE_SCENE_MATERIAL, IMAGE_TYPE_OBJECT_MATERIAL],
            )
            .await?;
        let mut images_by_profile: BTreeMap<(String, i64), Vec<generated_image::Model>> =
            BTreeMap::new();
        for image in rows.images {
            if let Some((kind, profile_id)) = generated_image_profile_material_identity(&image) {
                images_by_profile
                    .entry((kind.to_string(), profile_id))
                    .or_default()
                    .push(image);
            }
        }
        let primary_by_profile = rows
            .primary_rows
            .into_iter()
            .map(|row| ((row.profile_kind, row.profile_id), row.primary_image_id))
            .collect::<BTreeMap<_, _>>();
        let mut groups = sources
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
}

struct ProfileMaterialUploadSource {
    summary: ProfileMaterialSource,
    scene_profile: Option<scene_profile::Model>,
    object_profile: Option<object_profile::Model>,
}

fn requested_profile_material_kinds(kind: Option<&str>) -> Result<Vec<&'static str>, AppError> {
    match kind.map(str::trim).filter(|value| !value.is_empty()) {
        Some(PROFILE_KIND_SCENE) => Ok(vec![PROFILE_KIND_SCENE]),
        Some(PROFILE_KIND_OBJECT) => Ok(vec![PROFILE_KIND_OBJECT]),
        Some("all") | None => Ok(vec![PROFILE_KIND_SCENE, PROFILE_KIND_OBJECT]),
        Some(other) => Err(AppError::InvalidArgument(format!(
            "不支持的素材档案类型: {other}"
        ))),
    }
}

fn normalize_profile_material_kind(kind: &str) -> Result<&'static str, AppError> {
    match kind.trim() {
        PROFILE_KIND_SCENE => Ok(PROFILE_KIND_SCENE),
        PROFILE_KIND_OBJECT => Ok(PROFILE_KIND_OBJECT),
        other => Err(AppError::InvalidArgument(format!(
            "不支持的档案素材类型: {other}"
        ))),
    }
}

fn image_type_for_profile_material_kind(kind: &str) -> &'static str {
    if kind == PROFILE_KIND_SCENE {
        IMAGE_TYPE_SCENE_MATERIAL
    } else {
        IMAGE_TYPE_OBJECT_MATERIAL
    }
}

fn scene_source_from_model(profile: scene_profile::Model) -> ProfileMaterialSource {
    let content_hash = source_content_sha256(&scene_profile_content(&profile));
    ProfileMaterialSource {
        kind: PROFILE_KIND_SCENE,
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
        kind: PROFILE_KIND_OBJECT,
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
        image: generated_image_summary(image),
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
            material_state_rank(&a.material_state)
                .cmp(&material_state_rank(&b.material_state))
                .then_with(|| {
                    a.profile
                        .first_seen_chapter
                        .cmp(&b.profile.first_seen_chapter)
                })
        }),
        Some("newest") => {
            groups.sort_by_key(|group| std::cmp::Reverse(group.newest_image_sort_key))
        }
        Some("importance") => groups.sort_by(|a, b| {
            material_importance_rank(&a.profile.importance)
                .cmp(&material_importance_rank(&b.profile.importance))
                .then_with(|| {
                    a.profile
                        .first_seen_chapter
                        .cmp(&b.profile.first_seen_chapter)
                })
        }),
        _ => groups.sort_by(|a, b| {
            material_state_rank(&a.material_state)
                .cmp(&material_state_rank(&b.material_state))
                .then_with(|| {
                    a.profile
                        .first_seen_chapter
                        .cmp(&b.profile.first_seen_chapter)
                })
                .then_with(|| a.profile.id.cmp(&b.profile.id))
        }),
    }
}

fn material_state_rank(state: &str) -> i32 {
    match state {
        "missing" => 0,
        "stale" => 1,
        "unknown" => 2,
        _ => 3,
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
    if value.is_empty() { None } else { Some(value) }
}

fn append_cleanup_error(primary: String, cleanup_error: Option<String>) -> String {
    match cleanup_error {
        Some(cleanup_error) if !cleanup_error.trim().is_empty() => {
            format!("{primary}；后续清理也失败：{cleanup_error}")
        }
        _ => primary,
    }
}

fn compact_json_hint(value: &Value, empty_text: &str) -> String {
    match value {
        Value::Null => empty_text.to_string(),
        Value::Array(values) if values.is_empty() => empty_text.to_string(),
        Value::Object(values) if values.is_empty() => empty_text.to_string(),
        _ => serde_json::to_string(value).expect("serde_json::Value serialization must succeed"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn profile_material_metadata_uses_canonical_fields() {
        let metadata = profile_material_metadata(
            PROFILE_KIND_SCENE,
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
    fn material_group_keeps_hidden_favorite_and_primary_facts() {
        let source = ProfileMaterialSource {
            kind: PROFILE_KIND_SCENE,
            profile: ProfileMaterialProfileSummary {
                id: 1,
                project_id: 1,
                name: "档案大厅".to_string(),
                category: "地点".to_string(),
                importance: "primary".to_string(),
                first_seen_chapter: 1,
                last_seen_chapter: 1,
                primary_text: String::new(),
                secondary_text: String::new(),
                updated_at: None,
            },
            content_hash: "hash".to_string(),
        };
        let image = |id, day, favorite| generated_image::Model {
            id,
            project_id: 1,
            image_type: IMAGE_TYPE_SCENE_MATERIAL.to_string(),
            file_path: format!("images/{id}.png"),
            prompt: "prompt".to_string(),
            model: "model".to_string(),
            width: 1,
            height: 1,
            file_size: None,
            ai_score: None,
            is_favorite: favorite,
            chapter_id: None,
            panel_id: None,
            metadata_json: Some(json!({
                "profile_kind": PROFILE_KIND_SCENE,
                "profile_id": 1,
                "profile_content_sha256": "hash"
            })),
            created_at: NaiveDate::from_ymd_opt(2026, 1, day)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
        };
        let mut images = BTreeMap::from([(
            (PROFILE_KIND_SCENE.to_string(), 1),
            vec![image(1, 1, false), image(2, 2, true)],
        )]);
        let primary = BTreeMap::from([((PROFILE_KIND_SCENE.to_string(), 1), 1)]);

        let group = material_group(source, &mut images, &primary, 1);

        assert_eq!(group.image_count, 2);
        assert_eq!(group.images[0].image.id, 1);
        assert!(group.has_favorite_image);
    }
}
