use crate::file_utils::{canonicalize_data_file_path, path_to_db_string};
use crate::state::AppState;
use a4n_core::models::{
    ai::{generated_image, generated_video},
    canvas::{canvas_edge, canvas_graph, canvas_node, canvas_text_asset},
    generation::{generation_spec, prompt_artifact},
    media::{media_asset, media_binding, media_provenance},
    project::novel_project,
    video_project::{assembly_clip, assembly_timeline, video_segment},
};
use a4n_services::project::{PROJECT_KIND_VIDEO, SOURCE_MODE_FREEFORM, SOURCE_SYNC_CURRENT};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
    TransactionTrait,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

const VIDEO_STUDIO_PACKAGE_VERSION: &str = "video_studio_project_v1";
const MANIFEST_PATH: &str = "a4n-video-studio-project.json";
const PACKAGE_MODE: &str = "video_studio_project";
const MEDIA_OWNER_IMAGE: &str = "generated_image";
const MEDIA_OWNER_VIDEO: &str = "generated_video";
const MEDIA_OWNER_MEDIA_ASSET: &str = "media_asset";
const MEDIA_FIELD_FILE: &str = "file_path";
const MEDIA_FIELD_THUMBNAIL: &str = "thumbnail_path";

mod package_io;
mod validation;

use package_io::{package_zip_bytes, read_video_studio_package_reader};
use validation::{validate_freeform_video_project, validate_package_manifest};

#[derive(Debug, Serialize, Deserialize)]
struct VideoStudioProjectPackage {
    version: String,
    exported_at: String,
    mode: String,
    project: novel_project::Model,
    canvas_graphs: Vec<canvas_graph::Model>,
    canvas_nodes: Vec<canvas_node::Model>,
    canvas_edges: Vec<canvas_edge::Model>,
    canvas_text_assets: Vec<canvas_text_asset::Model>,
    generation_specs: Vec<generation_spec::Model>,
    prompt_artifacts: Vec<prompt_artifact::Model>,
    generated_images: Vec<generated_image::Model>,
    generated_videos: Vec<generated_video::Model>,
    media_assets: Vec<media_asset::Model>,
    media_provenance: Vec<media_provenance::Model>,
    media_bindings: Vec<media_binding::Model>,
    video_segments: Vec<video_segment::Model>,
    assembly_timelines: Vec<assembly_timeline::Model>,
    assembly_clips: Vec<assembly_clip::Model>,
    media_files: Vec<VideoStudioMediaFileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VideoStudioMediaFileEntry {
    owner_kind: String,
    owner_id: i64,
    field: String,
    original_path: String,
    package_path: String,
}

#[derive(Debug)]
struct ImportMediaPayload {
    entries: HashMap<(String, i64, String), VideoStudioMediaFileEntry>,
    bytes: HashMap<String, Vec<u8>>,
}

struct DeferredImportGenerationSpecTarget {
    new_spec_id: i64,
    target_entity_kind: String,
    old_target_entity_id: i64,
}

struct InitialGenerationSpecTarget {
    target_entity_id: Option<i64>,
    deferred: Option<DeferredGenerationSpecTarget>,
}

struct DeferredGenerationSpecTarget {
    target_entity_kind: String,
    old_target_entity_id: i64,
}

#[derive(Default)]
struct ImportIdMaps {
    image_ids: HashMap<i64, i64>,
    image_file_paths: HashMap<i64, String>,
    video_ids: HashMap<i64, i64>,
    video_file_paths: HashMap<i64, Option<String>>,
    video_thumbnail_paths: HashMap<i64, Option<String>>,
    text_asset_ids: HashMap<i64, i64>,
    graph_ids: HashMap<i64, i64>,
    node_ids: HashMap<i64, i64>,
    generation_spec_ids: HashMap<i64, i64>,
    prompt_artifact_ids: HashMap<i64, i64>,
    media_asset_ids: HashMap<i64, i64>,
    media_binding_ids: HashMap<i64, i64>,
    video_segment_ids: HashMap<i64, i64>,
    assembly_timeline_ids: HashMap<i64, i64>,
    assembly_clip_ids: HashMap<i64, i64>,
}

pub(super) async fn export_video_studio_project_package_bytes(
    state: &AppState,
    project_id: i64,
) -> Result<Vec<u8>, String> {
    let project = novel_project::Entity::find_by_id(project_id)
        .one(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("视频画布项目不存在: {project_id}"))?;
    validate_freeform_video_project(&project, "视频画布项目导出")?;

    let package = build_export_package(state, project).await?;
    package_zip_bytes(state, &package)
}

pub(super) async fn import_video_studio_project_package_bytes(
    state: &AppState,
    user_id: i64,
    bytes: Vec<u8>,
) -> Result<novel_project::Model, String> {
    if bytes.is_empty() {
        return Err("视频画布项目包不能为空".to_string());
    }

    let (package, media_payload) = read_video_studio_package_reader(bytes)?;
    validate_package_manifest(&package, &media_payload)?;

    let txn = state.db.begin().await.map_err(|err| err.to_string())?;
    let mut written_files = Vec::new();
    let import_result = import_package_rows(
        state,
        user_id,
        &txn,
        package,
        media_payload,
        &mut written_files,
    )
    .await;
    let project = match import_result {
        Ok(project) => project,
        Err(err) => {
            cleanup_written_files(&written_files);
            return Err(err);
        }
    };
    if let Err(err) = txn.commit().await {
        cleanup_written_files(&written_files);
        return Err(format!("提交视频画布项目导入事务失败: {err}"));
    }

    Ok(project)
}

async fn build_export_package(
    state: &AppState,
    project: novel_project::Model,
) -> Result<VideoStudioProjectPackage, String> {
    let project_id = project.id;
    let canvas_graphs = canvas_graph::Entity::find()
        .filter(canvas_graph::Column::ProjectId.eq(project_id))
        .order_by_asc(canvas_graph::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?;
    let graph_ids = canvas_graphs
        .iter()
        .map(|graph| graph.id)
        .collect::<Vec<_>>();
    let canvas_nodes = if graph_ids.is_empty() {
        Vec::new()
    } else {
        canvas_node::Entity::find()
            .filter(canvas_node::Column::GraphId.is_in(graph_ids.clone()))
            .order_by_asc(canvas_node::Column::Id)
            .all(state.db.as_ref())
            .await
            .map_err(|err| err.to_string())?
    };
    let canvas_edges = if graph_ids.is_empty() {
        Vec::new()
    } else {
        canvas_edge::Entity::find()
            .filter(canvas_edge::Column::GraphId.is_in(graph_ids))
            .order_by_asc(canvas_edge::Column::Id)
            .all(state.db.as_ref())
            .await
            .map_err(|err| err.to_string())?
    };
    let canvas_text_assets = canvas_text_asset::Entity::find()
        .filter(canvas_text_asset::Column::ProjectId.eq(project_id))
        .order_by_asc(canvas_text_asset::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?;
    let generation_specs = generation_spec::Entity::find()
        .filter(generation_spec::Column::ProjectId.eq(project_id))
        .order_by_asc(generation_spec::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?;
    let generation_spec_ids = generation_specs
        .iter()
        .map(|spec| spec.id)
        .collect::<Vec<_>>();
    let prompt_artifacts = if generation_spec_ids.is_empty() {
        Vec::new()
    } else {
        prompt_artifact::Entity::find()
            .filter(prompt_artifact::Column::GenerationSpecId.is_in(generation_spec_ids))
            .order_by_asc(prompt_artifact::Column::Id)
            .all(state.db.as_ref())
            .await
            .map_err(|err| err.to_string())?
    };
    let generated_images = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.eq(project_id))
        .order_by_asc(generated_image::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?;
    let generated_videos = generated_video::Entity::find()
        .filter(generated_video::Column::ProjectId.eq(project_id))
        .order_by_asc(generated_video::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?;
    let media_assets = media_asset::Entity::find()
        .filter(media_asset::Column::ProjectId.eq(project_id))
        .order_by_asc(media_asset::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?;
    let media_asset_ids = media_assets
        .iter()
        .map(|asset| asset.id)
        .collect::<Vec<_>>();
    let media_provenance = if media_asset_ids.is_empty() {
        Vec::new()
    } else {
        media_provenance::Entity::find()
            .filter(media_provenance::Column::MediaAssetId.is_in(media_asset_ids.clone()))
            .all(state.db.as_ref())
            .await
            .map_err(|err| err.to_string())?
    };
    let media_bindings = if media_asset_ids.is_empty() {
        Vec::new()
    } else {
        media_binding::Entity::find()
            .filter(media_binding::Column::MediaAssetId.is_in(media_asset_ids.clone()))
            .order_by_asc(media_binding::Column::Id)
            .all(state.db.as_ref())
            .await
            .map_err(|err| err.to_string())?
    };
    let assembly_timelines = assembly_timeline::Entity::find()
        .filter(assembly_timeline::Column::ProjectId.eq(project_id))
        .order_by_asc(assembly_timeline::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?;
    let video_segments = video_segment::Entity::find()
        .filter(video_segment::Column::ProjectId.eq(project_id))
        .order_by_asc(video_segment::Column::OrderIndex)
        .order_by_asc(video_segment::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?;
    let timeline_ids = assembly_timelines
        .iter()
        .map(|timeline| timeline.id)
        .collect::<Vec<_>>();
    let assembly_clips = if timeline_ids.is_empty() {
        Vec::new()
    } else {
        assembly_clip::Entity::find()
            .filter(assembly_clip::Column::TimelineId.is_in(timeline_ids))
            .order_by_asc(assembly_clip::Column::TimelineId)
            .order_by_asc(assembly_clip::Column::OrderIndex)
            .order_by_asc(assembly_clip::Column::Id)
            .all(state.db.as_ref())
            .await
            .map_err(|err| err.to_string())?
    };
    let media_files =
        collect_media_files(state, &generated_images, &generated_videos, &media_assets)?;

    Ok(VideoStudioProjectPackage {
        version: VIDEO_STUDIO_PACKAGE_VERSION.to_string(),
        exported_at: Utc::now().naive_utc().to_string(),
        mode: PACKAGE_MODE.to_string(),
        project,
        canvas_graphs,
        canvas_nodes,
        canvas_edges,
        canvas_text_assets,
        generation_specs,
        prompt_artifacts,
        generated_images,
        generated_videos,
        media_assets,
        media_provenance,
        media_bindings,
        video_segments,
        assembly_timelines,
        assembly_clips,
        media_files,
    })
}

fn collect_media_files(
    state: &AppState,
    images: &[generated_image::Model],
    videos: &[generated_video::Model],
    media_assets: &[media_asset::Model],
) -> Result<Vec<VideoStudioMediaFileEntry>, String> {
    let mut entries = Vec::new();
    let mut seen_package_paths = HashSet::new();
    for image in images {
        let package_path =
            media_package_path(MEDIA_OWNER_IMAGE, image.id, "file", &image.file_path)?;
        validate_export_media_file(
            state,
            &image.file_path,
            &package_path,
            &mut seen_package_paths,
        )?;
        entries.push(VideoStudioMediaFileEntry {
            owner_kind: MEDIA_OWNER_IMAGE.to_string(),
            owner_id: image.id,
            field: MEDIA_FIELD_FILE.to_string(),
            original_path: image.file_path.clone(),
            package_path,
        });
    }
    for video in videos {
        for (field, package_field, path) in [
            (MEDIA_FIELD_FILE, "file", video.file_path.as_deref()),
            (
                MEDIA_FIELD_THUMBNAIL,
                "thumbnail",
                video.thumbnail_path.as_deref(),
            ),
        ] {
            let Some(original_path) = path else {
                continue;
            };
            let package_path =
                media_package_path(MEDIA_OWNER_VIDEO, video.id, package_field, original_path)?;
            validate_export_media_file(
                state,
                original_path,
                &package_path,
                &mut seen_package_paths,
            )?;
            entries.push(VideoStudioMediaFileEntry {
                owner_kind: MEDIA_OWNER_VIDEO.to_string(),
                owner_id: video.id,
                field: field.to_string(),
                original_path: original_path.to_string(),
                package_path,
            });
        }
    }
    for asset in media_assets {
        for (field, package_field, path) in [
            (MEDIA_FIELD_FILE, "file", asset.file_path.as_deref()),
            (
                MEDIA_FIELD_THUMBNAIL,
                "thumbnail",
                asset.thumbnail_path.as_deref(),
            ),
        ] {
            let Some(original_path) = path else {
                continue;
            };
            let package_path = media_package_path(
                MEDIA_OWNER_MEDIA_ASSET,
                asset.id,
                package_field,
                original_path,
            )?;
            validate_export_media_file(
                state,
                original_path,
                &package_path,
                &mut seen_package_paths,
            )?;
            entries.push(VideoStudioMediaFileEntry {
                owner_kind: MEDIA_OWNER_MEDIA_ASSET.to_string(),
                owner_id: asset.id,
                field: field.to_string(),
                original_path: original_path.to_string(),
                package_path,
            });
        }
    }
    Ok(entries)
}

fn validate_export_media_file(
    state: &AppState,
    original_path: &str,
    package_path: &str,
    seen_package_paths: &mut HashSet<String>,
) -> Result<(), String> {
    canonicalize_data_file_path(
        &state.config.data_dir,
        original_path,
        "视频画布项目媒体文件",
    )?;
    if !seen_package_paths.insert(package_path.to_string()) {
        return Err(format!("视频画布项目媒体包路径重复: {package_path}"));
    }
    Ok(())
}

fn media_package_path(
    owner_kind: &str,
    owner_id: i64,
    field: &str,
    original_path: &str,
) -> Result<String, String> {
    let extension = Path::new(original_path)
        .extension()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("媒体文件缺少扩展名: {original_path}"))?;
    Ok(format!("media/{owner_kind}/{owner_id}/{field}.{extension}"))
}

async fn import_package_rows<C>(
    state: &AppState,
    user_id: i64,
    db: &C,
    package: VideoStudioProjectPackage,
    media_payload: ImportMediaPayload,
    written_files: &mut Vec<PathBuf>,
) -> Result<novel_project::Model, String>
where
    C: ConnectionTrait,
{
    let now = Utc::now().naive_utc();
    let project = insert_project(db, user_id, &package.project, now).await?;
    let mut maps = ImportIdMaps::default();

    let imported_videos = {
        let mut media_context = ImportMediaRowsContext {
            state,
            db,
            project_id: project.id,
            media_payload: &media_payload,
            maps: &mut maps,
            written_files,
            now,
        };
        import_generated_images(&mut media_context, &package.generated_images).await?;
        import_generated_videos(&mut media_context, &package.generated_videos).await?
    };
    update_imported_video_source_refs(db, &package.generated_videos, &imported_videos, &maps, now)
        .await?;
    import_canvas_text_assets(db, project.id, &package.canvas_text_assets, &mut maps, now).await?;
    import_canvas_graphs(db, project.id, &package, &mut maps, now).await?;
    let deferred_generation_spec_targets =
        import_generation_specs(db, project.id, &package.generation_specs, &mut maps, now).await?;
    import_prompt_artifacts(db, &package.prompt_artifacts, &mut maps, now).await?;
    let mut media_context = ImportMediaRowsContext {
        state,
        db,
        project_id: project.id,
        media_payload: &media_payload,
        maps: &mut maps,
        written_files,
        now,
    };
    import_media_assets(&mut media_context, &package.media_assets).await?;
    import_media_provenance(db, &package.media_provenance, &maps, now).await?;
    import_video_segments(db, project.id, &package.video_segments, &mut maps, now).await?;
    import_assembly_rows(db, project.id, &package, &mut maps, now).await?;
    update_imported_generation_spec_targets(db, &deferred_generation_spec_targets, &maps, now)
        .await?;
    import_media_bindings(db, &package.media_bindings, &mut maps, now).await?;
    update_imported_canvas_node_refs(db, &package, &maps, now).await?;
    update_imported_media_metadata(db, &package, &maps, now).await?;

    Ok(project)
}

async fn insert_project<C>(
    db: &C,
    user_id: i64,
    source: &novel_project::Model,
    now: chrono::NaiveDateTime,
) -> Result<novel_project::Model, String>
where
    C: ConnectionTrait,
{
    novel_project::ActiveModel {
        user_id: Set(user_id),
        title: Set(source.title.clone()),
        description: Set(source.description.clone()),
        project_type: Set(source.project_type.clone()),
        project_kind: Set(PROJECT_KIND_VIDEO.to_string()),
        source_mode: Set(SOURCE_MODE_FREEFORM.to_string()),
        root_project_id: Set(None),
        parent_project_id: Set(None),
        source_snapshot_id: Set(None),
        source_sync_status: Set(SOURCE_SYNC_CURRENT.to_string()),
        derived_project_status: Set(None),
        status: Set(source.status.clone()),
        target_chapters: Set(source.target_chapters),
        current_chapter: Set(source.current_chapter),
        total_chapters: Set(source.total_chapters),
        chapters_per_part: Set(source.chapters_per_part),
        avatar_svg: Set(source.avatar_svg.clone()),
        avatar_animal: Set(source.avatar_animal.clone()),
        raw_blueprint: Set(source.raw_blueprint.clone()),
        import_analysis_progress: Set(source.import_analysis_progress.clone()),
        export_config: Set(source.export_config.clone()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(|err| format!("导入视频画布项目失败: {err}"))
}

struct ImportMediaRowsContext<'a, C>
where
    C: ConnectionTrait,
{
    state: &'a AppState,
    db: &'a C,
    project_id: i64,
    media_payload: &'a ImportMediaPayload,
    maps: &'a mut ImportIdMaps,
    written_files: &'a mut Vec<PathBuf>,
    now: chrono::NaiveDateTime,
}

async fn import_generated_images<C>(
    context: &mut ImportMediaRowsContext<'_, C>,
    images: &[generated_image::Model],
) -> Result<(), String>
where
    C: ConnectionTrait,
{
    for image in images {
        let target_path = write_imported_media_file(
            context.state,
            context.project_id,
            context.media_payload,
            MEDIA_OWNER_IMAGE,
            image.id,
            MEDIA_FIELD_FILE,
            "images",
            "imported_package_image",
            context.written_files,
        )?
        .ok_or_else(|| format!("生成图片缺少媒体文件: {}", image.id))?;
        let target_relative = path_to_db_string(&target_path);
        let row = generated_image::ActiveModel {
            project_id: Set(context.project_id),
            image_type: Set(image.image_type.clone()),
            file_path: Set(target_relative.clone()),
            prompt: Set(image.prompt.clone()),
            model: Set(image.model.clone()),
            width: Set(image.width),
            height: Set(image.height),
            file_size: Set(image.file_size),
            ai_score: Set(image.ai_score),
            is_favorite: Set(image.is_favorite),
            chapter_id: Set(None),
            panel_id: Set(image.panel_id.clone()),
            metadata_json: Set(image.metadata_json.clone()),
            created_at: Set(context.now),
            ..Default::default()
        }
        .insert(context.db)
        .await
        .map_err(|err| err.to_string())?;
        context.maps.image_ids.insert(image.id, row.id);
        context
            .maps
            .image_file_paths
            .insert(image.id, target_relative);
    }
    Ok(())
}

async fn import_generated_videos<C>(
    context: &mut ImportMediaRowsContext<'_, C>,
    videos: &[generated_video::Model],
) -> Result<HashMap<i64, generated_video::Model>, String>
where
    C: ConnectionTrait,
{
    let mut imported = HashMap::new();
    for video in videos {
        let file_path = write_imported_media_file(
            context.state,
            context.project_id,
            context.media_payload,
            MEDIA_OWNER_VIDEO,
            video.id,
            MEDIA_FIELD_FILE,
            "videos",
            "imported_package_video",
            context.written_files,
        )?
        .map(|path| path_to_db_string(&path));
        let thumbnail_path = write_imported_media_file(
            context.state,
            context.project_id,
            context.media_payload,
            MEDIA_OWNER_VIDEO,
            video.id,
            MEDIA_FIELD_THUMBNAIL,
            "videos",
            "imported_package_video_thumbnail",
            context.written_files,
        )?
        .map(|path| path_to_db_string(&path));
        let row = generated_video::ActiveModel {
            project_id: Set(context.project_id),
            chapter_id: Set(None),
            video_type: Set(video.video_type.clone()),
            workflow_mode: Set(video.workflow_mode.clone()),
            file_path: Set(file_path.clone()),
            thumbnail_path: Set(thumbnail_path.clone()),
            prompt: Set(video.prompt.clone()),
            negative_prompt: Set(video.negative_prompt.clone()),
            model: Set(video.model.clone()),
            width: Set(video.width),
            height: Set(video.height),
            duration_secs: Set(video.duration_secs),
            fps: Set(video.fps),
            file_size: Set(video.file_size),
            source_video_id: Set(None),
            source_image_id: Set(None),
            status: Set(video.status.clone()),
            provider_job_id: Set(video.provider_job_id.clone()),
            error_message: Set(video.error_message.clone()),
            metadata_json: Set(video.metadata_json.clone()),
            is_favorite: Set(video.is_favorite),
            created_at: Set(context.now),
            updated_at: Set(context.now),
            ..Default::default()
        }
        .insert(context.db)
        .await
        .map_err(|err| err.to_string())?;
        context.maps.video_ids.insert(video.id, row.id);
        context.maps.video_file_paths.insert(video.id, file_path);
        context
            .maps
            .video_thumbnail_paths
            .insert(video.id, thumbnail_path);
        imported.insert(video.id, row);
    }
    Ok(imported)
}

async fn update_imported_video_source_refs<C>(
    db: &C,
    source_videos: &[generated_video::Model],
    imported_videos: &HashMap<i64, generated_video::Model>,
    maps: &ImportIdMaps,
    now: chrono::NaiveDateTime,
) -> Result<(), String>
where
    C: ConnectionTrait,
{
    for source in source_videos {
        let Some(imported) = imported_videos.get(&source.id) else {
            continue;
        };
        let source_video_id = remap_optional_id(
            &maps.video_ids,
            source.source_video_id,
            "generated_video.source_video_id",
        )?;
        let source_image_id = remap_optional_id(
            &maps.image_ids,
            source.source_image_id,
            "generated_video.source_image_id",
        )?;
        if source_video_id == imported.source_video_id
            && source_image_id == imported.source_image_id
        {
            continue;
        }
        generated_video::ActiveModel {
            id: Set(imported.id),
            source_video_id: Set(source_video_id),
            source_image_id: Set(source_image_id),
            updated_at: Set(now),
            ..Default::default()
        }
        .update(db)
        .await
        .map_err(|err| err.to_string())?;
    }
    Ok(())
}

async fn import_canvas_text_assets<C>(
    db: &C,
    project_id: i64,
    assets: &[canvas_text_asset::Model],
    maps: &mut ImportIdMaps,
    now: chrono::NaiveDateTime,
) -> Result<(), String>
where
    C: ConnectionTrait,
{
    for asset in assets {
        let row = canvas_text_asset::ActiveModel {
            project_id: Set(project_id),
            title: Set(asset.title.clone()),
            text_kind: Set(asset.text_kind.clone()),
            content: Set(asset.content.clone()),
            content_hash: Set(asset.content_hash.clone()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|err| err.to_string())?;
        maps.text_asset_ids.insert(asset.id, row.id);
    }
    Ok(())
}

async fn import_canvas_graphs<C>(
    db: &C,
    project_id: i64,
    package: &VideoStudioProjectPackage,
    maps: &mut ImportIdMaps,
    now: chrono::NaiveDateTime,
) -> Result<(), String>
where
    C: ConnectionTrait,
{
    for graph in &package.canvas_graphs {
        let row = canvas_graph::ActiveModel {
            project_id: Set(project_id),
            name: Set(graph.name.clone()),
            graph_kind: Set(graph.graph_kind.clone()),
            version: Set(graph.version),
            viewport_json: Set(graph.viewport_json.clone()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|err| err.to_string())?;
        maps.graph_ids.insert(graph.id, row.id);
    }

    for node in &package.canvas_nodes {
        let graph_id = remap_required_id(&maps.graph_ids, node.graph_id, "canvas_node.graph_id")?;
        let (ref_type, ref_id, config_json) = remap_canvas_node_refs(
            node.ref_type.clone(),
            node.ref_id,
            node.config_json.clone(),
            maps,
        )?;
        let row = canvas_node::ActiveModel {
            graph_id: Set(graph_id),
            node_kind: Set(node.node_kind.clone()),
            ref_type: Set(ref_type),
            ref_id: Set(ref_id),
            config_json: Set(config_json),
            position_json: Set(node.position_json.clone()),
            state: Set(node.state.clone()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|err| err.to_string())?;
        maps.node_ids.insert(node.id, row.id);
    }

    for edge in &package.canvas_edges {
        canvas_edge::ActiveModel {
            graph_id: Set(remap_required_id(
                &maps.graph_ids,
                edge.graph_id,
                "canvas_edge.graph_id",
            )?),
            source_node_id: Set(remap_required_id(
                &maps.node_ids,
                edge.source_node_id,
                "canvas_edge.source_node_id",
            )?),
            source_port: Set(edge.source_port.clone()),
            target_node_id: Set(remap_required_id(
                &maps.node_ids,
                edge.target_node_id,
                "canvas_edge.target_node_id",
            )?),
            target_port: Set(edge.target_port.clone()),
            role: Set(edge.role.clone()),
            config_json: Set(edge.config_json.clone()),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|err| err.to_string())?;
    }
    Ok(())
}

async fn import_generation_specs<C>(
    db: &C,
    project_id: i64,
    specs: &[generation_spec::Model],
    maps: &mut ImportIdMaps,
    now: chrono::NaiveDateTime,
) -> Result<Vec<DeferredImportGenerationSpecTarget>, String>
where
    C: ConnectionTrait,
{
    let mut deferred_targets = Vec::new();
    for spec in specs {
        let canvas_text_asset_id = spec
            .canvas_text_asset_id
            .map(|old_id| {
                remap_required_id(
                    &maps.text_asset_ids,
                    old_id,
                    "generation_spec.canvas_text_asset_id",
                )
            })
            .transpose()?;
        let target = initial_generation_spec_target_entity_id(spec, maps)?;
        let params_json = remap_canvas_metadata(spec.params_json.clone(), maps)?;
        let row = generation_spec::ActiveModel {
            project_id: Set(project_id),
            root_project_id: Set(project_id),
            source_context_kind: Set(Some(SOURCE_MODE_FREEFORM.to_string())),
            source_snapshot_id: Set(None),
            source_chapter_id: Set(None),
            source_snapshot_chapter_id: Set(None),
            canvas_text_asset_id: Set(canvas_text_asset_id),
            target_kind: Set(spec.target_kind.clone()),
            target_entity_kind: Set(spec.target_entity_kind.clone()),
            target_entity_id: Set(target.target_entity_id),
            status: Set(spec.status.clone()),
            source_version_hash: Set(spec.source_version_hash.clone()),
            params_json: Set(params_json),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|err| err.to_string())?;
        if let Some(deferred) = target.deferred {
            deferred_targets.push(DeferredImportGenerationSpecTarget {
                new_spec_id: row.id,
                target_entity_kind: deferred.target_entity_kind,
                old_target_entity_id: deferred.old_target_entity_id,
            });
        }
        maps.generation_spec_ids.insert(spec.id, row.id);
    }
    Ok(deferred_targets)
}

async fn import_prompt_artifacts<C>(
    db: &C,
    artifacts: &[prompt_artifact::Model],
    maps: &mut ImportIdMaps,
    now: chrono::NaiveDateTime,
) -> Result<(), String>
where
    C: ConnectionTrait,
{
    for artifact in artifacts {
        let generation_spec_id = remap_required_id(
            &maps.generation_spec_ids,
            artifact.generation_spec_id,
            "prompt_artifact.generation_spec_id",
        )?;
        let row = prompt_artifact::ActiveModel {
            generation_spec_id: Set(generation_spec_id),
            prompt_kind: Set(artifact.prompt_kind.clone()),
            prompt_text: Set(artifact.prompt_text.clone()),
            renderer_version: Set(artifact.renderer_version.clone()),
            edited_by_user: Set(artifact.edited_by_user),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|err| err.to_string())?;
        maps.prompt_artifact_ids.insert(artifact.id, row.id);
    }
    Ok(())
}

async fn update_imported_generation_spec_targets<C>(
    db: &C,
    targets: &[DeferredImportGenerationSpecTarget],
    maps: &ImportIdMaps,
    now: chrono::NaiveDateTime,
) -> Result<(), String>
where
    C: ConnectionTrait,
{
    for target in targets {
        let target_entity_id = remap_generation_spec_target_entity_id_parts(
            target.new_spec_id,
            &target.target_entity_kind,
            target.old_target_entity_id,
            maps,
        )?;
        generation_spec::ActiveModel {
            id: Set(target.new_spec_id),
            target_entity_id: Set(Some(target_entity_id)),
            updated_at: Set(now),
            ..Default::default()
        }
        .update(db)
        .await
        .map_err(|err| err.to_string())?;
    }
    Ok(())
}

async fn import_media_assets<C>(
    context: &mut ImportMediaRowsContext<'_, C>,
    assets: &[media_asset::Model],
) -> Result<(), String>
where
    C: ConnectionTrait,
{
    for asset in assets {
        let folder = match asset.media_kind.as_str() {
            "image" => "media/image",
            "video" => "media/video",
            "audio" => "media/audio",
            other => {
                return Err(format!(
                    "媒体资产类型无效: media_asset_id={}, media_kind={other}",
                    asset.id
                ));
            }
        };
        let file_path = write_imported_media_file(
            context.state,
            context.project_id,
            context.media_payload,
            MEDIA_OWNER_MEDIA_ASSET,
            asset.id,
            MEDIA_FIELD_FILE,
            folder,
            "imported_package_media",
            context.written_files,
        )?
        .map(|path| path_to_db_string(&path));
        let thumbnail_path = write_imported_media_file(
            context.state,
            context.project_id,
            context.media_payload,
            MEDIA_OWNER_MEDIA_ASSET,
            asset.id,
            MEDIA_FIELD_THUMBNAIL,
            folder,
            "imported_package_media_thumbnail",
            context.written_files,
        )?
        .map(|path| path_to_db_string(&path));
        let row = media_asset::ActiveModel {
            project_id: Set(context.project_id),
            root_project_id: Set(context.project_id),
            source_chapter_id: Set(None),
            source_snapshot_chapter_id: Set(None),
            media_kind: Set(asset.media_kind.clone()),
            origin: Set(asset.origin.clone()),
            file_path: Set(file_path),
            thumbnail_path: Set(thumbnail_path),
            width: Set(asset.width),
            height: Set(asset.height),
            duration_secs: Set(asset.duration_secs),
            fps: Set(asset.fps),
            file_size: Set(asset.file_size),
            checksum: Set(asset.checksum.clone()),
            status: Set(asset.status.clone()),
            error_message: Set(asset.error_message.clone()),
            created_at: Set(context.now),
            updated_at: Set(context.now),
            ..Default::default()
        }
        .insert(context.db)
        .await
        .map_err(|err| err.to_string())?;
        context.maps.media_asset_ids.insert(asset.id, row.id);
    }
    Ok(())
}

async fn import_media_provenance<C>(
    db: &C,
    records: &[media_provenance::Model],
    maps: &ImportIdMaps,
    now: chrono::NaiveDateTime,
) -> Result<(), String>
where
    C: ConnectionTrait,
{
    for record in records {
        media_provenance::ActiveModel {
            media_asset_id: Set(remap_required_id(
                &maps.media_asset_ids,
                record.media_asset_id,
                "media_provenance.media_asset_id",
            )?),
            generation_spec_id: Set(record
                .generation_spec_id
                .map(|old_id| {
                    remap_required_id(
                        &maps.generation_spec_ids,
                        old_id,
                        "media_provenance.generation_spec_id",
                    )
                })
                .transpose()?),
            prompt_artifact_id: Set(record
                .prompt_artifact_id
                .map(|old_id| {
                    remap_required_id(
                        &maps.prompt_artifact_ids,
                        old_id,
                        "media_provenance.prompt_artifact_id",
                    )
                })
                .transpose()?),
            provider_job_id: Set(record.provider_job_id.clone()),
            source_media_asset_id: Set(record
                .source_media_asset_id
                .map(|old_id| {
                    remap_required_id(
                        &maps.media_asset_ids,
                        old_id,
                        "media_provenance.source_media_asset_id",
                    )
                })
                .transpose()?),
            source_external_path: Set(remap_source_external_path(
                record.source_external_path.as_deref(),
                maps,
            )?),
            imported_from_user: Set(record.imported_from_user),
            request_snapshot_json: Set(record
                .request_snapshot_json
                .clone()
                .map(|value| remap_canvas_metadata(value, maps))
                .transpose()?),
            created_at: Set(now),
        }
        .insert(db)
        .await
        .map_err(|err| err.to_string())?;
    }
    Ok(())
}

async fn import_video_segments<C>(
    db: &C,
    project_id: i64,
    segments: &[video_segment::Model],
    maps: &mut ImportIdMaps,
    now: chrono::NaiveDateTime,
) -> Result<(), String>
where
    C: ConnectionTrait,
{
    for segment in segments {
        let row = video_segment::ActiveModel {
            project_id: Set(project_id),
            root_project_id: Set(project_id),
            source_chapter_id: Set(None),
            source_snapshot_chapter_id: Set(None),
            segment_type: Set(segment.segment_type.clone()),
            order_index: Set(segment.order_index),
            title: Set(segment.title.clone()),
            duration_secs: Set(segment.duration_secs),
            source_generation_spec_id: Set(segment
                .source_generation_spec_id
                .map(|old_id| {
                    remap_required_id(
                        &maps.generation_spec_ids,
                        old_id,
                        "video_segment.source_generation_spec_id",
                    )
                })
                .transpose()?),
            result_media_asset_id: Set(segment
                .result_media_asset_id
                .map(|old_id| {
                    remap_required_id(
                        &maps.media_asset_ids,
                        old_id,
                        "video_segment.result_media_asset_id",
                    )
                })
                .transpose()?),
            previous_segment_id: Set(None),
            next_segment_id: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|err| err.to_string())?;
        maps.video_segment_ids.insert(segment.id, row.id);
    }

    for segment in segments {
        let new_id = remap_required_id(&maps.video_segment_ids, segment.id, "video_segment.id")?;
        video_segment::ActiveModel {
            id: Set(new_id),
            previous_segment_id: Set(segment
                .previous_segment_id
                .map(|old_id| {
                    remap_required_id(
                        &maps.video_segment_ids,
                        old_id,
                        "video_segment.previous_segment_id",
                    )
                })
                .transpose()?),
            next_segment_id: Set(segment
                .next_segment_id
                .map(|old_id| {
                    remap_required_id(
                        &maps.video_segment_ids,
                        old_id,
                        "video_segment.next_segment_id",
                    )
                })
                .transpose()?),
            updated_at: Set(now),
            ..Default::default()
        }
        .update(db)
        .await
        .map_err(|err| err.to_string())?;
    }
    Ok(())
}

async fn import_assembly_rows<C>(
    db: &C,
    project_id: i64,
    package: &VideoStudioProjectPackage,
    maps: &mut ImportIdMaps,
    now: chrono::NaiveDateTime,
) -> Result<(), String>
where
    C: ConnectionTrait,
{
    for timeline in &package.assembly_timelines {
        let row = assembly_timeline::ActiveModel {
            project_id: Set(project_id),
            source_chapter_id: Set(None),
            source_snapshot_chapter_id: Set(None),
            version: Set(timeline.version),
            status: Set(timeline.status.clone()),
            preview_media_asset_id: Set(timeline
                .preview_media_asset_id
                .map(|old_id| {
                    remap_required_id(
                        &maps.media_asset_ids,
                        old_id,
                        "assembly_timeline.preview_media_asset_id",
                    )
                })
                .transpose()?),
            preview_cache_key: Set(remap_canvas_cache_key(
                timeline.preview_cache_key.as_deref(),
                maps,
            )?),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|err| err.to_string())?;
        maps.assembly_timeline_ids.insert(timeline.id, row.id);
    }
    for clip in &package.assembly_clips {
        let row = assembly_clip::ActiveModel {
            timeline_id: Set(remap_required_id(
                &maps.assembly_timeline_ids,
                clip.timeline_id,
                "assembly_clip.timeline_id",
            )?),
            media_asset_id: Set(remap_required_id(
                &maps.media_asset_ids,
                clip.media_asset_id,
                "assembly_clip.media_asset_id",
            )?),
            order_index: Set(clip.order_index),
            trim_start_secs: Set(clip.trim_start_secs),
            trim_end_secs: Set(clip.trim_end_secs),
            volume: Set(clip.volume),
            transition_kind: Set(clip.transition_kind.clone()),
            source_segment_id: Set(clip
                .source_segment_id
                .map(|old_id| {
                    remap_required_id(
                        &maps.video_segment_ids,
                        old_id,
                        "assembly_clip.source_segment_id",
                    )
                })
                .transpose()?),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|err| err.to_string())?;
        maps.assembly_clip_ids.insert(clip.id, row.id);
    }
    Ok(())
}

async fn import_media_bindings<C>(
    db: &C,
    bindings: &[media_binding::Model],
    maps: &mut ImportIdMaps,
    now: chrono::NaiveDateTime,
) -> Result<(), String>
where
    C: ConnectionTrait,
{
    for binding in bindings {
        let target_id = match binding.target_kind.as_str() {
            "generated_video" => remap_required_id(
                &maps.video_ids,
                binding.target_id,
                "media_binding.target_id",
            )?,
            "assembly_timeline" => remap_required_id(
                &maps.assembly_timeline_ids,
                binding.target_id,
                "media_binding.target_id",
            )?,
            "video_segment" => remap_required_id(
                &maps.video_segment_ids,
                binding.target_id,
                "media_binding.target_id",
            )?,
            other => {
                return Err(format!(
                    "视频画布项目包暂不支持媒体绑定目标类型: target_kind={other}"
                ));
            }
        };
        let row = media_binding::ActiveModel {
            media_asset_id: Set(remap_required_id(
                &maps.media_asset_ids,
                binding.media_asset_id,
                "media_binding.media_asset_id",
            )?),
            target_kind: Set(binding.target_kind.clone()),
            target_id: Set(target_id),
            role: Set(binding.role.clone()),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|err| err.to_string())?;
        maps.media_binding_ids.insert(binding.id, row.id);
    }
    Ok(())
}

async fn update_imported_canvas_node_refs<C>(
    db: &C,
    package: &VideoStudioProjectPackage,
    maps: &ImportIdMaps,
    now: chrono::NaiveDateTime,
) -> Result<(), String>
where
    C: ConnectionTrait,
{
    for node in &package.canvas_nodes {
        let node_id = remap_required_id(&maps.node_ids, node.id, "canvas_node.id")?;
        let (ref_type, ref_id, config_json) = remap_canvas_node_refs(
            node.ref_type.clone(),
            node.ref_id,
            node.config_json.clone(),
            maps,
        )?;
        canvas_node::ActiveModel {
            id: Set(node_id),
            ref_type: Set(ref_type),
            ref_id: Set(ref_id),
            config_json: Set(config_json),
            updated_at: Set(now),
            ..Default::default()
        }
        .update(db)
        .await
        .map_err(|err| err.to_string())?;
    }
    Ok(())
}

async fn update_imported_media_metadata<C>(
    db: &C,
    package: &VideoStudioProjectPackage,
    maps: &ImportIdMaps,
    now: chrono::NaiveDateTime,
) -> Result<(), String>
where
    C: ConnectionTrait,
{
    for image in &package.generated_images {
        let Some(new_id) = maps.image_ids.get(&image.id).copied() else {
            continue;
        };
        let metadata_json = image
            .metadata_json
            .clone()
            .map(|metadata| remap_canvas_metadata(metadata, maps))
            .transpose()?;
        generated_image::ActiveModel {
            id: Set(new_id),
            metadata_json: Set(metadata_json),
            ..Default::default()
        }
        .update(db)
        .await
        .map_err(|err| err.to_string())?;
    }
    for video in &package.generated_videos {
        let Some(new_id) = maps.video_ids.get(&video.id).copied() else {
            continue;
        };
        let metadata_json = video
            .metadata_json
            .clone()
            .map(|metadata| remap_canvas_metadata(metadata, maps))
            .transpose()?;
        generated_video::ActiveModel {
            id: Set(new_id),
            metadata_json: Set(metadata_json),
            updated_at: Set(now),
            ..Default::default()
        }
        .update(db)
        .await
        .map_err(|err| err.to_string())?;
    }
    Ok(())
}

fn write_imported_media_file(
    state: &AppState,
    project_id: i64,
    media_payload: &ImportMediaPayload,
    owner_kind: &str,
    owner_id: i64,
    field: &str,
    folder: &str,
    stem: &str,
    written_files: &mut Vec<PathBuf>,
) -> Result<Option<PathBuf>, String> {
    let key = media_file_key(owner_kind, owner_id, field);
    let Some(entry) = media_payload.entries.get(&key) else {
        return Ok(None);
    };
    let bytes = media_payload.bytes.get(&entry.package_path).ok_or_else(|| {
        format!(
            "视频画布项目包缺少媒体内容: owner_kind={owner_kind}, owner_id={owner_id}, field={field}"
        )
    })?;
    let extension = required_package_extension(&entry.package_path)?;
    let relative = PathBuf::from(format!(
        "projects/{project_id}/{folder}/{stem}_{owner_id}_{field}.{extension}"
    ));
    let path = state.config.data_dir.join(&relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("无法创建媒体导入目录: {err}"))?;
    }
    fs::write(&path, bytes).map_err(|err| format!("写入导入媒体文件失败: {err}"))?;
    written_files.push(path);
    Ok(Some(relative))
}

fn remap_canvas_node_refs(
    ref_type: Option<String>,
    ref_id: Option<i64>,
    mut config_json: Value,
    maps: &ImportIdMaps,
) -> Result<(Option<String>, Option<i64>, Value), String> {
    remap_canvas_node_config_ids(&mut config_json, maps)?;
    let Some(kind) = ref_type.as_deref().map(str::to_string) else {
        if ref_id.is_some() {
            return Err("画布节点 ref_type 为空时不能设置 ref_id".to_string());
        }
        return Ok((ref_type, ref_id, config_json));
    };
    match kind.as_str() {
        "inline" => {
            if ref_id.is_some() {
                return Err("inline 画布节点不能设置 ref_id".to_string());
            }
            Ok((ref_type, None, config_json))
        }
        "config" => Ok((ref_type, ref_id, config_json)),
        "legacy_generated_image" => {
            let old_id = required_ref_id(ref_id, kind.as_str())?;
            let new_id = remap_required_id(&maps.image_ids, old_id, "canvas_node.ref_id")?;
            if let Some(file_path) = maps.image_file_paths.get(&old_id) {
                set_json_string(&mut config_json, "file_path", file_path);
            }
            Ok((ref_type, Some(new_id), config_json))
        }
        "legacy_generated_video" => {
            let old_id = required_ref_id(ref_id, kind.as_str())?;
            let new_id = remap_required_id(&maps.video_ids, old_id, "canvas_node.ref_id")?;
            if let Some(file_path) = maps
                .video_file_paths
                .get(&old_id)
                .and_then(|value| value.as_ref())
            {
                set_json_string(&mut config_json, "file_path", file_path);
            }
            if let Some(thumbnail_path) = maps
                .video_thumbnail_paths
                .get(&old_id)
                .and_then(|value| value.as_ref())
            {
                set_json_string(&mut config_json, "thumbnail_path", thumbnail_path);
            }
            Ok((ref_type, Some(new_id), config_json))
        }
        "canvas_text_asset" => Ok((
            ref_type,
            Some(remap_required_id(
                &maps.text_asset_ids,
                required_ref_id(ref_id, kind.as_str())?,
                "canvas_node.ref_id",
            )?),
            config_json,
        )),
        "generation_spec" => Ok((
            ref_type,
            Some(remap_required_id(
                &maps.generation_spec_ids,
                required_ref_id(ref_id, kind.as_str())?,
                "canvas_node.ref_id",
            )?),
            config_json,
        )),
        "prompt_artifact" => Ok((
            ref_type,
            Some(remap_required_id(
                &maps.prompt_artifact_ids,
                required_ref_id(ref_id, kind.as_str())?,
                "canvas_node.ref_id",
            )?),
            config_json,
        )),
        "media_asset" => Ok((
            ref_type,
            Some(remap_required_id(
                &maps.media_asset_ids,
                required_ref_id(ref_id, kind.as_str())?,
                "canvas_node.ref_id",
            )?),
            config_json,
        )),
        "video_segment" => Ok((
            ref_type,
            Some(remap_required_id(
                &maps.video_segment_ids,
                required_ref_id(ref_id, kind.as_str())?,
                "canvas_node.ref_id",
            )?),
            config_json,
        )),
        "assembly_timeline" => Ok((
            ref_type,
            Some(remap_required_id(
                &maps.assembly_timeline_ids,
                required_ref_id(ref_id, kind.as_str())?,
                "canvas_node.ref_id",
            )?),
            config_json,
        )),
        "legacy_chapter_video_prompt" => {
            Err("视频画布项目包不能导入 legacy_chapter_video_prompt 节点引用".to_string())
        }
        other => Err(format!("未知画布节点引用类型: {other}")),
    }
}

fn remap_canvas_node_config_ids(
    config_json: &mut Value,
    maps: &ImportIdMaps,
) -> Result<(), String> {
    let Value::Object(object) = config_json else {
        return Ok(());
    };
    remap_json_i64_field(object, "canvas_text_asset_id", &maps.text_asset_ids)?;
    remap_json_i64_field(object, "generation_spec_id", &maps.generation_spec_ids)?;
    remap_json_i64_field(object, "prompt_artifact_id", &maps.prompt_artifact_ids)?;
    remap_json_i64_field(object, "generated_image_id", &maps.image_ids)?;
    remap_json_i64_field(object, "legacy_generated_image_id", &maps.image_ids)?;
    remap_json_i64_field(object, "source_image_id", &maps.image_ids)?;
    remap_json_i64_field(object, "generated_video_id", &maps.video_ids)?;
    remap_json_i64_field(object, "legacy_generated_video_id", &maps.video_ids)?;
    remap_json_i64_field(object, "first_frame_image_id", &maps.image_ids)?;
    remap_json_i64_array_field(object, "reference_image_ids", &maps.image_ids)?;
    remap_json_i64_array_field(object, "video_ids", &maps.video_ids)?;
    remap_json_i64_field(object, "video_id", &maps.video_ids)?;
    remap_json_i64_field(object, "source_video_id", &maps.video_ids)?;
    remap_json_i64_field(object, "media_asset_id", &maps.media_asset_ids)?;
    remap_json_i64_array_field(object, "media_asset_ids", &maps.media_asset_ids)?;
    remap_json_i64_field(object, "preview_media_asset_id", &maps.media_asset_ids)?;
    remap_json_i64_field(object, "video_segment_id", &maps.video_segment_ids)?;
    remap_json_i64_array_field(object, "video_segment_ids", &maps.video_segment_ids)?;
    remap_json_i64_field(object, "source_segment_id", &maps.video_segment_ids)?;
    remap_json_i64_field(object, "assembly_timeline_id", &maps.assembly_timeline_ids)?;
    remap_json_i64_array_field(object, "assembly_clip_ids", &maps.assembly_clip_ids)?;
    Ok(())
}

fn remap_canvas_metadata(mut metadata: Value, maps: &ImportIdMaps) -> Result<Value, String> {
    if let Some(object) = metadata.as_object_mut() {
        remap_json_i64_field(object, "generation_spec_id", &maps.generation_spec_ids)?;
        remap_json_i64_field(object, "prompt_artifact_id", &maps.prompt_artifact_ids)?;
        remap_json_i64_field(object, "canvas_text_asset_id", &maps.text_asset_ids)?;
        remap_json_i64_field(object, "generated_image_id", &maps.image_ids)?;
        remap_json_i64_field(object, "generated_video_id", &maps.video_ids)?;
        remap_json_i64_field(object, "image_id", &maps.image_ids)?;
        remap_json_i64_field(object, "video_id", &maps.video_ids)?;
        remap_json_i64_field(object, "source_video_id", &maps.video_ids)?;
        remap_json_i64_field(object, "source_image_id", &maps.image_ids)?;
        remap_json_i64_field(object, "media_asset_id", &maps.media_asset_ids)?;
        remap_json_i64_array_field(object, "media_asset_ids", &maps.media_asset_ids)?;
        remap_json_i64_field(object, "preview_media_asset_id", &maps.media_asset_ids)?;
        remap_json_i64_field(object, "video_segment_id", &maps.video_segment_ids)?;
        remap_json_i64_array_field(object, "video_segment_ids", &maps.video_segment_ids)?;
        remap_json_i64_field(object, "source_segment_id", &maps.video_segment_ids)?;
        remap_json_i64_field(object, "assembly_timeline_id", &maps.assembly_timeline_ids)?;
        remap_json_i64_array_field(object, "assembly_clip_ids", &maps.assembly_clip_ids)?;
    }
    let Some(canvas) = metadata.get_mut("canvas").and_then(Value::as_object_mut) else {
        return Ok(metadata);
    };
    remap_json_i64_field(canvas, "graph_id", &maps.graph_ids)?;
    remap_json_i64_field(canvas, "node_id", &maps.node_ids)?;
    remap_json_i64_field(canvas, "generation_spec_id", &maps.generation_spec_ids)?;
    remap_json_i64_field(canvas, "prompt_artifact_id", &maps.prompt_artifact_ids)?;
    remap_json_i64_field(canvas, "generated_image_id", &maps.image_ids)?;
    remap_json_i64_field(canvas, "generated_video_id", &maps.video_ids)?;
    remap_json_i64_field(canvas, "media_asset_id", &maps.media_asset_ids)?;
    remap_json_i64_array_field(canvas, "media_asset_ids", &maps.media_asset_ids)?;
    remap_json_i64_field(canvas, "preview_media_asset_id", &maps.media_asset_ids)?;
    remap_json_i64_field(canvas, "video_segment_id", &maps.video_segment_ids)?;
    remap_json_i64_array_field(canvas, "video_segment_ids", &maps.video_segment_ids)?;
    remap_json_i64_field(canvas, "source_segment_id", &maps.video_segment_ids)?;
    remap_json_i64_field(canvas, "assembly_timeline_id", &maps.assembly_timeline_ids)?;
    remap_json_i64_array_field(canvas, "assembly_clip_ids", &maps.assembly_clip_ids)?;
    Ok(metadata)
}

fn remap_source_external_path(
    value: Option<&str>,
    maps: &ImportIdMaps,
) -> Result<Option<String>, String> {
    let Some(raw) = value else {
        return Ok(None);
    };
    if let Some(id) = raw
        .strip_prefix("generated_video:")
        .and_then(|suffix| suffix.parse::<i64>().ok())
    {
        let new_id =
            remap_required_id(&maps.video_ids, id, "media_provenance.source_external_path")?;
        return Ok(Some(format!("generated_video:{new_id}")));
    }
    if let Some(id) = raw
        .strip_prefix("generated_image:")
        .and_then(|suffix| suffix.parse::<i64>().ok())
    {
        let new_id =
            remap_required_id(&maps.image_ids, id, "media_provenance.source_external_path")?;
        return Ok(Some(format!("generated_image:{new_id}")));
    }
    Ok(Some(raw.to_string()))
}

fn remap_canvas_cache_key(
    value: Option<&str>,
    maps: &ImportIdMaps,
) -> Result<Option<String>, String> {
    let Some(raw) = value else {
        return Ok(None);
    };
    let Some((graph_id, node_id)) = parse_canvas_cache_key(raw) else {
        return Ok(Some(raw.to_string()));
    };
    let new_graph_id = remap_required_id(
        &maps.graph_ids,
        graph_id,
        "assembly_timeline.preview_cache_key graph_id",
    )?;
    let new_node_id = remap_required_id(
        &maps.node_ids,
        node_id,
        "assembly_timeline.preview_cache_key node_id",
    )?;
    Ok(Some(format!(
        "canvas_graph:{new_graph_id}:node:{new_node_id}"
    )))
}

fn parse_canvas_cache_key(value: &str) -> Option<(i64, i64)> {
    let parts = value.split(':').collect::<Vec<_>>();
    if parts.len() == 4
        && parts[0] == "canvas_graph"
        && parts[2] == "node"
        && let (Ok(graph_id), Ok(node_id)) = (parts[1].parse::<i64>(), parts[3].parse::<i64>())
    {
        return Some((graph_id, node_id));
    }
    None
}

fn remap_json_i64_field(
    object: &mut serde_json::Map<String, Value>,
    key: &str,
    map: &HashMap<i64, i64>,
) -> Result<(), String> {
    let Some(value) = object.get(key) else {
        return Ok(());
    };
    let old_id = value
        .as_i64()
        .ok_or_else(|| format!("视频画布项目包字段 {key} 必须是整数 ID"))?;
    let new_id = remap_required_id(map, old_id, key)?;
    object.insert(key.to_string(), Value::Number(new_id.into()));
    Ok(())
}

fn remap_json_i64_array_field(
    object: &mut serde_json::Map<String, Value>,
    key: &str,
    map: &HashMap<i64, i64>,
) -> Result<(), String> {
    let Some(value) = object.get(key) else {
        return Ok(());
    };
    let values = value
        .as_array()
        .ok_or_else(|| format!("视频画布项目包字段 {key} 必须是整数 ID 数组"))?;
    let remapped = values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let old_id = value
                .as_i64()
                .ok_or_else(|| format!("视频画布项目包字段 {key}[{index}] 必须是整数 ID"))?;
            let new_id = remap_required_id(map, old_id, key)?;
            Ok(Value::Number(new_id.into()))
        })
        .collect::<Result<Vec<_>, String>>()?;
    object.insert(key.to_string(), Value::Array(remapped));
    Ok(())
}

fn set_json_string(config_json: &mut Value, key: &str, value: &str) {
    if let Value::Object(object) = config_json {
        object.insert(key.to_string(), Value::String(value.to_string()));
    }
}

fn media_file_key(owner_kind: &str, owner_id: i64, field: &str) -> (String, i64, String) {
    (owner_kind.to_string(), owner_id, field.to_string())
}

fn remap_required_id(map: &HashMap<i64, i64>, old_id: i64, field: &str) -> Result<i64, String> {
    map.get(&old_id)
        .copied()
        .ok_or_else(|| format!("导入视频画布项目时无法重映射 {field}: {old_id}"))
}

fn remap_optional_id(
    map: &HashMap<i64, i64>,
    old_id: Option<i64>,
    field: &str,
) -> Result<Option<i64>, String> {
    old_id
        .map(|id| remap_required_id(map, id, field))
        .transpose()
}

#[cfg(test)]
fn remap_generation_spec_target_entity_id(
    spec: &generation_spec::Model,
    maps: &ImportIdMaps,
) -> Result<Option<i64>, String> {
    let Some(old_id) = spec.target_entity_id else {
        return Ok(None);
    };
    remap_generation_spec_target_entity_id_parts(spec.id, &spec.target_entity_kind, old_id, maps)
        .map(Some)
}

fn initial_generation_spec_target_entity_id(
    spec: &generation_spec::Model,
    maps: &ImportIdMaps,
) -> Result<InitialGenerationSpecTarget, String> {
    let Some(old_id) = spec.target_entity_id else {
        return Ok(InitialGenerationSpecTarget {
            target_entity_id: None,
            deferred: None,
        });
    };
    match spec.target_entity_kind.as_str() {
        "canvas_node" => Ok(InitialGenerationSpecTarget {
            target_entity_id: Some(remap_required_id(
                &maps.node_ids,
                old_id,
                "generation_spec.target_entity_id",
            )?),
            deferred: None,
        }),
        "media_asset" | "video_segment" | "assembly_timeline" => Ok(InitialGenerationSpecTarget {
            target_entity_id: None,
            deferred: Some(DeferredGenerationSpecTarget {
                target_entity_kind: spec.target_entity_kind.clone(),
                old_target_entity_id: old_id,
            }),
        }),
        other => Err(format!(
            "视频画布项目包暂不支持生成意图目标类型: generation_spec_id={}, target_entity_kind={other}",
            spec.id
        )),
    }
}

fn remap_generation_spec_target_entity_id_parts(
    spec_id: i64,
    target_entity_kind: &str,
    old_id: i64,
    maps: &ImportIdMaps,
) -> Result<i64, String> {
    let map = match target_entity_kind {
        "canvas_node" => &maps.node_ids,
        "media_asset" => &maps.media_asset_ids,
        "video_segment" => &maps.video_segment_ids,
        "assembly_timeline" => &maps.assembly_timeline_ids,
        other => {
            return Err(format!(
                "视频画布项目包暂不支持生成意图目标类型: generation_spec_id={}, target_entity_kind={other}",
                spec_id
            ));
        }
    };
    remap_required_id(map, old_id, "generation_spec.target_entity_id")
}

fn required_ref_id(ref_id: Option<i64>, ref_type: &str) -> Result<i64, String> {
    let ref_id = ref_id.ok_or_else(|| format!("{ref_type} 画布节点必须设置 ref_id"))?;
    if ref_id <= 0 {
        return Err(format!("{ref_type} 画布节点 ref_id 必须是正整数"));
    }
    Ok(ref_id)
}

fn required_package_extension(path: &str) -> Result<String, String> {
    Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase())
        .ok_or_else(|| format!("媒体包路径缺少扩展名: {path}"))
}

fn cleanup_written_files(paths: &[PathBuf]) {
    for path in paths.iter().rev() {
        let _ = fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests;
