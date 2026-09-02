use a4n_api_contract::profile::{
    ListProfileMaterialsInput, ProfileMaterialGroup, ProfileMaterialImageSummary,
    ProfileMaterialProfileSummary, SetProfilePrimaryMaterialInput, UploadProfileMaterialImageInput,
};
pub(crate) use a4n_db::profile_material::{
    generated_image_profile_material_identity, is_project_reusable_video_material_image,
    profile_material_identity,
};

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::Write;
use std::path::Path;

use a4n_core::{
    error::{AppError, append_secondary_message},
    files::{
        create_unique_timestamped_file, paths::ProjectPaths, persist_created_media_file,
        remove_existing_data_file,
    },
    models::{
        ai::generated_image,
        asset::{object_profile, profile_primary_material},
        scene::scene_profile,
    },
    source_spans::source_text_hash as source_content_sha256,
};
use a4n_db::video::CreateGeneratedImageRecordDraftInput;
use a4n_domain::{
    image::{IMAGE_TYPE_OBJECT_MATERIAL, IMAGE_TYPE_SCENE_MATERIAL},
    profile_material::{
        ProfileMaterialFilter, ProfileMaterialImageStaleness, ProfileMaterialKind,
        ProfileMaterialKindFilter, ProfileMaterialSort, ProfileMaterialState,
    },
};
use chrono::{NaiveDateTime, Utc};
use serde_json::{Value, json};

use crate::{
    OrchestratorContext,
    asset::asset_error,
    collaboration::ProjectAccessRole,
    media::{decode_uploaded_image_data, generated_image_summary},
    scene_profile::scene_profile_db_error,
};

const MANUAL_UPLOAD_MODEL: &str = "manual_upload";
const MAX_UPLOAD_IMAGE_BYTES: usize = 20 * 1024 * 1024;

struct PersistProfileMaterialImageInput<'a> {
    project_id: i64,
    profile_kind: &'a str,
    profile_id: i64,
    extension: &'a str,
    bytes: &'a [u8],
}

struct PersistedProfileMaterialImage {
    relative_path: String,
    file_size: i64,
}

struct ProfileMaterialSource {
    kind: ProfileMaterialKind,
    profile: ProfileMaterialProfileSummary,
    content_hash: String,
}

struct ProfileMaterialGroupProjection {
    group: ProfileMaterialGroup,
    newest_image_sort_key: Option<(NaiveDateTime, i64)>,
    has_favorite_image: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct ProfileMaterialStyleMetadata {
    pub(crate) style_template_id: Option<String>,
    pub(crate) style_template_name: Option<String>,
    pub(crate) style_template_version: Option<String>,
    pub(crate) custom_style: Option<String>,
}

pub(crate) fn profile_material_metadata(
    kind: ProfileMaterialKind,
    profile_id: i64,
    profile_name: &str,
    profile_content_sha256: &str,
    style: &ProfileMaterialStyleMetadata,
) -> Value {
    let mut metadata = serde_json::Map::new();
    metadata.insert("material_kind".to_string(), json!("profile_material"));
    metadata.insert("profile_kind".to_string(), json!(kind.as_str()));
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

pub(crate) fn scene_profile_image_prompt(
    profile: &scene_profile::Model,
    style: Option<&str>,
) -> String {
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
        let kind = input.profile_kind;
        if let Some(image_id) = input.image_id {
            let profile_check =
                self.ensure_profile_material_profile(input.project_id, kind, input.profile_id);
            let image_result = async {
                self.generated_image_by_id(image_id)
                    .await?
                    .ok_or_else(|| AppError::NotFound("图片不存在".to_string()))
            };
            let (profile_result, image_result) = futures::join!(profile_check, image_result);
            profile_result?;
            let image = image_result?;
            if image.project_id != input.project_id {
                return Err(AppError::InvalidArgument(format!(
                    "图片 #{image_id} 不属于项目 {}",
                    input.project_id
                )));
            }
            if generated_image_profile_material_identity(&image)
                != Some((kind.as_str(), input.profile_id))
            {
                return Err(AppError::InvalidArgument(format!(
                    "图片 #{image_id} 不是 {} 档案 #{} 的素材图",
                    kind.as_str(),
                    input.profile_id
                )));
            }
            self.upsert_profile_primary_material(
                input.project_id,
                kind.as_str(),
                input.profile_id,
                image_id,
            )
            .await?;
        } else {
            self.ensure_profile_material_profile(input.project_id, kind, input.profile_id)
                .await?;
            self.delete_profile_primary_material(input.project_id, kind.as_str(), input.profile_id)
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
        user_id: i64,
        input: UploadProfileMaterialImageInput,
    ) -> Result<ProfileMaterialGroup, AppError> {
        self.require_profile_material_editor(user_id, input.project_id)
            .await?;
        let kind = input.profile_kind;
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
        let prompt = non_empty(&input.prompt_snapshot)
            .ok_or_else(|| AppError::InvalidArgument("prompt_snapshot 不能为空".to_string()))?
            .to_string();
        let persisted = persist_profile_material_image(
            self.data_dir(),
            PersistProfileMaterialImageInput {
                project_id: input.project_id,
                profile_kind: kind.as_str(),
                profile_id: input.profile_id,
                extension: uploaded.extension,
                bytes: &uploaded.bytes,
            },
        )?;
        let style = ProfileMaterialStyleMetadata {
            style_template_id: input.style_template_id,
            style_template_name: input.style_template_name,
            style_template_version: input.style_template_version,
            custom_style: input.custom_style,
        };
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
                panel_id: Some(format!("{}_profile_{}", kind.as_str(), input.profile_id)),
                metadata_json: Some(metadata),
                created_at: Some(Utc::now().naive_utc()),
            })
            .await
        {
            Ok(image) => image,
            Err(error) => {
                let cleanup_error = remove_existing_data_file(
                    self.data_dir(),
                    &persisted.relative_path,
                    "未提交素材图",
                )
                .err();
                return Err(AppError::InvalidState {
                    current: "profile_material_upload".to_string(),
                    operation: append_secondary_message(
                        format!("素材图记录创建失败：{error}"),
                        cleanup_error.map(|cleanup| format!("后续清理也失败：{cleanup}")),
                    ),
                });
            }
        };

        if input.set_primary
            && let Err(error) = self
                .upsert_profile_primary_material(
                    input.project_id,
                    kind.as_str(),
                    input.profile_id,
                    image.id,
                )
                .await
        {
            let cleanup_error = self
                .delete_generated_image_record_with_cleanup(image.id)
                .await
                .err()
                .map(|error| format!("素材图记录回滚失败：{error}"));
            return Err(AppError::InvalidState {
                current: "profile_material_upload".to_string(),
                operation: append_secondary_message(
                    error.to_string(),
                    cleanup_error.map(|cleanup| format!("后续清理也失败：{cleanup}")),
                ),
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
        kind: ProfileMaterialKind,
        profile_id: i64,
    ) -> Result<i64, AppError> {
        let count = self
            .profile_material_image_identity_rows_by_type(
                project_id,
                image_type_for_profile_material_kind(kind),
            )
            .await?
            .into_iter()
            .filter(|image| {
                profile_material_identity(&image.image_type, image.metadata_json.as_ref())
                    == Some((kind.as_str(), profile_id))
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
        kind: ProfileMaterialKind,
        profile_ids: &[i64],
    ) -> Result<BTreeMap<i64, i64>, AppError> {
        if profile_ids.is_empty() {
            return Ok(BTreeMap::new());
        }
        let requested = profile_ids.iter().copied().collect::<HashSet<_>>();
        let mut counts = HashMap::new();
        for image in self
            .profile_material_image_identity_rows_by_type(
                project_id,
                image_type_for_profile_material_kind(kind),
            )
            .await?
        {
            if let Some((image_kind, profile_id)) =
                profile_material_identity(&image.image_type, image.metadata_json.as_ref())
                && image_kind == kind.as_str()
                && requested.contains(&profile_id)
            {
                *counts.entry(profile_id).or_insert(0) += 1;
            }
        }
        Ok(counts.into_iter().collect())
    }

    pub async fn profile_material_image_ids(
        &self,
        project_id: i64,
        kind: ProfileMaterialKind,
        profile_id: i64,
    ) -> Result<Vec<i64>, AppError> {
        Ok(self
            .profile_material_image_identity_rows_by_type(
                project_id,
                image_type_for_profile_material_kind(kind),
            )
            .await?
            .into_iter()
            .filter(|image| {
                profile_material_identity(&image.image_type, image.metadata_json.as_ref())
                    == Some((kind.as_str(), profile_id))
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
        kind: ProfileMaterialKind,
        profile_id: i64,
    ) -> Result<(), AppError> {
        let profile_project_id = match kind {
            ProfileMaterialKind::Scene => a4n_db::scene_profile::scene_profile_project_id(
                self.database.connection(),
                profile_id,
            )
            .await
            .map_err(scene_profile_db_error)?,
            ProfileMaterialKind::Object => {
                a4n_db::asset::object_profile_project_id(self.database.connection(), profile_id)
                    .await
                    .map_err(asset_error)?
            }
        };
        if profile_project_id != project_id {
            let label = match kind {
                ProfileMaterialKind::Scene => "场景档案",
                ProfileMaterialKind::Object => "物体档案",
            };
            return Err(AppError::InvalidArgument(format!(
                "{label} #{profile_id} 不属于项目 {project_id}"
            )));
        }
        Ok(())
    }

    async fn profile_material_source(
        &self,
        project_id: i64,
        kind: ProfileMaterialKind,
        profile_id: i64,
    ) -> Result<ProfileMaterialSource, AppError> {
        match kind {
            ProfileMaterialKind::Scene => {
                let profile = self.scene_profile_by_id(profile_id).await?;
                if profile.project_id != project_id {
                    return Err(AppError::InvalidArgument(format!(
                        "场景档案 #{profile_id} 不属于项目 {project_id}"
                    )));
                }
                Ok(scene_source_from_model(profile))
            }
            ProfileMaterialKind::Object => {
                let profile = self.object_profile_by_id(profile_id).await?;
                if profile.project_id != project_id {
                    return Err(AppError::InvalidArgument(format!(
                        "物体档案 #{profile_id} 不属于项目 {project_id}"
                    )));
                }
                Ok(object_source_from_model(profile))
            }
        }
    }

    async fn profile_material_group(
        &self,
        project_id: i64,
        kind: ProfileMaterialKind,
        profile_id: i64,
    ) -> Result<Option<ProfileMaterialGroup>, AppError> {
        let image_types = [image_type_for_profile_material_kind(kind)];
        let (source, rows) = futures::join!(
            self.profile_material_source(project_id, kind, profile_id),
            self.profile_material_list_rows(project_id, &image_types),
        );
        let source = source?;
        let rows = rows?;
        Ok(
            profile_material_group_projections(vec![source], rows.images, rows.primary_rows, 32)
                .into_iter()
                .next()
                .map(|projection| projection.group),
        )
    }

    async fn profile_material_groups(
        &self,
        input: ListProfileMaterialsInput,
    ) -> Result<Vec<ProfileMaterialGroup>, AppError> {
        let kinds = requested_profile_material_kinds(input.kind);
        let image_limit = input.image_limit_per_profile;
        let image_types = kinds
            .iter()
            .map(|kind| image_type_for_profile_material_kind(*kind))
            .collect::<Vec<_>>();
        let include_scene = kinds.contains(&ProfileMaterialKind::Scene);
        let include_object = kinds.contains(&ProfileMaterialKind::Object);
        let (scene_profiles, object_profiles, rows) = futures::join!(
            async {
                if include_scene {
                    self.scene_profiles_by_project(input.project_id).await
                } else {
                    Ok(Vec::new())
                }
            },
            async {
                if include_object {
                    self.object_profiles_by_project(input.project_id).await
                } else {
                    Ok(Vec::new())
                }
            },
            self.profile_material_list_rows(input.project_id, &image_types),
        );
        let scene_profiles = scene_profiles?;
        let object_profiles = object_profiles?;
        let rows = rows?;
        let mut sources = scene_profiles
            .into_iter()
            .map(scene_source_from_model)
            .collect::<Vec<_>>();
        sources.extend(object_profiles.into_iter().map(object_source_from_model));
        let mut groups = profile_material_group_projections(
            sources,
            rows.images,
            rows.primary_rows,
            image_limit,
        );
        apply_group_filters(&mut groups, input.query.as_deref(), input.filter);
        apply_group_sort(&mut groups, input.sort);
        Ok(groups
            .into_iter()
            .map(|projection| projection.group)
            .collect())
    }
}

fn requested_profile_material_kinds(kind: ProfileMaterialKindFilter) -> Vec<ProfileMaterialKind> {
    match kind {
        ProfileMaterialKindFilter::Scene => vec![ProfileMaterialKind::Scene],
        ProfileMaterialKindFilter::Object => vec![ProfileMaterialKind::Object],
        ProfileMaterialKindFilter::All => {
            vec![ProfileMaterialKind::Scene, ProfileMaterialKind::Object]
        }
    }
}

fn image_type_for_profile_material_kind(kind: ProfileMaterialKind) -> &'static str {
    match kind {
        ProfileMaterialKind::Scene => IMAGE_TYPE_SCENE_MATERIAL,
        ProfileMaterialKind::Object => IMAGE_TYPE_OBJECT_MATERIAL,
    }
}

fn scene_source_from_model(profile: scene_profile::Model) -> ProfileMaterialSource {
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

fn object_source_from_model(profile: object_profile::Model) -> ProfileMaterialSource {
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

fn profile_material_group_projections(
    sources: Vec<ProfileMaterialSource>,
    images: Vec<generated_image::Model>,
    primary_rows: Vec<profile_primary_material::Model>,
    image_limit: usize,
) -> Vec<ProfileMaterialGroupProjection> {
    let mut images_by_profile: HashMap<(String, i64), Vec<generated_image::Model>> = HashMap::new();
    for image in images {
        if let Some((kind, profile_id)) = generated_image_profile_material_identity(&image) {
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
        ProfileMaterialFilter::Missing => {
            groups.retain(|projection| {
                projection.group.material_state == ProfileMaterialState::Missing
            });
        }
        ProfileMaterialFilter::Ready => {
            groups.retain(|projection| {
                projection.group.material_state == ProfileMaterialState::Ready
            });
        }
        ProfileMaterialFilter::Stale => {
            groups.retain(|projection| {
                projection.group.material_state == ProfileMaterialState::Stale
            });
        }
        ProfileMaterialFilter::Unknown => {
            groups.retain(|projection| {
                projection.group.material_state == ProfileMaterialState::Unknown
            });
        }
        ProfileMaterialFilter::Favorite => {
            groups.retain(|projection| projection.has_favorite_image);
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
    if value.is_empty() { None } else { Some(value) }
}

fn persist_profile_material_image(
    data_dir: &Path,
    input: PersistProfileMaterialImageInput<'_>,
) -> Result<PersistedProfileMaterialImage, AppError> {
    let dir = ProjectPaths::new(data_dir, input.project_id)
        .images_dir()
        .join("profile_materials");
    std::fs::create_dir_all(&dir).map_err(|err| AppError::InvalidState {
        current: "profile_material_upload".to_string(),
        operation: format!("无法创建素材图目录：{err}"),
    })?;
    let (path, mut output_file) = create_unique_timestamped_file(
        &dir,
        &format!("{}_{}", input.profile_kind, input.profile_id),
        input.extension,
    )
    .map_err(|error| AppError::InvalidState {
        current: "profile_material_upload".to_string(),
        operation: format!("创建上传素材图失败：{error}"),
    })?;
    let (relative_path, file_size) = persist_created_media_file(
        data_dir,
        &path,
        move |_| {
            output_file
                .write_all(input.bytes)
                .map_err(|err| format!("写入上传素材图失败：{err}"))
        },
        "上传素材图",
        "上传素材图",
        "上传素材图",
    )
    .map_err(|operation| AppError::InvalidState {
        current: "profile_material_upload".to_string(),
        operation,
    })?;
    Ok(PersistedProfileMaterialImage {
        relative_path,
        file_size,
    })
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
    use a4n_db::profile_material::PROFILE_KIND_SCENE;
    use chrono::NaiveDate;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn persists_profile_material_under_project_directory() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock after epoch")
            .as_nanos();
        let data_dir = std::env::temp_dir().join(format!(
            "a4n_profile_material_{}_{}",
            std::process::id(),
            stamp
        ));
        let persisted = persist_profile_material_image(
            &data_dir,
            PersistProfileMaterialImageInput {
                project_id: 7,
                profile_kind: "scene",
                profile_id: 42,
                extension: "png",
                bytes: b"profile-material",
            },
        )
        .expect("persist profile material");
        assert!(
            persisted
                .relative_path
                .starts_with("projects/7/images/profile_materials/scene_42_")
        );
        assert_eq!(persisted.file_size, 16);
        let absolute = data_dir.join(&persisted.relative_path);
        assert!(absolute.is_file());
        let _ = std::fs::remove_dir_all(data_dir);
    }

    #[test]
    fn profile_material_metadata_uses_canonical_fields() {
        let metadata = profile_material_metadata(
            ProfileMaterialKind::Scene,
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
            kind: ProfileMaterialKind::Scene,
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
        let mut images = HashMap::from([(
            (PROFILE_KIND_SCENE.to_string(), 1),
            vec![image(1, 1, false), image(2, 2, true)],
        )]);
        let primary = HashMap::from([((PROFILE_KIND_SCENE.to_string(), 1), 1)]);

        let projection = material_group(source, &mut images, &primary, 1);

        assert_eq!(projection.group.image_count, 2);
        assert_eq!(projection.group.images[0].image.id, 1);
        assert!(projection.has_favorite_image);
    }

    #[test]
    fn stale_first_sort_differs_from_missing_first() {
        let projection = |id, material_state| ProfileMaterialGroupProjection {
            group: ProfileMaterialGroup {
                kind: ProfileMaterialKind::Scene,
                profile: ProfileMaterialProfileSummary {
                    id,
                    project_id: 1,
                    name: format!("档案 {id}"),
                    category: String::new(),
                    importance: String::new(),
                    first_seen_chapter: 1,
                    last_seen_chapter: 1,
                    primary_text: String::new(),
                    secondary_text: String::new(),
                    updated_at: None,
                },
                material_state,
                current_profile_content_sha256: String::new(),
                image_count: 0,
                primary_image_id: None,
                primary_image: None,
                images: Vec::new(),
            },
            newest_image_sort_key: None,
            has_favorite_image: false,
        };
        let mut groups = vec![
            projection(1, ProfileMaterialState::Missing),
            projection(2, ProfileMaterialState::Stale),
        ];

        apply_group_sort(&mut groups, ProfileMaterialSort::StaleFirst);

        assert_eq!(groups[0].group.profile.id, 2);
    }
}
