use super::{CANVAS_RENDERER_VERSION, FREEFORM_GENERATION_CONTEXT};
use crate::AppState;
use crate::actions::video_generation::GeneratedVideoSummary;
use a4n_core::models::ai::{generated_image, generated_video};
use a4n_core::models::canvas::canvas_node;
use a4n_core::models::generation::{generation_spec, prompt_artifact};
use a4n_core::models::media::{media_asset, media_binding, media_provenance};
use a4n_core::models::project::{novel_project, source_snapshot_chapter};
use a4n_core::models::video_project::{assembly_clip, assembly_timeline, video_segment};
use a4n_services::project::{PROJECT_KIND_VIDEO, SOURCE_MODE_DERIVED, SOURCE_MODE_FREEFORM};
use a4n_services::project_family::load_derived_project_context;
use a4n_services::workflow::WORKFLOW_STATUS_COMPLETED;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
    TransactionTrait,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub(super) struct FreeformCanvasSpecInput<'a> {
    pub(super) project_id: i64,
    pub(super) graph_id: i64,
    pub(super) node_id: i64,
    pub(super) canvas_text_asset_id: Option<i64>,
    pub(super) target_kind: &'a str,
    pub(super) target_entity_kind: &'a str,
    pub(super) status: &'a str,
    pub(super) source_material_json: Value,
    pub(super) params_json: Value,
}

pub(super) async fn upsert_freeform_canvas_generation_spec(
    state: &AppState,
    input: FreeformCanvasSpecInput<'_>,
) -> Result<Option<generation_spec::Model>, String> {
    let project = novel_project::Entity::find_by_id(input.project_id)
        .one(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("项目不存在: {}", input.project_id))?;
    if project.source_mode != SOURCE_MODE_FREEFORM {
        return Ok(None);
    }

    let now = chrono::Utc::now().naive_utc();
    let source_version_hash = sha256_hex_value(&input.source_material_json)?;
    let params_json = json!({
        "canvas_graph_id": input.graph_id,
        "canvas_node_id": input.node_id,
        "source_material": input.source_material_json,
        "params": input.params_json,
    });
    let existing = generation_spec::Entity::find()
        .filter(generation_spec::Column::ProjectId.eq(input.project_id))
        .filter(generation_spec::Column::SourceContextKind.eq(FREEFORM_GENERATION_CONTEXT))
        .filter(generation_spec::Column::TargetEntityKind.eq(input.target_entity_kind))
        .filter(generation_spec::Column::TargetEntityId.eq(input.node_id))
        .order_by_asc(generation_spec::Column::Id)
        .one(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?;

    let spec = if let Some(existing) = existing {
        let mut active: generation_spec::ActiveModel = existing.into();
        active.root_project_id = Set(input.project_id);
        active.source_context_kind = Set(Some(FREEFORM_GENERATION_CONTEXT.to_string()));
        active.source_snapshot_id = Set(None);
        active.source_chapter_id = Set(None);
        active.source_snapshot_chapter_id = Set(None);
        active.canvas_text_asset_id = Set(input.canvas_text_asset_id);
        active.target_kind = Set(input.target_kind.to_string());
        active.target_entity_kind = Set(input.target_entity_kind.to_string());
        active.target_entity_id = Set(Some(input.node_id));
        active.status = Set(input.status.to_string());
        active.source_version_hash = Set(source_version_hash);
        active.params_json = Set(params_json);
        active.updated_at = Set(now);
        active
            .update(state.db.as_ref())
            .await
            .map_err(|err| err.to_string())?
    } else {
        generation_spec::ActiveModel {
            project_id: Set(input.project_id),
            root_project_id: Set(input.project_id),
            source_context_kind: Set(Some(FREEFORM_GENERATION_CONTEXT.to_string())),
            source_snapshot_id: Set(None),
            source_chapter_id: Set(None),
            source_snapshot_chapter_id: Set(None),
            canvas_text_asset_id: Set(input.canvas_text_asset_id),
            target_kind: Set(input.target_kind.to_string()),
            target_entity_kind: Set(input.target_entity_kind.to_string()),
            target_entity_id: Set(Some(input.node_id)),
            status: Set(input.status.to_string()),
            source_version_hash: Set(source_version_hash),
            params_json: Set(params_json),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?
    };
    Ok(Some(spec))
}

pub(super) async fn mark_canvas_generation_spec_status(
    state: &AppState,
    generation_spec_id: i64,
    status: &str,
) -> Result<(), String> {
    generation_spec::ActiveModel {
        id: Set(generation_spec_id),
        status: Set(status.to_string()),
        updated_at: Set(chrono::Utc::now().naive_utc()),
        ..Default::default()
    }
    .update(state.db.as_ref())
    .await
    .map_err(|err| err.to_string())?;
    Ok(())
}

pub(super) async fn upsert_canvas_prompt_artifact(
    state: &AppState,
    generation_spec_id: i64,
    prompt_kind: &str,
    prompt_text: &str,
) -> Result<prompt_artifact::Model, String> {
    let now = chrono::Utc::now().naive_utc();
    let existing = prompt_artifact::Entity::find()
        .filter(prompt_artifact::Column::GenerationSpecId.eq(generation_spec_id))
        .order_by_asc(prompt_artifact::Column::Id)
        .one(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?;
    if let Some(existing) = existing {
        if existing.edited_by_user {
            return Ok(existing);
        }
        let mut active: prompt_artifact::ActiveModel = existing.into();
        active.prompt_kind = Set(prompt_kind.to_string());
        active.prompt_text = Set(prompt_text.to_string());
        active.renderer_version = Set(CANVAS_RENDERER_VERSION.to_string());
        active.edited_by_user = Set(false);
        active.updated_at = Set(now);
        return active
            .update(state.db.as_ref())
            .await
            .map_err(|err| err.to_string());
    }
    prompt_artifact::ActiveModel {
        generation_spec_id: Set(generation_spec_id),
        prompt_kind: Set(prompt_kind.to_string()),
        prompt_text: Set(prompt_text.to_string()),
        renderer_version: Set(CANVAS_RENDERER_VERSION.to_string()),
        edited_by_user: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(state.db.as_ref())
    .await
    .map_err(|err| err.to_string())
}

fn sha256_hex_value(value: &Value) -> Result<String, String> {
    let bytes = serde_json::to_vec(value).map_err(|err| err.to_string())?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

pub(super) struct CanvasAssemblyContract {
    pub(super) preview_media_asset_id: i64,
}

#[derive(Debug, Clone)]
struct CanvasMediaSourceContext {
    root_project_id: i64,
    source_snapshot_id: Option<i64>,
    source_chapter_id: Option<i64>,
    source_snapshot_chapter_id: Option<i64>,
}

async fn load_canvas_media_source_context<C>(
    db: &C,
    project_id: i64,
    source_chapter_id: Option<i64>,
) -> Result<CanvasMediaSourceContext, String>
where
    C: ConnectionTrait,
{
    let project = novel_project::Entity::find_by_id(project_id)
        .one(db)
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("项目不存在: {project_id}"))?;
    if project.project_kind != PROJECT_KIND_VIDEO {
        return Err(format!("画布媒体资产只支持视频项目: {project_id}"));
    }
    match project.source_mode.as_str() {
        SOURCE_MODE_FREEFORM => {
            if let Some(chapter_id) = source_chapter_id {
                return Err(format!(
                    "freeform 视频项目媒体资产不能绑定源章节: project_id={project_id}, chapter_id={chapter_id}"
                ));
            }
            Ok(CanvasMediaSourceContext {
                root_project_id: project.id,
                source_snapshot_id: None,
                source_chapter_id: None,
                source_snapshot_chapter_id: None,
            })
        }
        SOURCE_MODE_DERIVED => {
            let context = load_derived_project_context(db, project_id)
                .await
                .map_err(|err| err.to_string())?;
            let root_project_id = context.root_project.id;
            let source_snapshot_id = context.source_snapshot.id;
            let source_snapshot_chapter_id = if let Some(chapter_id) = source_chapter_id {
                Some(
                    source_snapshot_chapter::Entity::find()
                        .filter(source_snapshot_chapter::Column::SnapshotId.eq(source_snapshot_id))
                        .filter(source_snapshot_chapter::Column::RootProjectId.eq(root_project_id))
                        .filter(source_snapshot_chapter::Column::SourceChapterId.eq(chapter_id))
                        .one(db)
                        .await
                        .map_err(|err| err.to_string())?
                        .ok_or_else(|| {
                            format!("派生视频项目 {project_id} 的源快照不包含章节 {chapter_id}")
                        })?
                        .id,
                )
            } else {
                None
            };
            Ok(CanvasMediaSourceContext {
                root_project_id,
                source_snapshot_id: Some(source_snapshot_id),
                source_chapter_id,
                source_snapshot_chapter_id,
            })
        }
        other => Err(format!(
            "画布媒体资产只支持 source_mode=freeform/derived 的视频项目，当前 source_mode={other}"
        )),
    }
}

pub(super) async fn sync_canvas_assembly_contract(
    state: &AppState,
    project_id: i64,
    graph_id: i64,
    node_id: i64,
    assembly_video: &GeneratedVideoSummary,
    source_video_ids: &[i64],
    generation_spec_id: Option<i64>,
) -> Result<CanvasAssemblyContract, String> {
    let tx = state.db.begin().await.map_err(|err| err.to_string())?;
    let mut source_assets = Vec::with_capacity(source_video_ids.len());
    for video_id in source_video_ids {
        let video = generated_video::Entity::find_by_id(*video_id)
            .one(&tx)
            .await
            .map_err(|err| err.to_string())?
            .ok_or_else(|| format!("成片源视频不存在: {video_id}"))?;
        if video.project_id != project_id {
            return Err(format!("成片源视频不属于当前画布项目: {video_id}"));
        }
        source_assets.push(
            ensure_canvas_video_media_asset(
                &tx,
                project_id,
                &video,
                None,
                canvas_video_media_origin(&video),
            )
            .await?,
        );
    }

    let assembly_row = generated_video::Entity::find_by_id(assembly_video.id)
        .one(&tx)
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("画布成片视频不存在: {}", assembly_video.id))?;
    let preview_asset = ensure_canvas_video_media_asset(
        &tx,
        project_id,
        &assembly_row,
        generation_spec_id,
        "assembled_preview",
    )
    .await?;
    ensure_media_binding(
        &tx,
        preview_asset.id,
        "generated_video",
        assembly_row.id,
        "generated_result",
    )
    .await?;

    let timeline =
        upsert_canvas_assembly_timeline(&tx, project_id, graph_id, node_id, preview_asset.id)
            .await?;
    let stale_segment_ids = load_timeline_source_segment_ids(&tx, timeline.id).await?;
    assembly_clip::Entity::delete_many()
        .filter(assembly_clip::Column::TimelineId.eq(timeline.id))
        .exec(&tx)
        .await
        .map_err(|err| err.to_string())?;
    delete_canvas_segments(&tx, &stale_segment_ids).await?;
    let mut previous_segment_id = None;
    for (index, asset) in source_assets.iter().enumerate() {
        let order_index = i32::try_from(index + 1).unwrap_or(i32::MAX);
        let segment = video_segment::ActiveModel {
            project_id: Set(project_id),
            root_project_id: Set(asset.root_project_id),
            source_chapter_id: Set(asset.source_chapter_id),
            source_snapshot_chapter_id: Set(asset.source_snapshot_chapter_id),
            segment_type: Set("canvas_clip".to_string()),
            order_index: Set(order_index),
            title: Set(format!("画布片段 {}", index + 1)),
            duration_secs: Set(asset.duration_secs.unwrap_or(0.0).max(0.0)),
            source_generation_spec_id: Set(None),
            result_media_asset_id: Set(Some(asset.id)),
            previous_segment_id: Set(previous_segment_id),
            next_segment_id: Set(None),
            created_at: Set(chrono::Utc::now().naive_utc()),
            updated_at: Set(chrono::Utc::now().naive_utc()),
            ..Default::default()
        }
        .insert(&tx)
        .await
        .map_err(|err| err.to_string())?;
        if let Some(previous_id) = previous_segment_id {
            video_segment::ActiveModel {
                id: Set(previous_id),
                next_segment_id: Set(Some(segment.id)),
                updated_at: Set(chrono::Utc::now().naive_utc()),
                ..Default::default()
            }
            .update(&tx)
            .await
            .map_err(|err| err.to_string())?;
        }
        ensure_media_binding(
            &tx,
            asset.id,
            "video_segment",
            segment.id,
            "generated_result",
        )
        .await?;
        assembly_clip::ActiveModel {
            timeline_id: Set(timeline.id),
            media_asset_id: Set(asset.id),
            order_index: Set(order_index),
            trim_start_secs: Set(0.0),
            trim_end_secs: Set(asset.duration_secs.unwrap_or(0.0).max(0.0)),
            volume: Set(1.0),
            transition_kind: Set("cut".to_string()),
            source_segment_id: Set(Some(segment.id)),
            ..Default::default()
        }
        .insert(&tx)
        .await
        .map_err(|err| err.to_string())?;
        previous_segment_id = Some(segment.id);
    }
    ensure_media_binding(
        &tx,
        preview_asset.id,
        "assembly_timeline",
        timeline.id,
        "generated_result",
    )
    .await?;
    tx.commit().await.map_err(|err| err.to_string())?;

    Ok(CanvasAssemblyContract {
        preview_media_asset_id: preview_asset.id,
    })
}

async fn load_timeline_source_segment_ids<C>(db: &C, timeline_id: i64) -> Result<Vec<i64>, String>
where
    C: ConnectionTrait,
{
    let clips = assembly_clip::Entity::find()
        .filter(assembly_clip::Column::TimelineId.eq(timeline_id))
        .all(db)
        .await
        .map_err(|err| err.to_string())?;
    Ok(clips
        .into_iter()
        .filter_map(|clip| clip.source_segment_id)
        .collect())
}

async fn delete_canvas_segments<C>(db: &C, segment_ids: &[i64]) -> Result<(), String>
where
    C: ConnectionTrait,
{
    if segment_ids.is_empty() {
        return Ok(());
    }
    media_binding::Entity::delete_many()
        .filter(media_binding::Column::TargetKind.eq("video_segment"))
        .filter(media_binding::Column::TargetId.is_in(segment_ids.to_vec()))
        .exec(db)
        .await
        .map_err(|err| err.to_string())?;
    video_segment::Entity::delete_many()
        .filter(video_segment::Column::Id.is_in(segment_ids.to_vec()))
        .exec(db)
        .await
        .map_err(|err| err.to_string())?;
    Ok(())
}

pub(super) async fn ensure_canvas_image_media_asset<C>(
    db: &C,
    project_id: i64,
    image: &generated_image::Model,
    generation_spec_id: Option<i64>,
    origin: &str,
) -> Result<media_asset::Model, String>
where
    C: ConnectionTrait,
{
    if image.project_id != project_id {
        return Err(format!("图片不属于当前项目: {}", image.id));
    }
    let source_context = load_canvas_media_source_context(db, project_id, image.chapter_id).await?;
    let file_path = image.file_path.trim();
    if file_path.is_empty() {
        return Err(format!(
            "图片缺少文件路径，不能登记为媒体资产: {}",
            image.id
        ));
    }
    let now = chrono::Utc::now().naive_utc();
    let existing = media_asset::Entity::find()
        .filter(media_asset::Column::ProjectId.eq(project_id))
        .filter(media_asset::Column::FilePath.eq(file_path.to_string()))
        .order_by_asc(media_asset::Column::Id)
        .one(db)
        .await
        .map_err(|err| err.to_string())?;
    let asset = if let Some(existing) = existing {
        let mut active: media_asset::ActiveModel = existing.into();
        active.root_project_id = Set(source_context.root_project_id);
        active.source_snapshot_id = Set(source_context.source_snapshot_id);
        active.source_chapter_id = Set(source_context.source_chapter_id);
        active.source_snapshot_chapter_id = Set(source_context.source_snapshot_chapter_id);
        active.media_kind = Set("image".to_string());
        active.origin = Set(origin.to_string());
        active.file_path = Set(Some(file_path.to_string()));
        active.thumbnail_path = Set(None);
        active.width = Set(Some(image.width));
        active.height = Set(Some(image.height));
        active.duration_secs = Set(None);
        active.fps = Set(None);
        active.file_size = Set(image.file_size);
        active.status = Set(WORKFLOW_STATUS_COMPLETED.to_string());
        active.error_message = Set(None);
        active.updated_at = Set(now);
        active.update(db).await.map_err(|err| err.to_string())?
    } else {
        media_asset::ActiveModel {
            project_id: Set(project_id),
            root_project_id: Set(source_context.root_project_id),
            source_snapshot_id: Set(source_context.source_snapshot_id),
            source_chapter_id: Set(source_context.source_chapter_id),
            source_snapshot_chapter_id: Set(source_context.source_snapshot_chapter_id),
            media_kind: Set("image".to_string()),
            origin: Set(origin.to_string()),
            file_path: Set(Some(file_path.to_string())),
            thumbnail_path: Set(None),
            width: Set(Some(image.width)),
            height: Set(Some(image.height)),
            duration_secs: Set(None),
            fps: Set(None),
            file_size: Set(image.file_size),
            checksum: Set(None),
            status: Set(WORKFLOW_STATUS_COMPLETED.to_string()),
            error_message: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|err| err.to_string())?
    };

    upsert_canvas_image_media_provenance(db, &asset, image, generation_spec_id, now).await?;
    ensure_media_binding(
        db,
        asset.id,
        "generated_image",
        image.id,
        "generated_result",
    )
    .await?;
    Ok(asset)
}

pub(super) async fn ensure_canvas_video_media_asset<C>(
    db: &C,
    project_id: i64,
    video: &generated_video::Model,
    generation_spec_id: Option<i64>,
    origin: &str,
) -> Result<media_asset::Model, String>
where
    C: ConnectionTrait,
{
    if video.project_id != project_id {
        return Err(format!("视频不属于当前项目: {}", video.id));
    }
    if video.status != WORKFLOW_STATUS_COMPLETED {
        return Err(format!("视频尚未完成，不能登记为媒体资产: {}", video.id));
    }
    let source_context = load_canvas_media_source_context(db, project_id, video.chapter_id).await?;
    let file_path = video
        .file_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("视频缺少文件路径，不能登记为媒体资产: {}", video.id))?
        .to_string();
    let now = chrono::Utc::now().naive_utc();
    let existing = media_asset::Entity::find()
        .filter(media_asset::Column::ProjectId.eq(project_id))
        .filter(media_asset::Column::FilePath.eq(file_path.clone()))
        .order_by_asc(media_asset::Column::Id)
        .one(db)
        .await
        .map_err(|err| err.to_string())?;
    let asset = if let Some(existing) = existing {
        let mut active: media_asset::ActiveModel = existing.into();
        active.root_project_id = Set(source_context.root_project_id);
        active.source_snapshot_id = Set(source_context.source_snapshot_id);
        active.source_chapter_id = Set(source_context.source_chapter_id);
        active.source_snapshot_chapter_id = Set(source_context.source_snapshot_chapter_id);
        active.media_kind = Set("video".to_string());
        active.origin = Set(origin.to_string());
        active.file_path = Set(Some(file_path.clone()));
        active.thumbnail_path = Set(video.thumbnail_path.clone());
        active.width = Set(Some(video.width));
        active.height = Set(Some(video.height));
        active.duration_secs = Set(Some(video.duration_secs));
        active.fps = Set(Some(video.fps as f32));
        active.file_size = Set(video.file_size);
        active.status = Set(WORKFLOW_STATUS_COMPLETED.to_string());
        active.error_message = Set(None);
        active.updated_at = Set(now);
        active.update(db).await.map_err(|err| err.to_string())?
    } else {
        media_asset::ActiveModel {
            project_id: Set(project_id),
            root_project_id: Set(source_context.root_project_id),
            source_snapshot_id: Set(source_context.source_snapshot_id),
            source_chapter_id: Set(source_context.source_chapter_id),
            source_snapshot_chapter_id: Set(source_context.source_snapshot_chapter_id),
            media_kind: Set("video".to_string()),
            origin: Set(origin.to_string()),
            file_path: Set(Some(file_path.clone())),
            thumbnail_path: Set(video.thumbnail_path.clone()),
            width: Set(Some(video.width)),
            height: Set(Some(video.height)),
            duration_secs: Set(Some(video.duration_secs)),
            fps: Set(Some(video.fps as f32)),
            file_size: Set(video.file_size),
            checksum: Set(None),
            status: Set(WORKFLOW_STATUS_COMPLETED.to_string()),
            error_message: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|err| err.to_string())?
    };

    upsert_canvas_media_provenance(db, &asset, video, generation_spec_id, now).await?;
    ensure_media_binding(
        db,
        asset.id,
        "generated_video",
        video.id,
        "generated_result",
    )
    .await?;
    Ok(asset)
}

async fn upsert_canvas_image_media_provenance<C>(
    db: &C,
    asset: &media_asset::Model,
    image: &generated_image::Model,
    generation_spec_id: Option<i64>,
    now: chrono::NaiveDateTime,
) -> Result<(), String>
where
    C: ConnectionTrait,
{
    let request_snapshot_json = Some(json!({
        "source": "video_canvas",
        "generated_image_id": image.id,
        "image_type": image.image_type,
        "metadata": image.metadata_json,
    }));
    let existing = media_provenance::Entity::find_by_id(asset.id)
        .one(db)
        .await
        .map_err(|err| err.to_string())?;
    let model = media_provenance::ActiveModel {
        media_asset_id: Set(asset.id),
        generation_spec_id: Set(generation_spec_id),
        prompt_artifact_id: Set(None),
        provider_job_id: Set(None),
        source_media_asset_id: Set(None),
        source_external_path: Set(Some(format!("generated_image:{}", image.id))),
        imported_from_user: Set(image.model == "external-import"),
        request_snapshot_json: Set(request_snapshot_json),
        created_at: Set(now),
    };
    if existing.is_some() {
        model.update(db).await.map_err(|err| err.to_string())?;
    } else {
        model.insert(db).await.map_err(|err| err.to_string())?;
    }
    Ok(())
}

async fn upsert_canvas_media_provenance<C>(
    db: &C,
    asset: &media_asset::Model,
    video: &generated_video::Model,
    generation_spec_id: Option<i64>,
    now: chrono::NaiveDateTime,
) -> Result<(), String>
where
    C: ConnectionTrait,
{
    let request_snapshot_json = Some(json!({
        "source": "video_canvas",
        "generated_video_id": video.id,
        "video_type": video.video_type,
        "workflow_mode": video.workflow_mode,
        "metadata": video.metadata_json,
    }));
    let existing = media_provenance::Entity::find_by_id(asset.id)
        .one(db)
        .await
        .map_err(|err| err.to_string())?;
    if existing.is_some() {
        media_provenance::ActiveModel {
            media_asset_id: Set(asset.id),
            generation_spec_id: Set(generation_spec_id),
            prompt_artifact_id: Set(None),
            provider_job_id: Set(video.provider_job_id.clone()),
            source_media_asset_id: Set(None),
            source_external_path: Set(Some(format!("generated_video:{}", video.id))),
            imported_from_user: Set(video.model == "external-import"),
            request_snapshot_json: Set(request_snapshot_json),
            created_at: Set(now),
        }
        .update(db)
        .await
        .map_err(|err| err.to_string())?;
    } else {
        media_provenance::ActiveModel {
            media_asset_id: Set(asset.id),
            generation_spec_id: Set(generation_spec_id),
            prompt_artifact_id: Set(None),
            provider_job_id: Set(video.provider_job_id.clone()),
            source_media_asset_id: Set(None),
            source_external_path: Set(Some(format!("generated_video:{}", video.id))),
            imported_from_user: Set(video.model == "external-import"),
            request_snapshot_json: Set(request_snapshot_json),
            created_at: Set(now),
        }
        .insert(db)
        .await
        .map_err(|err| err.to_string())?;
    }
    Ok(())
}

async fn ensure_media_binding<C>(
    db: &C,
    media_asset_id: i64,
    target_kind: &str,
    target_id: i64,
    role: &str,
) -> Result<(), String>
where
    C: ConnectionTrait,
{
    let exists = media_binding::Entity::find()
        .filter(media_binding::Column::MediaAssetId.eq(media_asset_id))
        .filter(media_binding::Column::TargetKind.eq(target_kind))
        .filter(media_binding::Column::TargetId.eq(target_id))
        .filter(media_binding::Column::Role.eq(role))
        .one(db)
        .await
        .map_err(|err| err.to_string())?
        .is_some();
    if exists {
        return Ok(());
    }
    media_binding::ActiveModel {
        media_asset_id: Set(media_asset_id),
        target_kind: Set(target_kind.to_string()),
        target_id: Set(target_id),
        role: Set(role.to_string()),
        created_at: Set(chrono::Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(|err| err.to_string())?;
    Ok(())
}

async fn upsert_canvas_assembly_timeline<C>(
    db: &C,
    project_id: i64,
    graph_id: i64,
    node_id: i64,
    preview_media_asset_id: i64,
) -> Result<assembly_timeline::Model, String>
where
    C: ConnectionTrait,
{
    let now = chrono::Utc::now().naive_utc();
    let cache_key = format!("canvas_graph:{graph_id}:node:{node_id}");
    let existing = assembly_timeline::Entity::find()
        .filter(assembly_timeline::Column::ProjectId.eq(project_id))
        .filter(assembly_timeline::Column::PreviewCacheKey.eq(cache_key.clone()))
        .order_by_asc(assembly_timeline::Column::Id)
        .one(db)
        .await
        .map_err(|err| err.to_string())?;
    if let Some(existing) = existing {
        let next_version = existing.version.saturating_add(1);
        let mut active: assembly_timeline::ActiveModel = existing.into();
        active.source_chapter_id = Set(None);
        active.source_snapshot_chapter_id = Set(None);
        active.version = Set(next_version);
        active.status = Set(WORKFLOW_STATUS_COMPLETED.to_string());
        active.preview_media_asset_id = Set(Some(preview_media_asset_id));
        active.preview_cache_key = Set(Some(cache_key));
        active.updated_at = Set(now);
        return active.update(db).await.map_err(|err| err.to_string());
    }
    assembly_timeline::ActiveModel {
        project_id: Set(project_id),
        source_chapter_id: Set(None),
        source_snapshot_chapter_id: Set(None),
        version: Set(1),
        status: Set(WORKFLOW_STATUS_COMPLETED.to_string()),
        preview_media_asset_id: Set(Some(preview_media_asset_id)),
        preview_cache_key: Set(Some(cache_key)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(|err| err.to_string())
}

fn canvas_video_media_origin(video: &generated_video::Model) -> &str {
    if video.video_type == "canvas_assembly" {
        "assembled_preview"
    } else if video.model == "external-import" {
        "imported"
    } else {
        "generated"
    }
}

pub(super) async fn update_canvas_node_artifact(
    state: &AppState,
    node_id: i64,
    ref_type: &str,
    ref_id: i64,
    config_patch: Value,
) -> Result<(), String> {
    let node = canvas_node::Entity::find_by_id(node_id)
        .one(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("画布节点不存在: {node_id}"))?;
    let mut config_json = node.config_json;
    if let (Value::Object(base), Value::Object(patch)) = (&mut config_json, config_patch) {
        for (key, value) in patch {
            if value.is_null() {
                base.remove(&key);
            } else {
                base.insert(key, value);
            }
        }
    }
    canvas_node::ActiveModel {
        id: Set(node_id),
        ref_type: Set(Some(ref_type.to_string())),
        ref_id: Set(Some(ref_id)),
        config_json: Set(config_json),
        state: Set(WORKFLOW_STATUS_COMPLETED.to_string()),
        updated_at: Set(chrono::Utc::now().naive_utc()),
        ..Default::default()
    }
    .update(state.db.as_ref())
    .await
    .map_err(|err| err.to_string())?;
    Ok(())
}

pub(super) async fn update_canvas_node_observation(
    state: &AppState,
    node_id: i64,
    config_patch: Value,
) -> Result<(), String> {
    let node = canvas_node::Entity::find_by_id(node_id)
        .one(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("画布节点不存在: {node_id}"))?;
    let mut config_json = node.config_json;
    if let (Value::Object(base), Value::Object(patch)) = (&mut config_json, config_patch) {
        for (key, value) in patch {
            if !value.is_null() {
                base.insert(key, value);
            }
        }
    }
    canvas_node::ActiveModel {
        id: Set(node_id),
        config_json: Set(config_json),
        state: Set(WORKFLOW_STATUS_COMPLETED.to_string()),
        updated_at: Set(chrono::Utc::now().naive_utc()),
        ..Default::default()
    }
    .update(state.db.as_ref())
    .await
    .map_err(|err| err.to_string())?;
    Ok(())
}
