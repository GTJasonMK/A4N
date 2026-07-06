//! 项目管理服务：项目CRUD + 状态机验证 + 工作流回退
//!
//! 对应文档 01-项目管理与状态机

mod cleanup;
mod deletion;
mod dependency_graph;
mod reset;
mod status;

use chrono::Utc;
use sea_orm::sea_query::Expr;
use sea_orm::*;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::sync::Arc;

use a4n_core::error::AppError;
use a4n_core::event::bridge::EventBus;
use a4n_core::models::{
    asset::object_profile,
    chapter::{chapter, chapter_outline, part_outline},
    character::protagonist_profile,
    project::{
        blueprint_version, novel_project, project_derivation, project_source_snapshot,
        source_snapshot_chapter, source_snapshot_outline, source_snapshot_profile,
    },
    scene::scene_profile,
};
use a4n_core::state::status::ProjectStatus;

use crate::text_anchor::{InsertContentAnchorsInput, insert_content_anchors};

pub use dependency_graph::{
    AssetDependencyGraph, AssetGraphEdge, AssetGraphLayer, AssetGraphNode, AssetGraphNodeDetail,
    AssetLineageRecord, AssetLineageRef, GenerationInputSource, GenerationRunSnapshot,
    PromptSource,
};
pub use reset::ProjectGeneratedDataResetResult;

pub const PROJECT_KIND_TEXT: &str = "text";
const PROJECT_KIND_COMIC: &str = "comic";
pub const PROJECT_KIND_VIDEO: &str = "video";
const PROJECT_KIND_GALGAME: &str = "galgame";

pub const SOURCE_MODE_SELF: &str = "self";
pub const SOURCE_MODE_DERIVED: &str = "derived";
pub const SOURCE_MODE_FREEFORM: &str = "freeform";

pub const PROJECT_TYPE_NOVEL_LONG: &str = "novel_long";
const PROJECT_TYPE_NOVEL_SHORT: &str = "novel_short";
const PROJECT_TYPE_SCRIPT: &str = "script";
const PROJECT_TYPE_PROSE: &str = "prose";

pub const SOURCE_SYNC_CURRENT: &str = "current";
pub const SOURCE_SYNC_ROOT_UPDATED: &str = "root_updated";
pub const SOURCE_SYNC_CONFLICT: &str = "conflict";
pub const SOURCE_SYNC_DETACHED: &str = "detached";

pub const DERIVED_PROJECT_STATUS_DRAFT: &str = "draft";
const SOURCE_SYNC_STATUSES: [&str; 4] = [
    SOURCE_SYNC_CURRENT,
    SOURCE_SYNC_ROOT_UPDATED,
    SOURCE_SYNC_CONFLICT,
    SOURCE_SYNC_DETACHED,
];
const DERIVED_PROJECT_STATUSES: [&str; 1] = [DERIVED_PROJECT_STATUS_DRAFT];

pub fn is_valid_source_sync_status(value: &str) -> bool {
    SOURCE_SYNC_STATUSES.contains(&value)
}

pub fn is_valid_derived_project_status(value: &str) -> bool {
    DERIVED_PROJECT_STATUSES.contains(&value)
}

#[derive(Debug, Clone, Copy)]
pub struct DerivedChapterScope {
    pub start_chapter: i32,
    pub end_chapter: i32,
}

#[derive(Debug, Clone)]
pub struct CreateDerivedProjectParams {
    pub root_project_id: i64,
    pub derived_kind: String,
    pub title: String,
    pub description: String,
    pub chapter_scope: DerivedChapterScope,
    pub creation_options_json: Value,
}

#[derive(Debug, Clone)]
pub struct CreateFreeformVideoProjectParams {
    pub title: String,
    pub description: String,
}

/// 项目服务
#[derive(Clone)]
pub struct ProjectService {
    db: Arc<DatabaseConnection>,
    event_bus: Arc<dyn EventBus>,
}

impl ProjectService {
    pub fn new(db: Arc<DatabaseConnection>, event_bus: Arc<dyn EventBus>) -> Self {
        Self { db, event_bus }
    }

    /// 获取项目详情
    pub async fn get(&self, project_id: i64) -> Result<novel_project::Model, AppError> {
        novel_project::Entity::find_by_id(project_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))
    }

    /// 创建项目
    pub async fn create(
        &self,
        user_id: i64,
        title: &str,
        description: &str,
        project_type: &str,
    ) -> Result<novel_project::Model, AppError> {
        let title = required_project_title(title)?;
        let description = description.trim();
        let project_type = normalize_project_type(project_type)?;
        let now = Utc::now().naive_utc();
        let project = novel_project::ActiveModel {
            user_id: Set(user_id),
            title: Set(title.to_string()),
            description: Set(description.to_string()),
            project_type: Set(project_type.to_string()),
            project_kind: Set(PROJECT_KIND_TEXT.to_string()),
            source_mode: Set(SOURCE_MODE_SELF.to_string()),
            root_project_id: Set(None),
            parent_project_id: Set(None),
            source_snapshot_id: Set(None),
            source_sync_status: Set(SOURCE_SYNC_CURRENT.to_string()),
            derived_project_status: Set(None),
            status: Set(ProjectStatus::Draft.as_str().to_string()),
            target_chapters: Set(100),
            current_chapter: Set(0),
            total_chapters: Set(None),
            chapters_per_part: Set(Some(25)),
            avatar_svg: Set(None),
            avatar_animal: Set(None),
            raw_blueprint: Set(None),
            import_analysis_progress: Set(None),
            export_config: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };

        project
            .insert(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    pub async fn create_derived_project(
        &self,
        user_id: i64,
        params: CreateDerivedProjectParams,
    ) -> Result<novel_project::Model, AppError> {
        let derived_kind = normalize_derived_project_kind(&params.derived_kind)?;
        let title = required_project_title(&params.title)?.to_string();
        let description = params.description.trim().to_string();
        let scope = normalize_chapter_scope(params.chapter_scope)?;
        if !params.creation_options_json.is_object() {
            return Err(AppError::InvalidArgument(
                "creation_options_json 必须是 JSON 对象".to_string(),
            ));
        }

        let tx = self.db.begin().await.map_err(AppError::Database)?;
        let root = novel_project::Entity::find_by_id(params.root_project_id)
            .one(&tx)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| {
                AppError::NotFound(format!("根文本项目不存在: {}", params.root_project_id))
            })?;
        if root.user_id != user_id {
            return Err(AppError::Forbidden("无权操作根文本项目".into()));
        }
        if root.project_kind != PROJECT_KIND_TEXT {
            return Err(AppError::InvalidArgument(
                "派生项目的根项目必须是文本项目".to_string(),
            ));
        }

        let now = Utc::now().naive_utc();
        let snapshot = create_source_snapshot_in(&tx, root.id, scope, now).await?;
        let derived = novel_project::ActiveModel {
            user_id: Set(user_id),
            title: Set(title),
            description: Set(description),
            project_type: Set(root.project_type.clone()),
            project_kind: Set(derived_kind.to_string()),
            source_mode: Set(SOURCE_MODE_DERIVED.to_string()),
            root_project_id: Set(Some(root.id)),
            parent_project_id: Set(Some(root.id)),
            source_snapshot_id: Set(Some(snapshot.id)),
            source_sync_status: Set(SOURCE_SYNC_CURRENT.to_string()),
            derived_project_status: Set(Some(DERIVED_PROJECT_STATUS_DRAFT.to_string())),
            status: Set(ProjectStatus::Draft.as_str().to_string()),
            target_chapters: Set(scope.end_chapter - scope.start_chapter + 1),
            current_chapter: Set(0),
            total_chapters: Set(Some(scope.end_chapter - scope.start_chapter + 1)),
            chapters_per_part: Set(root.chapters_per_part),
            avatar_svg: Set(root.avatar_svg.clone()),
            avatar_animal: Set(root.avatar_animal.clone()),
            raw_blueprint: Set(None),
            import_analysis_progress: Set(None),
            export_config: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&tx)
        .await
        .map_err(AppError::Database)?;

        project_derivation::ActiveModel {
            root_project_id: Set(root.id),
            derived_project_id: Set(derived.id),
            derived_kind: Set(derived_kind.to_string()),
            source_snapshot_id: Set(snapshot.id),
            chapter_scope_json: Set(chapter_scope_json(scope)),
            creation_options_json: Set(params.creation_options_json),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(&tx)
        .await
        .map_err(AppError::Database)?;

        tx.commit().await.map_err(AppError::Database)?;
        Ok(derived)
    }

    pub async fn create_freeform_video_project(
        &self,
        user_id: i64,
        params: CreateFreeformVideoProjectParams,
    ) -> Result<novel_project::Model, AppError> {
        let title = required_project_title(&params.title)?;
        let description = params.description.trim();
        let now = Utc::now().naive_utc();
        novel_project::ActiveModel {
            user_id: Set(user_id),
            title: Set(title.to_string()),
            description: Set(description.to_string()),
            project_type: Set(PROJECT_TYPE_SCRIPT.to_string()),
            project_kind: Set(PROJECT_KIND_VIDEO.to_string()),
            source_mode: Set(SOURCE_MODE_FREEFORM.to_string()),
            root_project_id: Set(None),
            parent_project_id: Set(None),
            source_snapshot_id: Set(None),
            source_sync_status: Set(SOURCE_SYNC_CURRENT.to_string()),
            derived_project_status: Set(None),
            status: Set(ProjectStatus::Draft.as_str().to_string()),
            target_chapters: Set(0),
            current_chapter: Set(0),
            total_chapters: Set(None),
            chapters_per_part: Set(None),
            avatar_svg: Set(None),
            avatar_animal: Set(None),
            raw_blueprint: Set(None),
            import_analysis_progress: Set(None),
            export_config: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(self.db.as_ref())
        .await
        .map_err(AppError::Database)
    }

    pub async fn mark_derived_projects_root_updated(
        &self,
        root_project_id: i64,
    ) -> Result<u64, AppError> {
        let root = self.get(root_project_id).await?;
        if root.project_kind != PROJECT_KIND_TEXT {
            return Err(AppError::InvalidArgument(
                "只能标记文本项目的派生项目源状态".to_string(),
            ));
        }
        mark_derived_projects_root_updated_in(
            self.db.as_ref(),
            root_project_id,
            Utc::now().naive_utc(),
        )
        .await
    }

    pub async fn create_source_snapshot(
        &self,
        root_project_id: i64,
        scope: DerivedChapterScope,
    ) -> Result<project_source_snapshot::Model, AppError> {
        let root = self.get(root_project_id).await?;
        if root.project_kind != PROJECT_KIND_TEXT {
            return Err(AppError::InvalidArgument(
                "只能为文本根项目创建源文本快照".to_string(),
            ));
        }
        let scope = normalize_chapter_scope(scope)?;
        let tx = self.db.begin().await.map_err(AppError::Database)?;
        let snapshot =
            create_source_snapshot_in(&tx, root_project_id, scope, Utc::now().naive_utc()).await?;
        tx.commit().await.map_err(AppError::Database)?;
        Ok(snapshot)
    }

    /// 更新项目基本信息
    pub async fn update(
        &self,
        project_id: i64,
        title: Option<&str>,
        description: Option<&str>,
    ) -> Result<novel_project::Model, AppError> {
        let project = self.get(project_id).await?;
        let mut active: novel_project::ActiveModel = project.into();

        if let Some(t) = title {
            active.title = Set(required_project_title(t)?.to_string());
        }
        if let Some(d) = description {
            active.description = Set(d.trim().to_string());
        }
        active.updated_at = Set(Utc::now().naive_utc());

        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }
}

pub fn normalize_project_kind(project_kind: &str) -> Result<&str, AppError> {
    let project_kind = project_kind.trim();
    match project_kind {
        "" => Err(AppError::InvalidArgument("项目族类型不能为空".to_string())),
        PROJECT_KIND_TEXT | PROJECT_KIND_COMIC | PROJECT_KIND_VIDEO | PROJECT_KIND_GALGAME => {
            Ok(project_kind)
        }
        other => Err(AppError::InvalidArgument(format!(
            "项目族类型无效: {other}"
        ))),
    }
}

pub fn normalize_derived_project_kind(project_kind: &str) -> Result<&str, AppError> {
    let project_kind = normalize_project_kind(project_kind)?;
    if project_kind == PROJECT_KIND_TEXT {
        return Err(AppError::InvalidArgument(
            "派生项目类型不能是 text".to_string(),
        ));
    }
    Ok(project_kind)
}

pub fn normalize_project_type(project_type: &str) -> Result<&str, AppError> {
    let project_type = project_type.trim();
    match project_type {
        "" => Err(AppError::InvalidArgument("项目类型不能为空".to_string())),
        PROJECT_TYPE_NOVEL_LONG
        | PROJECT_TYPE_NOVEL_SHORT
        | PROJECT_TYPE_SCRIPT
        | PROJECT_TYPE_PROSE => Ok(project_type),
        other => Err(AppError::InvalidArgument(format!("项目类型无效: {other}"))),
    }
}

fn normalize_chapter_scope(scope: DerivedChapterScope) -> Result<DerivedChapterScope, AppError> {
    if scope.start_chapter <= 0 || scope.end_chapter <= 0 {
        return Err(AppError::InvalidArgument(
            "派生章节范围必须为正整数".to_string(),
        ));
    }
    if scope.start_chapter > scope.end_chapter {
        return Err(AppError::InvalidArgument(
            "派生章节范围起点不能大于终点".to_string(),
        ));
    }
    Ok(scope)
}

fn chapter_scope_json(scope: DerivedChapterScope) -> Value {
    json!({
        "start": scope.start_chapter,
        "end": scope.end_chapter,
    })
}

pub(crate) async fn create_source_snapshot_in<C>(
    db: &C,
    root_project_id: i64,
    scope: DerivedChapterScope,
    now: chrono::NaiveDateTime,
) -> Result<project_source_snapshot::Model, AppError>
where
    C: ConnectionTrait,
{
    let data = build_source_snapshot_data(db, root_project_id, scope).await?;

    let snapshot = project_source_snapshot::ActiveModel {
        root_project_id: Set(root_project_id),
        source_version_hash: Set(data.source_version_hash),
        blueprint_version_id: Set(data.latest_blueprint_version_id),
        chapter_scope_json: Set(data.chapter_scope_json.clone()),
        chapter_hashes_json: Set(Value::Array(data.chapter_hashes_json.clone())),
        profile_hashes_json: Set(data.profile_hashes_json.clone()),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AppError::Database)?;

    for chapter in data.chapters {
        let snapshot_chapter = source_snapshot_chapter::ActiveModel {
            snapshot_id: Set(snapshot.id),
            root_project_id: Set(root_project_id),
            source_chapter_id: Set(chapter.id),
            chapter_number: Set(chapter.chapter_number),
            title: Set(chapter.title),
            content: Set(chapter.content.clone()),
            summary: Set(chapter.summary),
            analysis_data_json: Set(chapter.analysis_data.clone()),
            content_hash: Set(hash_text(&chapter.content)),
            analysis_hash: Set(chapter.analysis_data.as_ref().map(hash_json)),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(AppError::Database)?;
        insert_content_anchors(
            db,
            InsertContentAnchorsInput {
                project_id: root_project_id,
                source_chapter_id: Some(snapshot_chapter.source_chapter_id),
                source_snapshot_id: Some(snapshot.id),
                source_snapshot_chapter_id: Some(snapshot_chapter.id),
                source_id: snapshot_chapter.id,
                chapter_number: snapshot_chapter.chapter_number,
                content: &snapshot_chapter.content,
                source_version_hash: &snapshot_chapter.content_hash,
            },
        )
        .await?;
    }

    freeze_outlines(db, snapshot.id, root_project_id, scope).await?;
    freeze_profiles(db, snapshot.id, root_project_id).await?;

    Ok(snapshot)
}

pub(crate) async fn calculate_source_version_hash<C>(
    db: &C,
    root_project_id: i64,
    scope: DerivedChapterScope,
) -> Result<String, AppError>
where
    C: ConnectionTrait,
{
    Ok(build_source_snapshot_data(db, root_project_id, scope)
        .await?
        .source_version_hash)
}

struct SourceSnapshotBuildData {
    chapters: Vec<chapter::Model>,
    chapter_hashes_json: Vec<Value>,
    profile_hashes_json: Value,
    latest_blueprint_version_id: Option<i64>,
    chapter_scope_json: Value,
    source_version_hash: String,
}

async fn build_source_snapshot_data<C>(
    db: &C,
    root_project_id: i64,
    scope: DerivedChapterScope,
) -> Result<SourceSnapshotBuildData, AppError>
where
    C: ConnectionTrait,
{
    let chapters = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(root_project_id))
        .filter(chapter::Column::ChapterNumber.gte(scope.start_chapter))
        .filter(chapter::Column::ChapterNumber.lte(scope.end_chapter))
        .order_by_asc(chapter::Column::ChapterNumber)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    if chapters.is_empty() {
        return Err(AppError::InvalidArgument(
            "源文本快照范围内没有章节".to_string(),
        ));
    }
    let expected_numbers = (scope.start_chapter..=scope.end_chapter).collect::<Vec<_>>();
    let actual_numbers = chapters
        .iter()
        .map(|chapter| chapter.chapter_number)
        .collect::<Vec<_>>();
    if actual_numbers != expected_numbers {
        let missing = expected_numbers
            .into_iter()
            .filter(|number| !actual_numbers.contains(number))
            .collect::<Vec<_>>();
        return Err(AppError::InvalidArgument(format!(
            "源文本快照范围内缺少章节: {:?}",
            missing
        )));
    }

    let chapter_hashes_json = chapters
        .iter()
        .map(|chapter| {
            json!({
                "source_chapter_id": chapter.id,
                "chapter_number": chapter.chapter_number,
                "content_hash": hash_text(&chapter.content),
                "analysis_hash": chapter.analysis_data.as_ref().map(hash_json),
            })
        })
        .collect::<Vec<_>>();
    let profile_hashes_json = profile_hashes_json(db, root_project_id).await?;
    let latest_blueprint_version_id = blueprint_version::Entity::find()
        .filter(blueprint_version::Column::ProjectId.eq(root_project_id))
        .order_by_desc(blueprint_version::Column::CreatedAt)
        .order_by_desc(blueprint_version::Column::Id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .map(|version| version.id);
    let chapter_scope_json = chapter_scope_json(scope);
    let source_version_hash = hash_json(&json!({
        "root_project_id": root_project_id,
        "chapter_scope": chapter_scope_json,
        "chapter_hashes": chapter_hashes_json,
        "profile_hashes": profile_hashes_json,
        "blueprint_version_id": latest_blueprint_version_id,
    }));

    Ok(SourceSnapshotBuildData {
        chapters,
        chapter_hashes_json,
        profile_hashes_json,
        latest_blueprint_version_id,
        chapter_scope_json,
        source_version_hash,
    })
}

async fn freeze_outlines<C>(
    db: &C,
    snapshot_id: i64,
    root_project_id: i64,
    scope: DerivedChapterScope,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    let part_outlines = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(root_project_id))
        .filter(part_outline::Column::EndChapter.gte(scope.start_chapter))
        .filter(part_outline::Column::StartChapter.lte(scope.end_chapter))
        .order_by_asc(part_outline::Column::PartNumber)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    for outline in part_outlines {
        insert_snapshot_outline(
            db,
            snapshot_id,
            root_project_id,
            outline.id,
            "part",
            &outline,
        )
        .await?;
    }

    let chapter_outlines = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(root_project_id))
        .filter(chapter_outline::Column::ChapterNumber.gte(scope.start_chapter))
        .filter(chapter_outline::Column::ChapterNumber.lte(scope.end_chapter))
        .order_by_asc(chapter_outline::Column::ChapterNumber)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    for outline in chapter_outlines {
        insert_snapshot_outline(
            db,
            snapshot_id,
            root_project_id,
            outline.id,
            "chapter",
            &outline,
        )
        .await?;
    }
    Ok(())
}

async fn insert_snapshot_outline<C, T>(
    db: &C,
    snapshot_id: i64,
    root_project_id: i64,
    source_outline_id: i64,
    outline_kind: &str,
    outline: &T,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
    T: serde::Serialize,
{
    let payload = to_json(outline)?;
    source_snapshot_outline::ActiveModel {
        snapshot_id: Set(snapshot_id),
        root_project_id: Set(root_project_id),
        source_outline_id: Set(source_outline_id),
        outline_kind: Set(outline_kind.to_string()),
        payload_hash: Set(hash_json(&payload)),
        payload_json: Set(payload),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

async fn freeze_profiles<C>(db: &C, snapshot_id: i64, root_project_id: i64) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    for profile in protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(root_project_id))
        .all(db)
        .await
        .map_err(AppError::Database)?
    {
        insert_snapshot_profile(
            db,
            snapshot_id,
            root_project_id,
            "character",
            profile.id,
            &profile,
        )
        .await?;
    }
    for profile in object_profile::Entity::find()
        .filter(object_profile::Column::ProjectId.eq(root_project_id))
        .all(db)
        .await
        .map_err(AppError::Database)?
    {
        insert_snapshot_profile(
            db,
            snapshot_id,
            root_project_id,
            "object",
            profile.id,
            &profile,
        )
        .await?;
    }
    for profile in scene_profile::Entity::find()
        .filter(scene_profile::Column::ProjectId.eq(root_project_id))
        .all(db)
        .await
        .map_err(AppError::Database)?
    {
        insert_snapshot_profile(
            db,
            snapshot_id,
            root_project_id,
            "scene",
            profile.id,
            &profile,
        )
        .await?;
    }
    Ok(())
}

async fn insert_snapshot_profile<C, T>(
    db: &C,
    snapshot_id: i64,
    root_project_id: i64,
    profile_kind: &str,
    source_profile_id: i64,
    profile: &T,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
    T: serde::Serialize,
{
    let payload = to_json(profile)?;
    source_snapshot_profile::ActiveModel {
        snapshot_id: Set(snapshot_id),
        root_project_id: Set(root_project_id),
        profile_kind: Set(profile_kind.to_string()),
        source_profile_id: Set(source_profile_id),
        payload_hash: Set(hash_json(&payload)),
        payload_json: Set(payload),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AppError::Database)?;
    Ok(())
}

async fn profile_hashes_json<C>(db: &C, root_project_id: i64) -> Result<Value, AppError>
where
    C: ConnectionTrait,
{
    let mut profiles = Vec::new();
    for profile in protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(root_project_id))
        .all(db)
        .await
        .map_err(AppError::Database)?
    {
        profiles.push(
            json!({"kind": "character", "id": profile.id, "hash": hash_json(&to_json(&profile)?)}),
        );
    }
    for profile in object_profile::Entity::find()
        .filter(object_profile::Column::ProjectId.eq(root_project_id))
        .all(db)
        .await
        .map_err(AppError::Database)?
    {
        profiles.push(
            json!({"kind": "object", "id": profile.id, "hash": hash_json(&to_json(&profile)?)}),
        );
    }
    for profile in scene_profile::Entity::find()
        .filter(scene_profile::Column::ProjectId.eq(root_project_id))
        .all(db)
        .await
        .map_err(AppError::Database)?
    {
        profiles.push(
            json!({"kind": "scene", "id": profile.id, "hash": hash_json(&to_json(&profile)?)}),
        );
    }
    Ok(Value::Array(profiles))
}

async fn mark_derived_projects_root_updated_in<C>(
    db: &C,
    root_project_id: i64,
    now: chrono::NaiveDateTime,
) -> Result<u64, AppError>
where
    C: ConnectionTrait,
{
    let result = novel_project::Entity::update_many()
        .col_expr(
            novel_project::Column::SourceSyncStatus,
            Expr::value(SOURCE_SYNC_ROOT_UPDATED),
        )
        .col_expr(novel_project::Column::UpdatedAt, Expr::value(now))
        .filter(novel_project::Column::RootProjectId.eq(root_project_id))
        .filter(novel_project::Column::SourceSyncStatus.eq(SOURCE_SYNC_CURRENT))
        .exec(db)
        .await
        .map_err(AppError::Database)?;
    Ok(result.rows_affected)
}

fn to_json<T>(value: &T) -> Result<Value, AppError>
where
    T: serde::Serialize,
{
    serde_json::to_value(value)
        .map_err(|err| AppError::Internal(anyhow::anyhow!("源快照序列化失败: {err}")))
}

fn hash_text(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn hash_json(value: &Value) -> String {
    let bytes = serde_json::to_vec(value).expect("serde_json::Value serialization cannot fail");
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    format!("{:x}", hasher.finalize())
}

fn required_project_title(title: &str) -> Result<&str, AppError> {
    let title = title.trim();
    if title.is_empty() {
        Err(AppError::InvalidArgument("项目标题不能为空".to_string()))
    } else {
        Ok(title)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_core::event::bridge::BroadcastEventBus;
    use a4n_core::models::project::text_anchor;
    use sea_orm::{
        ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait,
        QueryFilter, Set,
    };
    use std::sync::Arc;

    async fn setup_db() -> DatabaseConnection {
        a4n_migration::test_support::connect_test_database().await
    }

    fn service(db: Arc<DatabaseConnection>) -> ProjectService {
        ProjectService::new(db, Arc::new(BroadcastEventBus::new(16)))
    }

    async fn project_count(db: &DatabaseConnection) -> u64 {
        novel_project::Entity::find().count(db).await.unwrap()
    }

    async fn insert_chapter(
        db: &DatabaseConnection,
        project_id: i64,
        chapter_number: i32,
        title: &str,
        content: &str,
    ) -> chapter::Model {
        let now = Utc::now().naive_utc();
        chapter::ActiveModel {
            project_id: Set(project_id),
            outline_id: Set(None),
            chapter_number: Set(chapter_number),
            title: Set(Some(title.to_string())),
            content: Set(content.to_string()),
            word_count: Set(content.chars().count() as i32),
            status: Set(chapter::STATUS_IMPORTED.to_string()),
            summary: Set(Some(format!("{title} 摘要"))),
            style_tags: Set(None),
            analysis_data: Set(Some(json!({"beats": ["开端"]}))),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn create_rejects_unknown_project_type_without_insert() {
        let db = Arc::new(setup_db().await);
        let service = service(db.clone());

        let err = service
            .create(1, "测试项目", "测试简介", "unknown")
            .await
            .expect_err("unknown project type should be rejected");

        assert!(
            matches!(err, AppError::InvalidArgument(message) if message.contains("项目类型无效"))
        );
        assert_eq!(project_count(db.as_ref()).await, 0);
    }

    #[tokio::test]
    async fn create_normalizes_project_title_description_and_type() {
        let db = Arc::new(setup_db().await);
        let service = service(db);

        let project = service
            .create(1, "  测试项目  ", "  测试简介  ", " novel_short ")
            .await
            .unwrap();

        assert_eq!(project.title, "测试项目");
        assert_eq!(project.description, "测试简介");
        assert_eq!(project.project_type, PROJECT_TYPE_NOVEL_SHORT);
        assert_eq!(project.project_kind, PROJECT_KIND_TEXT);
        assert_eq!(project.root_project_id, None);
        assert_eq!(project.source_snapshot_id, None);
        assert_eq!(project.source_sync_status, SOURCE_SYNC_CURRENT);
    }

    #[tokio::test]
    async fn create_freeform_video_project_uses_fixed_script_type() {
        let db = Arc::new(setup_db().await);
        let service = service(db);

        let project = service
            .create_freeform_video_project(
                1,
                CreateFreeformVideoProjectParams {
                    title: "  视频画布  ".to_string(),
                    description: "  自由项目  ".to_string(),
                },
            )
            .await
            .unwrap();

        assert_eq!(project.title, "视频画布");
        assert_eq!(project.description, "自由项目");
        assert_eq!(project.project_type, PROJECT_TYPE_SCRIPT);
        assert_eq!(project.project_kind, PROJECT_KIND_VIDEO);
        assert_eq!(project.source_mode, SOURCE_MODE_FREEFORM);
        assert_eq!(project.root_project_id, None);
        assert_eq!(project.parent_project_id, None);
        assert_eq!(project.source_snapshot_id, None);
        assert_eq!(project.derived_project_status, None);
    }

    #[tokio::test]
    async fn database_rejects_freeform_project_with_source_snapshot() {
        let db = Arc::new(setup_db().await);
        let service = service(db.clone());
        let root = service
            .create(1, "根文本", "测试简介", PROJECT_TYPE_NOVEL_LONG)
            .await
            .unwrap();
        insert_chapter(db.as_ref(), root.id, 1, "第一章", "他推开门。").await;
        let snapshot = service
            .create_source_snapshot(
                root.id,
                DerivedChapterScope {
                    start_chapter: 1,
                    end_chapter: 1,
                },
            )
            .await
            .unwrap();
        let now = Utc::now().naive_utc();

        let err = novel_project::ActiveModel {
            user_id: Set(1),
            title: Set("非法自由视频项目".to_string()),
            description: Set(String::new()),
            project_type: Set(PROJECT_TYPE_SCRIPT.to_string()),
            project_kind: Set(PROJECT_KIND_VIDEO.to_string()),
            source_mode: Set(SOURCE_MODE_FREEFORM.to_string()),
            root_project_id: Set(None),
            parent_project_id: Set(None),
            source_snapshot_id: Set(Some(snapshot.id)),
            source_sync_status: Set(SOURCE_SYNC_CURRENT.to_string()),
            derived_project_status: Set(None),
            status: Set(ProjectStatus::Draft.as_str().to_string()),
            target_chapters: Set(0),
            current_chapter: Set(0),
            total_chapters: Set(None),
            chapters_per_part: Set(None),
            avatar_svg: Set(None),
            avatar_animal: Set(None),
            raw_blueprint: Set(None),
            import_analysis_progress: Set(None),
            export_config: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db.as_ref())
        .await
        .expect_err("database trigger should reject freeform project with source snapshot");

        assert!(
            err.to_string()
                .contains("self/freeform 项目不能绑定派生项目字段")
        );
    }

    #[tokio::test]
    async fn create_derived_video_project_freezes_source_snapshot_and_derivation() {
        let db = Arc::new(setup_db().await);
        let service = service(db.clone());
        let root = service
            .create(1, "根文本", "测试简介", PROJECT_TYPE_NOVEL_LONG)
            .await
            .unwrap();
        let chapter =
            insert_chapter(db.as_ref(), root.id, 1, "第一章", "他推开门，走进雨夜。").await;

        let derived = service
            .create_derived_project(
                1,
                CreateDerivedProjectParams {
                    root_project_id: root.id,
                    derived_kind: PROJECT_KIND_VIDEO.to_string(),
                    title: "视频项目".to_string(),
                    description: "视频派生".to_string(),
                    chapter_scope: DerivedChapterScope {
                        start_chapter: 1,
                        end_chapter: 1,
                    },
                    creation_options_json: json!({}),
                },
            )
            .await
            .unwrap();

        assert_eq!(derived.project_kind, PROJECT_KIND_VIDEO);
        assert_eq!(derived.root_project_id, Some(root.id));
        assert_eq!(derived.parent_project_id, Some(root.id));
        assert_eq!(derived.source_sync_status, SOURCE_SYNC_CURRENT);
        let snapshot_id = derived.source_snapshot_id.unwrap();

        let derivation = project_derivation::Entity::find()
            .filter(project_derivation::Column::DerivedProjectId.eq(derived.id))
            .one(db.as_ref())
            .await
            .unwrap()
            .expect("derived project should have derivation row");
        assert_eq!(derivation.root_project_id, root.id);
        assert_eq!(derivation.derived_kind, PROJECT_KIND_VIDEO);
        assert_eq!(derivation.source_snapshot_id, snapshot_id);
        assert_eq!(derivation.chapter_scope_json, json!({"start": 1, "end": 1}));

        let frozen_chapter = source_snapshot_chapter::Entity::find()
            .filter(source_snapshot_chapter::Column::SnapshotId.eq(snapshot_id))
            .one(db.as_ref())
            .await
            .unwrap()
            .expect("source snapshot should freeze selected chapter");
        assert_eq!(frozen_chapter.root_project_id, root.id);
        assert_eq!(frozen_chapter.source_chapter_id, chapter.id);
        assert_eq!(frozen_chapter.chapter_number, 1);
        assert_eq!(frozen_chapter.title.as_deref(), Some("第一章"));
        assert_eq!(frozen_chapter.content, "他推开门，走进雨夜。");
        assert_eq!(frozen_chapter.summary.as_deref(), Some("第一章 摘要"));
        assert_eq!(
            frozen_chapter.analysis_data_json,
            Some(json!({"beats": ["开端"]}))
        );

        let frozen_anchors = text_anchor::Entity::find()
            .filter(text_anchor::Column::SourceSnapshotId.eq(snapshot_id))
            .filter(text_anchor::Column::SourceSnapshotChapterId.eq(frozen_chapter.id))
            .all(db.as_ref())
            .await
            .unwrap();
        assert!(!frozen_anchors.is_empty());
        assert!(
            frozen_anchors
                .iter()
                .all(|anchor| anchor.project_id == root.id
                    && anchor.source_chapter_id == Some(chapter.id)
                    && anchor.source_snapshot_id == Some(snapshot_id)
                    && anchor.source_snapshot_chapter_id == Some(frozen_chapter.id))
        );
    }

    #[tokio::test]
    async fn create_derived_project_rejects_non_text_root_project() {
        let db = Arc::new(setup_db().await);
        let service = service(db.clone());
        let root = service
            .create(1, "根文本", "测试简介", PROJECT_TYPE_NOVEL_LONG)
            .await
            .unwrap();
        insert_chapter(db.as_ref(), root.id, 1, "第一章", "他推开门。").await;
        let derived = service
            .create_derived_project(
                1,
                CreateDerivedProjectParams {
                    root_project_id: root.id,
                    derived_kind: PROJECT_KIND_VIDEO.to_string(),
                    title: "视频项目".to_string(),
                    description: "".to_string(),
                    chapter_scope: DerivedChapterScope {
                        start_chapter: 1,
                        end_chapter: 1,
                    },
                    creation_options_json: json!({}),
                },
            )
            .await
            .unwrap();

        let err = service
            .create_derived_project(
                1,
                CreateDerivedProjectParams {
                    root_project_id: derived.id,
                    derived_kind: PROJECT_KIND_COMIC.to_string(),
                    title: "漫画项目".to_string(),
                    description: "".to_string(),
                    chapter_scope: DerivedChapterScope {
                        start_chapter: 1,
                        end_chapter: 1,
                    },
                    creation_options_json: json!({}),
                },
            )
            .await
            .expect_err("derived project cannot be used as root");

        assert!(
            matches!(err, AppError::InvalidArgument(message) if message.contains("根项目必须是文本项目"))
        );
    }

    #[tokio::test]
    async fn create_derived_project_rejects_incomplete_chapter_scope() {
        let db = Arc::new(setup_db().await);
        let service = service(db.clone());
        let root = service
            .create(1, "根文本", "测试简介", PROJECT_TYPE_NOVEL_LONG)
            .await
            .unwrap();
        insert_chapter(db.as_ref(), root.id, 1, "第一章", "他推开门。").await;
        insert_chapter(db.as_ref(), root.id, 3, "第三章", "雨停了。").await;

        let err = service
            .create_derived_project(
                1,
                CreateDerivedProjectParams {
                    root_project_id: root.id,
                    derived_kind: PROJECT_KIND_VIDEO.to_string(),
                    title: "视频项目".to_string(),
                    description: "".to_string(),
                    chapter_scope: DerivedChapterScope {
                        start_chapter: 1,
                        end_chapter: 3,
                    },
                    creation_options_json: json!({}),
                },
            )
            .await
            .expect_err("source snapshot must cover every chapter in selected scope");

        assert!(
            matches!(err, AppError::InvalidArgument(message) if message.contains("源文本快照范围内缺少章节"))
        );
        assert_eq!(project_count(db.as_ref()).await, 1);
    }

    #[tokio::test]
    async fn derived_project_rejects_text_state_machine() {
        let db = Arc::new(setup_db().await);
        let service = service(db.clone());
        let root = service
            .create(1, "根文本", "测试简介", PROJECT_TYPE_NOVEL_LONG)
            .await
            .unwrap();
        insert_chapter(db.as_ref(), root.id, 1, "第一章", "他推开门。").await;
        let derived = service
            .create_derived_project(
                1,
                CreateDerivedProjectParams {
                    root_project_id: root.id,
                    derived_kind: PROJECT_KIND_VIDEO.to_string(),
                    title: "视频项目".to_string(),
                    description: "".to_string(),
                    chapter_scope: DerivedChapterScope {
                        start_chapter: 1,
                        end_chapter: 1,
                    },
                    creation_options_json: json!({}),
                },
            )
            .await
            .unwrap();

        let err = service
            .forward_status(derived.id, ProjectStatus::BlueprintReady.as_str())
            .await
            .expect_err("text state machine should reject derived projects");

        assert!(
            matches!(err, AppError::InvalidArgument(message) if message.contains("非文本项目不能使用文本写作状态机"))
        );
    }

    #[tokio::test]
    async fn mark_derived_projects_root_updated_marks_current_derivatives() {
        let db = Arc::new(setup_db().await);
        let service = service(db.clone());
        let root = service
            .create(1, "根文本", "测试简介", PROJECT_TYPE_NOVEL_LONG)
            .await
            .unwrap();
        insert_chapter(db.as_ref(), root.id, 1, "第一章", "他推开门。").await;
        let derived = service
            .create_derived_project(
                1,
                CreateDerivedProjectParams {
                    root_project_id: root.id,
                    derived_kind: PROJECT_KIND_VIDEO.to_string(),
                    title: "视频项目".to_string(),
                    description: "".to_string(),
                    chapter_scope: DerivedChapterScope {
                        start_chapter: 1,
                        end_chapter: 1,
                    },
                    creation_options_json: json!({}),
                },
            )
            .await
            .unwrap();

        let changed = service
            .mark_derived_projects_root_updated(root.id)
            .await
            .unwrap();

        assert_eq!(changed, 1);
        let stored = service.get(derived.id).await.unwrap();
        assert_eq!(stored.source_sync_status, SOURCE_SYNC_ROOT_UPDATED);
    }

    #[tokio::test]
    async fn database_rejects_derived_project_without_source_snapshot() {
        let db = Arc::new(setup_db().await);
        let service = service(db.clone());
        let root = service
            .create(1, "根文本", "测试简介", PROJECT_TYPE_NOVEL_LONG)
            .await
            .unwrap();
        let now = Utc::now().naive_utc();

        let err = novel_project::ActiveModel {
            user_id: Set(1),
            title: Set("非法视频项目".to_string()),
            description: Set(String::new()),
            project_type: Set(root.project_type.clone()),
            project_kind: Set(PROJECT_KIND_VIDEO.to_string()),
            source_mode: Set(SOURCE_MODE_DERIVED.to_string()),
            root_project_id: Set(Some(root.id)),
            parent_project_id: Set(Some(root.id)),
            source_snapshot_id: Set(None),
            source_sync_status: Set(SOURCE_SYNC_CURRENT.to_string()),
            derived_project_status: Set(Some(DERIVED_PROJECT_STATUS_DRAFT.to_string())),
            status: Set(ProjectStatus::Draft.as_str().to_string()),
            target_chapters: Set(1),
            current_chapter: Set(0),
            total_chapters: Set(Some(1)),
            chapters_per_part: Set(Some(25)),
            avatar_svg: Set(None),
            avatar_animal: Set(None),
            raw_blueprint: Set(None),
            import_analysis_progress: Set(None),
            export_config: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db.as_ref())
        .await
        .expect_err("database trigger should reject derived project without snapshot");

        assert!(err.to_string().contains("派生项目必须绑定根项目"));
        assert_eq!(project_count(db.as_ref()).await, 1);
    }

    #[tokio::test]
    async fn database_rejects_derivation_snapshot_mismatch() {
        let db = Arc::new(setup_db().await);
        let service = service(db.clone());
        let root = service
            .create(1, "根文本", "测试简介", PROJECT_TYPE_NOVEL_LONG)
            .await
            .unwrap();
        insert_chapter(db.as_ref(), root.id, 1, "第一章", "他推开门。").await;
        let derived = service
            .create_derived_project(
                1,
                CreateDerivedProjectParams {
                    root_project_id: root.id,
                    derived_kind: PROJECT_KIND_VIDEO.to_string(),
                    title: "视频项目".to_string(),
                    description: String::new(),
                    chapter_scope: DerivedChapterScope {
                        start_chapter: 1,
                        end_chapter: 1,
                    },
                    creation_options_json: json!({}),
                },
            )
            .await
            .unwrap();
        let new_snapshot = service
            .create_source_snapshot(
                root.id,
                DerivedChapterScope {
                    start_chapter: 1,
                    end_chapter: 1,
                },
            )
            .await
            .unwrap();
        let derivation = project_derivation::Entity::find()
            .filter(project_derivation::Column::DerivedProjectId.eq(derived.id))
            .one(db.as_ref())
            .await
            .unwrap()
            .unwrap();
        let mut active: project_derivation::ActiveModel = derivation.into();
        active.source_snapshot_id = Set(new_snapshot.id);

        let err = active
            .update(db.as_ref())
            .await
            .expect_err("database trigger should reject derivation/project mismatch");

        assert!(err.to_string().contains("派生关系必须与派生项目主体一致"));
    }

    #[tokio::test]
    async fn update_rejects_blank_title_without_mutating_project() {
        let db = Arc::new(setup_db().await);
        let service = service(db);
        let project = service
            .create(1, "测试项目", "测试简介", PROJECT_TYPE_NOVEL_LONG)
            .await
            .unwrap();

        let err = service
            .update(project.id, Some(" \n\t "), None)
            .await
            .expect_err("blank title should be rejected");

        assert!(
            matches!(err, AppError::InvalidArgument(message) if message.contains("项目标题不能为空"))
        );
        let stored = service.get(project.id).await.unwrap();
        assert_eq!(stored.title, "测试项目");
    }

    #[tokio::test]
    async fn update_normalizes_present_title_and_description() {
        let db = Arc::new(setup_db().await);
        let service = service(db);
        let project = service
            .create(1, "测试项目", "测试简介", PROJECT_TYPE_NOVEL_LONG)
            .await
            .unwrap();

        let updated = service
            .update(project.id, Some("  新标题  "), Some("  新简介  "))
            .await
            .unwrap();

        assert_eq!(updated.title, "新标题");
        assert_eq!(updated.description, "新简介");
    }

    #[test]
    fn normalize_project_type_rejects_blank_and_unknown_values() {
        let blank = normalize_project_type(" \n\t ").expect_err("blank type should be rejected");
        assert!(
            matches!(blank, AppError::InvalidArgument(message) if message.contains("项目类型不能为空"))
        );

        let unknown =
            normalize_project_type("novel_medium").expect_err("unknown type should be rejected");
        assert!(
            matches!(unknown, AppError::InvalidArgument(message) if message.contains("项目类型无效"))
        );
    }
}
