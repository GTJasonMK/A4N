use super::*;

pub(super) async fn duplicate_freeform_project_data<C>(
    db: &C,
    original: &novel_project::Model,
    user_id: i64,
    new_title: &str,
    now: chrono::NaiveDateTime,
    data_dir: &Path,
    copied_media_files: &mut Vec<PathBuf>,
) -> Result<novel_project::Model, String>
where
    C: ConnectionTrait,
{
    if original.source_mode != SOURCE_MODE_FREEFORM {
        return Err("复制自由视频项目只能接收 source_mode=freeform 的项目".to_string());
    }
    let new_project = insert_project_copy(
        db,
        original,
        ProjectCopyInsert {
            user_id,
            title: new_title.to_string(),
            root_project_id: None,
            parent_project_id: None,
            source_snapshot_id: None,
            status: original.status.clone(),
            current_chapter: original.current_chapter,
            total_chapters: original.total_chapters,
            raw_blueprint: original.raw_blueprint.clone(),
            import_analysis_progress: original.import_analysis_progress.clone(),
        },
        now,
    )
    .await?;

    let copied_images = copy_freeform_generated_images(
        db,
        original.id,
        new_project.id,
        now,
        data_dir,
        copied_media_files,
    )
    .await?;
    let copied_videos = copy_freeform_generated_videos(
        db,
        original.id,
        new_project.id,
        now,
        data_dir,
        copied_media_files,
    )
    .await?;
    let mut maps = FreeformCopyMaps {
        images: copied_images,
        videos: copied_videos,
        ..Default::default()
    };
    update_freeform_generated_video_source_refs(db, original.id, &maps, now).await?;

    let text_asset_id_map = copy_canvas_text_assets(db, original.id, new_project.id, now).await?;
    maps.text_asset_ids = text_asset_id_map;
    let copied_specs =
        copy_freeform_generation_specs(db, original.id, new_project.id, &maps.text_asset_ids, now)
            .await?;
    maps.generation_spec_ids = copied_specs.spec_id_map;
    maps.prompt_artifact_ids =
        copy_prompt_artifacts(db, &copied_specs.spec_ids, &maps.generation_spec_ids, now).await?;
    maps.media_asset_ids = copy_freeform_media_assets(
        db,
        original.id,
        new_project.id,
        now,
        data_dir,
        copied_media_files,
    )
    .await?;
    maps.video_segment_ids = copy_freeform_video_segments(
        db,
        original.id,
        new_project.id,
        &maps.generation_spec_ids,
        &maps.media_asset_ids,
        now,
    )
    .await?;
    maps.assembly_timeline_ids = copy_freeform_assembly_timelines(
        db,
        original.id,
        new_project.id,
        &maps.media_asset_ids,
        now,
    )
    .await?;
    maps.assembly_clip_ids = copy_assembly_clips(
        db,
        &maps.assembly_timeline_ids,
        &maps.media_asset_ids,
        &maps.video_segment_ids,
    )
    .await?;
    let copied_canvas = copy_canvas_graphs(db, original.id, new_project.id, &maps, now).await?;
    maps.graph_ids = copied_canvas.graph_ids;
    maps.node_ids = copied_canvas.node_ids;

    update_freeform_generation_spec_deferred_targets(
        db,
        &copied_specs.deferred_targets,
        &maps,
        now,
    )
    .await?;
    update_freeform_generation_spec_metadata(db, &maps.generation_spec_ids, &maps, now).await?;
    copy_generation_spec_media_refs(
        db,
        &copied_specs.spec_ids,
        &maps.generation_spec_ids,
        &maps.media_asset_ids,
    )
    .await?;
    copy_freeform_media_provenance(db, &maps.media_asset_ids, &maps, now).await?;
    copy_media_bindings(
        db,
        original.id,
        &maps.media_asset_ids,
        &maps.video_segment_ids,
        &maps.assembly_timeline_ids,
        Some(&maps.videos.ids),
        now,
    )
    .await?;
    update_freeform_generated_media_metadata(db, original.id, &maps, now).await?;
    update_freeform_assembly_timeline_cache_keys(db, &maps.assembly_timeline_ids, &maps, now)
        .await?;

    Ok(new_project)
}

#[derive(Clone, Default)]
struct CopiedImageRecords {
    ids: HashMap<i64, i64>,
    file_paths: HashMap<i64, String>,
}

#[derive(Clone, Default)]
struct CopiedVideoRecords {
    ids: HashMap<i64, i64>,
    file_paths: HashMap<i64, Option<String>>,
    thumbnail_paths: HashMap<i64, Option<String>>,
}

#[derive(Clone, Default)]
struct FreeformCopyMaps {
    images: CopiedImageRecords,
    videos: CopiedVideoRecords,
    text_asset_ids: HashMap<i64, i64>,
    graph_ids: HashMap<i64, i64>,
    node_ids: HashMap<i64, i64>,
    generation_spec_ids: HashMap<i64, i64>,
    prompt_artifact_ids: HashMap<i64, i64>,
    media_asset_ids: HashMap<i64, i64>,
    video_segment_ids: HashMap<i64, i64>,
    assembly_timeline_ids: HashMap<i64, i64>,
    assembly_clip_ids: HashMap<i64, i64>,
}

#[derive(Default)]
struct CanvasCopyMaps {
    graph_ids: HashMap<i64, i64>,
    node_ids: HashMap<i64, i64>,
}

struct CopiedFreeformGenerationSpecs {
    spec_ids: Vec<i64>,
    spec_id_map: HashMap<i64, i64>,
    deferred_targets: Vec<DeferredFreeformSpecTarget>,
}

struct DeferredFreeformSpecTarget {
    new_spec_id: i64,
    target_entity_kind: String,
    old_target_entity_id: i64,
}

async fn copy_freeform_generated_images<C>(
    db: &C,
    source_project_id: i64,
    new_project_id: i64,
    now: chrono::NaiveDateTime,
    data_dir: &Path,
    copied_media_files: &mut Vec<PathBuf>,
) -> Result<CopiedImageRecords, String>
where
    C: ConnectionTrait,
{
    let images = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.eq(source_project_id))
        .order_by_asc(generated_image::Column::Id)
        .all(db)
        .await
        .map_err(|err| err.to_string())?;
    let mut copied = CopiedImageRecords::default();
    for image in images {
        let file_path = copy_media_file(
            data_dir,
            new_project_id,
            image.id,
            "image",
            Some(image.file_path.as_str()),
            copied_media_files,
        )?
        .ok_or_else(|| format!("生成图片缺少 file_path: {}", image.id))?;
        let row = generated_image::ActiveModel {
            project_id: Set(new_project_id),
            image_type: Set(image.image_type),
            file_path: Set(file_path.clone()),
            prompt: Set(image.prompt),
            model: Set(image.model),
            width: Set(image.width),
            height: Set(image.height),
            file_size: Set(image.file_size),
            ai_score: Set(image.ai_score),
            is_favorite: Set(image.is_favorite),
            chapter_id: Set(None),
            panel_id: Set(image.panel_id),
            metadata_json: Set(image.metadata_json),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|err| err.to_string())?;
        copied.file_paths.insert(image.id, file_path);
        copied.ids.insert(image.id, row.id);
    }
    Ok(copied)
}

async fn copy_freeform_generated_videos<C>(
    db: &C,
    source_project_id: i64,
    new_project_id: i64,
    now: chrono::NaiveDateTime,
    data_dir: &Path,
    copied_media_files: &mut Vec<PathBuf>,
) -> Result<CopiedVideoRecords, String>
where
    C: ConnectionTrait,
{
    let videos = generated_video::Entity::find()
        .filter(generated_video::Column::ProjectId.eq(source_project_id))
        .order_by_asc(generated_video::Column::Id)
        .all(db)
        .await
        .map_err(|err| err.to_string())?;
    let mut copied = CopiedVideoRecords::default();
    for video in videos {
        let file_path = copy_media_file(
            data_dir,
            new_project_id,
            video.id,
            "video",
            video.file_path.as_deref(),
            copied_media_files,
        )?;
        let thumbnail_path = copy_media_file(
            data_dir,
            new_project_id,
            video.id,
            "thumbnail",
            video.thumbnail_path.as_deref(),
            copied_media_files,
        )?;
        let row = generated_video::ActiveModel {
            project_id: Set(new_project_id),
            chapter_id: Set(None),
            video_type: Set(video.video_type),
            workflow_mode: Set(video.workflow_mode),
            file_path: Set(file_path.clone()),
            thumbnail_path: Set(thumbnail_path.clone()),
            prompt: Set(video.prompt),
            negative_prompt: Set(video.negative_prompt),
            model: Set(video.model),
            width: Set(video.width),
            height: Set(video.height),
            duration_secs: Set(video.duration_secs),
            fps: Set(video.fps),
            file_size: Set(video.file_size),
            source_video_id: Set(None),
            source_image_id: Set(None),
            status: Set(video.status),
            provider_job_id: Set(video.provider_job_id),
            error_message: Set(video.error_message),
            metadata_json: Set(video.metadata_json),
            is_favorite: Set(video.is_favorite),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|err| err.to_string())?;
        copied.file_paths.insert(video.id, file_path);
        copied.thumbnail_paths.insert(video.id, thumbnail_path);
        copied.ids.insert(video.id, row.id);
    }
    Ok(copied)
}

async fn copy_canvas_text_assets<C>(
    db: &C,
    source_project_id: i64,
    new_project_id: i64,
    now: chrono::NaiveDateTime,
) -> Result<HashMap<i64, i64>, String>
where
    C: ConnectionTrait,
{
    let assets = canvas_text_asset::Entity::find()
        .filter(canvas_text_asset::Column::ProjectId.eq(source_project_id))
        .order_by_asc(canvas_text_asset::Column::Id)
        .all(db)
        .await
        .map_err(|err| err.to_string())?;
    let mut id_map = HashMap::new();
    for asset in assets {
        let row = canvas_text_asset::ActiveModel {
            project_id: Set(new_project_id),
            title: Set(asset.title),
            text_kind: Set(asset.text_kind),
            content: Set(asset.content),
            content_hash: Set(asset.content_hash),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|err| err.to_string())?;
        id_map.insert(asset.id, row.id);
    }
    Ok(id_map)
}

async fn update_freeform_generated_video_source_refs<C>(
    db: &C,
    source_project_id: i64,
    maps: &FreeformCopyMaps,
    now: chrono::NaiveDateTime,
) -> Result<(), String>
where
    C: ConnectionTrait,
{
    let videos = generated_video::Entity::find()
        .filter(generated_video::Column::ProjectId.eq(source_project_id))
        .order_by_asc(generated_video::Column::Id)
        .all(db)
        .await
        .map_err(|err| err.to_string())?;
    for video in videos {
        let new_id = remap_required_id(&maps.videos.ids, video.id, "generated_video.id")?;
        generated_video::ActiveModel {
            id: Set(new_id),
            source_video_id: Set(remap_optional_id(
                &maps.videos.ids,
                video.source_video_id,
                "generated_video.source_video_id",
            )?),
            source_image_id: Set(remap_optional_id(
                &maps.images.ids,
                video.source_image_id,
                "generated_video.source_image_id",
            )?),
            updated_at: Set(now),
            ..Default::default()
        }
        .update(db)
        .await
        .map_err(|err| err.to_string())?;
    }
    Ok(())
}

async fn copy_freeform_generation_specs<C>(
    db: &C,
    source_project_id: i64,
    new_project_id: i64,
    text_asset_id_map: &HashMap<i64, i64>,
    now: chrono::NaiveDateTime,
) -> Result<CopiedFreeformGenerationSpecs, String>
where
    C: ConnectionTrait,
{
    let specs = generation_spec::Entity::find()
        .filter(generation_spec::Column::ProjectId.eq(source_project_id))
        .order_by_asc(generation_spec::Column::Id)
        .all(db)
        .await
        .map_err(|err| err.to_string())?;
    let mut spec_id_map = HashMap::new();
    let mut deferred_targets = Vec::new();
    for spec in &specs {
        if spec.source_snapshot_id.is_some()
            || spec.source_chapter_id.is_some()
            || spec.source_snapshot_chapter_id.is_some()
        {
            return Err(format!(
                "自由视频项目生成意图不能包含源快照引用: generation_spec_id={}",
                spec.id
            ));
        }
        let canvas_text_asset_id = remap_optional_id(
            text_asset_id_map,
            spec.canvas_text_asset_id,
            "generation_spec.canvas_text_asset_id",
        )?;
        let target_entity_id = match (spec.target_entity_kind.as_str(), spec.target_entity_id) {
            (
                "canvas_node" | "video_segment" | "assembly_timeline" | "media_asset",
                Some(old_id),
            ) => {
                deferred_targets.push(DeferredFreeformSpecTarget {
                    new_spec_id: 0,
                    target_entity_kind: spec.target_entity_kind.clone(),
                    old_target_entity_id: old_id,
                });
                None
            }
            _ => spec.target_entity_id,
        };
        let copied = generation_spec::ActiveModel {
            project_id: Set(new_project_id),
            root_project_id: Set(new_project_id),
            source_context_kind: Set(Some(SOURCE_MODE_FREEFORM.to_string())),
            source_snapshot_id: Set(None),
            source_chapter_id: Set(None),
            source_snapshot_chapter_id: Set(None),
            canvas_text_asset_id: Set(canvas_text_asset_id),
            target_kind: Set(spec.target_kind.clone()),
            target_entity_kind: Set(spec.target_entity_kind.clone()),
            target_entity_id: Set(target_entity_id),
            status: Set(spec.status.clone()),
            source_version_hash: Set(spec.source_version_hash.clone()),
            params_json: Set(spec.params_json.clone()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|err| err.to_string())?;
        if let Some(target) = deferred_targets.last_mut()
            && target.new_spec_id == 0
        {
            target.new_spec_id = copied.id;
        }
        spec_id_map.insert(spec.id, copied.id);
    }
    Ok(CopiedFreeformGenerationSpecs {
        spec_ids: specs.iter().map(|spec| spec.id).collect(),
        spec_id_map,
        deferred_targets,
    })
}

async fn copy_freeform_media_assets<C>(
    db: &C,
    source_project_id: i64,
    new_project_id: i64,
    now: chrono::NaiveDateTime,
    data_dir: &Path,
    copied_media_files: &mut Vec<PathBuf>,
) -> Result<HashMap<i64, i64>, String>
where
    C: ConnectionTrait,
{
    let assets = media_asset::Entity::find()
        .filter(media_asset::Column::ProjectId.eq(source_project_id))
        .order_by_asc(media_asset::Column::Id)
        .all(db)
        .await
        .map_err(|err| err.to_string())?;
    let mut asset_id_map = HashMap::new();
    for asset in assets {
        if asset.root_project_id != source_project_id
            || asset.source_chapter_id.is_some()
            || asset.source_snapshot_chapter_id.is_some()
        {
            return Err(format!(
                "自由视频项目媒体资产包含派生项目上下文: media_asset_id={}",
                asset.id
            ));
        }
        if asset.status == "completed" && asset.file_path.is_none() {
            return Err(format!("已完成媒体资产缺少 file_path: {}", asset.id));
        }
        let file_path = copy_media_file(
            data_dir,
            new_project_id,
            asset.id,
            "file",
            asset.file_path.as_deref(),
            copied_media_files,
        )?;
        let thumbnail_path = copy_media_file(
            data_dir,
            new_project_id,
            asset.id,
            "thumbnail",
            asset.thumbnail_path.as_deref(),
            copied_media_files,
        )?;
        let copied = media_asset::ActiveModel {
            project_id: Set(new_project_id),
            root_project_id: Set(new_project_id),
            source_chapter_id: Set(None),
            source_snapshot_chapter_id: Set(None),
            media_kind: Set(asset.media_kind),
            origin: Set(asset.origin),
            file_path: Set(file_path),
            thumbnail_path: Set(thumbnail_path),
            width: Set(asset.width),
            height: Set(asset.height),
            duration_secs: Set(asset.duration_secs),
            fps: Set(asset.fps),
            file_size: Set(asset.file_size),
            checksum: Set(asset.checksum),
            status: Set(asset.status),
            error_message: Set(asset.error_message),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|err| err.to_string())?;
        asset_id_map.insert(asset.id, copied.id);
    }
    Ok(asset_id_map)
}

async fn copy_freeform_video_segments<C>(
    db: &C,
    source_project_id: i64,
    new_project_id: i64,
    spec_id_map: &HashMap<i64, i64>,
    asset_id_map: &HashMap<i64, i64>,
    now: chrono::NaiveDateTime,
) -> Result<HashMap<i64, i64>, String>
where
    C: ConnectionTrait,
{
    let segments = video_segment::Entity::find()
        .filter(video_segment::Column::ProjectId.eq(source_project_id))
        .order_by_asc(video_segment::Column::OrderIndex)
        .order_by_asc(video_segment::Column::Id)
        .all(db)
        .await
        .map_err(|err| err.to_string())?;
    let mut segment_id_map = HashMap::new();
    for segment in &segments {
        if segment.root_project_id != source_project_id
            || segment.source_chapter_id.is_some()
            || segment.source_snapshot_chapter_id.is_some()
        {
            return Err(format!(
                "自由视频项目片段包含派生项目上下文: video_segment_id={}",
                segment.id
            ));
        }
        let copied = video_segment::ActiveModel {
            project_id: Set(new_project_id),
            root_project_id: Set(new_project_id),
            source_chapter_id: Set(None),
            source_snapshot_chapter_id: Set(None),
            segment_type: Set(segment.segment_type.clone()),
            order_index: Set(segment.order_index),
            title: Set(segment.title.clone()),
            duration_secs: Set(segment.duration_secs),
            source_generation_spec_id: Set(remap_optional_id(
                spec_id_map,
                segment.source_generation_spec_id,
                "video_segment.source_generation_spec_id",
            )?),
            result_media_asset_id: Set(remap_optional_id(
                asset_id_map,
                segment.result_media_asset_id,
                "video_segment.result_media_asset_id",
            )?),
            previous_segment_id: Set(None),
            next_segment_id: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|err| err.to_string())?;
        segment_id_map.insert(segment.id, copied.id);
    }
    for segment in &segments {
        let new_id = remap_required_id(&segment_id_map, segment.id, "video_segment.id")?;
        video_segment::ActiveModel {
            id: Set(new_id),
            previous_segment_id: Set(remap_optional_id(
                &segment_id_map,
                segment.previous_segment_id,
                "video_segment.previous_segment_id",
            )?),
            next_segment_id: Set(remap_optional_id(
                &segment_id_map,
                segment.next_segment_id,
                "video_segment.next_segment_id",
            )?),
            updated_at: Set(now),
            ..Default::default()
        }
        .update(db)
        .await
        .map_err(|err| err.to_string())?;
    }
    Ok(segment_id_map)
}

async fn copy_freeform_assembly_timelines<C>(
    db: &C,
    source_project_id: i64,
    new_project_id: i64,
    asset_id_map: &HashMap<i64, i64>,
    now: chrono::NaiveDateTime,
) -> Result<HashMap<i64, i64>, String>
where
    C: ConnectionTrait,
{
    let timelines = assembly_timeline::Entity::find()
        .filter(assembly_timeline::Column::ProjectId.eq(source_project_id))
        .order_by_asc(assembly_timeline::Column::Id)
        .all(db)
        .await
        .map_err(|err| err.to_string())?;
    let mut timeline_id_map = HashMap::new();
    for timeline in timelines {
        if timeline.source_chapter_id.is_some() || timeline.source_snapshot_chapter_id.is_some() {
            return Err(format!(
                "自由视频项目成片时间线包含派生项目上下文: assembly_timeline_id={}",
                timeline.id
            ));
        }
        let copied = assembly_timeline::ActiveModel {
            project_id: Set(new_project_id),
            source_chapter_id: Set(None),
            source_snapshot_chapter_id: Set(None),
            version: Set(timeline.version),
            status: Set(timeline.status),
            preview_media_asset_id: Set(remap_optional_id(
                asset_id_map,
                timeline.preview_media_asset_id,
                "assembly_timeline.preview_media_asset_id",
            )?),
            preview_cache_key: Set(timeline.preview_cache_key),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|err| err.to_string())?;
        timeline_id_map.insert(timeline.id, copied.id);
    }
    Ok(timeline_id_map)
}

async fn copy_canvas_graphs<C>(
    db: &C,
    source_project_id: i64,
    new_project_id: i64,
    maps: &FreeformCopyMaps,
    now: chrono::NaiveDateTime,
) -> Result<CanvasCopyMaps, String>
where
    C: ConnectionTrait,
{
    let mut canvas_maps = CanvasCopyMaps::default();
    let mut copied_nodes = Vec::new();
    let graphs = canvas_graph::Entity::find()
        .filter(canvas_graph::Column::ProjectId.eq(source_project_id))
        .order_by_asc(canvas_graph::Column::Id)
        .all(db)
        .await
        .map_err(|err| err.to_string())?;
    for graph in graphs {
        let new_graph = canvas_graph::ActiveModel {
            project_id: Set(new_project_id),
            name: Set(graph.name),
            graph_kind: Set(graph.graph_kind),
            version: Set(graph.version),
            viewport_json: Set(graph.viewport_json),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|err| err.to_string())?;
        canvas_maps.graph_ids.insert(graph.id, new_graph.id);
        let nodes = canvas_node::Entity::find()
            .filter(canvas_node::Column::GraphId.eq(graph.id))
            .order_by_asc(canvas_node::Column::Id)
            .all(db)
            .await
            .map_err(|err| err.to_string())?;
        let mut node_id_map = HashMap::new();
        for node in nodes {
            let row = canvas_node::ActiveModel {
                graph_id: Set(new_graph.id),
                node_kind: Set(node.node_kind.clone()),
                ref_type: Set(node.ref_type.clone()),
                ref_id: Set(node.ref_id),
                config_json: Set(node.config_json.clone()),
                position_json: Set(node.position_json.clone()),
                state: Set(node.state.clone()),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(db)
            .await
            .map_err(|err| err.to_string())?;
            node_id_map.insert(node.id, row.id);
            canvas_maps.node_ids.insert(node.id, row.id);
            copied_nodes.push((node, row.id));
        }
        let edges = canvas_edge::Entity::find()
            .filter(canvas_edge::Column::GraphId.eq(graph.id))
            .order_by_asc(canvas_edge::Column::Id)
            .all(db)
            .await
            .map_err(|err| err.to_string())?;
        for edge in edges {
            canvas_edge::ActiveModel {
                graph_id: Set(new_graph.id),
                source_node_id: Set(remap_required_id(
                    &node_id_map,
                    edge.source_node_id,
                    "canvas_edge.source_node_id",
                )?),
                source_port: Set(edge.source_port),
                target_node_id: Set(remap_required_id(
                    &node_id_map,
                    edge.target_node_id,
                    "canvas_edge.target_node_id",
                )?),
                target_port: Set(edge.target_port),
                role: Set(edge.role),
                config_json: Set(edge.config_json),
                ..Default::default()
            }
            .insert(db)
            .await
            .map_err(|err| err.to_string())?;
        }
    }

    let mut full_maps = maps.clone();
    full_maps.graph_ids = canvas_maps.graph_ids.clone();
    full_maps.node_ids = canvas_maps.node_ids.clone();
    for (source_node, copied_node_id) in copied_nodes {
        let (ref_type, ref_id, config_json) = remap_canvas_node_refs(
            source_node.ref_type,
            source_node.ref_id,
            source_node.config_json,
            &full_maps,
            Some(&canvas_maps),
        )?;
        canvas_node::ActiveModel {
            id: Set(copied_node_id),
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
    Ok(canvas_maps)
}

fn remap_canvas_node_refs(
    ref_type: Option<String>,
    ref_id: Option<i64>,
    mut config_json: Value,
    maps: &FreeformCopyMaps,
    canvas_maps: Option<&CanvasCopyMaps>,
) -> Result<(Option<String>, Option<i64>, Value), String> {
    remap_freeform_canvas_json_ids(&mut config_json, maps, canvas_maps)?;
    let Some(kind) = ref_type.as_deref() else {
        return Ok((ref_type, ref_id, config_json));
    };
    match kind {
        "inline" | "config" => Ok((ref_type, ref_id, config_json)),
        "legacy_generated_image" => {
            let old_id = required_canvas_ref_id(ref_id, kind)?;
            let new_id = remap_canvas_ref_id(&maps.images.ids, old_id, kind)?;
            if let Some(file_path) = maps.images.file_paths.get(&old_id) {
                set_canvas_node_config_string(&mut config_json, "file_path", file_path);
            }
            Ok((ref_type, Some(new_id), config_json))
        }
        "legacy_generated_video" => {
            let old_id = required_canvas_ref_id(ref_id, kind)?;
            let new_id = remap_canvas_ref_id(&maps.videos.ids, old_id, kind)?;
            if let Some(file_path) = maps
                .videos
                .file_paths
                .get(&old_id)
                .and_then(|value| value.as_ref())
            {
                set_canvas_node_config_string(&mut config_json, "file_path", file_path);
            }
            if let Some(thumbnail_path) = maps
                .videos
                .thumbnail_paths
                .get(&old_id)
                .and_then(|value| value.as_ref())
            {
                set_canvas_node_config_string(&mut config_json, "thumbnail_path", thumbnail_path);
            }
            Ok((ref_type, Some(new_id), config_json))
        }
        "canvas_text_asset" => {
            let old_id = required_canvas_ref_id(ref_id, kind)?;
            let new_id = remap_canvas_ref_id(&maps.text_asset_ids, old_id, kind)?;
            Ok((ref_type, Some(new_id), config_json))
        }
        "generation_spec" => {
            let old_id = required_canvas_ref_id(ref_id, kind)?;
            let new_id = remap_canvas_ref_id(&maps.generation_spec_ids, old_id, kind)?;
            Ok((ref_type, Some(new_id), config_json))
        }
        "prompt_artifact" => {
            let old_id = required_canvas_ref_id(ref_id, kind)?;
            let new_id = remap_canvas_ref_id(&maps.prompt_artifact_ids, old_id, kind)?;
            Ok((ref_type, Some(new_id), config_json))
        }
        "media_asset" => {
            let old_id = required_canvas_ref_id(ref_id, kind)?;
            let new_id = remap_canvas_ref_id(&maps.media_asset_ids, old_id, kind)?;
            Ok((ref_type, Some(new_id), config_json))
        }
        "video_segment" => {
            let old_id = required_canvas_ref_id(ref_id, kind)?;
            let new_id = remap_canvas_ref_id(&maps.video_segment_ids, old_id, kind)?;
            Ok((ref_type, Some(new_id), config_json))
        }
        "assembly_timeline" => {
            let old_id = required_canvas_ref_id(ref_id, kind)?;
            let new_id = remap_canvas_ref_id(&maps.assembly_timeline_ids, old_id, kind)?;
            Ok((ref_type, Some(new_id), config_json))
        }
        "legacy_chapter_video_prompt" => Err(format!(
            "自由视频项目复制暂不支持重映射画布节点 {kind} 引用，不能保留源项目 ref_id: {}",
            ref_id
                .map(|value| value.to_string())
                .unwrap_or_else(|| "NULL".to_string())
        )),
        other => Err(format!("自由视频项目复制遇到未知画布节点引用类型: {other}")),
    }
}

fn required_canvas_ref_id(ref_id: Option<i64>, ref_type: &str) -> Result<i64, String> {
    ref_id.ok_or_else(|| format!("画布节点 {ref_type} 引用缺少 ref_id"))
}

fn remap_canvas_ref_id(
    map: &HashMap<i64, i64>,
    old_id: i64,
    ref_type: &str,
) -> Result<i64, String> {
    map.get(&old_id)
        .copied()
        .ok_or_else(|| format!("复制自由视频项目时无法重映射画布节点 {ref_type} 引用: {old_id}"))
}

fn set_canvas_node_config_string(config_json: &mut Value, key: &str, value: &str) {
    if let Value::Object(object) = config_json {
        object.insert(key.to_string(), Value::String(value.to_string()));
    }
}

async fn update_freeform_generation_spec_deferred_targets<C>(
    db: &C,
    targets: &[DeferredFreeformSpecTarget],
    maps: &FreeformCopyMaps,
    now: chrono::NaiveDateTime,
) -> Result<(), String>
where
    C: ConnectionTrait,
{
    for target in targets {
        let target_entity_id = match target.target_entity_kind.as_str() {
            "canvas_node" => remap_required_id(
                &maps.node_ids,
                target.old_target_entity_id,
                "generation_spec.target_entity_id",
            )?,
            "video_segment" => remap_required_id(
                &maps.video_segment_ids,
                target.old_target_entity_id,
                "generation_spec.target_entity_id",
            )?,
            "assembly_timeline" => remap_required_id(
                &maps.assembly_timeline_ids,
                target.old_target_entity_id,
                "generation_spec.target_entity_id",
            )?,
            "media_asset" => remap_required_id(
                &maps.media_asset_ids,
                target.old_target_entity_id,
                "generation_spec.target_entity_id",
            )?,
            other => {
                return Err(format!(
                    "自由视频项目生成意图暂不支持重映射 target_entity_kind={other}"
                ));
            }
        };
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

async fn update_freeform_generation_spec_metadata<C>(
    db: &C,
    spec_id_map: &HashMap<i64, i64>,
    maps: &FreeformCopyMaps,
    now: chrono::NaiveDateTime,
) -> Result<(), String>
where
    C: ConnectionTrait,
{
    for new_spec_id in spec_id_map.values().copied() {
        let spec = generation_spec::Entity::find_by_id(new_spec_id)
            .one(db)
            .await
            .map_err(|err| err.to_string())?
            .ok_or_else(|| format!("复制后的生成意图不存在: {new_spec_id}"))?;
        let params_json = remap_freeform_metadata(spec.params_json, maps)?;
        generation_spec::ActiveModel {
            id: Set(new_spec_id),
            params_json: Set(params_json),
            updated_at: Set(now),
            ..Default::default()
        }
        .update(db)
        .await
        .map_err(|err| err.to_string())?;
    }
    Ok(())
}

async fn copy_freeform_media_provenance<C>(
    db: &C,
    asset_id_map: &HashMap<i64, i64>,
    maps: &FreeformCopyMaps,
    now: chrono::NaiveDateTime,
) -> Result<(), String>
where
    C: ConnectionTrait,
{
    if asset_id_map.is_empty() {
        return Ok(());
    }
    let records = media_provenance::Entity::find()
        .filter(media_provenance::Column::MediaAssetId.is_in(asset_id_map.keys().copied()))
        .all(db)
        .await
        .map_err(|err| err.to_string())?;
    for item in records {
        let request_snapshot_json = item
            .request_snapshot_json
            .map(|value| remap_freeform_metadata(value, maps))
            .transpose()?;
        media_provenance::ActiveModel {
            media_asset_id: Set(remap_required_id(
                asset_id_map,
                item.media_asset_id,
                "media_provenance.media_asset_id",
            )?),
            generation_spec_id: Set(remap_optional_id(
                &maps.generation_spec_ids,
                item.generation_spec_id,
                "media_provenance.generation_spec_id",
            )?),
            prompt_artifact_id: Set(remap_optional_id(
                &maps.prompt_artifact_ids,
                item.prompt_artifact_id,
                "media_provenance.prompt_artifact_id",
            )?),
            provider_job_id: Set(item.provider_job_id),
            source_media_asset_id: Set(remap_optional_id(
                &maps.media_asset_ids,
                item.source_media_asset_id,
                "media_provenance.source_media_asset_id",
            )?),
            source_external_path: Set(remap_source_external_path(
                item.source_external_path.as_deref(),
                maps,
            )?),
            imported_from_user: Set(item.imported_from_user),
            request_snapshot_json: Set(request_snapshot_json),
            created_at: Set(now),
        }
        .insert(db)
        .await
        .map_err(|err| err.to_string())?;
    }
    Ok(())
}

async fn update_freeform_generated_media_metadata<C>(
    db: &C,
    source_project_id: i64,
    maps: &FreeformCopyMaps,
    now: chrono::NaiveDateTime,
) -> Result<(), String>
where
    C: ConnectionTrait,
{
    let images = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.eq(source_project_id))
        .all(db)
        .await
        .map_err(|err| err.to_string())?;
    for image in images {
        let Some(metadata) = image.metadata_json else {
            continue;
        };
        let metadata = remap_freeform_metadata(metadata, maps)?;
        generated_image::ActiveModel {
            id: Set(remap_required_id(
                &maps.images.ids,
                image.id,
                "generated_image.id",
            )?),
            metadata_json: Set(Some(metadata)),
            ..Default::default()
        }
        .update(db)
        .await
        .map_err(|err| err.to_string())?;
    }

    let videos = generated_video::Entity::find()
        .filter(generated_video::Column::ProjectId.eq(source_project_id))
        .all(db)
        .await
        .map_err(|err| err.to_string())?;
    for video in videos {
        let Some(metadata) = video.metadata_json else {
            continue;
        };
        let metadata = remap_freeform_metadata(metadata, maps)?;
        generated_video::ActiveModel {
            id: Set(remap_required_id(
                &maps.videos.ids,
                video.id,
                "generated_video.id",
            )?),
            metadata_json: Set(Some(metadata)),
            updated_at: Set(now),
            ..Default::default()
        }
        .update(db)
        .await
        .map_err(|err| err.to_string())?;
    }
    Ok(())
}

async fn update_freeform_assembly_timeline_cache_keys<C>(
    db: &C,
    timeline_id_map: &HashMap<i64, i64>,
    maps: &FreeformCopyMaps,
    now: chrono::NaiveDateTime,
) -> Result<(), String>
where
    C: ConnectionTrait,
{
    for new_timeline_id in timeline_id_map.values().copied() {
        let timeline = assembly_timeline::Entity::find_by_id(new_timeline_id)
            .one(db)
            .await
            .map_err(|err| err.to_string())?
            .ok_or_else(|| format!("复制后的成片时间线不存在: {new_timeline_id}"))?;
        let preview_cache_key =
            remap_canvas_cache_key(timeline.preview_cache_key.as_deref(), maps)?;
        if preview_cache_key == timeline.preview_cache_key {
            continue;
        }
        assembly_timeline::ActiveModel {
            id: Set(new_timeline_id),
            preview_cache_key: Set(preview_cache_key),
            updated_at: Set(now),
            ..Default::default()
        }
        .update(db)
        .await
        .map_err(|err| err.to_string())?;
    }
    Ok(())
}

fn remap_freeform_metadata(mut value: Value, maps: &FreeformCopyMaps) -> Result<Value, String> {
    remap_freeform_canvas_json_ids(&mut value, maps, None)?;
    if let Some(canvas) = value.get_mut("canvas") {
        remap_freeform_canvas_json_ids(canvas, maps, None)?;
    }
    Ok(value)
}

fn remap_freeform_canvas_json_ids(
    value: &mut Value,
    maps: &FreeformCopyMaps,
    canvas_maps: Option<&CanvasCopyMaps>,
) -> Result<(), String> {
    let Some(object) = value.as_object_mut() else {
        return Ok(());
    };
    remap_json_i64_field(object, "canvas_text_asset_id", &maps.text_asset_ids)?;
    remap_json_i64_field(object, "generation_spec_id", &maps.generation_spec_ids)?;
    remap_json_i64_field(object, "prompt_artifact_id", &maps.prompt_artifact_ids)?;
    remap_json_i64_field(object, "generated_image_id", &maps.images.ids)?;
    remap_json_i64_field(object, "generated_video_id", &maps.videos.ids)?;
    remap_json_i64_field(object, "image_id", &maps.images.ids)?;
    remap_json_i64_field(object, "video_id", &maps.videos.ids)?;
    remap_json_i64_array_field(object, "video_ids", &maps.videos.ids)?;
    remap_json_i64_field(object, "source_video_id", &maps.videos.ids)?;
    remap_json_i64_field(object, "source_image_id", &maps.images.ids)?;
    remap_json_i64_field(object, "first_frame_image_id", &maps.images.ids)?;
    remap_json_i64_array_field(object, "reference_image_ids", &maps.images.ids)?;
    remap_json_i64_field(object, "media_asset_id", &maps.media_asset_ids)?;
    remap_json_i64_array_field(object, "media_asset_ids", &maps.media_asset_ids)?;
    remap_json_i64_field(object, "preview_media_asset_id", &maps.media_asset_ids)?;
    remap_json_i64_field(object, "video_segment_id", &maps.video_segment_ids)?;
    remap_json_i64_array_field(object, "video_segment_ids", &maps.video_segment_ids)?;
    remap_json_i64_field(object, "source_segment_id", &maps.video_segment_ids)?;
    remap_json_i64_field(object, "assembly_timeline_id", &maps.assembly_timeline_ids)?;
    remap_json_i64_array_field(object, "assembly_clip_ids", &maps.assembly_clip_ids)?;
    if let Some(canvas_maps) = canvas_maps {
        remap_json_i64_field(object, "graph_id", &canvas_maps.graph_ids)?;
        remap_json_i64_field(object, "canvas_graph_id", &canvas_maps.graph_ids)?;
        remap_json_i64_field(object, "node_id", &canvas_maps.node_ids)?;
        remap_json_i64_field(object, "canvas_node_id", &canvas_maps.node_ids)?;
    } else {
        remap_json_i64_field(object, "graph_id", &maps.graph_ids)?;
        remap_json_i64_field(object, "canvas_graph_id", &maps.graph_ids)?;
        remap_json_i64_field(object, "node_id", &maps.node_ids)?;
        remap_json_i64_field(object, "canvas_node_id", &maps.node_ids)?;
    }
    Ok(())
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
        .ok_or_else(|| format!("复制自由视频项目字段 {key} 必须是整数 ID"))?;
    let new_id = map
        .get(&old_id)
        .copied()
        .ok_or_else(|| format!("复制自由视频项目时无法重映射 {key}: {old_id}"))?;
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
        .ok_or_else(|| format!("复制自由视频项目字段 {key} 必须是整数 ID 数组"))?;
    let remapped = values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let old_id = value
                .as_i64()
                .ok_or_else(|| format!("复制自由视频项目字段 {key}[{index}] 必须是整数 ID"))?;
            let new_id = map
                .get(&old_id)
                .copied()
                .ok_or_else(|| format!("复制自由视频项目时无法重映射 {key}: {old_id}"))?;
            Ok(Value::Number(new_id.into()))
        })
        .collect::<Result<Vec<_>, String>>()?;
    object.insert(key.to_string(), Value::Array(remapped));
    Ok(())
}

fn remap_source_external_path(
    value: Option<&str>,
    maps: &FreeformCopyMaps,
) -> Result<Option<String>, String> {
    let Some(raw) = value else {
        return Ok(None);
    };
    if let Some(id) = raw
        .strip_prefix("generated_video:")
        .and_then(|suffix| suffix.parse::<i64>().ok())
    {
        let new_id = maps.videos.ids.get(&id).copied().ok_or_else(|| {
            format!("复制自由视频项目时无法重映射 media_provenance.source_external_path: {id}")
        })?;
        return Ok(Some(format!("generated_video:{new_id}")));
    }
    if let Some(id) = raw
        .strip_prefix("generated_image:")
        .and_then(|suffix| suffix.parse::<i64>().ok())
    {
        let new_id = maps.images.ids.get(&id).copied().ok_or_else(|| {
            format!("复制自由视频项目时无法重映射 media_provenance.source_external_path: {id}")
        })?;
        return Ok(Some(format!("generated_image:{new_id}")));
    }
    Ok(Some(raw.to_string()))
}

fn remap_canvas_cache_key(
    value: Option<&str>,
    maps: &FreeformCopyMaps,
) -> Result<Option<String>, String> {
    let Some(raw) = value else {
        return Ok(None);
    };
    let parts = raw.split(':').collect::<Vec<_>>();
    if parts.len() == 4
        && parts[0] == "canvas_graph"
        && parts[2] == "node"
        && let (Ok(graph_id), Ok(node_id)) = (parts[1].parse::<i64>(), parts[3].parse::<i64>())
    {
        let new_graph_id = maps.graph_ids.get(&graph_id).copied().ok_or_else(|| {
            format!("复制自由视频项目时无法重映射 assembly_timeline.preview_cache_key graph_id: {graph_id}")
        })?;
        let new_node_id = maps.node_ids.get(&node_id).copied().ok_or_else(|| {
            format!("复制自由视频项目时无法重映射 assembly_timeline.preview_cache_key node_id: {node_id}")
        })?;
        return Ok(Some(format!(
            "canvas_graph:{new_graph_id}:node:{new_node_id}"
        )));
    }
    Ok(Some(raw.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn remap_freeform_canvas_legacy_image_ref_updates_id_and_file_path() {
        let mut copied_images = CopiedImageRecords::default();
        copied_images.ids.insert(10, 20);
        copied_images
            .file_paths
            .insert(10, "projects/20/media/duplicated/10_image.png".to_string());
        let maps = FreeformCopyMaps {
            images: copied_images,
            ..Default::default()
        };

        let (_ref_type, ref_id, config_json) = remap_canvas_node_refs(
            Some("legacy_generated_image".to_string()),
            Some(10),
            json!({"file_path": "projects/10/images/source.png"}),
            &maps,
            None,
        )
        .expect("legacy generated image ref should be remapped");

        assert_eq!(ref_id, Some(20));
        assert_eq!(
            config_json["file_path"],
            json!("projects/20/media/duplicated/10_image.png")
        );
    }

    #[test]
    fn remap_freeform_canvas_text_asset_ref_requires_mapping() {
        let err = remap_canvas_node_refs(
            Some("canvas_text_asset".to_string()),
            Some(33),
            json!({}),
            &FreeformCopyMaps::default(),
            None,
        )
        .expect_err("text asset ref must not keep the source project id");

        assert!(err.contains("无法重映射画布节点 canvas_text_asset 引用: 33"));
    }

    #[test]
    fn remap_freeform_canvas_canonical_refs_update_id_and_config() {
        let mut maps = FreeformCopyMaps::default();
        maps.media_asset_ids.insert(44, 144);
        maps.video_segment_ids.insert(45, 145);
        maps.assembly_timeline_ids.insert(46, 146);
        let (_ref_type, ref_id, config_json) = remap_canvas_node_refs(
            Some("media_asset".to_string()),
            Some(44),
            json!({
                "media_asset_id": 44,
                "video_segment_ids": [45],
                "assembly_timeline_id": 46
            }),
            &maps,
            None,
        )
        .expect("canonical media refs should be remapped");

        assert_eq!(ref_id, Some(144));
        assert_eq!(config_json["media_asset_id"], json!(144));
        assert_eq!(config_json["video_segment_ids"], json!([145]));
        assert_eq!(config_json["assembly_timeline_id"], json!(146));
    }

    #[test]
    fn remap_freeform_metadata_rejects_missing_project_local_id() {
        let err = remap_freeform_metadata(
            json!({ "media_asset_id": 999 }),
            &FreeformCopyMaps::default(),
        )
        .expect_err("copy metadata must not keep source project ids");

        assert!(err.contains("media_asset_id"));
        assert!(err.contains("999"));
    }

    #[test]
    fn remap_freeform_metadata_rejects_non_integer_id_array() {
        let mut maps = FreeformCopyMaps::default();
        maps.video_segment_ids.insert(45, 145);

        let err = remap_freeform_metadata(json!({ "video_segment_ids": [45, "stale"] }), &maps)
            .expect_err("copy metadata arrays must contain only integer ids");

        assert!(err.contains("video_segment_ids[1]"));
        assert!(err.contains("整数 ID"));
    }

    #[test]
    fn remap_freeform_metadata_remaps_nested_canvas_ids() {
        let mut maps = FreeformCopyMaps::default();
        maps.media_asset_ids.insert(44, 144);
        maps.graph_ids.insert(50, 150);
        maps.node_ids.insert(51, 151);

        let metadata = remap_freeform_metadata(
            json!({
                "media_asset_id": 44,
                "canvas": {
                    "graph_id": 50,
                    "node_id": 51
                }
            }),
            &maps,
        )
        .expect("known project-local ids should remap");

        assert_eq!(metadata["media_asset_id"], json!(144));
        assert_eq!(metadata["canvas"]["graph_id"], json!(150));
        assert_eq!(metadata["canvas"]["node_id"], json!(151));
    }

    #[test]
    fn remap_source_external_path_rejects_missing_generated_ref() {
        let err =
            remap_source_external_path(Some("generated_video:99"), &FreeformCopyMaps::default())
                .expect_err("source external path must not keep stale generated video ids");

        assert!(err.contains("media_provenance.source_external_path"));
        assert!(err.contains("99"));
    }

    #[test]
    fn remap_canvas_cache_key_rejects_missing_node_mapping() {
        let mut maps = FreeformCopyMaps::default();
        maps.graph_ids.insert(10, 110);

        let err = remap_canvas_cache_key(Some("canvas_graph:10:node:20"), &maps)
            .expect_err("cache key must not keep stale canvas node ids");

        assert!(err.contains("assembly_timeline.preview_cache_key node_id"));
        assert!(err.contains("20"));
    }
}
