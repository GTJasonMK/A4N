use super::*;

pub(super) async fn export_project_family_package_bytes(
    state: &AppState,
    root_project_id: i64,
    include_derived_projects: bool,
) -> Result<Vec<u8>, String> {
    let (root, derived_projects) =
        load_project_family_export_selection(state, root_project_id, include_derived_projects)
            .await?;
    let package = build_export_package(state, root, derived_projects, "project_family").await?;
    package_zip_bytes(state, &package)
}

pub(super) async fn export_derived_project_package_bytes(
    state: &AppState,
    derived_project_id: i64,
) -> Result<Vec<u8>, String> {
    let (root, derived_projects) =
        load_derived_project_export_selection(state, derived_project_id).await?;
    let package = build_export_package(state, root, derived_projects, "derived_project").await?;
    package_zip_bytes(state, &package)
}

async fn load_project_family_export_selection(
    state: &AppState,
    root_project_id: i64,
    include_derived_projects: bool,
) -> Result<(novel_project::Model, Vec<novel_project::Model>), String> {
    let root = novel_project::Entity::find_by_id(root_project_id)
        .one(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("根文本项目不存在: {root_project_id}"))?;
    if root.project_kind != PROJECT_KIND_TEXT {
        return Err("项目族导出的根项目必须是文本项目".to_string());
    }
    let derived_projects = if include_derived_projects {
        novel_project::Entity::find()
            .filter(novel_project::Column::RootProjectId.eq(root.id))
            .order_by_asc(novel_project::Column::Id)
            .all(state.db.as_ref())
            .await
            .map_err(|err| err.to_string())?
    } else {
        Vec::new()
    };
    Ok((root, derived_projects))
}

async fn load_derived_project_export_selection(
    state: &AppState,
    derived_project_id: i64,
) -> Result<(novel_project::Model, Vec<novel_project::Model>), String> {
    let derived = novel_project::Entity::find_by_id(derived_project_id)
        .one(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("派生项目不存在: {derived_project_id}"))?;
    if derived.project_kind == PROJECT_KIND_TEXT {
        return Err("派生项目导出不能传入文本项目".to_string());
    }
    let root_project_id = derived
        .root_project_id
        .ok_or_else(|| format!("派生项目缺少 root_project_id: {derived_project_id}"))?;
    let root = novel_project::Entity::find_by_id(root_project_id)
        .one(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())?
        .ok_or_else(|| format!("派生项目引用的根文本项目不存在: {root_project_id}"))?;
    if root.project_kind != PROJECT_KIND_TEXT {
        return Err("派生项目引用的根项目不是文本项目".to_string());
    }
    Ok((root, vec![derived]))
}

async fn build_export_package(
    state: &AppState,
    root: novel_project::Model,
    derived_projects: Vec<novel_project::Model>,
    mode: &str,
) -> Result<ProjectFamilyPackage, String> {
    validate_same_user(&root, &derived_projects)?;
    let derived_project_ids = derived_projects
        .iter()
        .map(|project| project.id)
        .collect::<Vec<_>>();
    let mut projects = Vec::with_capacity(1 + derived_projects.len());
    projects.push(root.clone());
    projects.extend(derived_projects.clone());

    let text_core = load_text_core(state, root.id).await?;
    let project_derivations = load_project_derivations(state, &derived_project_ids).await?;
    let source_sync_runs = load_source_sync_runs(state, &derived_project_ids).await?;
    let snapshot_ids =
        collect_snapshot_ids(&derived_projects, &project_derivations, &source_sync_runs);
    let source_snapshots = load_source_snapshots(state, &snapshot_ids).await?;
    validate_snapshot_coverage(&snapshot_ids, &source_snapshots)?;
    let source_snapshot_chapters = load_source_snapshot_chapters(state, &snapshot_ids).await?;
    let source_snapshot_outlines = load_source_snapshot_outlines(state, &snapshot_ids).await?;
    let source_snapshot_profiles = load_source_snapshot_profiles(state, &snapshot_ids).await?;
    let text_anchors = load_text_anchors(state, root.id, &snapshot_ids).await?;
    let profile_evidence = load_profile_evidence(state, &text_anchors).await?;

    let generation_specs = load_generation_specs(state, &derived_project_ids).await?;
    let generation_spec_ids = generation_specs
        .iter()
        .map(|record| record.id)
        .collect::<Vec<_>>();
    let generation_spec_text_anchors =
        load_generation_spec_text_anchors(state, &generation_spec_ids).await?;
    let generation_spec_entity_refs =
        load_generation_spec_entity_refs(state, &generation_spec_ids).await?;
    let prompt_artifacts = load_prompt_artifacts(state, &generation_spec_ids).await?;

    let media_assets = load_media_assets(state, &derived_project_ids).await?;
    let media_asset_ids = media_assets
        .iter()
        .map(|record| record.id)
        .collect::<Vec<_>>();
    let generation_spec_media_refs =
        load_generation_spec_media_refs(state, &generation_spec_ids, &media_asset_ids).await?;
    let media_provenance = load_media_provenance(state, &media_asset_ids).await?;
    let media_bindings = load_media_bindings(state, &media_asset_ids).await?;

    let galgame_scripts = load_galgame_scripts(state, &derived_project_ids).await?;
    let galgame_script_ids = galgame_scripts
        .iter()
        .map(|record| record.id)
        .collect::<Vec<_>>();
    let galgame_scenes = load_galgame_scenes(state, &galgame_script_ids).await?;
    let galgame_scene_ids = galgame_scenes
        .iter()
        .map(|record| record.id)
        .collect::<Vec<_>>();
    let galgame_lines = load_galgame_lines(state, &galgame_scene_ids).await?;
    let galgame_line_ids = galgame_lines
        .iter()
        .map(|record| record.id)
        .collect::<Vec<_>>();
    let galgame_choices = load_galgame_choices(state, &galgame_line_ids).await?;

    let video_segments = load_video_segments(state, &derived_project_ids).await?;
    let assembly_timelines = load_assembly_timelines(state, &derived_project_ids).await?;
    let timeline_ids = assembly_timelines
        .iter()
        .map(|record| record.id)
        .collect::<Vec<_>>();
    let assembly_clips = load_assembly_clips(state, &timeline_ids).await?;

    let sync_run_ids = source_sync_runs
        .iter()
        .map(|record| record.id)
        .collect::<Vec<_>>();
    let source_sync_conflicts = load_source_sync_conflicts(state, &sync_run_ids).await?;
    let conflict_ids = source_sync_conflicts
        .iter()
        .map(|record| record.id)
        .collect::<Vec<_>>();
    let source_sync_decisions = load_source_sync_decisions(state, &conflict_ids).await?;

    let media_files = collect_media_files(state, &media_assets)?;
    Ok(ProjectFamilyPackage {
        version: PROJECT_FAMILY_PACKAGE_VERSION.to_string(),
        exported_at: Utc::now().naive_utc().to_string(),
        mode: mode.to_string(),
        root_project_id: root.id,
        projects,
        text_core,
        project_derivations,
        source_snapshots,
        source_snapshot_chapters,
        source_snapshot_outlines,
        source_snapshot_profiles,
        text_anchors,
        profile_evidence,
        generation_specs,
        generation_spec_text_anchors,
        generation_spec_entity_refs,
        generation_spec_media_refs,
        prompt_artifacts,
        media_assets,
        media_provenance,
        media_bindings,
        galgame_scripts,
        galgame_scenes,
        galgame_lines,
        galgame_choices,
        video_segments,
        assembly_timelines,
        assembly_clips,
        source_sync_runs,
        source_sync_conflicts,
        source_sync_decisions,
        media_files,
    })
}

fn validate_same_user(
    root: &novel_project::Model,
    derived_projects: &[novel_project::Model],
) -> Result<(), String> {
    for project in derived_projects {
        if project.user_id != root.user_id {
            return Err(format!(
                "派生项目所属用户与根文本项目不一致: {}",
                project.id
            ));
        }
        if project.root_project_id != Some(root.id) {
            return Err(format!("派生项目不属于根文本项目: {}", project.id));
        }
    }
    Ok(())
}

macro_rules! load_by_ids {
    ($fn_name:ident, $entity:path, $filter_col:expr $(, $order_col:expr)* $(,)?) => {
        async fn $fn_name(
            state: &AppState,
            ids: &[i64],
        ) -> Result<Vec<<$entity as EntityTrait>::Model>, String> {
            if ids.is_empty() {
                return Ok(Vec::new());
            }
            let query = <$entity as EntityTrait>::find().filter($filter_col.is_in(ids.to_vec()));
            $(
                let query = query.order_by_asc($order_col);
            )*
            query
                .all(state.db.as_ref())
                .await
                .map_err(|err| err.to_string())
        }
    };
}

async fn load_text_core(state: &AppState, root_project_id: i64) -> Result<TextCorePackage, String> {
    Ok(TextCorePackage {
        blueprint: novel_blueprint::Entity::find()
            .filter(novel_blueprint::Column::ProjectId.eq(root_project_id))
            .one(state.db.as_ref())
            .await
            .map_err(|err| err.to_string())?,
        blueprint_versions: blueprint_version::Entity::find()
            .filter(blueprint_version::Column::ProjectId.eq(root_project_id))
            .order_by_asc(blueprint_version::Column::Id)
            .all(state.db.as_ref())
            .await
            .map_err(|err| err.to_string())?,
        blueprint_characters: blueprint_character::Entity::find()
            .filter(blueprint_character::Column::ProjectId.eq(root_project_id))
            .order_by_asc(blueprint_character::Column::Id)
            .all(state.db.as_ref())
            .await
            .map_err(|err| err.to_string())?,
        blueprint_relationships: blueprint_relationship::Entity::find()
            .filter(blueprint_relationship::Column::ProjectId.eq(root_project_id))
            .order_by_asc(blueprint_relationship::Column::Id)
            .all(state.db.as_ref())
            .await
            .map_err(|err| err.to_string())?,
        part_outlines: part_outline::Entity::find()
            .filter(part_outline::Column::ProjectId.eq(root_project_id))
            .order_by_asc(part_outline::Column::PartNumber)
            .all(state.db.as_ref())
            .await
            .map_err(|err| err.to_string())?,
        chapter_outlines: chapter_outline::Entity::find()
            .filter(chapter_outline::Column::ProjectId.eq(root_project_id))
            .order_by_asc(chapter_outline::Column::ChapterNumber)
            .all(state.db.as_ref())
            .await
            .map_err(|err| err.to_string())?,
        chapters: chapter::Entity::find()
            .filter(chapter::Column::ProjectId.eq(root_project_id))
            .order_by_asc(chapter::Column::ChapterNumber)
            .all(state.db.as_ref())
            .await
            .map_err(|err| err.to_string())?,
        character_profiles: protagonist_profile::Entity::find()
            .filter(protagonist_profile::Column::ProjectId.eq(root_project_id))
            .order_by_asc(protagonist_profile::Column::Id)
            .all(state.db.as_ref())
            .await
            .map_err(|err| err.to_string())?,
        object_profiles: object_profile::Entity::find()
            .filter(object_profile::Column::ProjectId.eq(root_project_id))
            .order_by_asc(object_profile::Column::Id)
            .all(state.db.as_ref())
            .await
            .map_err(|err| err.to_string())?,
        scene_profiles: scene_profile::Entity::find()
            .filter(scene_profile::Column::ProjectId.eq(root_project_id))
            .order_by_asc(scene_profile::Column::Id)
            .all(state.db.as_ref())
            .await
            .map_err(|err| err.to_string())?,
        scene_occurrences: scene_occurrence::Entity::find()
            .filter(scene_occurrence::Column::ProjectId.eq(root_project_id))
            .order_by_asc(scene_occurrence::Column::Id)
            .all(state.db.as_ref())
            .await
            .map_err(|err| err.to_string())?,
        scene_transitions: scene_transition::Entity::find()
            .filter(scene_transition::Column::ProjectId.eq(root_project_id))
            .order_by_asc(scene_transition::Column::Id)
            .all(state.db.as_ref())
            .await
            .map_err(|err| err.to_string())?,
        scene_entity_states: scene_entity_state::Entity::find()
            .filter(scene_entity_state::Column::ProjectId.eq(root_project_id))
            .order_by_asc(scene_entity_state::Column::Id)
            .all(state.db.as_ref())
            .await
            .map_err(|err| err.to_string())?,
    })
}

load_by_ids!(
    load_project_derivations,
    project_derivation::Entity,
    project_derivation::Column::DerivedProjectId,
    project_derivation::Column::Id,
);

fn collect_snapshot_ids(
    derived_projects: &[novel_project::Model],
    derivations: &[project_derivation::Model],
    sync_runs: &[source_sync_run::Model],
) -> Vec<i64> {
    let mut ids = BTreeSet::new();
    for project in derived_projects {
        if let Some(snapshot_id) = project.source_snapshot_id {
            ids.insert(snapshot_id);
        }
    }
    for derivation in derivations {
        ids.insert(derivation.source_snapshot_id);
    }
    for run in sync_runs {
        ids.insert(run.from_snapshot_id);
        ids.insert(run.to_snapshot_id);
    }
    ids.into_iter().collect()
}

load_by_ids!(
    load_source_snapshots,
    project_source_snapshot::Entity,
    project_source_snapshot::Column::Id,
    project_source_snapshot::Column::Id,
);

fn validate_snapshot_coverage(
    expected_ids: &[i64],
    snapshots: &[project_source_snapshot::Model],
) -> Result<(), String> {
    let found = snapshots
        .iter()
        .map(|snapshot| snapshot.id)
        .collect::<HashSet<_>>();
    for id in expected_ids {
        if !found.contains(id) {
            return Err(format!("派生项目引用的源文本快照不存在: {id}"));
        }
    }
    Ok(())
}

load_by_ids!(
    load_source_snapshot_chapters,
    source_snapshot_chapter::Entity,
    source_snapshot_chapter::Column::SnapshotId,
    source_snapshot_chapter::Column::SnapshotId,
    source_snapshot_chapter::Column::ChapterNumber,
);

load_by_ids!(
    load_source_snapshot_outlines,
    source_snapshot_outline::Entity,
    source_snapshot_outline::Column::SnapshotId,
    source_snapshot_outline::Column::Id,
);

load_by_ids!(
    load_source_snapshot_profiles,
    source_snapshot_profile::Entity,
    source_snapshot_profile::Column::SnapshotId,
    source_snapshot_profile::Column::Id,
);

async fn load_text_anchors(
    state: &AppState,
    root_project_id: i64,
    snapshot_ids: &[i64],
) -> Result<Vec<text_anchor::Model>, String> {
    let mut query =
        text_anchor::Entity::find().filter(text_anchor::Column::ProjectId.eq(root_project_id));
    if !snapshot_ids.is_empty() {
        query = query.filter(
            text_anchor::Column::SourceSnapshotId
                .is_null()
                .or(text_anchor::Column::SourceSnapshotId.is_in(snapshot_ids.to_vec())),
        );
    }
    query
        .order_by_asc(text_anchor::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())
}

async fn load_profile_evidence(
    state: &AppState,
    anchors: &[text_anchor::Model],
) -> Result<Vec<profile_evidence::Model>, String> {
    let anchor_ids = anchors.iter().map(|anchor| anchor.id).collect::<Vec<_>>();
    if anchor_ids.is_empty() {
        return Ok(Vec::new());
    }
    profile_evidence::Entity::find()
        .filter(profile_evidence::Column::TextAnchorId.is_in(anchor_ids))
        .order_by_asc(profile_evidence::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())
}

load_by_ids!(
    load_generation_specs,
    generation_spec::Entity,
    generation_spec::Column::ProjectId,
    generation_spec::Column::Id,
);

load_by_ids!(
    load_generation_spec_text_anchors,
    generation_spec_text_anchor::Entity,
    generation_spec_text_anchor::Column::GenerationSpecId,
);

load_by_ids!(
    load_generation_spec_entity_refs,
    generation_spec_entity_ref::Entity,
    generation_spec_entity_ref::Column::GenerationSpecId,
);

async fn load_generation_spec_media_refs(
    state: &AppState,
    spec_ids: &[i64],
    media_asset_ids: &[i64],
) -> Result<Vec<generation_spec_media_ref::Model>, String> {
    if spec_ids.is_empty() && media_asset_ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut query = generation_spec_media_ref::Entity::find();
    if !spec_ids.is_empty() {
        query = query
            .filter(generation_spec_media_ref::Column::GenerationSpecId.is_in(spec_ids.to_vec()));
    }
    if !media_asset_ids.is_empty() {
        query = query.filter(
            generation_spec_media_ref::Column::MediaAssetId.is_in(media_asset_ids.to_vec()),
        );
    }
    query
        .all(state.db.as_ref())
        .await
        .map_err(|err| err.to_string())
}

load_by_ids!(
    load_prompt_artifacts,
    prompt_artifact::Entity,
    prompt_artifact::Column::GenerationSpecId,
    prompt_artifact::Column::Id,
);

load_by_ids!(
    load_media_assets,
    media_asset::Entity,
    media_asset::Column::ProjectId,
    media_asset::Column::Id,
);

load_by_ids!(
    load_media_provenance,
    media_provenance::Entity,
    media_provenance::Column::MediaAssetId,
);

load_by_ids!(
    load_media_bindings,
    media_binding::Entity,
    media_binding::Column::MediaAssetId,
    media_binding::Column::Id,
);

load_by_ids!(
    load_galgame_scripts,
    galgame_script::Entity,
    galgame_script::Column::ProjectId,
    galgame_script::Column::Id,
);

load_by_ids!(
    load_galgame_scenes,
    galgame_scene::Entity,
    galgame_scene::Column::ScriptId,
    galgame_scene::Column::SceneIndex,
    galgame_scene::Column::Id,
);

load_by_ids!(
    load_galgame_lines,
    galgame_line::Entity,
    galgame_line::Column::SceneId,
    galgame_line::Column::LineIndex,
    galgame_line::Column::Id,
);

load_by_ids!(
    load_galgame_choices,
    galgame_choice::Entity,
    galgame_choice::Column::LineId,
    galgame_choice::Column::OptionIndex,
    galgame_choice::Column::Id,
);

load_by_ids!(
    load_video_segments,
    video_segment::Entity,
    video_segment::Column::ProjectId,
    video_segment::Column::Id,
);

load_by_ids!(
    load_assembly_timelines,
    assembly_timeline::Entity,
    assembly_timeline::Column::ProjectId,
    assembly_timeline::Column::Id,
);

load_by_ids!(
    load_assembly_clips,
    assembly_clip::Entity,
    assembly_clip::Column::TimelineId,
    assembly_clip::Column::Id,
);

load_by_ids!(
    load_source_sync_runs,
    source_sync_run::Entity,
    source_sync_run::Column::DerivedProjectId,
    source_sync_run::Column::Id,
);

load_by_ids!(
    load_source_sync_conflicts,
    source_sync_conflict::Entity,
    source_sync_conflict::Column::SyncRunId,
    source_sync_conflict::Column::Id,
);

load_by_ids!(
    load_source_sync_decisions,
    source_sync_decision::Entity,
    source_sync_decision::Column::ConflictId,
    source_sync_decision::Column::Id,
);

fn collect_media_files(
    state: &AppState,
    media_assets: &[media_asset::Model],
) -> Result<Vec<MediaFileEntry>, String> {
    let mut entries = Vec::new();
    let mut seen_package_paths = HashSet::new();
    for asset in media_assets {
        for (field, package_field) in MEDIA_FILE_FIELDS {
            let Some(path) = media_asset_path(asset, field)? else {
                continue;
            };
            let package_path = media_package_path(asset.id, package_field, path)?;
            validate_media_file(state, path, &package_path, &mut seen_package_paths)?;
            entries.push(MediaFileEntry {
                media_asset_id: asset.id,
                field: field.to_string(),
                original_path: path.to_string(),
                package_path,
            });
        }
    }
    Ok(entries)
}

fn validate_media_file(
    state: &AppState,
    original_path: &str,
    package_path: &str,
    seen_package_paths: &mut HashSet<String>,
) -> Result<(), String> {
    canonicalize_data_file_path(&state.config.data_dir, original_path, "媒体文件")?;
    if !seen_package_paths.insert(package_path.to_string()) {
        return Err(format!("项目族导出媒体包路径重复: {package_path}"));
    }
    Ok(())
}

fn media_package_path(
    media_asset_id: i64,
    field: &str,
    original_path: &str,
) -> Result<String, String> {
    let extension = Path::new(original_path)
        .extension()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("媒体文件缺少扩展名: {original_path}"))?;
    Ok(format!("media/{media_asset_id}/{field}.{extension}",))
}

fn package_zip_bytes(state: &AppState, package: &ProjectFamilyPackage) -> Result<Vec<u8>, String> {
    let cursor = Cursor::new(Vec::new());
    let cursor = write_package_zip_to_writer(state, cursor, package)?;
    Ok(cursor.into_inner())
}

fn write_package_zip_to_writer<W: Write + Seek>(
    state: &AppState,
    writer: W,
    package: &ProjectFamilyPackage,
) -> Result<W, String> {
    let mut zip = ZipWriter::new(writer);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let manifest = serde_json::to_vec_pretty(package)
        .map_err(|err| format!("项目族导出包序列化失败: {err}"))?;
    start_zip_file(&mut zip, MANIFEST_PATH, options)?;
    zip.write_all(&manifest)
        .map_err(|err| format!("写入项目族清单失败: {err}"))?;
    for media in &package.media_files {
        let source =
            canonicalize_data_file_path(&state.config.data_dir, &media.original_path, "媒体文件")?;
        let bytes = std::fs::read(&source)
            .map_err(|err| format!("读取媒体文件失败: {}: {err}", source.display()))?;
        start_zip_file(&mut zip, &media.package_path, options)?;
        zip.write_all(&bytes)
            .map_err(|err| format!("写入媒体文件失败: {}: {err}", media.package_path))?;
    }
    zip.finish()
        .map_err(|err| format!("完成项目族导出包失败: {err}"))
}

fn start_zip_file<W: Write + Seek>(
    zip: &mut ZipWriter<W>,
    path: &str,
    options: SimpleFileOptions,
) -> Result<(), String> {
    zip.start_file(path, options)
        .map_err(|err| format!("写入 zip 条目失败: {path}: {err}"))
}
