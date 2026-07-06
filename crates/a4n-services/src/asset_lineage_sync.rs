//! Legacy manga/video artifact projection into the unified generation/media chain.

use std::collections::BTreeMap;
use std::sync::Arc;

use a4n_core::error::AppError;
use a4n_core::models::{
    ai::{generated_image, generated_video},
    chapter::{chapter_manga_prompt, chapter_video_prompt},
    media::media_asset,
    project::{novel_project, project_source_snapshot, source_snapshot_chapter},
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::manga::{
    MANGA_EXTRACTION_SUMMARY_TYPE, MANGA_PAGE_PROMPT_RAW_TYPE, MANGA_PAGE_PROMPT_TYPE,
    PIPELINE_STATUS_PROMPT_TYPE,
};
use crate::project::{PROJECT_KIND_TEXT, SOURCE_MODE_FREEFORM, SOURCE_MODE_SELF};

mod persistence;

use persistence::{
    GenerationSpecUpsertInput, MediaUpsertInput, ensure_media_binding,
    sync_video_prompt_media_refs, upsert_generation_spec, upsert_media_asset,
    upsert_prompt_artifact,
};

const CANONICAL_SCHEMA_VERSION: u64 = 1;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AssetLineageBackfillSummary {
    pub scanned_projects: u64,
    pub scanned_manga_prompts: u64,
    pub scanned_video_prompts: u64,
    pub scanned_generated_images: u64,
    pub scanned_generated_videos: u64,
    pub created_generation_specs: u64,
    pub updated_generation_specs: u64,
    pub created_prompt_artifacts: u64,
    pub updated_prompt_artifacts: u64,
    pub created_media_assets: u64,
    pub updated_media_assets: u64,
    pub created_media_provenance: u64,
    pub updated_media_provenance: u64,
    pub created_media_bindings: u64,
    pub created_generation_media_refs: u64,
    pub warnings: Vec<String>,
}

#[derive(Clone)]
pub struct AssetLineageSyncService {
    db: Arc<DatabaseConnection>,
}

#[derive(Debug, Clone)]
struct ProjectLineageContext {
    project_id: i64,
    root_project_id: i64,
    source_mode: String,
    snapshot: Option<project_source_snapshot::Model>,
    snapshot_chapters_by_source: BTreeMap<i64, source_snapshot_chapter::Model>,
}

#[derive(Debug, Clone)]
struct FamilyLineageContext {
    project_ids: Vec<i64>,
    contexts_by_project_id: BTreeMap<i64, ProjectLineageContext>,
}

#[derive(Debug, Clone)]
struct SourceLineageContext {
    project_id: i64,
    root_project_id: i64,
    source_snapshot_id: i64,
    source_version_hash: String,
    source_chapter_id: Option<i64>,
    source_snapshot_chapter_id: Option<i64>,
}

#[derive(Debug, Clone)]
struct MediaSourceLineageContext {
    source_snapshot_id: Option<i64>,
    source_chapter_id: Option<i64>,
    source_snapshot_chapter_id: Option<i64>,
}

#[derive(Debug, Clone)]
struct SyncedPromptRef {
    generation_spec_id: i64,
    prompt_artifact_id: i64,
}

impl AssetLineageSyncService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn sync_project_asset_lineage(
        &self,
        project_id: i64,
    ) -> Result<AssetLineageBackfillSummary, AppError> {
        validate_positive_id(project_id, "project_id")?;
        let tx = self.db.begin().await.map_err(AppError::Database)?;
        let mut summary = AssetLineageBackfillSummary::default();
        let family = load_family_lineage_context(&tx, project_id).await?;
        summary.scanned_projects = family.project_ids.len() as u64;
        sync_family_asset_lineage(&tx, &family, &mut summary).await?;
        tx.commit().await.map_err(AppError::Database)?;
        Ok(summary)
    }
}

async fn sync_family_asset_lineage<C>(
    db: &C,
    family: &FamilyLineageContext,
    summary: &mut AssetLineageBackfillSummary,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    let manga_prompts = chapter_manga_prompt::Entity::find()
        .filter(chapter_manga_prompt::Column::ProjectId.is_in(family.project_ids.iter().copied()))
        .filter(chapter_manga_prompt::Column::PromptType.ne("dsl"))
        .filter(chapter_manga_prompt::Column::PromptType.ne(PIPELINE_STATUS_PROMPT_TYPE))
        .order_by_asc(chapter_manga_prompt::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    summary.scanned_manga_prompts = manga_prompts.len() as u64;

    let mut manga_prompt_refs = BTreeMap::<i64, SyncedPromptRef>::new();
    let mut manga_prompt_by_image = BTreeMap::<i64, chapter_manga_prompt::Model>::new();
    for prompt in &manga_prompts {
        if let Some(prompt_ref) = sync_manga_prompt(db, family, prompt, summary).await? {
            manga_prompt_refs.insert(prompt.id, prompt_ref);
        }
        if let Some(image_id) = prompt.generated_image_id {
            manga_prompt_by_image.insert(image_id, prompt.clone());
        }
    }

    let video_prompts = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.is_in(family.project_ids.iter().copied()))
        .order_by_asc(chapter_video_prompt::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    summary.scanned_video_prompts = video_prompts.len() as u64;

    let mut video_prompt_refs = BTreeMap::<i64, SyncedPromptRef>::new();
    let mut video_prompt_by_video = BTreeMap::<i64, chapter_video_prompt::Model>::new();
    for prompt in &video_prompts {
        if let Some(prompt_ref) = sync_video_prompt(db, family, prompt, summary).await? {
            video_prompt_refs.insert(prompt.id, prompt_ref);
        }
        if let Some(video_id) = prompt.generated_video_id {
            video_prompt_by_video.insert(video_id, prompt.clone());
        }
    }

    let generated_images = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.is_in(family.project_ids.iter().copied()))
        .order_by_asc(generated_image::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    summary.scanned_generated_images = generated_images.len() as u64;

    let mut image_media_by_legacy_id = BTreeMap::<i64, i64>::new();
    for image in &generated_images {
        if !should_sync_generated_image(image, &manga_prompt_by_image) {
            continue;
        }
        let prompt = manga_prompt_by_image.get(&image.id);
        let prompt_ref = prompt.and_then(|prompt| manga_prompt_refs.get(&prompt.id));
        if let Some(media) =
            sync_generated_image_media(db, family, image, prompt, prompt_ref, None, summary).await?
        {
            image_media_by_legacy_id.insert(image.id, media.id);
        }
    }

    let generated_videos = generated_video::Entity::find()
        .filter(generated_video::Column::ProjectId.is_in(family.project_ids.iter().copied()))
        .order_by_asc(generated_video::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    summary.scanned_generated_videos = generated_videos.len() as u64;

    let mut video_media_by_legacy_id = BTreeMap::<i64, i64>::new();
    for video in &generated_videos {
        let prompt = video_prompt_by_video
            .get(&video.id)
            .or_else(|| prompt_from_video_metadata(video, &video_prompts));
        let prompt_ref = prompt.and_then(|prompt| video_prompt_refs.get(&prompt.id));
        let source_media_asset_id = video
            .source_image_id
            .and_then(|id| image_media_by_legacy_id.get(&id).copied())
            .or_else(|| {
                video
                    .source_video_id
                    .and_then(|id| video_media_by_legacy_id.get(&id).copied())
            });
        let media = sync_generated_video_media(
            db,
            family,
            video,
            prompt,
            prompt_ref,
            source_media_asset_id,
            summary,
        )
        .await?;
        if let Some(media) = media {
            video_media_by_legacy_id.insert(video.id, media.id);
        }
    }

    for image in &generated_images {
        if image.image_type != "video_tail_frame" {
            continue;
        }
        let Some(source_video_id) = image
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("video_id"))
            .and_then(Value::as_i64)
        else {
            continue;
        };
        let Some(source_media_asset_id) = video_media_by_legacy_id.get(&source_video_id).copied()
        else {
            continue;
        };
        let prompt = manga_prompt_by_image.get(&image.id);
        let prompt_ref = prompt.and_then(|prompt| manga_prompt_refs.get(&prompt.id));
        let media = sync_generated_image_media(
            db,
            family,
            image,
            prompt,
            prompt_ref,
            Some(source_media_asset_id),
            summary,
        )
        .await?;
        if let Some(media) = media {
            image_media_by_legacy_id.insert(image.id, media.id);
        }
    }

    for prompt in &video_prompts {
        if let Some(prompt_ref) = video_prompt_refs.get(&prompt.id) {
            sync_video_prompt_media_refs(
                db,
                prompt_ref.generation_spec_id,
                prompt.reference_image_ids_json.as_ref(),
                &image_media_by_legacy_id,
                summary,
            )
            .await?;
        }
    }

    Ok(())
}

async fn sync_manga_prompt<C>(
    db: &C,
    family: &FamilyLineageContext,
    row: &chapter_manga_prompt::Model,
    summary: &mut AssetLineageBackfillSummary,
) -> Result<Option<SyncedPromptRef>, AppError>
where
    C: ConnectionTrait,
{
    let Some(source) = source_context_for_row(
        family,
        row.project_id,
        Some(row.chapter_id),
        "chapter_manga_prompt",
        row.id,
        summary,
    ) else {
        return Ok(None);
    };
    let Some((target_kind, prompt_kind)) = manga_generation_kinds(&row.prompt_type) else {
        push_unsupported_prompt_type_warning(
            summary,
            "chapter_manga_prompt",
            row.id,
            &row.prompt_type,
        );
        return Ok(None);
    };
    let Some(status) = canonical_spec_status(&row.status) else {
        push_unsupported_status_warning(summary, "chapter_manga_prompt", row.id, &row.status);
        return Ok(None);
    };
    let canonical_key = legacy_key("chapter_manga_prompt", row.id, "prompt");
    let params_json = json!({
        "canonical_schema_version": CANONICAL_SCHEMA_VERSION,
        "canonical_key": canonical_key,
        "legacy_table": "chapter_manga_prompt",
        "legacy_id": row.id,
        "asset_family": "manga",
        "prompt_type": row.prompt_type,
        "page_number": row.page_number,
        "panel_number": row.panel_number,
        "dialogue_text": row.dialogue_text,
        "characters_in_frame": row.characters_in_frame,
        "layout": row.layout,
        "camera_angle": row.camera_angle,
        "dialogue_position": row.dialogue_position,
        "generated_image_id": row.generated_image_id,
    });
    let spec = upsert_generation_spec(
        db,
        family,
        GenerationSpecUpsertInput {
            canonical_key,
            project_id: source.project_id,
            root_project_id: source.root_project_id,
            source_snapshot_id: source.source_snapshot_id,
            source_chapter_id: source.source_chapter_id,
            source_snapshot_chapter_id: source.source_snapshot_chapter_id,
            target_kind: target_kind.to_string(),
            target_entity_kind: "chapter_manga_prompt".to_string(),
            target_entity_id: Some(row.id),
            status: status.to_string(),
            source_version_hash: source.source_version_hash,
            params_json,
            created_at: row.created_at,
            updated_at: row.updated_at,
        },
        summary,
    )
    .await?;
    let artifact = upsert_prompt_artifact(
        db,
        spec.id,
        prompt_kind,
        &row.image_prompt,
        row.created_at,
        row.updated_at,
        summary,
    )
    .await?;
    Ok(Some(SyncedPromptRef {
        generation_spec_id: spec.id,
        prompt_artifact_id: artifact.id,
    }))
}

async fn sync_video_prompt<C>(
    db: &C,
    family: &FamilyLineageContext,
    row: &chapter_video_prompt::Model,
    summary: &mut AssetLineageBackfillSummary,
) -> Result<Option<SyncedPromptRef>, AppError>
where
    C: ConnectionTrait,
{
    let Some(source) = source_context_for_row(
        family,
        row.project_id,
        Some(row.chapter_id),
        "chapter_video_prompt",
        row.id,
        summary,
    ) else {
        return Ok(None);
    };
    let Some((target_kind, prompt_kind)) =
        video_generation_kinds(&row.prompt_type, row.workflow_mode.as_str())
    else {
        push_unsupported_prompt_type_warning(
            summary,
            "chapter_video_prompt",
            row.id,
            &row.prompt_type,
        );
        return Ok(None);
    };
    let Some(status) = canonical_spec_status(&row.status) else {
        push_unsupported_status_warning(summary, "chapter_video_prompt", row.id, &row.status);
        return Ok(None);
    };
    let canonical_key = legacy_key("chapter_video_prompt", row.id, "prompt");
    let params_json = json!({
        "canonical_schema_version": CANONICAL_SCHEMA_VERSION,
        "canonical_key": canonical_key,
        "legacy_table": "chapter_video_prompt",
        "legacy_id": row.id,
        "asset_family": "video",
        "prompt_type": row.prompt_type,
        "workflow_mode": row.workflow_mode,
        "shot_number": row.shot_number,
        "title": row.title,
        "scene_summary": row.scene_summary,
        "visual_prompt": row.visual_prompt,
        "motion_prompt": row.motion_prompt,
        "negative_prompt": row.negative_prompt,
        "duration_secs": row.duration_secs,
        "aspect_ratio": row.aspect_ratio,
        "reference_image_ids": row.reference_image_ids_json,
        "metadata": row.metadata_json,
        "generated_video_id": row.generated_video_id,
    });
    let spec = upsert_generation_spec(
        db,
        family,
        GenerationSpecUpsertInput {
            canonical_key,
            project_id: source.project_id,
            root_project_id: source.root_project_id,
            source_snapshot_id: source.source_snapshot_id,
            source_chapter_id: source.source_chapter_id,
            source_snapshot_chapter_id: source.source_snapshot_chapter_id,
            target_kind: target_kind.to_string(),
            target_entity_kind: "chapter_video_prompt".to_string(),
            target_entity_id: Some(row.id),
            status: status.to_string(),
            source_version_hash: source.source_version_hash,
            params_json,
            created_at: row.created_at,
            updated_at: row.updated_at,
        },
        summary,
    )
    .await?;
    let artifact = upsert_prompt_artifact(
        db,
        spec.id,
        prompt_kind,
        &video_prompt_artifact_text(row),
        row.created_at,
        row.updated_at,
        summary,
    )
    .await?;
    Ok(Some(SyncedPromptRef {
        generation_spec_id: spec.id,
        prompt_artifact_id: artifact.id,
    }))
}

async fn sync_generated_image_media<C>(
    db: &C,
    family: &FamilyLineageContext,
    row: &generated_image::Model,
    prompt: Option<&chapter_manga_prompt::Model>,
    prompt_ref: Option<&SyncedPromptRef>,
    source_media_asset_id: Option<i64>,
    summary: &mut AssetLineageBackfillSummary,
) -> Result<Option<media_asset::Model>, AppError>
where
    C: ConnectionTrait,
{
    let Some(project_context) = family.contexts_by_project_id.get(&row.project_id) else {
        summary.warnings.push(format!(
            "generated_image #{} 所属项目 {} 不在当前项目族中，已跳过。",
            row.id, row.project_id
        ));
        return Ok(None);
    };
    let Some(source) = media_source_context_for_row(
        project_context,
        row.chapter_id,
        "generated_image",
        row.id,
        summary,
    ) else {
        return Ok(None);
    };
    let request_snapshot_json = json!({
        "canonical_schema_version": CANONICAL_SCHEMA_VERSION,
        "canonical_key": legacy_key("generated_image", row.id, "media"),
        "legacy_table": "generated_image",
        "legacy_id": row.id,
        "asset_family": if row.image_type == "video_tail_frame" { "video" } else { "manga" },
        "image_type": row.image_type,
        "prompt": row.prompt,
        "model": row.model,
        "chapter_id": row.chapter_id,
        "panel_id": row.panel_id,
        "metadata": row.metadata_json,
        "linked_chapter_manga_prompt_id": prompt.map(|prompt| prompt.id),
    });
    let media = upsert_media_asset(
        db,
        family,
        MediaUpsertInput {
            canonical_key: legacy_key("generated_image", row.id, "media"),
            project_id: row.project_id,
            root_project_id: project_context.root_project_id,
            source_snapshot_id: source.source_snapshot_id,
            source_chapter_id: source.source_chapter_id,
            source_snapshot_chapter_id: source.source_snapshot_chapter_id,
            media_kind: "image".to_string(),
            origin: if row.image_type == "video_tail_frame" {
                "extracted_frame".to_string()
            } else {
                "generated".to_string()
            },
            file_path: Some(row.file_path.clone()),
            thumbnail_path: None,
            width: Some(row.width),
            height: Some(row.height),
            duration_secs: None,
            fps: None,
            file_size: row.file_size,
            status: "completed".to_string(),
            error_message: None,
            generation_spec_id: prompt_ref.map(|prompt| prompt.generation_spec_id),
            prompt_artifact_id: prompt_ref.map(|prompt| prompt.prompt_artifact_id),
            provider_job_id: None,
            source_media_asset_id,
            source_external_path: None,
            imported_from_user: false,
            request_snapshot_json,
            created_at: row.created_at,
        },
        summary,
    )
    .await?;
    if let Some(prompt) = prompt {
        ensure_media_binding(
            db,
            media.id,
            "chapter_manga_prompt",
            prompt.id,
            "generated_result",
            summary,
        )
        .await?;
    }
    if let Some(chapter_id) = row.chapter_id {
        ensure_media_binding(
            db,
            media.id,
            "chapter",
            chapter_id,
            if row.image_type == "video_tail_frame" {
                "end_frame"
            } else {
                "generated_result"
            },
            summary,
        )
        .await?;
    }
    Ok(Some(media))
}

async fn sync_generated_video_media<C>(
    db: &C,
    family: &FamilyLineageContext,
    row: &generated_video::Model,
    prompt: Option<&chapter_video_prompt::Model>,
    prompt_ref: Option<&SyncedPromptRef>,
    source_media_asset_id: Option<i64>,
    summary: &mut AssetLineageBackfillSummary,
) -> Result<Option<media_asset::Model>, AppError>
where
    C: ConnectionTrait,
{
    let Some(project_context) = family.contexts_by_project_id.get(&row.project_id) else {
        summary.warnings.push(format!(
            "generated_video #{} 所属项目 {} 不在当前项目族中，已跳过。",
            row.id, row.project_id
        ));
        return Ok(None);
    };
    let Some(status) = canonical_media_status(&row.status) else {
        push_unsupported_status_warning(summary, "generated_video", row.id, &row.status);
        return Ok(None);
    };
    let Some(source) = media_source_context_for_row(
        project_context,
        row.chapter_id,
        "generated_video",
        row.id,
        summary,
    ) else {
        return Ok(None);
    };
    let request_snapshot_json = json!({
        "canonical_schema_version": CANONICAL_SCHEMA_VERSION,
        "canonical_key": legacy_key("generated_video", row.id, "media"),
        "legacy_table": "generated_video",
        "legacy_id": row.id,
        "asset_family": "video",
        "video_type": row.video_type,
        "workflow_mode": row.workflow_mode,
        "prompt": row.prompt,
        "negative_prompt": row.negative_prompt,
        "model": row.model,
        "chapter_id": row.chapter_id,
        "source_video_id": row.source_video_id,
        "source_image_id": row.source_image_id,
        "metadata": row.metadata_json,
        "linked_chapter_video_prompt_id": prompt.map(|prompt| prompt.id),
    });
    let media = upsert_media_asset(
        db,
        family,
        MediaUpsertInput {
            canonical_key: legacy_key("generated_video", row.id, "media"),
            project_id: row.project_id,
            root_project_id: project_context.root_project_id,
            source_snapshot_id: source.source_snapshot_id,
            source_chapter_id: source.source_chapter_id,
            source_snapshot_chapter_id: source.source_snapshot_chapter_id,
            media_kind: "video".to_string(),
            origin: if row.video_type == "reference_media" {
                "imported".to_string()
            } else {
                "generated".to_string()
            },
            file_path: row.file_path.clone(),
            thumbnail_path: row.thumbnail_path.clone(),
            width: Some(row.width),
            height: Some(row.height),
            duration_secs: Some(row.duration_secs),
            fps: Some(row.fps as f32),
            file_size: row.file_size,
            status: status.to_string(),
            error_message: row.error_message.clone(),
            generation_spec_id: prompt_ref.map(|prompt| prompt.generation_spec_id),
            prompt_artifact_id: prompt_ref.map(|prompt| prompt.prompt_artifact_id),
            provider_job_id: row.provider_job_id.clone(),
            source_media_asset_id,
            source_external_path: None,
            imported_from_user: row.video_type == "reference_media",
            request_snapshot_json,
            created_at: row.created_at,
        },
        summary,
    )
    .await?;
    if let Some(prompt) = prompt {
        ensure_media_binding(
            db,
            media.id,
            "chapter_video_prompt",
            prompt.id,
            "generated_result",
            summary,
        )
        .await?;
    }
    if let Some(chapter_id) = row.chapter_id {
        ensure_media_binding(
            db,
            media.id,
            "chapter",
            chapter_id,
            "generated_result",
            summary,
        )
        .await?;
    }
    Ok(Some(media))
}

async fn load_family_lineage_context<C>(
    db: &C,
    project_id: i64,
) -> Result<FamilyLineageContext, AppError>
where
    C: ConnectionTrait,
{
    let requested = novel_project::Entity::find_by_id(project_id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
    let root_project_id = requested.root_project_id.unwrap_or(requested.id);
    let mut projects = novel_project::Entity::find()
        .filter(novel_project::Column::RootProjectId.eq(root_project_id))
        .order_by_asc(novel_project::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let root_project = novel_project::Entity::find_by_id(root_project_id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("根文本项目不存在: {root_project_id}")))?;
    projects.push(root_project);
    if !projects.iter().any(|project| project.id == requested.id) {
        projects.push(requested);
    }
    projects.sort_by_key(|project| project.id);
    projects.dedup_by_key(|project| project.id);

    let snapshots = project_source_snapshot::Entity::find()
        .filter(project_source_snapshot::Column::RootProjectId.eq(root_project_id))
        .order_by_desc(project_source_snapshot::Column::CreatedAt)
        .order_by_desc(project_source_snapshot::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let snapshots_by_id = snapshots
        .iter()
        .map(|snapshot| (snapshot.id, snapshot.clone()))
        .collect::<BTreeMap<_, _>>();
    let latest_snapshot = snapshots.first().cloned();
    let snapshot_ids = snapshots
        .iter()
        .map(|snapshot| snapshot.id)
        .collect::<Vec<_>>();
    let snapshot_chapters = if snapshot_ids.is_empty() {
        Vec::new()
    } else {
        source_snapshot_chapter::Entity::find()
            .filter(source_snapshot_chapter::Column::SnapshotId.is_in(snapshot_ids))
            .order_by_asc(source_snapshot_chapter::Column::SnapshotId)
            .order_by_asc(source_snapshot_chapter::Column::SourceChapterId)
            .all(db)
            .await
            .map_err(AppError::Database)?
    };
    let mut snapshot_chapters_by_snapshot_id =
        BTreeMap::<i64, BTreeMap<i64, source_snapshot_chapter::Model>>::new();
    for chapter in snapshot_chapters {
        snapshot_chapters_by_snapshot_id
            .entry(chapter.snapshot_id)
            .or_default()
            .insert(chapter.source_chapter_id, chapter);
    }

    let mut project_ids = Vec::with_capacity(projects.len());
    let mut contexts_by_project_id = BTreeMap::new();
    for project in projects {
        project_ids.push(project.id);
        let selected_snapshot = match project.source_mode.as_str() {
            SOURCE_MODE_FREEFORM => None,
            SOURCE_MODE_SELF if project.project_kind == PROJECT_KIND_TEXT => {
                latest_snapshot.clone()
            }
            _ => project
                .source_snapshot_id
                .and_then(|id| snapshots_by_id.get(&id).cloned()),
        };
        let snapshot_chapters_by_source = selected_snapshot
            .as_ref()
            .and_then(|snapshot| snapshot_chapters_by_snapshot_id.get(&snapshot.id).cloned())
            .unwrap_or_default();
        contexts_by_project_id.insert(
            project.id,
            ProjectLineageContext {
                project_id: project.id,
                root_project_id,
                source_mode: project.source_mode.clone(),
                snapshot: selected_snapshot,
                snapshot_chapters_by_source,
            },
        );
    }
    project_ids.sort_unstable();
    project_ids.dedup();

    Ok(FamilyLineageContext {
        project_ids,
        contexts_by_project_id,
    })
}

fn source_context_for_row(
    family: &FamilyLineageContext,
    project_id: i64,
    source_chapter_id: Option<i64>,
    table: &str,
    row_id: i64,
    summary: &mut AssetLineageBackfillSummary,
) -> Option<SourceLineageContext> {
    let Some(project_context) = family.contexts_by_project_id.get(&project_id) else {
        summary.warnings.push(format!(
            "{table} #{row_id} 所属项目 {project_id} 不在当前项目族中，已跳过。"
        ));
        return None;
    };
    let Some(snapshot) = project_context.snapshot.as_ref() else {
        summary.warnings.push(format!(
            "{table} #{row_id} 所属项目族缺少源文本快照，无法写入 generation_spec，已跳过。"
        ));
        return None;
    };
    let source_snapshot_chapter_id = source_chapter_id.and_then(|chapter_id| {
        project_context
            .snapshot_chapters_by_source
            .get(&chapter_id)
            .map(|chapter| chapter.id)
    });
    if source_chapter_id.is_some() && source_snapshot_chapter_id.is_none() {
        summary.warnings.push(format!(
            "{table} #{row_id} 的章节 {:?} 不在源文本快照 {} 中，无法写入 generation_spec，已跳过。",
            source_chapter_id, snapshot.id
        ));
        return None;
    }
    Some(SourceLineageContext {
        project_id: project_context.project_id,
        root_project_id: project_context.root_project_id,
        source_snapshot_id: snapshot.id,
        source_version_hash: snapshot.source_version_hash.clone(),
        source_chapter_id,
        source_snapshot_chapter_id,
    })
}

fn media_source_context_for_row(
    project_context: &ProjectLineageContext,
    source_chapter_id: Option<i64>,
    table: &str,
    row_id: i64,
    summary: &mut AssetLineageBackfillSummary,
) -> Option<MediaSourceLineageContext> {
    if project_context.source_mode == SOURCE_MODE_FREEFORM {
        if source_chapter_id.is_some() {
            summary.warnings.push(format!(
                "{table} #{row_id} 属于 freeform 项目但绑定了章节 {:?}，无法写入 media_asset，已跳过。",
                source_chapter_id
            ));
            return None;
        }
        return Some(MediaSourceLineageContext {
            source_snapshot_id: None,
            source_chapter_id: None,
            source_snapshot_chapter_id: None,
        });
    }

    let Some(snapshot) = project_context.snapshot.as_ref() else {
        summary.warnings.push(format!(
            "{table} #{row_id} 所属项目族缺少源文本快照，无法写入 media_asset，已跳过。"
        ));
        return None;
    };

    let Some(chapter_id) = source_chapter_id else {
        return Some(MediaSourceLineageContext {
            source_snapshot_id: Some(snapshot.id),
            source_chapter_id: None,
            source_snapshot_chapter_id: None,
        });
    };
    let Some(snapshot_chapter) = project_context.snapshot_chapters_by_source.get(&chapter_id)
    else {
        summary.warnings.push(format!(
            "{table} #{row_id} 的章节 {chapter_id} 不在源文本快照 {} 中，无法写入 media_asset，已跳过。",
            snapshot.id
        ));
        return None;
    };
    Some(MediaSourceLineageContext {
        source_snapshot_id: Some(snapshot.id),
        source_chapter_id: Some(chapter_id),
        source_snapshot_chapter_id: Some(snapshot_chapter.id),
    })
}

fn manga_generation_kinds(prompt_type: &str) -> Option<(&'static str, &'static str)> {
    match prompt_type {
        MANGA_EXTRACTION_SUMMARY_TYPE => {
            Some(("manga_extraction_summary", "manga_pipeline_output"))
        }
        "page_planning" => Some(("manga_page_plan", "manga_pipeline_output")),
        MANGA_PAGE_PROMPT_RAW_TYPE => Some(("manga_page_prompt_batch", "manga_page_prompt")),
        MANGA_PAGE_PROMPT_TYPE => Some(("manga_page", "image_prompt")),
        _ => None,
    }
}

fn video_generation_kinds(
    prompt_type: &str,
    workflow_mode: &str,
) -> Option<(&'static str, &'static str)> {
    match prompt_type {
        "frame_shot_analysis" | "first_frame_shot_analysis" | "chat_video_shot_analysis" => {
            Some(("video_shot_analysis", "video_shot_analysis"))
        }
        "frame_segment_plan" | "first_frame_segment_plan" | "chat_video_segment_plan" => {
            Some(("video_segment_plan", "video_segment_plan"))
        }
        "frame_prompt" | "first_frame_prompt" | "chat_video_frame_prompt" => {
            Some(("video_frame_prompt", "image_prompt"))
        }
        "video_prompt" if workflow_mode == "chat_video" => {
            Some(("video_segment", "chat_video_prompt"))
        }
        "video_prompt" => Some(("video_segment", "video_prompt")),
        _ => None,
    }
}

fn video_prompt_artifact_text(row: &chapter_video_prompt::Model) -> String {
    let mut parts = vec![row.visual_prompt.clone()];
    if let Some(motion) = row
        .motion_prompt
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        parts.push(format!("【运动提示】\n{motion}"));
    }
    if let Some(negative) = row
        .negative_prompt
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        parts.push(format!("【负面提示】\n{negative}"));
    }
    parts.join("\n\n")
}

fn should_sync_generated_image(
    image: &generated_image::Model,
    manga_prompt_by_image: &BTreeMap<i64, chapter_manga_prompt::Model>,
) -> bool {
    manga_prompt_by_image.contains_key(&image.id) || image.image_type == "video_tail_frame"
}

fn prompt_from_video_metadata<'a>(
    video: &generated_video::Model,
    prompts: &'a [chapter_video_prompt::Model],
) -> Option<&'a chapter_video_prompt::Model> {
    let prompt_id = video
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("prompt_id"))
        .and_then(Value::as_i64)?;
    prompts.iter().find(|prompt| prompt.id == prompt_id)
}

fn canonical_spec_status(status: &str) -> Option<&'static str> {
    match status {
        "draft" => Some("draft"),
        "running" | "generating" => Some("running"),
        "completed" => Some("completed"),
        "failed" => Some("failed"),
        "cancelled" => Some("cancelled"),
        "pending" => Some("ready"),
        _ => None,
    }
}

fn canonical_media_status(status: &str) -> Option<&'static str> {
    match status {
        "pending" => Some("pending"),
        "running" | "generating" => Some("running"),
        "completed" => Some("completed"),
        "failed" => Some("failed"),
        "cancelled" => Some("cancelled"),
        _ => None,
    }
}

fn push_unsupported_prompt_type_warning(
    summary: &mut AssetLineageBackfillSummary,
    table: &str,
    row_id: i64,
    prompt_type: &str,
) {
    summary.warnings.push(format!(
        "{table} #{row_id} 的 prompt_type `{prompt_type}` 没有 canonical 映射，已跳过。"
    ));
}

fn push_unsupported_status_warning(
    summary: &mut AssetLineageBackfillSummary,
    table: &str,
    row_id: i64,
    status: &str,
) {
    summary.warnings.push(format!(
        "{table} #{row_id} 的状态 `{status}` 无法映射到 canonical 状态，已跳过。"
    ));
}

fn legacy_key(table: &str, id: i64, kind: &str) -> String {
    format!("legacy:{table}:{id}:{kind}")
}

fn validate_positive_id(value: i64, label: &str) -> Result<(), AppError> {
    if value > 0 {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(format!("{label} 必须为正数")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn snapshot(id: i64) -> project_source_snapshot::Model {
        project_source_snapshot::Model {
            id,
            root_project_id: 1,
            source_version_hash: format!("snapshot-{id}"),
            blueprint_version_id: None,
            chapter_scope_json: json!({ "start": 1, "end": 3 }),
            chapter_hashes_json: json!([]),
            profile_hashes_json: json!([]),
            created_at: chrono::Utc::now().naive_utc(),
        }
    }

    fn snapshot_chapter(
        id: i64,
        snapshot_id: i64,
        source_chapter_id: i64,
    ) -> source_snapshot_chapter::Model {
        source_snapshot_chapter::Model {
            id,
            snapshot_id,
            root_project_id: 1,
            source_chapter_id,
            chapter_number: 1,
            title: Some("第一章".to_string()),
            content: "正文".to_string(),
            summary: Some("摘要".to_string()),
            analysis_data_json: None,
            content_hash: "content-hash".to_string(),
            analysis_hash: None,
        }
    }

    fn family_with_snapshot_chapters(
        snapshot: Option<project_source_snapshot::Model>,
        chapters: BTreeMap<i64, source_snapshot_chapter::Model>,
    ) -> FamilyLineageContext {
        FamilyLineageContext {
            project_ids: vec![1, 2],
            contexts_by_project_id: BTreeMap::from([(
                2,
                ProjectLineageContext {
                    project_id: 2,
                    root_project_id: 1,
                    source_mode: "derived".to_string(),
                    snapshot,
                    snapshot_chapters_by_source: chapters,
                },
            )]),
        }
    }

    #[test]
    fn maps_legacy_manga_prompt_types_to_canonical_kinds() {
        assert_eq!(
            manga_generation_kinds(MANGA_EXTRACTION_SUMMARY_TYPE),
            Some(("manga_extraction_summary", "manga_pipeline_output"))
        );
        assert_eq!(
            manga_generation_kinds("page_planning"),
            Some(("manga_page_plan", "manga_pipeline_output"))
        );
        assert_eq!(
            manga_generation_kinds(MANGA_PAGE_PROMPT_RAW_TYPE),
            Some(("manga_page_prompt_batch", "manga_page_prompt"))
        );
        assert_eq!(
            manga_generation_kinds(MANGA_PAGE_PROMPT_TYPE),
            Some(("manga_page", "image_prompt"))
        );
        assert_eq!(manga_generation_kinds("unknown"), None);
    }

    #[test]
    fn maps_legacy_video_prompt_types_to_canonical_kinds() {
        assert_eq!(
            video_generation_kinds("frame_shot_analysis", "boundary_frames"),
            Some(("video_shot_analysis", "video_shot_analysis"))
        );
        assert_eq!(
            video_generation_kinds("frame_segment_plan", "first_frame_chain"),
            Some(("video_segment_plan", "video_segment_plan"))
        );
        assert_eq!(
            video_generation_kinds("first_frame_prompt", "first_frame_chain"),
            Some(("video_frame_prompt", "image_prompt"))
        );
        assert_eq!(
            video_generation_kinds("video_prompt", "chat_video"),
            Some(("video_segment", "chat_video_prompt"))
        );
        assert_eq!(
            video_generation_kinds("video_prompt", "first_frame_chain"),
            Some(("video_segment", "video_prompt"))
        );
        assert_eq!(
            video_generation_kinds("provider_prompt", "first_frame_chain"),
            None
        );
    }

    #[test]
    fn normalizes_legacy_statuses_for_canonical_chain() {
        assert_eq!(canonical_spec_status("pending"), Some("ready"));
        assert_eq!(canonical_spec_status("generating"), Some("running"));
        assert_eq!(canonical_spec_status("completed"), Some("completed"));
        assert_eq!(canonical_spec_status("queued"), None);
        assert_eq!(canonical_media_status("generating"), Some("running"));
        assert_eq!(canonical_media_status("failed"), Some("failed"));
        assert_eq!(canonical_media_status("queued"), None);
    }

    #[test]
    fn source_context_skips_generation_spec_when_snapshot_chapter_is_missing() {
        let family = family_with_snapshot_chapters(Some(snapshot(10)), BTreeMap::new());
        let mut summary = AssetLineageBackfillSummary::default();

        let context = source_context_for_row(
            &family,
            2,
            Some(99),
            "chapter_video_prompt",
            7,
            &mut summary,
        );

        assert!(context.is_none());
        assert_eq!(summary.warnings.len(), 1);
        assert!(summary.warnings[0].contains("无法写入 generation_spec"));
        assert!(summary.warnings[0].contains("已跳过"));
        assert!(!summary.warnings[0].contains("仍写入"));
    }

    #[test]
    fn source_context_keeps_complete_source_snapshot_chapter_pair() {
        let chapter = snapshot_chapter(20, 10, 99);
        let family = family_with_snapshot_chapters(
            Some(snapshot(10)),
            BTreeMap::from([(chapter.source_chapter_id, chapter)]),
        );
        let mut summary = AssetLineageBackfillSummary::default();

        let context = source_context_for_row(
            &family,
            2,
            Some(99),
            "chapter_video_prompt",
            7,
            &mut summary,
        )
        .expect("complete source chapter pair should be accepted");

        assert_eq!(context.source_chapter_id, Some(99));
        assert_eq!(context.source_snapshot_chapter_id, Some(20));
        assert!(summary.warnings.is_empty());
    }

    #[test]
    fn media_source_context_skips_media_asset_when_snapshot_chapter_is_missing() {
        let family = family_with_snapshot_chapters(Some(snapshot(10)), BTreeMap::new());
        let project_context = family.contexts_by_project_id.get(&2).unwrap();
        let mut summary = AssetLineageBackfillSummary::default();

        let source = media_source_context_for_row(
            project_context,
            Some(99),
            "generated_video",
            8,
            &mut summary,
        );

        assert!(source.is_none());
        assert_eq!(summary.warnings.len(), 1);
        assert!(summary.warnings[0].contains("无法写入 media_asset"));
        assert!(summary.warnings[0].contains("已跳过"));
    }

    #[test]
    fn media_source_context_keeps_snapshot_for_snapshot_backed_media_without_chapter_source() {
        let family = family_with_snapshot_chapters(Some(snapshot(10)), BTreeMap::new());
        let project_context = family.contexts_by_project_id.get(&2).unwrap();
        let mut summary = AssetLineageBackfillSummary::default();

        let source =
            media_source_context_for_row(project_context, None, "generated_video", 8, &mut summary)
                .expect("snapshot-backed media without chapter source should remain importable");

        assert_eq!(source.source_snapshot_id, Some(10));
        assert_eq!(source.source_chapter_id, None);
        assert_eq!(source.source_snapshot_chapter_id, None);
        assert!(summary.warnings.is_empty());
    }

    #[test]
    fn media_source_context_rejects_snapshot_backed_media_without_snapshot() {
        let family = family_with_snapshot_chapters(None, BTreeMap::new());
        let project_context = family.contexts_by_project_id.get(&2).unwrap();
        let mut summary = AssetLineageBackfillSummary::default();

        let source =
            media_source_context_for_row(project_context, None, "generated_video", 8, &mut summary);

        assert!(source.is_none());
        assert_eq!(summary.warnings.len(), 1);
        assert!(summary.warnings[0].contains("缺少源文本快照"));
        assert!(summary.warnings[0].contains("无法写入 media_asset"));
    }

    #[test]
    fn media_source_context_allows_freeform_media_without_snapshot() {
        let mut family = family_with_snapshot_chapters(None, BTreeMap::new());
        family
            .contexts_by_project_id
            .get_mut(&2)
            .unwrap()
            .source_mode = SOURCE_MODE_FREEFORM.to_string();
        let project_context = family.contexts_by_project_id.get(&2).unwrap();
        let mut summary = AssetLineageBackfillSummary::default();

        let source =
            media_source_context_for_row(project_context, None, "generated_video", 8, &mut summary)
                .expect("freeform media should not require a source snapshot");

        assert_eq!(source.source_snapshot_id, None);
        assert_eq!(source.source_chapter_id, None);
        assert_eq!(source.source_snapshot_chapter_id, None);
        assert!(summary.warnings.is_empty());
    }

    #[test]
    fn video_prompt_artifact_text_keeps_motion_and_negative_sections() {
        let now = chrono::Utc::now().naive_utc();
        let row = chapter_video_prompt::Model {
            id: 1,
            project_id: 2,
            chapter_id: 3,
            style: None,
            prompt_type: "video_prompt".to_string(),
            workflow_mode: "first_frame_chain".to_string(),
            shot_number: 1,
            title: None,
            scene_summary: None,
            visual_prompt: "画面主体".to_string(),
            motion_prompt: Some("从左向右走".to_string()),
            negative_prompt: Some("不要跳切".to_string()),
            duration_secs: 6,
            aspect_ratio: Some("16:9".to_string()),
            reference_image_ids_json: None,
            status: "pending".to_string(),
            generated_video_id: None,
            metadata_json: None,
            created_at: now,
            updated_at: now,
        };

        let text = video_prompt_artifact_text(&row);

        assert!(text.contains("画面主体"));
        assert!(text.contains("【运动提示】"));
        assert!(text.contains("从左向右走"));
        assert!(text.contains("【负面提示】"));
        assert!(text.contains("不要跳切"));
    }
}
