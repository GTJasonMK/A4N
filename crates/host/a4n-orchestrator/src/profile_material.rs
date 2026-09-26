use a4n_api_contract::profile::{ProfileMaterialGroup, UploadProfileMaterialImageInput};
use a4n_db::profile_material::{
    generated_image_profile_material_identity, profile_material_identity_from_fields,
};
use a4n_db_models::models::{
    asset::object_profile, media::image_publication, scene::scene_profile,
};
use a4n_domain::{
    image::{IMAGE_TYPE_OBJECT_MATERIAL, IMAGE_TYPE_SCENE_MATERIAL},
    media::{
        CreateGeneratedImageRecordDraftInput, CreateImagePublicationIntentInput,
        IMAGE_PUBLICATION_STATUS_PENDING, IMAGE_PUBLICATION_STATUS_PUBLISHED,
        ImagePublicationFileFacts,
    },
    profile_material::ProfileMaterialKind,
};
use a4n_error::AppError;
use a4n_image_host_contract::{
    ImageGenerationHostPort, PersistGeneratedImageFileInput, PrepareGeneratedImageFileInput,
    ProfileMaterialUploadApplicationPort,
};
use a4n_media::decode_uploaded_image_data;
use async_trait::async_trait;
use chrono::Utc;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{OrchestratorContext, asset::asset_error, scene_profile::scene_profile_db_error};
use a4n_domain::collaboration::ProjectAccessRole;
use a4n_profile_material_core::{
    ProfileMaterialSource, image_type_for_profile_material_kind, object_source_from_model,
    profile_material_group_projections, scene_source_from_model,
};
const MANUAL_UPLOAD_MODEL: &str = "manual_upload";
const MAX_UPLOAD_IMAGE_BYTES: usize = 20 * 1024 * 1024;

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
    let current_state = scene_visual_state_hint(&profile.current_state_json);
    format!(
        "主体：{name}\n素材定位：可复用的场景/地点设定图，只展示空间环境本身，不表现具体剧情动作。\n参考画面：{seed}\n场景基础：{base}\n视觉特征：{visual}\n空间布局：{layout}\n氛围与光线：{atmosphere}\n出入口与连接：{entry_points}\n场景规则：{rules}\n当前状态：{current_state}\n构图与比例：以空间关系、可行动线、入口出口、主体区域和关键环境物件为重点；不要让人物成为主体；背景不能空泛，适合作为后续视频参考素材。\n画面风格：{style}\n风格执行：如果画面风格中包含“风格模板”或“风格关键词”，必须保留其中的中文风格名和关键英文风格名，并体现对应光影、色彩、构图、镜头、材质、氛围、细节和避免项，不要只概括成统一风格。\n避免项：不要文字、不要水印、不要边框、不要纯抽象氛围图、不要把场景画成单个物体特写。",
        name = profile.name,
        base = profile.base_description,
        visual = profile.visual_description,
        layout = profile.spatial_layout,
        atmosphere = profile.atmosphere
    )
}

fn scene_visual_state_hint(state: &Value) -> String {
    ["summary", "weather_or_lighting", "mood"]
        .into_iter()
        .filter_map(|key| state.get(key).and_then(Value::as_str))
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("；")
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

pub(crate) use a4n_profile_material_core::{object_profile_content, scene_profile_content};

impl OrchestratorContext {
    pub(crate) async fn upload_profile_material_image_validated(
        &self,
        user_id: i64,
        host: &dyn ImageGenerationHostPort,
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
            map.insert("set_primary".to_string(), json!(input.set_primary));
            map.insert("prompt_snapshot".to_string(), json!(prompt));
            map.insert(
                "upload".to_string(),
                json!({
                    "original_file_name": uploaded.source_file_name,
                    "mime_type": uploaded.mime_type,
                    "file_size": uploaded.bytes.len(),
                }),
            );
        }

        let file_size = i64::try_from(uploaded.bytes.len())
            .map_err(|_| AppError::InvalidArgument("素材图过大，无法记录文件大小".to_string()))?;
        let expected_sha256 = format!("{:x}", Sha256::digest(&uploaded.bytes));
        let publication_id = format!(
            "image-publication-profile-material-{}",
            Uuid::new_v4().simple()
        );
        let file_extra = format!("profile_{}", input.profile_id);
        let prepared = host.prepare_generated_image_file(PrepareGeneratedImageFileInput {
            project_id: input.project_id,
            chapter_id: None,
            image_type: image_type_for_profile_material_kind(kind),
            file_extra: Some(&file_extra),
            format: uploaded.extension,
            publication_id: &publication_id,
            file_context: "档案素材图",
        })?;
        let (publication, _publication_guard) = self
            .create_image_publication_intent_for_user_validated(
                user_id,
                CreateImagePublicationIntentInput {
                    publication_id: publication_id.clone(),
                    project_id: input.project_id,
                    requested_by_user_id: user_id,
                    staging_path: prepared.staging_path.clone(),
                    final_path: prepared.final_path.clone(),
                    image: CreateGeneratedImageRecordDraftInput {
                        project_id: input.project_id,
                        image_type: image_type_for_profile_material_kind(kind).to_string(),
                        file_path: prepared.final_path.clone(),
                        prompt,
                        model: MANUAL_UPLOAD_MODEL.to_string(),
                        width: uploaded.width,
                        height: uploaded.height,
                        file_size: Some(file_size),
                        chapter_id: None,
                        panel_id: Some(file_extra.clone()),
                        metadata_json: Some(metadata),
                        created_at: Some(Utc::now().naive_utc()),
                    },
                    expected_file_size: file_size,
                    expected_sha256: expected_sha256.clone(),
                    allow_pending_replacement: false,
                },
            )
            .await?;
        let image = if publication.status == IMAGE_PUBLICATION_STATUS_PUBLISHED {
            let image_id = publication.image_id.ok_or_else(|| AppError::InvalidState {
                current: publication.status.clone(),
                operation: "读取已发布档案素材图".to_string(),
            })?;
            self.generated_image_by_id(image_id)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("已发布档案素材图不存在: {image_id}")))?
        } else if publication.status == IMAGE_PUBLICATION_STATUS_PENDING {
            let persisted = match host.persist_generated_image_file(
                PersistGeneratedImageFileInput {
                    project_id: input.project_id,
                    chapter_id: None,
                    image_type: image_type_for_profile_material_kind(kind),
                    file_extra: Some(&file_extra),
                    bytes: &uploaded.bytes,
                    format: uploaded.extension,
                    file_context: "档案素材图",
                },
                &prepared,
            ) {
                Ok(persisted) => persisted,
                Err(error) => {
                    return Err(cleanup_profile_staging_after_error(
                        host,
                        &prepared.staging_path,
                        error,
                    )
                    .await);
                }
            };
            let image = match self
                .publish_image_publication_for_user_validated(
                    user_id,
                    input.project_id,
                    &publication_id,
                    ImagePublicationFileFacts {
                        file_size: persisted.file_size,
                        sha256: expected_sha256,
                    },
                    None,
                )
                .await
            {
                Ok(image) => image,
                Err(error) => {
                    return Err(cleanup_profile_staging_after_error(
                        host,
                        &prepared.staging_path,
                        error,
                    )
                    .await);
                }
            };
            if let Err(error) = host
                .remove_generated_image_file(&prepared.staging_path, "档案素材图暂存文件")
                .await
            {
                tracing::warn!(
                    error = %error,
                    publication_id = %publication_id,
                    "档案素材图发布成功但暂存文件清理意图登记失败"
                );
            }
            image
        } else {
            return Err(AppError::InvalidState {
                current: publication.status,
                operation: "写入档案素材图".to_string(),
            });
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
            return Err(AppError::InvalidState {
                current: "profile_material_upload".to_string(),
                operation: format!("设置主素材失败，已发布素材图已保留：{error}"),
            });
        }

        self.profile_material_group(input.project_id, kind, input.profile_id)
            .await?
            .ok_or_else(|| AppError::InvalidState {
                current: "profile_material_uploaded".to_string(),
                operation: "素材图已上传，但无法重新读取档案素材".to_string(),
            })
    }

    pub(crate) async fn ensure_profile_material_publication_identity_for_user_validated(
        &self,
        user_id: i64,
        publication: &image_publication::Model,
    ) -> Result<(), AppError> {
        self.verify_project_permission(publication.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        if !matches!(
            publication.image_type.as_str(),
            IMAGE_TYPE_SCENE_MATERIAL | IMAGE_TYPE_OBJECT_MATERIAL
        ) {
            return Err(AppError::InvalidState {
                current: publication.image_type.clone(),
                operation: "恢复档案素材 publication".to_string(),
            });
        }
        if !matches!(
            publication.status.as_str(),
            IMAGE_PUBLICATION_STATUS_PENDING | IMAGE_PUBLICATION_STATUS_PUBLISHED
        ) {
            return Err(AppError::InvalidState {
                current: publication.status.clone(),
                operation: "恢复档案素材 publication".to_string(),
            });
        }
        let (kind, profile_id, set_primary) = profile_material_publication_details(
            &publication.image_type,
            publication.metadata_json.as_ref(),
        )?;
        self.ensure_profile_material_profile(publication.project_id, kind, profile_id)
            .await?;
        if publication.status == IMAGE_PUBLICATION_STATUS_PENDING {
            if publication.image_id.is_some() {
                return Err(AppError::InvalidState {
                    current: publication.publication_id.clone(),
                    operation: "待发布档案素材图片不应已有 image_id".to_string(),
                });
            }
            return Ok(());
        }
        let image_id = publication.image_id.ok_or_else(|| AppError::InvalidState {
            current: publication.publication_id.clone(),
            operation: "读取已发布档案素材图片身份".to_string(),
        })?;
        let image = self
            .generated_image_by_id(image_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("已发布档案素材图不存在: {image_id}")))?;
        if image.project_id != publication.project_id
            || image.file_path != publication.final_path
            || generated_image_profile_material_identity(&image)
                != Some((kind.as_str(), profile_id))
        {
            return Err(AppError::InvalidState {
                current: format!("publication_id={}", publication.publication_id),
                operation: "档案素材 publication 身份与图片记录不一致".to_string(),
            });
        }
        if set_primary {
            self.upsert_profile_primary_material(
                publication.project_id,
                kind.as_str(),
                profile_id,
                image_id,
            )
            .await?;
        }
        Ok(())
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
}

#[async_trait]
impl ProfileMaterialUploadApplicationPort for OrchestratorContext {
    async fn upload(
        &self,
        user_id: i64,
        host: &dyn ImageGenerationHostPort,
        input: UploadProfileMaterialImageInput,
    ) -> Result<ProfileMaterialGroup, AppError> {
        OrchestratorContext::upload_profile_material_image_validated(self, user_id, host, input)
            .await
    }
}

fn profile_material_publication_details(
    image_type: &str,
    metadata: Option<&Value>,
) -> Result<(ProfileMaterialKind, i64, bool), AppError> {
    let metadata = metadata
        .and_then(Value::as_object)
        .ok_or_else(|| AppError::InvalidState {
            current: image_type.to_string(),
            operation: "读取档案素材 publication 元数据".to_string(),
        })?;
    let (profile_kind, profile_id) = profile_material_identity_from_fields(
        image_type,
        metadata.get("profile_kind").and_then(Value::as_str),
        metadata.get("profile_id").and_then(Value::as_i64),
    )
    .ok_or_else(|| AppError::InvalidState {
        current: image_type.to_string(),
        operation: "读取档案素材 publication 档案身份".to_string(),
    })?;
    let kind = match profile_kind {
        a4n_db::profile_material::PROFILE_KIND_SCENE => ProfileMaterialKind::Scene,
        a4n_db::profile_material::PROFILE_KIND_OBJECT => ProfileMaterialKind::Object,
        _ => unreachable!("profile material identity only returns known profile kinds"),
    };
    let set_primary = parse_profile_material_set_primary(image_type, metadata.get("set_primary"))?;
    Ok((kind, profile_id, set_primary))
}

fn parse_profile_material_set_primary(
    image_type: &str,
    value: Option<&Value>,
) -> Result<bool, AppError> {
    let invalid = || AppError::InvalidState {
        current: image_type.to_string(),
        operation: "读取档案素材 publication set_primary".to_string(),
    };
    let Some(value) = value else {
        return Ok(false);
    };
    match value {
        Value::Null => Ok(false),
        Value::Bool(value) => Ok(*value),
        Value::String(value) => match value.trim() {
            "true" | "1" => Ok(true),
            "false" | "0" => Ok(false),
            _ => Err(invalid()),
        },
        Value::Number(value) => match value.as_i64() {
            Some(1) => Ok(true),
            Some(0) => Ok(false),
            _ => Err(invalid()),
        },
        _ => Err(invalid()),
    }
}

fn non_empty(value: &str) -> Option<&str> {
    let value = value.trim();
    if value.is_empty() { None } else { Some(value) }
}

async fn cleanup_profile_staging_after_error(
    host: &dyn ImageGenerationHostPort,
    staging_path: &str,
    error: AppError,
) -> AppError {
    match host
        .remove_generated_image_file(staging_path, "档案素材图暂存文件")
        .await
    {
        Ok(()) => error,
        Err(cleanup_error) => crate::errors::app_error_with_secondary(
            error,
            format!("清理档案素材图暂存文件失败：{cleanup_error}"),
        ),
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
    #[test]
    fn scene_image_hints_keep_visible_environment_without_entity_tracking() {
        let hint = scene_visual_state_hint(&json!({
            "summary": "门窗已被暴雨打坏", "weather_or_lighting": "阴天散射光", "mood": "安静",
            "characters": [{"profile_id": 19}], "objects": [{"object_profile_id": 8}],
            "entry_state": {"角色": "进入"}, "chapter_number": 4,
        }));
        assert!(hint.contains("门窗已被暴雨打坏"));
        assert!(hint.contains("阴天散射光"));
        assert!(!hint.contains("profile_id"));
        assert!(!hint.contains("角色"));
        assert_eq!(scene_visual_state_hint(&json!({})), "");
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
    fn profile_material_publication_details_validate_identity_and_primary_intent() {
        let details = profile_material_publication_details(
            IMAGE_TYPE_SCENE_MATERIAL,
            Some(&json!({
                "profile_kind": "scene",
                "profile_id": 9,
                "set_primary": true,
            })),
        )
        .expect("有效档案素材 publication 元数据必须可解析");
        assert_eq!(details, (ProfileMaterialKind::Scene, 9, true));

        assert_eq!(
            profile_material_publication_details(
                IMAGE_TYPE_OBJECT_MATERIAL,
                Some(&json!({
                    "profile_kind": "object",
                    "profile_id": 10,
                })),
            )
            .expect("旧档案素材 publication 缺少 set_primary 时应默认为 false"),
            (ProfileMaterialKind::Object, 10, false)
        );

        for value in [json!(null), json!("false"), json!("0"), json!(0)] {
            let mut metadata = json!({
                "profile_kind": "scene",
                "profile_id": 9,
            });
            metadata["set_primary"] = value;
            assert_eq!(
                profile_material_publication_details(IMAGE_TYPE_SCENE_MATERIAL, Some(&metadata),)
                    .expect("历史 false 形态必须可兼容读取"),
                (ProfileMaterialKind::Scene, 9, false)
            );
        }

        for value in [json!("true"), json!("1"), json!(1)] {
            let mut metadata = json!({
                "profile_kind": "scene",
                "profile_id": 9,
            });
            metadata["set_primary"] = value;
            assert_eq!(
                profile_material_publication_details(IMAGE_TYPE_SCENE_MATERIAL, Some(&metadata),)
                    .expect("历史 true 形态必须可兼容读取"),
                (ProfileMaterialKind::Scene, 9, true)
            );
        }

        for value in [json!("yes"), json!(2), json!([]), json!({})] {
            let mut metadata = json!({
                "profile_kind": "scene",
                "profile_id": 9,
            });
            metadata["set_primary"] = value;
            assert!(
                profile_material_publication_details(IMAGE_TYPE_SCENE_MATERIAL, Some(&metadata),)
                    .is_err(),
                "不明确的 set_primary 形态必须显式失败"
            );
        }

        assert!(
            profile_material_publication_details(
                IMAGE_TYPE_SCENE_MATERIAL,
                Some(&json!({
                    "profile_kind": "object",
                    "profile_id": 9,
                })),
            )
            .is_err()
        );
    }
}
