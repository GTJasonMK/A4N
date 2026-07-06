use super::remap::*;
use super::validation::{normalize_profile_evidence_role, snapshot_profile_key};
use super::*;

macro_rules! insert_import_row {
    ($txn:expr, $record:expr, $active_ty:path, $err:literal, |$active:ident| $prepare:block) => {{
        let mut $active: $active_ty = ($record).clone().into();
        $active.id = NotSet;
        $prepare
        $active
            .insert($txn)
            .await
            .map_err(|err| format!("{}: {}", $err, err))?
    }};
}

macro_rules! insert_mapped_import_row {
    ($txn:expr, $record:expr, $active_ty:path, $map:expr, $err:literal, |$active:ident| $prepare:block) => {{
        let inserted = insert_import_row!($txn, $record, $active_ty, $err, |$active| $prepare);
        ($map).insert(($record).id, inserted.id);
    }};
}

macro_rules! insert_import_rows {
    ($txn:expr, $records:expr, $record:ident, $active_ty:path, $err:literal, |$active:ident| $prepare:block) => {{
        for $record in $records {
            insert_import_row!($txn, $record, $active_ty, $err, |$active| $prepare);
        }
    }};
}

macro_rules! insert_mapped_import_rows {
    ($txn:expr, $records:expr, $record:ident, $active_ty:path, $map:expr, $err:literal, |$active:ident| $prepare:block) => {{
        for $record in $records {
            insert_mapped_import_row!($txn, $record, $active_ty, $map, $err, |$active| $prepare);
        }
    }};
}

macro_rules! insert_project_scoped_import_rows {
    ($txn:expr, $records:expr, $record:ident, $active_ty:path, $err:literal, $project_id:expr) => {{
        insert_import_rows!($txn, $records, $record, $active_ty, $err, |active| {
            active.project_id = Set($project_id);
        });
    }};
}

macro_rules! insert_project_scoped_mapped_import_rows {
    ($txn:expr, $records:expr, $record:ident, $active_ty:path, $map:expr, $err:literal, $project_id:expr) => {{
        insert_mapped_import_rows!($txn, $records, $record, $active_ty, $map, $err, |active| {
            active.project_id = Set($project_id);
        });
    }};
}

pub(super) fn cleanup_imported_media_files(paths: &[PathBuf]) {
    for path in paths {
        let _ = std::fs::remove_file(path);
    }
}

pub(super) async fn import_package_rows(
    state: &AppState,
    user_id: i64,
    txn: &DatabaseTransaction,
    package: &ProjectFamilyPackage,
    media_payload: &ImportMediaPayload,
    written_files: &mut Vec<PathBuf>,
) -> Result<Vec<novel_project::Model>, String> {
    let mut maps = ImportIdMaps::default();
    let root_project = insert_root_project(txn, user_id, package, &mut maps).await?;
    insert_text_core(txn, package, &mut maps).await?;
    insert_source_snapshots(txn, package, &mut maps).await?;
    let mut imported_projects = vec![root_project];
    imported_projects.extend(insert_derived_projects(txn, user_id, package, &mut maps).await?);
    insert_text_anchor_rows(txn, package, &mut maps).await?;
    insert_galgame_rows(txn, package, &mut maps).await?;
    insert_generation_rows_before_media(txn, package, &mut maps).await?;
    insert_media_rows(state, txn, package, media_payload, &mut maps, written_files).await?;
    insert_video_segments(txn, package, &mut maps).await?;
    insert_assembly_rows(txn, package, &mut maps).await?;
    update_generation_spec_deferred_targets(txn, package, &maps).await?;
    insert_mapped_import_rows!(
        txn,
        &package.media_bindings,
        binding,
        media_binding::ActiveModel,
        maps.media_binding_ids,
        "导入媒体绑定失败",
        |active| {
            active.media_asset_id = Set(map_required(
                &maps.media_asset_ids,
                binding.media_asset_id,
                "媒体绑定 media_asset_id",
            )?);
            active.target_id = Set(remap_media_binding_target_id(binding, &maps)?);
        }
    );
    insert_source_sync_rows(txn, package, &mut maps).await?;
    Ok(imported_projects)
}

async fn insert_root_project(
    txn: &DatabaseTransaction,
    user_id: i64,
    package: &ProjectFamilyPackage,
    maps: &mut ImportIdMaps,
) -> Result<novel_project::Model, String> {
    let root = package
        .projects
        .iter()
        .find(|project| project.id == package.root_project_id)
        .ok_or_else(|| format!("项目族导入包缺少根文本项目: {}", package.root_project_id))?;
    let root_imported = insert_project_row(txn, user_id, root, None, None, None).await?;
    maps.project_ids.insert(root.id, root_imported.id);
    Ok(root_imported)
}

async fn insert_derived_projects(
    txn: &DatabaseTransaction,
    user_id: i64,
    package: &ProjectFamilyPackage,
    maps: &mut ImportIdMaps,
) -> Result<Vec<novel_project::Model>, String> {
    let mut imported = Vec::new();
    for project in package
        .projects
        .iter()
        .filter(|project| project.id != package.root_project_id)
    {
        let new_root_id = map_required(
            &maps.project_ids,
            package.root_project_id,
            "派生项目 root_project_id",
        )?;
        let old_snapshot_id = project
            .source_snapshot_id
            .ok_or_else(|| format!("派生项目缺少 source_snapshot_id: {}", project.id))?;
        let new_snapshot_id = map_required(
            &maps.source_snapshot_ids,
            old_snapshot_id,
            "派生项目 source_snapshot_id",
        )?;
        let imported_project = insert_project_row(
            txn,
            user_id,
            project,
            Some(new_root_id),
            Some(new_root_id),
            Some(new_snapshot_id),
        )
        .await?;
        maps.project_ids.insert(project.id, imported_project.id);
        imported.push(imported_project);
    }
    insert_import_rows!(
        txn,
        &package.project_derivations,
        derivation,
        project_derivation::ActiveModel,
        "导入项目派生关系失败",
        |active| {
            active.root_project_id = Set(map_required(
                &maps.project_ids,
                derivation.root_project_id,
                "派生关系 root_project_id",
            )?);
            active.derived_project_id = Set(map_required(
                &maps.project_ids,
                derivation.derived_project_id,
                "派生关系 derived_project_id",
            )?);
            active.source_snapshot_id = Set(map_required(
                &maps.source_snapshot_ids,
                derivation.source_snapshot_id,
                "派生关系 source_snapshot_id",
            )?);
        }
    );
    Ok(imported)
}

async fn insert_project_row(
    txn: &DatabaseTransaction,
    user_id: i64,
    project: &novel_project::Model,
    root_project_id: Option<i64>,
    parent_project_id: Option<i64>,
    source_snapshot_id: Option<i64>,
) -> Result<novel_project::Model, String> {
    Ok(insert_import_row!(
        txn,
        project,
        novel_project::ActiveModel,
        "导入项目失败",
        |active| {
            active.user_id = Set(user_id);
            active.root_project_id = Set(root_project_id);
            active.parent_project_id = Set(parent_project_id);
            active.source_snapshot_id = Set(source_snapshot_id);
        }
    ))
}

async fn insert_text_core(
    txn: &DatabaseTransaction,
    package: &ProjectFamilyPackage,
    maps: &mut ImportIdMaps,
) -> Result<(), String> {
    let root_project_id = map_required(
        &maps.project_ids,
        package.root_project_id,
        "文本核心 root_project_id",
    )?;
    if let Some(blueprint) = &package.text_core.blueprint {
        insert_mapped_import_row!(
            txn,
            blueprint,
            novel_blueprint::ActiveModel,
            maps.blueprint_ids,
            "导入蓝图失败",
            |active| {
                active.project_id = Set(root_project_id);
            }
        );
    }
    insert_project_scoped_mapped_import_rows!(
        txn,
        &package.text_core.blueprint_versions,
        version,
        blueprint_version::ActiveModel,
        maps.blueprint_version_ids,
        "导入蓝图版本失败",
        root_project_id
    );
    insert_project_scoped_import_rows!(
        txn,
        &package.text_core.blueprint_characters,
        character,
        blueprint_character::ActiveModel,
        "导入蓝图角色失败",
        root_project_id
    );
    insert_project_scoped_import_rows!(
        txn,
        &package.text_core.blueprint_relationships,
        relationship,
        blueprint_relationship::ActiveModel,
        "导入蓝图关系失败",
        root_project_id
    );
    insert_project_scoped_mapped_import_rows!(
        txn,
        &package.text_core.part_outlines,
        part,
        part_outline::ActiveModel,
        maps.part_outline_ids,
        "导入分部大纲失败",
        root_project_id
    );
    insert_mapped_import_rows!(
        txn,
        &package.text_core.chapter_outlines,
        outline,
        chapter_outline::ActiveModel,
        maps.chapter_outline_ids,
        "导入章节大纲失败",
        |active| {
            active.project_id = Set(root_project_id);
            active.part_id = Set(map_optional(
                &maps.part_outline_ids,
                outline.part_id,
                "章节大纲 part_id",
            )?);
        }
    );
    insert_mapped_import_rows!(
        txn,
        &package.text_core.chapters,
        chapter,
        chapter::ActiveModel,
        maps.chapter_ids,
        "导入章节失败",
        |active| {
            active.project_id = Set(root_project_id);
            active.outline_id = Set(map_optional(
                &maps.chapter_outline_ids,
                chapter.outline_id,
                "章节 outline_id",
            )?);
        }
    );
    insert_project_scoped_mapped_import_rows!(
        txn,
        &package.text_core.character_profiles,
        profile,
        protagonist_profile::ActiveModel,
        maps.character_profile_ids,
        "导入人物档案失败",
        root_project_id
    );
    insert_project_scoped_mapped_import_rows!(
        txn,
        &package.text_core.object_profiles,
        profile,
        object_profile::ActiveModel,
        maps.object_profile_ids,
        "导入物体档案失败",
        root_project_id
    );
    insert_scene_profiles(txn, root_project_id, package, maps).await?;
    insert_mapped_import_rows!(
        txn,
        &package.text_core.scene_occurrences,
        occurrence,
        scene_occurrence::ActiveModel,
        maps.scene_occurrence_ids,
        "导入场景出场失败",
        |active| {
            active.project_id = Set(root_project_id);
            active.scene_profile_id = Set(map_required(
                &maps.scene_profile_ids,
                occurrence.scene_profile_id,
                "场景出场 scene_profile_id",
            )?);
            active.chapter_id = Set(map_required(
                &maps.chapter_ids,
                occurrence.chapter_id,
                "场景出场 chapter_id",
            )?);
        }
    );
    insert_mapped_import_rows!(
        txn,
        &package.text_core.scene_transitions,
        transition,
        scene_transition::ActiveModel,
        maps.scene_transition_ids,
        "导入场景位移失败",
        |active| {
            active.project_id = Set(root_project_id);
            active.chapter_id = Set(map_required(
                &maps.chapter_ids,
                transition.chapter_id,
                "场景位移 chapter_id",
            )?);
            active.from_scene_profile_id = Set(map_optional(
                &maps.scene_profile_ids,
                transition.from_scene_profile_id,
                "场景位移 from_scene_profile_id",
            )?);
            active.to_scene_profile_id = Set(map_required(
                &maps.scene_profile_ids,
                transition.to_scene_profile_id,
                "场景位移 to_scene_profile_id",
            )?);
            active.occurrence_id = Set(map_optional(
                &maps.scene_occurrence_ids,
                transition.occurrence_id,
                "场景位移 occurrence_id",
            )?);
        }
    );
    insert_mapped_import_rows!(
        txn,
        &package.text_core.scene_entity_states,
        state,
        scene_entity_state::ActiveModel,
        maps.scene_entity_state_ids,
        "导入场景实体状态失败",
        |active| {
            active.project_id = Set(root_project_id);
            active.scene_profile_id = Set(map_required(
                &maps.scene_profile_ids,
                state.scene_profile_id,
                "场景实体状态 scene_profile_id",
            )?);
            active.occurrence_id = Set(map_required(
                &maps.scene_occurrence_ids,
                state.occurrence_id,
                "场景实体状态 occurrence_id",
            )?);
            active.entity_id = Set(remap_optional_entity_id(
                &state.entity_type,
                state.entity_id,
                maps,
                "场景实体状态 entity_id",
            )?);
        }
    );
    Ok(())
}

async fn insert_scene_profiles(
    txn: &DatabaseTransaction,
    root_project_id: i64,
    package: &ProjectFamilyPackage,
    maps: &mut ImportIdMaps,
) -> Result<(), String> {
    insert_mapped_import_rows!(
        txn,
        &package.text_core.scene_profiles,
        profile,
        scene_profile::ActiveModel,
        maps.scene_profile_ids,
        "导入场景档案失败",
        |active| {
            active.project_id = Set(root_project_id);
            active.parent_scene_id = Set(None);
        }
    );
    for profile in &package.text_core.scene_profiles {
        let Some(parent_id) = profile.parent_scene_id else {
            continue;
        };
        let new_profile_id = map_required(&maps.scene_profile_ids, profile.id, "场景档案 ID")?;
        let new_parent_id = map_required(&maps.scene_profile_ids, parent_id, "场景父级 ID")?;
        let stored = scene_profile::Entity::find_by_id(new_profile_id)
            .one(txn)
            .await
            .map_err(|err| format!("读取导入场景档案失败: {err}"))?
            .ok_or_else(|| format!("导入场景档案不存在: {new_profile_id}"))?;
        let mut active: scene_profile::ActiveModel = stored.into();
        active.parent_scene_id = Set(Some(new_parent_id));
        active
            .update(txn)
            .await
            .map_err(|err| format!("更新场景父级失败: {err}"))?;
    }
    Ok(())
}

async fn insert_source_snapshots(
    txn: &DatabaseTransaction,
    package: &ProjectFamilyPackage,
    maps: &mut ImportIdMaps,
) -> Result<(), String> {
    let new_root_id = map_required(
        &maps.project_ids,
        package.root_project_id,
        "源文本快照 root_project_id",
    )?;
    let snapshot_profiles = package
        .source_snapshot_profiles
        .iter()
        .map(|profile| {
            (
                (
                    profile.snapshot_id,
                    snapshot_profile_key(&profile.profile_kind, profile.source_profile_id),
                ),
                profile,
            )
        })
        .collect::<HashMap<_, _>>();
    for snapshot in &package.source_snapshots {
        if snapshot.root_project_id != package.root_project_id {
            return Err(format!("源文本快照不属于导入包根项目: {}", snapshot.id));
        }
        let blueprint_version_id = map_optional(
            &maps.blueprint_version_ids,
            snapshot.blueprint_version_id,
            "源文本快照 blueprint_version_id",
        )?;
        let chapter_hashes_json = remap_chapter_hashes_json(&snapshot.chapter_hashes_json, maps)?;
        let profile_hashes_json = remap_profile_hashes_json(
            &snapshot.profile_hashes_json,
            snapshot.id,
            &snapshot_profiles,
            maps,
        )?;
        let source_version_hash = hash_json(&json!({
            "root_project_id": new_root_id,
            "chapter_scope": snapshot.chapter_scope_json,
            "chapter_hashes": chapter_hashes_json,
            "profile_hashes": profile_hashes_json,
            "blueprint_version_id": blueprint_version_id,
        }));
        let active = project_source_snapshot::ActiveModel {
            root_project_id: Set(new_root_id),
            source_version_hash: Set(source_version_hash.clone()),
            blueprint_version_id: Set(blueprint_version_id),
            chapter_scope_json: Set(snapshot.chapter_scope_json.clone()),
            chapter_hashes_json: Set(chapter_hashes_json),
            profile_hashes_json: Set(profile_hashes_json),
            created_at: Set(snapshot.created_at),
            ..Default::default()
        };
        let inserted = active
            .insert(txn)
            .await
            .map_err(|err| format!("导入源文本快照失败: {err}"))?;
        maps.source_snapshot_ids.insert(snapshot.id, inserted.id);
        maps.source_snapshot_version_hashes
            .insert(snapshot.id, source_version_hash);
    }
    insert_mapped_import_rows!(
        txn,
        &package.source_snapshot_chapters,
        chapter,
        source_snapshot_chapter::ActiveModel,
        maps.source_snapshot_chapter_ids,
        "导入源快照章节失败",
        |active| {
            active.snapshot_id = Set(map_required(
                &maps.source_snapshot_ids,
                chapter.snapshot_id,
                "源快照章节 snapshot_id",
            )?);
            active.root_project_id = Set(new_root_id);
            active.source_chapter_id = Set(map_required(
                &maps.chapter_ids,
                chapter.source_chapter_id,
                "源快照章节 source_chapter_id",
            )?);
        }
    );
    insert_mapped_import_rows!(
        txn,
        &package.source_snapshot_outlines,
        outline,
        source_snapshot_outline::ActiveModel,
        maps.source_snapshot_outline_ids,
        "导入源快照大纲失败",
        |active| {
            let (payload_json, payload_hash) =
                remap_source_snapshot_outline_payload(outline, maps)?;
            active.snapshot_id = Set(map_required(
                &maps.source_snapshot_ids,
                outline.snapshot_id,
                "源快照大纲 snapshot_id",
            )?);
            active.root_project_id = Set(new_root_id);
            active.source_outline_id = Set(remap_snapshot_outline_source_id(outline, maps)?);
            active.payload_json = Set(payload_json);
            active.payload_hash = Set(payload_hash);
        }
    );
    insert_mapped_import_rows!(
        txn,
        &package.source_snapshot_profiles,
        profile,
        source_snapshot_profile::ActiveModel,
        maps.source_snapshot_profile_ids,
        "导入源快照档案失败",
        |active| {
            let (payload_json, payload_hash) =
                remap_source_snapshot_profile_payload(profile, maps)?;
            active.snapshot_id = Set(map_required(
                &maps.source_snapshot_ids,
                profile.snapshot_id,
                "源快照档案 snapshot_id",
            )?);
            active.root_project_id = Set(new_root_id);
            active.source_profile_id = Set(remap_profile_id(
                &profile.profile_kind,
                profile.source_profile_id,
                maps,
                "源快照档案 source_profile_id",
            )?);
            active.payload_json = Set(payload_json);
            active.payload_hash = Set(payload_hash);
        }
    );
    Ok(())
}

async fn insert_text_anchor_rows(
    txn: &DatabaseTransaction,
    package: &ProjectFamilyPackage,
    maps: &mut ImportIdMaps,
) -> Result<(), String> {
    let root_project_id = map_required(
        &maps.project_ids,
        package.root_project_id,
        "文本锚点 project_id",
    )?;
    insert_mapped_import_rows!(
        txn,
        &package.text_anchors,
        anchor,
        text_anchor::ActiveModel,
        maps.text_anchor_ids,
        "导入文本锚点失败",
        |active| {
            active.project_id = Set(root_project_id);
            active.source_chapter_id = Set(map_optional(
                &maps.chapter_ids,
                anchor.source_chapter_id,
                "文本锚点 source_chapter_id",
            )?);
            active.source_snapshot_id = Set(map_optional(
                &maps.source_snapshot_ids,
                anchor.source_snapshot_id,
                "文本锚点 source_snapshot_id",
            )?);
            active.source_snapshot_chapter_id = Set(map_optional(
                &maps.source_snapshot_chapter_ids,
                anchor.source_snapshot_chapter_id,
                "文本锚点 source_snapshot_chapter_id",
            )?);
            active.source_id = Set(remap_text_anchor_source_id(anchor, maps)?);
        }
    );
    insert_mapped_import_rows!(
        txn,
        &package.profile_evidence,
        evidence,
        profile_evidence::ActiveModel,
        maps.profile_evidence_ids,
        "导入档案证据失败",
        |active| {
            active.profile_id = Set(remap_profile_id(
                &evidence.profile_kind,
                evidence.profile_id,
                maps,
                "档案证据 profile_id",
            )?);
            active.text_anchor_id = Set(map_required(
                &maps.text_anchor_ids,
                evidence.text_anchor_id,
                "档案证据 text_anchor_id",
            )?);
            active.evidence_role =
                Set(normalize_profile_evidence_role(&evidence.evidence_role)?.to_string());
        }
    );
    Ok(())
}

async fn insert_galgame_rows(
    txn: &DatabaseTransaction,
    package: &ProjectFamilyPackage,
    maps: &mut ImportIdMaps,
) -> Result<(), String> {
    insert_mapped_import_rows!(
        txn,
        &package.galgame_scripts,
        script,
        galgame_script::ActiveModel,
        maps.galgame_script_ids,
        "导入 Galgame 剧本失败",
        |active| {
            active.project_id = Set(map_required(
                &maps.project_ids,
                script.project_id,
                "Galgame 剧本 project_id",
            )?);
            active.root_project_id = Set(map_required(
                &maps.project_ids,
                script.root_project_id,
                "Galgame 剧本 root_project_id",
            )?);
            active.source_snapshot_id = Set(map_required(
                &maps.source_snapshot_ids,
                script.source_snapshot_id,
                "Galgame 剧本 source_snapshot_id",
            )?);
        }
    );
    insert_mapped_import_rows!(
        txn,
        &package.galgame_scenes,
        scene,
        galgame_scene::ActiveModel,
        maps.galgame_scene_ids,
        "导入 Galgame 场失败",
        |active| {
            active.script_id = Set(map_required(
                &maps.galgame_script_ids,
                scene.script_id,
                "Galgame 场 script_id",
            )?);
            active.source_snapshot_chapter_id = Set(map_required(
                &maps.source_snapshot_chapter_ids,
                scene.source_snapshot_chapter_id,
                "Galgame 场 source_snapshot_chapter_id",
            )?);
            active.participants_json = Set(remap_galgame_participants_json(
                &scene.participants_json,
                maps,
            )?);
            active.next_scene_id = Set(None);
        }
    );
    for scene in &package.galgame_scenes {
        let Some(next_scene_id) = scene.next_scene_id else {
            continue;
        };
        let new_scene_id = map_required(&maps.galgame_scene_ids, scene.id, "Galgame 场 ID")?;
        let new_next_scene_id = map_required(
            &maps.galgame_scene_ids,
            next_scene_id,
            "Galgame 场 next_scene_id",
        )?;
        let stored = galgame_scene::Entity::find_by_id(new_scene_id)
            .one(txn)
            .await
            .map_err(|err| format!("读取导入 Galgame 场失败: {err}"))?
            .ok_or_else(|| format!("导入 Galgame 场不存在: {new_scene_id}"))?;
        let mut active: galgame_scene::ActiveModel = stored.into();
        active.next_scene_id = Set(Some(new_next_scene_id));
        active
            .update(txn)
            .await
            .map_err(|err| format!("更新 Galgame 场 next_scene_id 失败: {err}"))?;
    }
    insert_mapped_import_rows!(
        txn,
        &package.galgame_lines,
        line,
        galgame_line::ActiveModel,
        maps.galgame_line_ids,
        "导入 Galgame 行失败",
        |active| {
            active.scene_id = Set(map_required(
                &maps.galgame_scene_ids,
                line.scene_id,
                "Galgame 行 scene_id",
            )?);
            active.speaker_profile_id = Set(map_optional(
                &maps.character_profile_ids,
                line.speaker_profile_id,
                "Galgame 行 speaker_profile_id",
            )?);
            active.text_anchor_id = Set(map_optional(
                &maps.text_anchor_ids,
                line.text_anchor_id,
                "Galgame 行 text_anchor_id",
            )?);
        }
    );
    insert_mapped_import_rows!(
        txn,
        &package.galgame_choices,
        choice,
        galgame_choice::ActiveModel,
        maps.galgame_choice_ids,
        "导入 Galgame 选项失败",
        |active| {
            active.line_id = Set(map_required(
                &maps.galgame_line_ids,
                choice.line_id,
                "Galgame 选项 line_id",
            )?);
            let target_scene_id = choice
                .target_scene_id
                .ok_or_else(|| "Galgame 选项 target_scene_id 不能为空".to_string())?;
            active.target_scene_id = Set(Some(map_required(
                &maps.galgame_scene_ids,
                target_scene_id,
                "Galgame 选项 target_scene_id",
            )?));
        }
    );
    Ok(())
}

fn remap_galgame_participants_json(value: &Value, maps: &ImportIdMaps) -> Result<Value, String> {
    let Some(items) = value.as_array() else {
        return Err("Galgame 场 participants_json 必须是数组".to_string());
    };
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let mut object = item
            .as_object()
            .cloned()
            .ok_or_else(|| "Galgame 场 participants_json 项必须是对象".to_string())?;
        if let Some(old_id) = object.get("character_profile_id").and_then(Value::as_i64) {
            object.insert(
                "character_profile_id".to_string(),
                Value::from(map_required(
                    &maps.character_profile_ids,
                    old_id,
                    "Galgame 场 participants_json.character_profile_id",
                )?),
            );
        }
        out.push(Value::Object(object));
    }
    Ok(Value::Array(out))
}

async fn insert_generation_rows_before_media(
    txn: &DatabaseTransaction,
    package: &ProjectFamilyPackage,
    maps: &mut ImportIdMaps,
) -> Result<(), String> {
    insert_mapped_import_rows!(
        txn,
        &package.generation_specs,
        spec,
        generation_spec::ActiveModel,
        maps.generation_spec_ids,
        "导入生成意图失败",
        |active| {
            let source_snapshot_id = spec
                .source_snapshot_id
                .ok_or_else(|| format!("项目族导入生成意图缺少 source_snapshot_id: {}", spec.id))?;
            active.project_id = Set(map_required(
                &maps.project_ids,
                spec.project_id,
                "生成意图 project_id",
            )?);
            active.root_project_id = Set(map_required(
                &maps.project_ids,
                spec.root_project_id,
                "生成意图 root_project_id",
            )?);
            active.source_context_kind = Set(Some("snapshot_backed".to_string()));
            active.source_snapshot_id = Set(Some(map_required(
                &maps.source_snapshot_ids,
                source_snapshot_id,
                "生成意图 source_snapshot_id",
            )?));
            active.source_chapter_id = Set(map_optional(
                &maps.chapter_ids,
                spec.source_chapter_id,
                "生成意图 source_chapter_id",
            )?);
            active.source_snapshot_chapter_id = Set(map_optional(
                &maps.source_snapshot_chapter_ids,
                spec.source_snapshot_chapter_id,
                "生成意图 source_snapshot_chapter_id",
            )?);
            active.source_version_hash =
                Set(remap_generation_spec_source_version_hash(spec, maps)?);
            active.target_entity_id = Set(remap_generation_target_id(spec, maps, false)?);
            active.params_json = Set(remap_generation_spec_params_json(spec, maps)?);
        }
    );
    for record in &package.generation_spec_text_anchors {
        generation_spec_text_anchor::ActiveModel {
            generation_spec_id: Set(map_required(
                &maps.generation_spec_ids,
                record.generation_spec_id,
                "生成意图锚点 generation_spec_id",
            )?),
            text_anchor_id: Set(map_required(
                &maps.text_anchor_ids,
                record.text_anchor_id,
                "生成意图锚点 text_anchor_id",
            )?),
            role: Set(record.role.clone()),
        }
        .insert(txn)
        .await
        .map_err(|err| format!("导入生成意图锚点引用失败: {err}"))?;
    }
    for record in &package.generation_spec_entity_refs {
        generation_spec_entity_ref::ActiveModel {
            generation_spec_id: Set(map_required(
                &maps.generation_spec_ids,
                record.generation_spec_id,
                "生成意图实体 generation_spec_id",
            )?),
            entity_kind: Set(record.entity_kind.clone()),
            entity_id: Set(remap_profile_id(
                &record.entity_kind,
                record.entity_id,
                maps,
                "生成意图实体 entity_id",
            )?),
            role: Set(record.role.clone()),
        }
        .insert(txn)
        .await
        .map_err(|err| format!("导入生成意图实体引用失败: {err}"))?;
    }
    insert_mapped_import_rows!(
        txn,
        &package.prompt_artifacts,
        artifact,
        prompt_artifact::ActiveModel,
        maps.prompt_artifact_ids,
        "导入提示词失败",
        |active| {
            active.generation_spec_id = Set(map_required(
                &maps.generation_spec_ids,
                artifact.generation_spec_id,
                "提示词 generation_spec_id",
            )?);
        }
    );
    Ok(())
}

async fn insert_media_rows(
    state: &AppState,
    txn: &DatabaseTransaction,
    package: &ProjectFamilyPackage,
    media_payload: &ImportMediaPayload,
    maps: &mut ImportIdMaps,
    written_files: &mut Vec<PathBuf>,
) -> Result<(), String> {
    for asset in &package.media_assets {
        let original_status = asset.status.clone();
        let inserted = insert_import_row!(
            txn,
            asset,
            media_asset::ActiveModel,
            "导入媒体资产失败",
            |active| {
                active.project_id = Set(map_required(
                    &maps.project_ids,
                    asset.project_id,
                    "媒体资产 project_id",
                )?);
                active.root_project_id = Set(map_required(
                    &maps.project_ids,
                    asset.root_project_id,
                    "媒体资产 root_project_id",
                )?);
                active.source_snapshot_id = Set(map_optional(
                    &maps.source_snapshot_ids,
                    asset.source_snapshot_id,
                    "媒体资产 source_snapshot_id",
                )?);
                active.source_chapter_id = Set(map_optional(
                    &maps.chapter_ids,
                    asset.source_chapter_id,
                    "媒体资产 source_chapter_id",
                )?);
                active.source_snapshot_chapter_id = Set(map_optional(
                    &maps.source_snapshot_chapter_ids,
                    asset.source_snapshot_chapter_id,
                    "媒体资产 source_snapshot_chapter_id",
                )?);
                active.file_path = Set(None);
                active.thumbnail_path = Set(None);
                active.file_size = Set(None);
                active.checksum = Set(None);
                active.status =
                    Set(media_asset_status_for_import_insert(&original_status).to_string());
            }
        );
        maps.media_asset_ids.insert(asset.id, inserted.id);

        let file_path = write_imported_media_field(
            state,
            asset,
            &inserted,
            "file_path",
            media_payload,
            written_files,
        )?;
        let thumbnail_path = write_imported_media_field(
            state,
            asset,
            &inserted,
            "thumbnail_path",
            media_payload,
            written_files,
        )?;
        let mut update: media_asset::ActiveModel = inserted.into();
        update.file_path = Set(file_path);
        update.thumbnail_path = Set(thumbnail_path);
        update.file_size = Set(asset.file_size);
        update.checksum = Set(asset.checksum.clone());
        update.status = Set(original_status);
        update
            .update(txn)
            .await
            .map_err(|err| format!("更新媒体资产文件路径失败: {err}"))?;
    }
    for record in &package.generation_spec_media_refs {
        generation_spec_media_ref::ActiveModel {
            generation_spec_id: Set(map_required(
                &maps.generation_spec_ids,
                record.generation_spec_id,
                "生成意图媒体 generation_spec_id",
            )?),
            media_asset_id: Set(map_required(
                &maps.media_asset_ids,
                record.media_asset_id,
                "生成意图媒体 media_asset_id",
            )?),
            role: Set(record.role.clone()),
        }
        .insert(txn)
        .await
        .map_err(|err| format!("导入生成意图媒体引用失败: {err}"))?;
    }
    for provenance in &package.media_provenance {
        media_provenance::ActiveModel {
            media_asset_id: Set(map_required(
                &maps.media_asset_ids,
                provenance.media_asset_id,
                "媒体来源 media_asset_id",
            )?),
            generation_spec_id: Set(map_optional(
                &maps.generation_spec_ids,
                provenance.generation_spec_id,
                "媒体来源 generation_spec_id",
            )?),
            prompt_artifact_id: Set(map_optional(
                &maps.prompt_artifact_ids,
                provenance.prompt_artifact_id,
                "媒体来源 prompt_artifact_id",
            )?),
            provider_job_id: Set(provenance.provider_job_id.clone()),
            source_media_asset_id: Set(map_optional(
                &maps.media_asset_ids,
                provenance.source_media_asset_id,
                "媒体来源 source_media_asset_id",
            )?),
            source_external_path: Set(provenance.source_external_path.clone()),
            imported_from_user: Set(provenance.imported_from_user),
            request_snapshot_json: Set(remap_media_request_snapshot(
                provenance.request_snapshot_json.as_ref(),
                maps,
            )?),
            created_at: Set(provenance.created_at),
        }
        .insert(txn)
        .await
        .map_err(|err| format!("导入媒体来源失败: {err}"))?;
    }
    Ok(())
}

fn write_imported_media_field(
    state: &AppState,
    old_asset: &media_asset::Model,
    new_asset: &media_asset::Model,
    field: &str,
    media_payload: &ImportMediaPayload,
    written_files: &mut Vec<PathBuf>,
) -> Result<Option<String>, String> {
    let old_value = media_asset_path(old_asset, field)?;
    if old_value.is_none() {
        return Ok(None);
    }
    let entry = media_payload
        .entries
        .get(&media_file_key(old_asset.id, field))
        .ok_or_else(|| format!("媒体资产缺少文件条目: {} {}", old_asset.id, field))?;
    let bytes = media_payload
        .bytes
        .get(&entry.package_path)
        .ok_or_else(|| format!("媒体文件字节缺失: {}", entry.package_path))?;
    let extension = required_package_extension(&entry.package_path)?;
    let suffix = if field == MEDIA_FILE_FIELD_THUMBNAIL {
        "_thumbnail"
    } else {
        ""
    };
    let relative = format!(
        "projects/{}/media/{}/imported_package_{}{}.{}",
        new_asset.project_id, new_asset.media_kind, new_asset.id, suffix, extension
    );
    write_relative_data_file(&state.config.data_dir, &relative, bytes, written_files)?;
    Ok(Some(relative))
}

pub(super) fn media_asset_status_for_import_insert(status: &str) -> &str {
    if status == MEDIA_STATUS_COMPLETED {
        MEDIA_STATUS_PENDING
    } else {
        status
    }
}

fn write_relative_data_file(
    data_dir: &Path,
    relative: &str,
    bytes: &[u8],
    written_files: &mut Vec<PathBuf>,
) -> Result<(), String> {
    let target = data_dir.join(relative);
    if target.exists() {
        return Err(format!("导入媒体目标文件已存在: {}", target.display()));
    }
    let parent = target
        .parent()
        .ok_or_else(|| format!("导入媒体目标路径缺少父目录: {}", target.display()))?;
    std::fs::create_dir_all(parent)
        .map_err(|err| format!("创建导入媒体目录失败: {}: {err}", parent.display()))?;
    std::fs::write(&target, bytes)
        .map_err(|err| format!("写入导入媒体文件失败: {}: {err}", target.display()))?;
    written_files.push(target);
    Ok(())
}

async fn insert_video_segments(
    txn: &DatabaseTransaction,
    package: &ProjectFamilyPackage,
    maps: &mut ImportIdMaps,
) -> Result<(), String> {
    insert_mapped_import_rows!(
        txn,
        &package.video_segments,
        segment,
        video_segment::ActiveModel,
        maps.video_segment_ids,
        "导入视频片段失败",
        |active| {
            active.project_id = Set(map_required(
                &maps.project_ids,
                segment.project_id,
                "视频片段 project_id",
            )?);
            active.root_project_id = Set(map_required(
                &maps.project_ids,
                segment.root_project_id,
                "视频片段 root_project_id",
            )?);
            active.source_chapter_id = Set(map_optional(
                &maps.chapter_ids,
                segment.source_chapter_id,
                "视频片段 source_chapter_id",
            )?);
            active.source_snapshot_chapter_id = Set(map_optional(
                &maps.source_snapshot_chapter_ids,
                segment.source_snapshot_chapter_id,
                "视频片段 source_snapshot_chapter_id",
            )?);
            active.source_generation_spec_id = Set(map_optional(
                &maps.generation_spec_ids,
                segment.source_generation_spec_id,
                "视频片段 source_generation_spec_id",
            )?);
            active.result_media_asset_id = Set(None);
            active.previous_segment_id = Set(None);
            active.next_segment_id = Set(None);
        }
    );
    for segment in &package.video_segments {
        let new_segment_id = map_required(&maps.video_segment_ids, segment.id, "视频片段 ID")?;
        let stored = video_segment::Entity::find_by_id(new_segment_id)
            .one(txn)
            .await
            .map_err(|err| format!("读取导入视频片段失败: {err}"))?
            .ok_or_else(|| format!("导入视频片段不存在: {new_segment_id}"))?;
        let mut active: video_segment::ActiveModel = stored.into();
        active.result_media_asset_id = Set(map_optional(
            &maps.media_asset_ids,
            segment.result_media_asset_id,
            "视频片段 result_media_asset_id",
        )?);
        active.previous_segment_id = Set(map_optional(
            &maps.video_segment_ids,
            segment.previous_segment_id,
            "视频片段 previous_segment_id",
        )?);
        active.next_segment_id = Set(map_optional(
            &maps.video_segment_ids,
            segment.next_segment_id,
            "视频片段 next_segment_id",
        )?);
        active
            .update(txn)
            .await
            .map_err(|err| format!("更新视频片段引用失败: {err}"))?;
    }
    Ok(())
}

async fn insert_assembly_rows(
    txn: &DatabaseTransaction,
    package: &ProjectFamilyPackage,
    maps: &mut ImportIdMaps,
) -> Result<(), String> {
    insert_mapped_import_rows!(
        txn,
        &package.assembly_timelines,
        timeline,
        assembly_timeline::ActiveModel,
        maps.assembly_timeline_ids,
        "导入成片时间线失败",
        |active| {
            active.project_id = Set(map_required(
                &maps.project_ids,
                timeline.project_id,
                "成片时间线 project_id",
            )?);
            active.source_chapter_id = Set(map_optional(
                &maps.chapter_ids,
                timeline.source_chapter_id,
                "成片时间线 source_chapter_id",
            )?);
            active.source_snapshot_chapter_id = Set(map_optional(
                &maps.source_snapshot_chapter_ids,
                timeline.source_snapshot_chapter_id,
                "成片时间线 source_snapshot_chapter_id",
            )?);
            active.preview_media_asset_id = Set(map_optional(
                &maps.media_asset_ids,
                timeline.preview_media_asset_id,
                "成片时间线 preview_media_asset_id",
            )?);
        }
    );
    insert_mapped_import_rows!(
        txn,
        &package.assembly_clips,
        clip,
        assembly_clip::ActiveModel,
        maps.assembly_clip_ids,
        "导入成片 clip 失败",
        |active| {
            active.timeline_id = Set(map_required(
                &maps.assembly_timeline_ids,
                clip.timeline_id,
                "成片 clip timeline_id",
            )?);
            active.media_asset_id = Set(map_required(
                &maps.media_asset_ids,
                clip.media_asset_id,
                "成片 clip media_asset_id",
            )?);
            active.source_segment_id = Set(map_optional(
                &maps.video_segment_ids,
                clip.source_segment_id,
                "成片 clip source_segment_id",
            )?);
        }
    );
    Ok(())
}

async fn update_generation_spec_deferred_targets(
    txn: &DatabaseTransaction,
    package: &ProjectFamilyPackage,
    maps: &ImportIdMaps,
) -> Result<(), String> {
    for spec in &package.generation_specs {
        if !matches!(spec.target_entity_kind.as_str(), "segment" | "assembly")
            || spec.target_entity_id.is_none()
        {
            continue;
        }
        let new_spec_id = map_required(&maps.generation_spec_ids, spec.id, "生成意图 ID")?;
        let new_target_id = remap_generation_target_id(spec, maps, true)?;
        let stored = generation_spec::Entity::find_by_id(new_spec_id)
            .one(txn)
            .await
            .map_err(|err| format!("读取导入生成意图失败: {err}"))?
            .ok_or_else(|| format!("导入生成意图不存在: {new_spec_id}"))?;
        let mut active: generation_spec::ActiveModel = stored.into();
        active.target_entity_id = Set(new_target_id);
        active
            .update(txn)
            .await
            .map_err(|err| format!("更新生成意图目标引用失败: {err}"))?;
    }
    Ok(())
}

async fn insert_source_sync_rows(
    txn: &DatabaseTransaction,
    package: &ProjectFamilyPackage,
    maps: &mut ImportIdMaps,
) -> Result<(), String> {
    insert_mapped_import_rows!(
        txn,
        &package.source_sync_runs,
        run,
        source_sync_run::ActiveModel,
        maps.source_sync_run_ids,
        "导入源同步运行失败",
        |active| {
            active.derived_project_id = Set(map_required(
                &maps.project_ids,
                run.derived_project_id,
                "同步运行 derived_project_id",
            )?);
            active.root_project_id = Set(map_required(
                &maps.project_ids,
                run.root_project_id,
                "同步运行 root_project_id",
            )?);
            active.from_snapshot_id = Set(map_required(
                &maps.source_snapshot_ids,
                run.from_snapshot_id,
                "同步运行 from_snapshot_id",
            )?);
            active.to_snapshot_id = Set(map_required(
                &maps.source_snapshot_ids,
                run.to_snapshot_id,
                "同步运行 to_snapshot_id",
            )?);
        }
    );
    insert_mapped_import_rows!(
        txn,
        &package.source_sync_conflicts,
        conflict,
        source_sync_conflict::ActiveModel,
        maps.source_sync_conflict_ids,
        "导入源同步冲突失败",
        |active| {
            active.sync_run_id = Set(map_required(
                &maps.source_sync_run_ids,
                conflict.sync_run_id,
                "同步冲突 sync_run_id",
            )?);
            active.derived_project_id = Set(map_required(
                &maps.project_ids,
                conflict.derived_project_id,
                "同步冲突 derived_project_id",
            )?);
            active.root_project_id = Set(map_required(
                &maps.project_ids,
                conflict.root_project_id,
                "同步冲突 root_project_id",
            )?);
            active.from_snapshot_id = Set(map_required(
                &maps.source_snapshot_ids,
                conflict.from_snapshot_id,
                "同步冲突 from_snapshot_id",
            )?);
            active.to_snapshot_id = Set(map_required(
                &maps.source_snapshot_ids,
                conflict.to_snapshot_id,
                "同步冲突 to_snapshot_id",
            )?);
            active.target_id = Set(remap_source_sync_target_id(conflict, maps)?);
            active.source_summary_json = Set(remap_conflict_source_summary(conflict, maps)?);
            active.local_summary_json = Set(remap_conflict_local_summary(conflict, maps)?);
            active.affected_refs_json = Set(remap_conflict_affected_refs(conflict, maps)?);
            active.decision_json = Set(remap_conflict_decision_json(conflict, maps)?);
        }
    );
    insert_mapped_import_rows!(
        txn,
        &package.source_sync_decisions,
        decision,
        source_sync_decision::ActiveModel,
        maps.source_sync_decision_ids,
        "导入源同步决策失败",
        |active| {
            active.conflict_id = Set(map_required(
                &maps.source_sync_conflict_ids,
                decision.conflict_id,
                "同步决策 conflict_id",
            )?);
            active.decision_payload_json = Set(remap_source_sync_decision_payload(
                &decision.strategy,
                &decision.decision_payload_json,
                maps,
                "同步决策 decision_payload_json",
            )?);
        }
    );
    Ok(())
}
