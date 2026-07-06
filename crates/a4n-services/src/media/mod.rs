//! 统一媒体资产服务。

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use sea_orm::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use a4n_core::error::AppError;
use a4n_core::models::{
    generation::{generation_spec, prompt_artifact},
    media::{media_asset, media_provenance},
    project::novel_project,
};

use crate::project::SOURCE_MODE_FREEFORM;
use crate::project_family::{
    ProjectFamilyContext, load_project_family_context, validate_snapshot_chapter,
};

pub const MEDIA_KIND_IMAGE: &str = "image";
pub const MEDIA_KIND_VIDEO: &str = "video";
pub const MEDIA_KIND_AUDIO: &str = "audio";

pub const MEDIA_ORIGIN_GENERATED: &str = "generated";
pub const MEDIA_ORIGIN_IMPORTED: &str = "imported";
pub const MEDIA_ORIGIN_EXTRACTED_FRAME: &str = "extracted_frame";
pub const MEDIA_ORIGIN_ASSEMBLED_PREVIEW: &str = "assembled_preview";

pub const MEDIA_STATUS_PENDING: &str = "pending";
pub const MEDIA_STATUS_RUNNING: &str = "running";
pub const MEDIA_STATUS_COMPLETED: &str = "completed";
pub const MEDIA_STATUS_FAILED: &str = "failed";
pub const MEDIA_STATUS_CANCELLED: &str = "cancelled";

const MEDIA_KINDS: &[&str] = &[MEDIA_KIND_IMAGE, MEDIA_KIND_VIDEO, MEDIA_KIND_AUDIO];
const MEDIA_ORIGINS: &[&str] = &[
    MEDIA_ORIGIN_GENERATED,
    MEDIA_ORIGIN_IMPORTED,
    MEDIA_ORIGIN_EXTRACTED_FRAME,
    MEDIA_ORIGIN_ASSEMBLED_PREVIEW,
];
const MEDIA_STATUSES: &[&str] = &[
    MEDIA_STATUS_PENDING,
    MEDIA_STATUS_RUNNING,
    MEDIA_STATUS_COMPLETED,
    MEDIA_STATUS_FAILED,
    MEDIA_STATUS_CANCELLED,
];
pub const MEDIA_BINDING_TARGET_KINDS: &[&str] = &[
    "character_profile",
    "object_profile",
    "scene_profile",
    "galgame_scene",
    "galgame_line",
    "video_segment",
    "chapter_manga_prompt",
    "chapter_video_prompt",
    "generated_video",
    "chapter",
    "assembly_timeline",
];
pub const MEDIA_BINDING_ROLES: &[&str] = &[
    "portrait",
    "object_material",
    "scene_material",
    "background",
    "cg",
    "bgm",
    "sfx",
    "first_frame",
    "end_frame",
    "reference",
    "generated_result",
    "inserted_bridge",
    "video_tail_frame",
];

pub const MEDIA_BINDING_TARGET_GALGAME_SCENE: &str = "galgame_scene";
pub const MEDIA_BINDING_TARGET_GALGAME_LINE: &str = "galgame_line";
pub const MEDIA_BINDING_ROLE_BACKGROUND: &str = "background";
pub const MEDIA_BINDING_ROLE_CG: &str = "cg";
pub const MEDIA_BINDING_ROLE_BGM: &str = "bgm";
pub const MEDIA_BINDING_ROLE_SFX: &str = "sfx";

pub fn is_valid_media_kind(value: &str) -> bool {
    MEDIA_KINDS.contains(&value)
}

pub fn is_valid_media_origin(value: &str) -> bool {
    MEDIA_ORIGINS.contains(&value)
}

pub fn is_valid_media_status(value: &str) -> bool {
    MEDIA_STATUSES.contains(&value)
}

pub fn is_valid_media_binding_target_kind(value: &str) -> bool {
    MEDIA_BINDING_TARGET_KINDS.contains(&value)
}

pub fn is_valid_media_binding_role(value: &str) -> bool {
    MEDIA_BINDING_ROLES.contains(&value)
}

pub fn galgame_media_binding_required_media_kind(
    target_kind: &str,
    role: &str,
) -> Option<&'static str> {
    match (target_kind, role) {
        (MEDIA_BINDING_TARGET_GALGAME_SCENE, MEDIA_BINDING_ROLE_BACKGROUND) => {
            Some(MEDIA_KIND_IMAGE)
        }
        (MEDIA_BINDING_TARGET_GALGAME_LINE, MEDIA_BINDING_ROLE_CG) => Some(MEDIA_KIND_IMAGE),
        (MEDIA_BINDING_TARGET_GALGAME_LINE, MEDIA_BINDING_ROLE_BGM | MEDIA_BINDING_ROLE_SFX) => {
            Some(MEDIA_KIND_AUDIO)
        }
        _ => None,
    }
}

pub fn is_valid_galgame_media_binding_pair(target_kind: &str, role: &str) -> bool {
    match target_kind {
        MEDIA_BINDING_TARGET_GALGAME_SCENE | MEDIA_BINDING_TARGET_GALGAME_LINE => {
            galgame_media_binding_required_media_kind(target_kind, role).is_some()
        }
        _ => true,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMediaAssetInput {
    pub project_id: i64,
    pub root_project_id: i64,
    pub source_snapshot_id: Option<i64>,
    pub source_chapter_id: Option<i64>,
    pub source_snapshot_chapter_id: Option<i64>,
    pub media_kind: String,
    pub origin: String,
    pub file_path: Option<String>,
    pub thumbnail_path: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub duration_secs: Option<f32>,
    pub fps: Option<f32>,
    pub status: String,
    pub error_message: Option<String>,
    pub provenance: CreateMediaProvenanceInput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMediaProvenanceInput {
    pub generation_spec_id: Option<i64>,
    pub prompt_artifact_id: Option<i64>,
    pub provider_job_id: Option<String>,
    pub source_media_asset_id: Option<i64>,
    pub source_external_path: Option<String>,
    pub imported_from_user: bool,
    pub request_snapshot_json: Option<Value>,
}

#[derive(Clone)]
pub struct MediaAssetService {
    db: Arc<DatabaseConnection>,
    data_dir: PathBuf,
}

enum MediaContext {
    SnapshotBacked(Box<ProjectFamilyContext>),
    Freeform { project_id: i64 },
}

impl MediaContext {
    fn project_id(&self) -> i64 {
        match self {
            Self::SnapshotBacked(context) => context.project.id,
            Self::Freeform { project_id } => *project_id,
        }
    }

    fn root_project_id(&self) -> i64 {
        match self {
            Self::SnapshotBacked(context) => context.root_project.id,
            Self::Freeform { project_id } => *project_id,
        }
    }
}

impl MediaAssetService {
    pub fn new(db: Arc<DatabaseConnection>, data_dir: PathBuf) -> Self {
        Self { db, data_dir }
    }

    pub async fn create_media_asset(
        &self,
        input: CreateMediaAssetInput,
    ) -> Result<media_asset::Model, AppError> {
        let media_kind = normalize_media_kind(&input.media_kind)?;
        let origin = normalize_origin(&input.origin)?;
        let status = normalize_status(&input.status)?;
        let context = load_media_context(
            self.db.as_ref(),
            input.project_id,
            input.root_project_id,
            input.source_snapshot_id,
        )
        .await?;
        if input.source_chapter_id.is_some() != input.source_snapshot_chapter_id.is_some() {
            return Err(AppError::InvalidArgument(
                "source_chapter_id 和 source_snapshot_chapter_id 必须同时提供或同时为空"
                    .to_string(),
            ));
        }
        if let Some(snapshot_chapter_id) = input.source_snapshot_chapter_id {
            let MediaContext::SnapshotBacked(context) = &context else {
                return Err(AppError::InvalidArgument(
                    "freeform 媒体资产不能设置 source_snapshot_chapter_id".to_string(),
                ));
            };
            validate_snapshot_chapter(
                self.db.as_ref(),
                context,
                snapshot_chapter_id,
                input.source_chapter_id,
            )
            .await?;
        }
        let checked_file = match input.file_path.as_deref() {
            Some(path) => Some(resolve_data_file(&self.data_dir, path, "媒体文件")?),
            None => None,
        };
        if let Some(path) = input.thumbnail_path.as_deref() {
            resolve_data_file(&self.data_dir, path, "媒体缩略图")?;
        }
        if status == MEDIA_STATUS_COMPLETED && checked_file.is_none() {
            return Err(AppError::InvalidArgument(
                "completed 媒体资产必须提供 file_path".to_string(),
            ));
        }
        let (file_size, checksum) = if let Some(path) = checked_file.as_ref() {
            (Some(file_size(path)?), Some(file_checksum(path)?))
        } else {
            (None, None)
        };
        let now = Utc::now().naive_utc();
        let tx = self.db.begin().await.map_err(AppError::Database)?;
        let asset = media_asset::ActiveModel {
            project_id: Set(context.project_id()),
            root_project_id: Set(context.root_project_id()),
            source_snapshot_id: Set(input.source_snapshot_id),
            source_chapter_id: Set(input.source_chapter_id),
            source_snapshot_chapter_id: Set(input.source_snapshot_chapter_id),
            media_kind: Set(media_kind.to_string()),
            origin: Set(origin.to_string()),
            file_path: Set(input.file_path),
            thumbnail_path: Set(input.thumbnail_path),
            width: Set(input.width),
            height: Set(input.height),
            duration_secs: Set(input.duration_secs),
            fps: Set(input.fps),
            file_size: Set(file_size),
            checksum: Set(checksum),
            status: Set(status.to_string()),
            error_message: Set(input.error_message),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&tx)
        .await
        .map_err(AppError::Database)?;
        validate_provenance_refs(&tx, &asset, &input.provenance).await?;
        insert_provenance(&tx, asset.id, input.provenance, now).await?;
        tx.commit().await.map_err(AppError::Database)?;
        Ok(asset)
    }
}

async fn load_media_context<C>(
    db: &C,
    project_id: i64,
    root_project_id: i64,
    source_snapshot_id: Option<i64>,
) -> Result<MediaContext, AppError>
where
    C: ConnectionTrait,
{
    if let Some(snapshot_id) = source_snapshot_id {
        let context =
            load_project_family_context(db, project_id, root_project_id, snapshot_id).await?;
        return Ok(MediaContext::SnapshotBacked(Box::new(context)));
    }

    let project = novel_project::Entity::find_by_id(project_id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
    if project.source_mode != SOURCE_MODE_FREEFORM {
        return Err(AppError::InvalidArgument(
            "缺少 source_snapshot_id 的媒体资产只能创建在 freeform 项目中".to_string(),
        ));
    }
    if root_project_id != project.id {
        return Err(AppError::InvalidArgument(
            "freeform 媒体资产 root_project_id 必须等于项目自身".to_string(),
        ));
    }
    if project.root_project_id.is_some()
        || project.parent_project_id.is_some()
        || project.source_snapshot_id.is_some()
    {
        return Err(AppError::InvalidArgument(
            "freeform 媒体资产项目不能包含文本派生上下文字段".to_string(),
        ));
    }
    Ok(MediaContext::Freeform {
        project_id: project.id,
    })
}

async fn validate_provenance_refs<C>(
    db: &C,
    asset: &media_asset::Model,
    input: &CreateMediaProvenanceInput,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    let referenced_spec = if let Some(generation_spec_id) = input.generation_spec_id {
        let spec = generation_spec::Entity::find_by_id(generation_spec_id)
            .one(db)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("生成意图不存在: {generation_spec_id}")))?;
        if spec.project_id != asset.project_id || spec.root_project_id != asset.root_project_id {
            return Err(AppError::InvalidArgument(format!(
                "媒体来源生成意图 {} 不属于媒体资产项目族",
                spec.id
            )));
        }
        validate_provenance_source_context(
            asset,
            spec.source_snapshot_id,
            spec.source_snapshot_chapter_id,
            spec.source_chapter_id,
            "生成意图",
            spec.id,
        )?;
        Some(spec)
    } else {
        None
    };

    if let Some(prompt_artifact_id) = input.prompt_artifact_id {
        let artifact = prompt_artifact::Entity::find_by_id(prompt_artifact_id)
            .one(db)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("提示词产物不存在: {prompt_artifact_id}")))?;
        let prompt_spec = generation_spec::Entity::find_by_id(artifact.generation_spec_id)
            .one(db)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| {
                AppError::NotFound(format!("生成意图不存在: {}", artifact.generation_spec_id))
            })?;
        if prompt_spec.project_id != asset.project_id
            || prompt_spec.root_project_id != asset.root_project_id
        {
            return Err(AppError::InvalidArgument(format!(
                "提示词产物 {} 不属于媒体资产项目族",
                artifact.id
            )));
        }
        validate_provenance_source_context(
            asset,
            prompt_spec.source_snapshot_id,
            prompt_spec.source_snapshot_chapter_id,
            prompt_spec.source_chapter_id,
            "提示词产物生成意图",
            prompt_spec.id,
        )?;
        if let Some(spec) = referenced_spec.as_ref()
            && artifact.generation_spec_id != spec.id
        {
            return Err(AppError::InvalidArgument(format!(
                "提示词产物 {} 不属于生成意图 {}",
                artifact.id, spec.id
            )));
        }
    }

    if let Some(source_media_asset_id) = input.source_media_asset_id {
        let source = media_asset::Entity::find_by_id(source_media_asset_id)
            .one(db)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| {
                AppError::NotFound(format!("源媒体资产不存在: {source_media_asset_id}"))
            })?;
        if source.project_id != asset.project_id || source.root_project_id != asset.root_project_id
        {
            return Err(AppError::InvalidArgument(format!(
                "源媒体资产 {} 不属于媒体资产项目族",
                source.id
            )));
        }
        validate_provenance_source_context(
            asset,
            source.source_snapshot_id,
            source.source_snapshot_chapter_id,
            source.source_chapter_id,
            "源媒体资产",
            source.id,
        )?;
        if source.status != MEDIA_STATUS_COMPLETED || source.file_path.is_none() {
            return Err(AppError::InvalidArgument(format!(
                "源媒体资产 {} 不是可追溯的已完成媒体",
                source.id
            )));
        }
    }
    Ok(())
}

fn validate_provenance_source_context(
    asset: &media_asset::Model,
    ref_source_snapshot_id: Option<i64>,
    ref_source_snapshot_chapter_id: Option<i64>,
    ref_source_chapter_id: Option<i64>,
    label: &str,
    ref_id: i64,
) -> Result<(), AppError> {
    match asset.source_snapshot_id {
        Some(asset_snapshot_id) if ref_source_snapshot_id != Some(asset_snapshot_id) => {
            return Err(AppError::InvalidArgument(format!(
                "媒体来源{label} {ref_id} 的 source_snapshot_id 与媒体资产不一致"
            )));
        }
        None if ref_source_snapshot_id.is_some() || ref_source_snapshot_chapter_id.is_some() => {
            return Err(AppError::InvalidArgument(format!(
                "freeform 媒体资产不能引用带源快照上下文的{label} {ref_id}"
            )));
        }
        _ => {}
    }

    if let Some(asset_snapshot_chapter_id) = asset.source_snapshot_chapter_id
        && ref_source_snapshot_chapter_id != Some(asset_snapshot_chapter_id)
    {
        return Err(AppError::InvalidArgument(format!(
            "媒体来源{label} {ref_id} 的 source_snapshot_chapter_id 与媒体资产不一致"
        )));
    }
    if let Some(asset_source_chapter_id) = asset.source_chapter_id
        && ref_source_chapter_id != Some(asset_source_chapter_id)
    {
        return Err(AppError::InvalidArgument(format!(
            "媒体来源{label} {ref_id} 的 source_chapter_id 与媒体资产不一致"
        )));
    }
    Ok(())
}

async fn insert_provenance<C>(
    db: &C,
    media_asset_id: i64,
    input: CreateMediaProvenanceInput,
    now: chrono::NaiveDateTime,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    media_provenance::ActiveModel {
        media_asset_id: Set(media_asset_id),
        generation_spec_id: Set(input.generation_spec_id),
        prompt_artifact_id: Set(input.prompt_artifact_id),
        provider_job_id: Set(input.provider_job_id),
        source_media_asset_id: Set(input.source_media_asset_id),
        source_external_path: Set(input.source_external_path),
        imported_from_user: Set(input.imported_from_user),
        request_snapshot_json: Set(input.request_snapshot_json),
        created_at: Set(now),
    }
    .insert(db)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

fn resolve_data_file(data_dir: &Path, path: &str, label: &str) -> Result<PathBuf, AppError> {
    let path = required_text(path, label)?;
    let raw = Path::new(path);
    if raw.is_absolute() {
        return Err(AppError::InvalidArgument(format!(
            "{label}必须是应用数据目录内的相对路径"
        )));
    }
    let canonical_data_dir = data_dir.canonicalize()?;
    let canonical_file = data_dir.join(raw).canonicalize()?;
    if !canonical_file.starts_with(&canonical_data_dir) || !canonical_file.is_file() {
        return Err(AppError::InvalidArgument(format!(
            "{label}不在应用数据目录内或不是文件"
        )));
    }
    Ok(canonical_file)
}

fn file_size(path: &Path) -> Result<i64, AppError> {
    i64::try_from(fs::metadata(path)?.len())
        .map_err(|_| AppError::InvalidArgument("媒体文件过大".to_string()))
}

fn file_checksum(path: &Path) -> Result<String, AppError> {
    let bytes = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

fn required_text<'a>(value: &'a str, label: &str) -> Result<&'a str, AppError> {
    let value = value.trim();
    if value.is_empty() {
        Err(AppError::InvalidArgument(format!("{label}不能为空")))
    } else {
        Ok(value)
    }
}

fn normalize_media_kind(value: &str) -> Result<&str, AppError> {
    normalize_one_of(value, "media_kind", MEDIA_KINDS)
}

fn normalize_origin(value: &str) -> Result<&str, AppError> {
    normalize_one_of(value, "origin", MEDIA_ORIGINS)
}

fn normalize_status(value: &str) -> Result<&str, AppError> {
    normalize_one_of(value, "status", MEDIA_STATUSES)
}

fn normalize_one_of<'a>(
    value: &'a str,
    label: &str,
    allowed: &[&str],
) -> Result<&'a str, AppError> {
    let value = value.trim();
    if allowed.contains(&value) {
        Ok(value)
    } else {
        Err(AppError::InvalidArgument(format!("{label} 无效: {value}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generation::SPEC_STATUS_DRAFT;
    use crate::project::{
        CreateDerivedProjectParams, DerivedChapterScope, PROJECT_KIND_VIDEO,
        PROJECT_TYPE_NOVEL_LONG, ProjectService,
    };
    use a4n_core::event::bridge::BroadcastEventBus;
    use a4n_core::models::chapter::chapter;
    use a4n_core::models::project::{novel_project, project_source_snapshot};
    use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};
    use serde_json::json;
    use std::sync::Arc;

    async fn setup_db() -> DatabaseConnection {
        a4n_migration::test_support::connect_test_database().await
    }

    fn project_service(db: Arc<DatabaseConnection>) -> ProjectService {
        ProjectService::new(db, Arc::new(BroadcastEventBus::new(16)))
    }

    async fn insert_chapter(db: &DatabaseConnection, project_id: i64) {
        let now = Utc::now().naive_utc();
        chapter::ActiveModel {
            project_id: Set(project_id),
            outline_id: Set(None),
            chapter_number: Set(1),
            title: Set(Some("第一章".to_string())),
            content: Set("他推开门。".to_string()),
            word_count: Set(5),
            status: Set(chapter::STATUS_IMPORTED.to_string()),
            summary: Set(None),
            style_tags: Set(None),
            analysis_data: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();
    }

    async fn create_video_project(
        service: &ProjectService,
        db: &DatabaseConnection,
        user_id: i64,
        title: &str,
    ) -> novel_project::Model {
        let root = service
            .create(user_id, title, "", PROJECT_TYPE_NOVEL_LONG)
            .await
            .unwrap();
        insert_chapter(db, root.id).await;
        service
            .create_derived_project(
                user_id,
                CreateDerivedProjectParams {
                    root_project_id: root.id,
                    derived_kind: PROJECT_KIND_VIDEO.to_string(),
                    title: format!("{title}视频"),
                    description: String::new(),
                    chapter_scope: DerivedChapterScope {
                        start_chapter: 1,
                        end_chapter: 1,
                    },
                    creation_options_json: json!({}),
                },
            )
            .await
            .unwrap()
    }

    async fn insert_generation_spec(
        db: &DatabaseConnection,
        project: &novel_project::Model,
    ) -> generation_spec::Model {
        let snapshot_id = project.source_snapshot_id.unwrap();
        insert_generation_spec_for_snapshot(db, project, snapshot_id).await
    }

    async fn insert_generation_spec_for_snapshot(
        db: &DatabaseConnection,
        project: &novel_project::Model,
        snapshot_id: i64,
    ) -> generation_spec::Model {
        let snapshot = project_source_snapshot::Entity::find_by_id(snapshot_id)
            .one(db)
            .await
            .unwrap()
            .expect("source snapshot exists");
        let now = Utc::now().naive_utc();
        generation_spec::ActiveModel {
            project_id: Set(project.id),
            root_project_id: Set(project.root_project_id.unwrap()),
            source_context_kind: Set(Some("snapshot_backed".to_string())),
            source_snapshot_id: Set(Some(snapshot.id)),
            source_chapter_id: Set(None),
            source_snapshot_chapter_id: Set(None),
            target_kind: Set("final_assembly".to_string()),
            target_entity_kind: Set("assembly".to_string()),
            target_entity_id: Set(None),
            status: Set(SPEC_STATUS_DRAFT.to_string()),
            source_version_hash: Set(snapshot.source_version_hash),
            params_json: Set(json!({})),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap()
    }

    async fn create_additional_source_snapshot(
        service: &ProjectService,
        root_project_id: i64,
    ) -> project_source_snapshot::Model {
        service
            .create_source_snapshot(
                root_project_id,
                DerivedChapterScope {
                    start_chapter: 1,
                    end_chapter: 1,
                },
            )
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn create_media_asset_rejects_cross_project_generation_spec_provenance() {
        let db = Arc::new(setup_db().await);
        let project_service = project_service(db.clone());
        let first = create_video_project(&project_service, db.as_ref(), 1, "根文本A").await;
        let second = create_video_project(&project_service, db.as_ref(), 1, "根文本B").await;
        let second_spec = insert_generation_spec(db.as_ref(), &second).await;

        let err = MediaAssetService::new(db, std::env::temp_dir())
            .create_media_asset(CreateMediaAssetInput {
                project_id: first.id,
                root_project_id: first.root_project_id.unwrap(),
                source_snapshot_id: Some(first.source_snapshot_id.unwrap()),
                source_chapter_id: None,
                source_snapshot_chapter_id: None,
                media_kind: "video".to_string(),
                origin: "generated".to_string(),
                file_path: None,
                thumbnail_path: None,
                width: None,
                height: None,
                duration_secs: None,
                fps: None,
                status: MEDIA_STATUS_PENDING.to_string(),
                error_message: None,
                provenance: CreateMediaProvenanceInput {
                    generation_spec_id: Some(second_spec.id),
                    prompt_artifact_id: None,
                    provider_job_id: None,
                    source_media_asset_id: None,
                    source_external_path: None,
                    imported_from_user: false,
                    request_snapshot_json: None,
                },
            })
            .await
            .expect_err("media provenance must not reference another project family");

        assert!(
            matches!(err, AppError::InvalidArgument(message) if message.contains("不属于媒体资产项目族"))
        );
    }

    #[tokio::test]
    async fn create_media_asset_rejects_cross_snapshot_generation_spec_provenance() {
        let db = Arc::new(setup_db().await);
        let project_service = project_service(db.clone());
        let project = create_video_project(&project_service, db.as_ref(), 1, "根文本").await;
        let other_snapshot =
            create_additional_source_snapshot(&project_service, project.root_project_id.unwrap())
                .await;
        let other_spec =
            insert_generation_spec_for_snapshot(db.as_ref(), &project, other_snapshot.id).await;

        let err = MediaAssetService::new(db, std::env::temp_dir())
            .create_media_asset(CreateMediaAssetInput {
                project_id: project.id,
                root_project_id: project.root_project_id.unwrap(),
                source_snapshot_id: Some(project.source_snapshot_id.unwrap()),
                source_chapter_id: None,
                source_snapshot_chapter_id: None,
                media_kind: "video".to_string(),
                origin: "generated".to_string(),
                file_path: None,
                thumbnail_path: None,
                width: None,
                height: None,
                duration_secs: None,
                fps: None,
                status: MEDIA_STATUS_PENDING.to_string(),
                error_message: None,
                provenance: CreateMediaProvenanceInput {
                    generation_spec_id: Some(other_spec.id),
                    prompt_artifact_id: None,
                    provider_job_id: None,
                    source_media_asset_id: None,
                    source_external_path: None,
                    imported_from_user: false,
                    request_snapshot_json: None,
                },
            })
            .await
            .expect_err("media provenance must not reference another source snapshot");

        assert!(
            matches!(err, AppError::InvalidArgument(message) if message.contains("source_snapshot_id 与媒体资产不一致"))
        );
    }

    #[tokio::test]
    async fn create_media_asset_rejects_thumbnail_path_outside_data_dir() {
        let db = Arc::new(setup_db().await);
        let project_service = project_service(db.clone());
        let project = create_video_project(&project_service, db.as_ref(), 1, "根文本").await;

        let err = MediaAssetService::new(db, std::env::temp_dir())
            .create_media_asset(CreateMediaAssetInput {
                project_id: project.id,
                root_project_id: project.root_project_id.unwrap(),
                source_snapshot_id: Some(project.source_snapshot_id.unwrap()),
                source_chapter_id: None,
                source_snapshot_chapter_id: None,
                media_kind: "video".to_string(),
                origin: "generated".to_string(),
                file_path: None,
                thumbnail_path: Some("/tmp/outside-thumb.png".to_string()),
                width: None,
                height: None,
                duration_secs: None,
                fps: None,
                status: MEDIA_STATUS_PENDING.to_string(),
                error_message: None,
                provenance: CreateMediaProvenanceInput {
                    generation_spec_id: None,
                    prompt_artifact_id: None,
                    provider_job_id: None,
                    source_media_asset_id: None,
                    source_external_path: None,
                    imported_from_user: false,
                    request_snapshot_json: None,
                },
            })
            .await
            .expect_err("thumbnail path must stay inside application data dir");

        assert!(
            matches!(err, AppError::InvalidArgument(message) if message.contains("媒体缩略图必须是应用数据目录内的相对路径"))
        );
    }

    #[tokio::test]
    async fn create_media_asset_accepts_prompt_artifact_provenance_without_repeated_spec_id() {
        let db = Arc::new(setup_db().await);
        let project_service = project_service(db.clone());
        let project = create_video_project(&project_service, db.as_ref(), 1, "根文本").await;
        let spec = insert_generation_spec(db.as_ref(), &project).await;
        let now = Utc::now().naive_utc();
        let artifact = prompt_artifact::ActiveModel {
            generation_spec_id: Set(spec.id),
            prompt_kind: Set("chat_video_prompt".to_string()),
            prompt_text: Set("自然语言视频提示词".to_string()),
            renderer_version: Set("test".to_string()),
            edited_by_user: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db.as_ref())
        .await
        .unwrap();

        let asset = MediaAssetService::new(db, std::env::temp_dir())
            .create_media_asset(CreateMediaAssetInput {
                project_id: project.id,
                root_project_id: project.root_project_id.unwrap(),
                source_snapshot_id: Some(project.source_snapshot_id.unwrap()),
                source_chapter_id: None,
                source_snapshot_chapter_id: None,
                media_kind: "video".to_string(),
                origin: "generated".to_string(),
                file_path: None,
                thumbnail_path: None,
                width: None,
                height: None,
                duration_secs: None,
                fps: None,
                status: MEDIA_STATUS_PENDING.to_string(),
                error_message: None,
                provenance: CreateMediaProvenanceInput {
                    generation_spec_id: None,
                    prompt_artifact_id: Some(artifact.id),
                    provider_job_id: None,
                    source_media_asset_id: None,
                    source_external_path: None,
                    imported_from_user: false,
                    request_snapshot_json: None,
                },
            })
            .await
            .unwrap();

        assert_eq!(asset.project_id, project.id);
    }

    #[tokio::test]
    async fn create_media_asset_rejects_cross_snapshot_prompt_artifact_provenance() {
        let db = Arc::new(setup_db().await);
        let project_service = project_service(db.clone());
        let project = create_video_project(&project_service, db.as_ref(), 1, "根文本").await;
        let other_snapshot =
            create_additional_source_snapshot(&project_service, project.root_project_id.unwrap())
                .await;
        let other_spec =
            insert_generation_spec_for_snapshot(db.as_ref(), &project, other_snapshot.id).await;
        let now = Utc::now().naive_utc();
        let artifact = prompt_artifact::ActiveModel {
            generation_spec_id: Set(other_spec.id),
            prompt_kind: Set("chat_video_prompt".to_string()),
            prompt_text: Set("自然语言视频提示词".to_string()),
            renderer_version: Set("test".to_string()),
            edited_by_user: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db.as_ref())
        .await
        .unwrap();

        let err = MediaAssetService::new(db, std::env::temp_dir())
            .create_media_asset(CreateMediaAssetInput {
                project_id: project.id,
                root_project_id: project.root_project_id.unwrap(),
                source_snapshot_id: Some(project.source_snapshot_id.unwrap()),
                source_chapter_id: None,
                source_snapshot_chapter_id: None,
                media_kind: "video".to_string(),
                origin: "generated".to_string(),
                file_path: None,
                thumbnail_path: None,
                width: None,
                height: None,
                duration_secs: None,
                fps: None,
                status: MEDIA_STATUS_PENDING.to_string(),
                error_message: None,
                provenance: CreateMediaProvenanceInput {
                    generation_spec_id: None,
                    prompt_artifact_id: Some(artifact.id),
                    provider_job_id: None,
                    source_media_asset_id: None,
                    source_external_path: None,
                    imported_from_user: false,
                    request_snapshot_json: None,
                },
            })
            .await
            .expect_err("prompt artifact provenance must match media source snapshot");

        assert!(
            matches!(err, AppError::InvalidArgument(message) if message.contains("source_snapshot_id 与媒体资产不一致"))
        );
    }
}
