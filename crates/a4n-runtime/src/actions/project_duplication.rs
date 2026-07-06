mod blueprint_graph;
mod content_graph;
mod freeform;
#[cfg(test)]
mod tests;

use self::blueprint_graph::{
    blueprint_graph_raw_blueprint, copy_blueprint_graph, load_blueprint_graph,
};
use self::content_graph::{copy_chapter_outlines, copy_chapters, copy_part_outlines};
use self::freeform::duplicate_freeform_project_data;
use crate::AppState;
use crate::actions::access;
use crate::file_utils;
use a4n_core::models::ai::{generated_image, generated_video};
use a4n_core::models::canvas::{canvas_edge, canvas_graph, canvas_node, canvas_text_asset};
use a4n_core::models::generation::{
    generation_spec, generation_spec_entity_ref, generation_spec_media_ref,
    generation_spec_text_anchor, prompt_artifact,
};
use a4n_core::models::media::{media_asset, media_binding, media_provenance};
use a4n_core::models::project::{
    novel_project, project_derivation, project_source_snapshot, source_snapshot_chapter,
    source_snapshot_outline, source_snapshot_profile, text_anchor,
};
use a4n_core::models::video_project::{assembly_clip, assembly_timeline, video_segment};
use a4n_core::state::{status::ProjectStatus, validator::StateValidator};
use a4n_services::project::{PROJECT_KIND_TEXT, SOURCE_MODE_FREEFORM};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
    TransactionTrait,
};
use serde::Deserialize;
use serde_json::Value;
use std::{
    collections::{BTreeSet, HashMap},
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Deserialize)]
pub struct DuplicateProjectInput {
    pub project_id: i64,
    pub new_title: String,
}

pub async fn duplicate_project(
    state: &AppState,
    user_id: i64,
    input: DuplicateProjectInput,
) -> Result<novel_project::Model, String> {
    let original = state
        .project
        .get(input.project_id)
        .await
        .map_err(|e| e.to_string())?;
    access::verify_project_editor_string(state, user_id, original.id).await?;
    let new_title = required_text(&input.new_title, "新项目标题")?;

    let txn = state.db.begin().await.map_err(|e| e.to_string())?;
    let now = Utc::now().naive_utc();
    let mut copied_media_files = Vec::new();
    let duplicate_result = if original.source_mode == SOURCE_MODE_FREEFORM {
        duplicate_freeform_project_data(
            &txn,
            &original,
            user_id,
            new_title,
            now,
            &state.config.data_dir,
            &mut copied_media_files,
        )
        .await
    } else if original.project_kind == PROJECT_KIND_TEXT {
        duplicate_project_data(&txn, &original, user_id, new_title, now).await
    } else {
        duplicate_derived_project_data(
            &txn,
            &original,
            user_id,
            new_title,
            now,
            &state.config.data_dir,
            &mut copied_media_files,
        )
        .await
    };
    let new_project = match duplicate_result {
        Ok(project) => project,
        Err(err) => {
            cleanup_copied_files(&copied_media_files);
            return Err(err);
        }
    };
    if let Err(err) = txn.commit().await {
        cleanup_copied_files(&copied_media_files);
        return Err(err.to_string());
    }

    Ok(new_project)
}

pub async fn branch_project(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    branch_title: String,
    at_status: String,
) -> Result<novel_project::Model, String> {
    let branch_title = required_text(&branch_title, "分支标题")?.to_string();
    let branch_status = required_project_status(&at_status, "分支状态")?;
    let original = state
        .project
        .get(project_id)
        .await
        .map_err(|e| e.to_string())?;
    access::verify_project_editor_string(state, user_id, original.id).await?;
    ensure_text_project_for_copy(&original, "创建项目分支")?;

    let txn = state.db.begin().await.map_err(|e| e.to_string())?;
    let now = Utc::now().naive_utc();
    let new_project =
        branch_project_data(&txn, &original, &branch_title, &branch_status, now).await?;
    txn.commit().await.map_err(|e| e.to_string())?;

    Ok(new_project)
}

struct ProjectCopyInsert {
    user_id: i64,
    title: String,
    root_project_id: Option<i64>,
    parent_project_id: Option<i64>,
    source_snapshot_id: Option<i64>,
    status: String,
    current_chapter: i32,
    total_chapters: Option<i32>,
    raw_blueprint: Option<String>,
    import_analysis_progress: Option<String>,
}

async fn insert_project_copy<C>(
    db: &C,
    original: &novel_project::Model,
    values: ProjectCopyInsert,
    now: chrono::NaiveDateTime,
) -> Result<novel_project::Model, String>
where
    C: ConnectionTrait,
{
    novel_project::ActiveModel {
        user_id: Set(values.user_id),
        title: Set(values.title),
        description: Set(original.description.clone()),
        project_type: Set(original.project_type.clone()),
        project_kind: Set(original.project_kind.clone()),
        source_mode: Set(original.source_mode.clone()),
        root_project_id: Set(values.root_project_id),
        parent_project_id: Set(values.parent_project_id),
        source_snapshot_id: Set(values.source_snapshot_id),
        source_sync_status: Set(original.source_sync_status.clone()),
        derived_project_status: Set(original.derived_project_status.clone()),
        status: Set(values.status),
        target_chapters: Set(original.target_chapters),
        current_chapter: Set(values.current_chapter),
        total_chapters: Set(values.total_chapters),
        chapters_per_part: Set(original.chapters_per_part),
        avatar_svg: Set(original.avatar_svg.clone()),
        avatar_animal: Set(original.avatar_animal.clone()),
        raw_blueprint: Set(values.raw_blueprint),
        import_analysis_progress: Set(values.import_analysis_progress),
        export_config: Set(original.export_config.clone()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(|e| e.to_string())
}

macro_rules! copy_generation_spec_ref_rows {
    ($fn_name:ident, $module:ident, $target_field:ident, $target_map:ident, $spec_label:literal, $target_label:literal) => {
        async fn $fn_name<C>(
            db: &C,
            old_spec_ids: &[i64],
            spec_id_map: &HashMap<i64, i64>,
            $target_map: &HashMap<i64, i64>,
        ) -> Result<(), String>
        where
            C: ConnectionTrait,
        {
            if old_spec_ids.is_empty() {
                return Ok(());
            }
            let refs = $module::Entity::find()
                .filter($module::Column::GenerationSpecId.is_in(old_spec_ids.to_vec()))
                .all(db)
                .await
                .map_err(|err| err.to_string())?;
            for item in refs {
                $module::ActiveModel {
                    generation_spec_id: Set(remap_required_id(
                        spec_id_map,
                        item.generation_spec_id,
                        $spec_label,
                    )?),
                    $target_field: Set(remap_required_id(
                        $target_map,
                        item.$target_field,
                        $target_label,
                    )?),
                    role: Set(item.role),
                }
                .insert(db)
                .await
                .map_err(|err| err.to_string())?;
            }
            Ok(())
        }
    };
}

macro_rules! copy_snapshot_detail_rows {
    ($db:expr, $source_snapshot_id:expr, $new_snapshot_id:expr, $module:ident, [$($field:ident),+ $(,)?]) => {{
        for row in $module::Entity::find()
            .filter($module::Column::SnapshotId.eq($source_snapshot_id))
            .order_by_asc($module::Column::Id)
            .all($db)
            .await
            .map_err(|e| e.to_string())?
        {
            $module::ActiveModel {
                snapshot_id: Set($new_snapshot_id),
                root_project_id: Set(row.root_project_id),
                $($field: Set(row.$field),)+
                ..Default::default()
            }
            .insert($db)
            .await
            .map_err(|e| e.to_string())?;
        }
    }};
}

async fn duplicate_project_data<C>(
    db: &C,
    original: &novel_project::Model,
    user_id: i64,
    new_title: &str,
    now: chrono::NaiveDateTime,
) -> Result<novel_project::Model, String>
where
    C: ConnectionTrait,
{
    let original_status =
        StateValidator::parse_status(&original.status).map_err(|e| e.to_string())?;
    let source_graph = load_blueprint_graph(db, original.id).await?;
    if branch_status_includes_blueprint(original_status) && source_graph.is_none() {
        return Err(format!(
            "源项目状态为 {}，但缺少可复制的蓝图数据",
            original.status
        ));
    }
    let raw_blueprint = match (&original.raw_blueprint, source_graph.as_ref()) {
        (Some(raw_blueprint), _) => Some(raw_blueprint.clone()),
        (None, Some(graph)) => Some(blueprint_graph_raw_blueprint(graph)?),
        (None, None) => None,
    };

    let new_project = insert_project_copy(
        db,
        original,
        ProjectCopyInsert {
            user_id,
            title: new_title.to_string(),
            root_project_id: original.root_project_id,
            parent_project_id: original.parent_project_id,
            source_snapshot_id: original.source_snapshot_id,
            status: original.status.clone(),
            current_chapter: original.current_chapter,
            total_chapters: original.total_chapters,
            raw_blueprint,
            import_analysis_progress: original.import_analysis_progress.clone(),
        },
        now,
    )
    .await?;

    if let Some(graph) = &source_graph {
        copy_blueprint_graph(db, new_project.id, graph, now).await?;
    }
    let part_id_map = copy_part_outlines(db, original.id, new_project.id, now).await?;
    let outline_id_map =
        copy_chapter_outlines(db, original.id, new_project.id, &part_id_map, now).await?;
    copy_chapters(db, original.id, new_project.id, &outline_id_map, now).await?;

    Ok(new_project)
}

async fn duplicate_derived_project_data<C>(
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
    if original.project_kind == PROJECT_KIND_TEXT {
        return Err("复制派生项目不能接收文本项目".to_string());
    }
    let root_project_id = original
        .root_project_id
        .ok_or_else(|| format!("派生项目缺少 root_project_id: {}", original.id))?;
    let source_snapshot_id = original
        .source_snapshot_id
        .ok_or_else(|| format!("派生项目缺少 source_snapshot_id: {}", original.id))?;
    let derivation = project_derivation::Entity::find()
        .filter(project_derivation::Column::DerivedProjectId.eq(original.id))
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("派生项目缺少派生关系: {}", original.id))?;
    if derivation.root_project_id != root_project_id
        || derivation.source_snapshot_id != source_snapshot_id
        || derivation.derived_kind != original.project_kind
    {
        return Err(format!("派生项目关系与项目主体不一致: {}", original.id));
    }

    let duplicated_snapshot =
        duplicate_source_snapshot(db, root_project_id, source_snapshot_id, now).await?;
    let new_snapshot_id = duplicated_snapshot.snapshot.id;
    let new_project = insert_project_copy(
        db,
        original,
        ProjectCopyInsert {
            user_id,
            title: new_title.to_string(),
            root_project_id: Some(root_project_id),
            parent_project_id: original.parent_project_id,
            source_snapshot_id: Some(new_snapshot_id),
            status: original.status.clone(),
            current_chapter: original.current_chapter,
            total_chapters: original.total_chapters,
            raw_blueprint: None,
            import_analysis_progress: None,
        },
        now,
    )
    .await?;

    project_derivation::ActiveModel {
        root_project_id: Set(root_project_id),
        derived_project_id: Set(new_project.id),
        derived_kind: Set(original.project_kind.clone()),
        source_snapshot_id: Set(new_snapshot_id),
        chapter_scope_json: Set(derivation.chapter_scope_json),
        creation_options_json: Set(derivation.creation_options_json),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(|e| e.to_string())?;

    duplicate_derived_workflow(
        db,
        original.id,
        &new_project,
        root_project_id,
        source_snapshot_id,
        &duplicated_snapshot,
        now,
        data_dir,
        copied_media_files,
    )
    .await?;

    Ok(new_project)
}

struct DuplicatedSourceSnapshot {
    snapshot: project_source_snapshot::Model,
    chapter_ids: HashMap<i64, i64>,
}

async fn duplicate_source_snapshot<C>(
    db: &C,
    root_project_id: i64,
    source_snapshot_id: i64,
    now: chrono::NaiveDateTime,
) -> Result<DuplicatedSourceSnapshot, String>
where
    C: ConnectionTrait,
{
    let snapshot = project_source_snapshot::Entity::find_by_id(source_snapshot_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("源文本快照不存在: {source_snapshot_id}"))?;
    if snapshot.root_project_id != root_project_id {
        return Err(format!(
            "源文本快照不属于根文本项目: snapshot_id={source_snapshot_id}, root_project_id={root_project_id}"
        ));
    }
    let new_snapshot = project_source_snapshot::ActiveModel {
        root_project_id: Set(snapshot.root_project_id),
        source_version_hash: Set(snapshot.source_version_hash),
        blueprint_version_id: Set(snapshot.blueprint_version_id),
        chapter_scope_json: Set(snapshot.chapter_scope_json),
        chapter_hashes_json: Set(snapshot.chapter_hashes_json),
        profile_hashes_json: Set(snapshot.profile_hashes_json),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(|e| e.to_string())?;

    let mut chapter_ids = HashMap::new();
    for chapter in source_snapshot_chapter::Entity::find()
        .filter(source_snapshot_chapter::Column::SnapshotId.eq(source_snapshot_id))
        .order_by_asc(source_snapshot_chapter::Column::ChapterNumber)
        .all(db)
        .await
        .map_err(|e| e.to_string())?
    {
        let copied = source_snapshot_chapter::ActiveModel {
            snapshot_id: Set(new_snapshot.id),
            root_project_id: Set(chapter.root_project_id),
            source_chapter_id: Set(chapter.source_chapter_id),
            chapter_number: Set(chapter.chapter_number),
            title: Set(chapter.title),
            content: Set(chapter.content),
            summary: Set(chapter.summary),
            analysis_data_json: Set(chapter.analysis_data_json),
            content_hash: Set(chapter.content_hash),
            analysis_hash: Set(chapter.analysis_hash),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|e| e.to_string())?;
        chapter_ids.insert(chapter.id, copied.id);
    }
    copy_snapshot_detail_rows!(
        db,
        source_snapshot_id,
        new_snapshot.id,
        source_snapshot_outline,
        [source_outline_id, outline_kind, payload_json, payload_hash]
    );
    copy_snapshot_detail_rows!(
        db,
        source_snapshot_id,
        new_snapshot.id,
        source_snapshot_profile,
        [profile_kind, source_profile_id, payload_json, payload_hash]
    );

    Ok(DuplicatedSourceSnapshot {
        snapshot: new_snapshot,
        chapter_ids,
    })
}

async fn duplicate_derived_workflow<C>(
    db: &C,
    source_project_id: i64,
    new_project: &novel_project::Model,
    root_project_id: i64,
    old_snapshot_id: i64,
    duplicated_snapshot: &DuplicatedSourceSnapshot,
    now: chrono::NaiveDateTime,
    data_dir: &Path,
    copied_media_files: &mut Vec<PathBuf>,
) -> Result<(), String>
where
    C: ConnectionTrait,
{
    let copied_specs = copy_generation_specs(
        db,
        source_project_id,
        new_project.id,
        root_project_id,
        duplicated_snapshot,
        now,
    )
    .await?;
    let prompt_id_map =
        copy_prompt_artifacts(db, &copied_specs.spec_ids, &copied_specs.spec_id_map, now).await?;
    let text_anchor_id_map = copy_referenced_text_anchors(
        db,
        root_project_id,
        old_snapshot_id,
        duplicated_snapshot,
        &copied_specs.spec_ids,
        now,
    )
    .await?;
    copy_generation_spec_text_anchor_refs(
        db,
        &copied_specs.spec_ids,
        &copied_specs.spec_id_map,
        &text_anchor_id_map,
    )
    .await?;
    copy_generation_spec_entity_refs(db, &copied_specs.spec_ids, &copied_specs.spec_id_map).await?;
    let asset_id_map = copy_media_assets(
        db,
        source_project_id,
        new_project.id,
        root_project_id,
        duplicated_snapshot,
        now,
        data_dir,
        copied_media_files,
    )
    .await?;
    copy_media_provenance(
        db,
        &asset_id_map,
        &copied_specs.spec_id_map,
        &prompt_id_map,
        now,
    )
    .await?;
    let segment_id_map = copy_video_segments(
        db,
        source_project_id,
        new_project.id,
        root_project_id,
        duplicated_snapshot,
        &copied_specs.spec_id_map,
        &asset_id_map,
        now,
    )
    .await?;
    update_generation_spec_segment_targets(
        db,
        &copied_specs.segment_target_updates,
        &segment_id_map,
        now,
    )
    .await?;
    let timeline_id_map = copy_assembly_timelines(
        db,
        source_project_id,
        new_project.id,
        duplicated_snapshot,
        &asset_id_map,
        now,
    )
    .await?;
    let _clip_id_map =
        copy_assembly_clips(db, &timeline_id_map, &asset_id_map, &segment_id_map).await?;
    copy_generation_spec_media_refs(
        db,
        &copied_specs.spec_ids,
        &copied_specs.spec_id_map,
        &asset_id_map,
    )
    .await?;
    copy_media_bindings(
        db,
        source_project_id,
        &asset_id_map,
        &segment_id_map,
        &timeline_id_map,
        None,
        now,
    )
    .await?;
    Ok(())
}

struct CopiedGenerationSpecs {
    spec_ids: Vec<i64>,
    spec_id_map: HashMap<i64, i64>,
    segment_target_updates: Vec<(i64, i64)>,
}

async fn copy_generation_specs<C>(
    db: &C,
    source_project_id: i64,
    new_project_id: i64,
    root_project_id: i64,
    duplicated_snapshot: &DuplicatedSourceSnapshot,
    now: chrono::NaiveDateTime,
) -> Result<CopiedGenerationSpecs, String>
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
    let mut segment_target_updates = Vec::new();
    for spec in &specs {
        let source_snapshot_chapter_id = remap_optional_id(
            &duplicated_snapshot.chapter_ids,
            spec.source_snapshot_chapter_id,
            "generation_spec.source_snapshot_chapter_id",
        )?;
        let target_entity_id = if spec.target_entity_kind == "chapter" {
            remap_optional_id(
                &duplicated_snapshot.chapter_ids,
                spec.target_entity_id,
                "generation_spec.target_entity_id",
            )?
        } else if spec.target_entity_kind == "segment" {
            None
        } else {
            spec.target_entity_id
        };
        let copied = generation_spec::ActiveModel {
            project_id: Set(new_project_id),
            root_project_id: Set(root_project_id),
            source_context_kind: Set(Some("snapshot_backed".to_string())),
            source_snapshot_id: Set(Some(duplicated_snapshot.snapshot.id)),
            source_chapter_id: Set(spec.source_chapter_id),
            source_snapshot_chapter_id: Set(source_snapshot_chapter_id),
            target_kind: Set(spec.target_kind.clone()),
            target_entity_kind: Set(spec.target_entity_kind.clone()),
            target_entity_id: Set(target_entity_id),
            status: Set(spec.status.clone()),
            source_version_hash: Set(duplicated_snapshot.snapshot.source_version_hash.clone()),
            params_json: Set(spec.params_json.clone()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|err| err.to_string())?;
        if spec.target_entity_kind == "segment"
            && let Some(old_segment_id) = spec.target_entity_id
        {
            segment_target_updates.push((copied.id, old_segment_id));
        }
        spec_id_map.insert(spec.id, copied.id);
    }
    Ok(CopiedGenerationSpecs {
        spec_ids: specs.iter().map(|spec| spec.id).collect(),
        spec_id_map,
        segment_target_updates,
    })
}

async fn copy_prompt_artifacts<C>(
    db: &C,
    old_spec_ids: &[i64],
    spec_id_map: &HashMap<i64, i64>,
    now: chrono::NaiveDateTime,
) -> Result<HashMap<i64, i64>, String>
where
    C: ConnectionTrait,
{
    if old_spec_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let prompts = prompt_artifact::Entity::find()
        .filter(prompt_artifact::Column::GenerationSpecId.is_in(old_spec_ids.to_vec()))
        .order_by_asc(prompt_artifact::Column::Id)
        .all(db)
        .await
        .map_err(|err| err.to_string())?;
    let mut prompt_id_map = HashMap::new();
    for prompt in prompts {
        let generation_spec_id = remap_required_id(
            spec_id_map,
            prompt.generation_spec_id,
            "prompt_artifact.generation_spec_id",
        )?;
        let copied = prompt_artifact::ActiveModel {
            generation_spec_id: Set(generation_spec_id),
            prompt_kind: Set(prompt.prompt_kind),
            prompt_text: Set(prompt.prompt_text),
            renderer_version: Set(prompt.renderer_version),
            edited_by_user: Set(prompt.edited_by_user),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|err| err.to_string())?;
        prompt_id_map.insert(prompt.id, copied.id);
    }
    Ok(prompt_id_map)
}

async fn copy_referenced_text_anchors<C>(
    db: &C,
    root_project_id: i64,
    old_snapshot_id: i64,
    duplicated_snapshot: &DuplicatedSourceSnapshot,
    old_spec_ids: &[i64],
    now: chrono::NaiveDateTime,
) -> Result<HashMap<i64, i64>, String>
where
    C: ConnectionTrait,
{
    if old_spec_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let refs = generation_spec_text_anchor::Entity::find()
        .filter(generation_spec_text_anchor::Column::GenerationSpecId.is_in(old_spec_ids.to_vec()))
        .all(db)
        .await
        .map_err(|err| err.to_string())?;
    let anchor_ids = refs
        .iter()
        .map(|item| item.text_anchor_id)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if anchor_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let anchors = text_anchor::Entity::find()
        .filter(text_anchor::Column::Id.is_in(anchor_ids))
        .order_by_asc(text_anchor::Column::Id)
        .all(db)
        .await
        .map_err(|err| err.to_string())?;
    let mut anchor_id_map = HashMap::new();
    for anchor in anchors {
        if anchor.project_id != root_project_id
            || anchor.source_snapshot_id != Some(old_snapshot_id)
        {
            return Err(format!(
                "派生项目生成意图引用的文本锚点不属于源快照: text_anchor_id={}",
                anchor.id
            ));
        }
        let source_snapshot_chapter_id = remap_optional_id(
            &duplicated_snapshot.chapter_ids,
            anchor.source_snapshot_chapter_id,
            "text_anchor.source_snapshot_chapter_id",
        )?;
        let source_id = source_snapshot_chapter_id.ok_or_else(|| {
            format!(
                "源快照文本锚点缺少 source_snapshot_chapter_id: text_anchor_id={}",
                anchor.id
            )
        })?;
        let copied = text_anchor::ActiveModel {
            project_id: Set(anchor.project_id),
            source_chapter_id: Set(anchor.source_chapter_id),
            source_snapshot_id: Set(Some(duplicated_snapshot.snapshot.id)),
            source_snapshot_chapter_id: Set(source_snapshot_chapter_id),
            source_kind: Set(anchor.source_kind),
            source_id: Set(source_id),
            paragraph_index: Set(anchor.paragraph_index),
            start_offset: Set(anchor.start_offset),
            end_offset: Set(anchor.end_offset),
            quote: Set(anchor.quote),
            quote_hash: Set(anchor.quote_hash),
            source_version_hash: Set(duplicated_snapshot.snapshot.source_version_hash.clone()),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|err| err.to_string())?;
        anchor_id_map.insert(anchor.id, copied.id);
    }
    Ok(anchor_id_map)
}

copy_generation_spec_ref_rows!(
    copy_generation_spec_text_anchor_refs,
    generation_spec_text_anchor,
    text_anchor_id,
    anchor_id_map,
    "generation_spec_text_anchor.generation_spec_id",
    "generation_spec_text_anchor.text_anchor_id"
);

async fn copy_generation_spec_entity_refs<C>(
    db: &C,
    old_spec_ids: &[i64],
    spec_id_map: &HashMap<i64, i64>,
) -> Result<(), String>
where
    C: ConnectionTrait,
{
    if old_spec_ids.is_empty() {
        return Ok(());
    }
    let refs = generation_spec_entity_ref::Entity::find()
        .filter(generation_spec_entity_ref::Column::GenerationSpecId.is_in(old_spec_ids.to_vec()))
        .all(db)
        .await
        .map_err(|err| err.to_string())?;
    for item in refs {
        generation_spec_entity_ref::ActiveModel {
            generation_spec_id: Set(remap_required_id(
                spec_id_map,
                item.generation_spec_id,
                "generation_spec_entity_ref.generation_spec_id",
            )?),
            entity_kind: Set(item.entity_kind),
            entity_id: Set(item.entity_id),
            role: Set(item.role),
        }
        .insert(db)
        .await
        .map_err(|err| err.to_string())?;
    }
    Ok(())
}

async fn copy_media_assets<C>(
    db: &C,
    source_project_id: i64,
    new_project_id: i64,
    root_project_id: i64,
    duplicated_snapshot: &DuplicatedSourceSnapshot,
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
        if asset.status == "completed" && asset.file_path.is_none() {
            return Err(format!("已完成媒体资产缺少 file_path: {}", asset.id));
        }
        let source_snapshot_chapter_id = remap_optional_id(
            &duplicated_snapshot.chapter_ids,
            asset.source_snapshot_chapter_id,
            "media_asset.source_snapshot_chapter_id",
        )?;
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
            root_project_id: Set(root_project_id),
            source_chapter_id: Set(asset.source_chapter_id),
            source_snapshot_chapter_id: Set(source_snapshot_chapter_id),
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

async fn copy_media_provenance<C>(
    db: &C,
    asset_id_map: &HashMap<i64, i64>,
    spec_id_map: &HashMap<i64, i64>,
    prompt_id_map: &HashMap<i64, i64>,
    now: chrono::NaiveDateTime,
) -> Result<(), String>
where
    C: ConnectionTrait,
{
    if asset_id_map.is_empty() {
        return Ok(());
    }
    let old_asset_ids = asset_id_map.keys().copied().collect::<Vec<_>>();
    let records = media_provenance::Entity::find()
        .filter(media_provenance::Column::MediaAssetId.is_in(old_asset_ids))
        .all(db)
        .await
        .map_err(|err| err.to_string())?;
    for item in records {
        let media_asset_id = remap_required_id(
            asset_id_map,
            item.media_asset_id,
            "media_provenance.media_asset_id",
        )?;
        media_provenance::ActiveModel {
            media_asset_id: Set(media_asset_id),
            generation_spec_id: Set(remap_optional_id(
                spec_id_map,
                item.generation_spec_id,
                "media_provenance.generation_spec_id",
            )?),
            prompt_artifact_id: Set(remap_optional_id(
                prompt_id_map,
                item.prompt_artifact_id,
                "media_provenance.prompt_artifact_id",
            )?),
            provider_job_id: Set(item.provider_job_id),
            source_media_asset_id: Set(remap_optional_id(
                asset_id_map,
                item.source_media_asset_id,
                "media_provenance.source_media_asset_id",
            )?),
            source_external_path: Set(item.source_external_path),
            imported_from_user: Set(item.imported_from_user),
            request_snapshot_json: Set(item.request_snapshot_json),
            created_at: Set(now),
        }
        .insert(db)
        .await
        .map_err(|err| err.to_string())?;
    }
    Ok(())
}

async fn copy_video_segments<C>(
    db: &C,
    source_project_id: i64,
    new_project_id: i64,
    root_project_id: i64,
    duplicated_snapshot: &DuplicatedSourceSnapshot,
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
        let copied = video_segment::ActiveModel {
            project_id: Set(new_project_id),
            root_project_id: Set(root_project_id),
            source_chapter_id: Set(segment.source_chapter_id),
            source_snapshot_chapter_id: Set(remap_optional_id(
                &duplicated_snapshot.chapter_ids,
                segment.source_snapshot_chapter_id,
                "video_segment.source_snapshot_chapter_id",
            )?),
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

async fn update_generation_spec_segment_targets<C>(
    db: &C,
    updates: &[(i64, i64)],
    segment_id_map: &HashMap<i64, i64>,
    now: chrono::NaiveDateTime,
) -> Result<(), String>
where
    C: ConnectionTrait,
{
    for (new_spec_id, old_segment_id) in updates {
        generation_spec::ActiveModel {
            id: Set(*new_spec_id),
            target_entity_id: Set(Some(remap_required_id(
                segment_id_map,
                *old_segment_id,
                "generation_spec.target_entity_id",
            )?)),
            updated_at: Set(now),
            ..Default::default()
        }
        .update(db)
        .await
        .map_err(|err| err.to_string())?;
    }
    Ok(())
}

async fn copy_assembly_timelines<C>(
    db: &C,
    source_project_id: i64,
    new_project_id: i64,
    duplicated_snapshot: &DuplicatedSourceSnapshot,
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
        let copied = assembly_timeline::ActiveModel {
            project_id: Set(new_project_id),
            source_chapter_id: Set(timeline.source_chapter_id),
            source_snapshot_chapter_id: Set(remap_optional_id(
                &duplicated_snapshot.chapter_ids,
                timeline.source_snapshot_chapter_id,
                "assembly_timeline.source_snapshot_chapter_id",
            )?),
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

async fn copy_assembly_clips<C>(
    db: &C,
    timeline_id_map: &HashMap<i64, i64>,
    asset_id_map: &HashMap<i64, i64>,
    segment_id_map: &HashMap<i64, i64>,
) -> Result<HashMap<i64, i64>, String>
where
    C: ConnectionTrait,
{
    if timeline_id_map.is_empty() {
        return Ok(HashMap::new());
    }
    let clips = assembly_clip::Entity::find()
        .filter(
            assembly_clip::Column::TimelineId
                .is_in(timeline_id_map.keys().copied().collect::<Vec<_>>()),
        )
        .order_by_asc(assembly_clip::Column::TimelineId)
        .order_by_asc(assembly_clip::Column::OrderIndex)
        .all(db)
        .await
        .map_err(|err| err.to_string())?;
    let mut clip_id_map = HashMap::new();
    for clip in clips {
        let old_id = clip.id;
        let copied = assembly_clip::ActiveModel {
            timeline_id: Set(remap_required_id(
                timeline_id_map,
                clip.timeline_id,
                "assembly_clip.timeline_id",
            )?),
            media_asset_id: Set(remap_required_id(
                asset_id_map,
                clip.media_asset_id,
                "assembly_clip.media_asset_id",
            )?),
            order_index: Set(clip.order_index),
            trim_start_secs: Set(clip.trim_start_secs),
            trim_end_secs: Set(clip.trim_end_secs),
            volume: Set(clip.volume),
            transition_kind: Set(clip.transition_kind),
            source_segment_id: Set(remap_optional_id(
                segment_id_map,
                clip.source_segment_id,
                "assembly_clip.source_segment_id",
            )?),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|err| err.to_string())?;
        clip_id_map.insert(old_id, copied.id);
    }
    Ok(clip_id_map)
}

copy_generation_spec_ref_rows!(
    copy_generation_spec_media_refs,
    generation_spec_media_ref,
    media_asset_id,
    asset_id_map,
    "generation_spec_media_ref.generation_spec_id",
    "generation_spec_media_ref.media_asset_id"
);

async fn copy_media_bindings<C>(
    db: &C,
    source_project_id: i64,
    asset_id_map: &HashMap<i64, i64>,
    segment_id_map: &HashMap<i64, i64>,
    timeline_id_map: &HashMap<i64, i64>,
    video_id_map: Option<&HashMap<i64, i64>>,
    now: chrono::NaiveDateTime,
) -> Result<(), String>
where
    C: ConnectionTrait,
{
    if asset_id_map.is_empty() {
        return Ok(());
    }
    let bindings = media_binding::Entity::find()
        .filter(
            media_binding::Column::MediaAssetId
                .is_in(asset_id_map.keys().copied().collect::<Vec<_>>()),
        )
        .order_by_asc(media_binding::Column::Id)
        .all(db)
        .await
        .map_err(|err| err.to_string())?;
    for binding in bindings {
        let target_id = match binding.target_kind.as_str() {
            "video_segment" => {
                remap_required_id(segment_id_map, binding.target_id, "media_binding.target_id")?
            }
            "assembly_timeline" => remap_required_id(
                timeline_id_map,
                binding.target_id,
                "media_binding.target_id",
            )?,
            "generated_video" => remap_required_id(
                video_id_map.ok_or_else(|| {
                    format!(
                        "不支持复制 generated_video 媒体绑定: source_project_id={source_project_id}"
                    )
                })?,
                binding.target_id,
                "media_binding.target_id",
            )?,
            "character_profile" | "object_profile" | "scene_profile" | "chapter" => {
                binding.target_id
            }
            other => {
                return Err(format!(
                    "不支持复制的媒体绑定目标类型: source_project_id={source_project_id}, target_kind={other}"
                ));
            }
        };
        media_binding::ActiveModel {
            media_asset_id: Set(remap_required_id(
                asset_id_map,
                binding.media_asset_id,
                "media_binding.media_asset_id",
            )?),
            target_kind: Set(binding.target_kind),
            target_id: Set(target_id),
            role: Set(binding.role),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|err| err.to_string())?;
    }
    Ok(())
}

fn copy_media_file(
    data_dir: &Path,
    new_project_id: i64,
    source_media_asset_id: i64,
    field: &str,
    relative_path: Option<&str>,
    copied_media_files: &mut Vec<PathBuf>,
) -> Result<Option<String>, String> {
    let Some(relative_path) = relative_path else {
        return Ok(None);
    };
    let source = file_utils::canonicalize_data_file_path(data_dir, relative_path, "媒体文件")?;
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("媒体文件缺少扩展名: {relative_path}"))?;
    let target_relative = format!(
        "projects/{new_project_id}/media/duplicated/{source_media_asset_id}_{field}.{extension}"
    );
    let target = data_dir.join(&target_relative);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("无法创建媒体复制目录: {err}"))?;
    }
    fs::copy(&source, &target).map_err(|err| {
        format!(
            "复制媒体文件失败: {} -> {}: {err}",
            source.display(),
            target.display()
        )
    })?;
    copied_media_files.push(target);
    Ok(Some(target_relative))
}

fn remap_required_id(map: &HashMap<i64, i64>, old_id: i64, field: &str) -> Result<i64, String> {
    map.get(&old_id)
        .copied()
        .ok_or_else(|| format!("复制派生项目时无法重映射 {field}: {old_id}"))
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

fn cleanup_copied_files(paths: &[PathBuf]) {
    for path in paths.iter().rev() {
        let _ = fs::remove_file(path);
    }
}

fn required_text<'a>(value: &'a str, label: &str) -> Result<&'a str, String> {
    let value = value.trim();
    if value.is_empty() {
        Err(format!("{label}不能为空"))
    } else {
        Ok(value)
    }
}

fn required_project_status(value: &str, label: &str) -> Result<String, String> {
    let value = required_text(value, label)?;
    let status = StateValidator::parse_status(value).map_err(|e| e.to_string())?;
    Ok(status.as_str().to_string())
}

async fn branch_project_data<C>(
    db: &C,
    original: &novel_project::Model,
    branch_title: &str,
    branch_status: &str,
    now: chrono::NaiveDateTime,
) -> Result<novel_project::Model, String>
where
    C: ConnectionTrait,
{
    let original_status =
        StateValidator::parse_status(&original.status).map_err(|e| e.to_string())?;
    let branch_status = StateValidator::parse_status(branch_status).map_err(|e| e.to_string())?;
    if branch_status.ordinal() > original_status.ordinal() {
        return Err(format!(
            "分支状态不能超过源项目当前状态: source={}, branch={}",
            original_status.as_str(),
            branch_status.as_str()
        ));
    }

    let source_graph = load_blueprint_graph(db, original.id).await?;
    let should_copy_blueprint = branch_status_includes_blueprint(branch_status);
    if should_copy_blueprint && source_graph.is_none() {
        return Err(format!(
            "源项目状态为 {}，但缺少可复制的蓝图数据",
            branch_status.as_str()
        ));
    }
    let raw_blueprint = if should_copy_blueprint {
        match (&original.raw_blueprint, source_graph.as_ref()) {
            (Some(raw_blueprint), _) => Some(raw_blueprint.clone()),
            (None, Some(graph)) => Some(blueprint_graph_raw_blueprint(graph)?),
            (None, None) => None,
        }
    } else {
        None
    };
    let current_chapter = if branch_status_includes_chapters(branch_status) {
        original.current_chapter
    } else {
        0
    };
    let total_chapters = if should_copy_blueprint {
        original.total_chapters
    } else {
        None
    };

    let new_project = insert_project_copy(
        db,
        original,
        ProjectCopyInsert {
            user_id: original.user_id,
            title: branch_title.to_string(),
            root_project_id: original.root_project_id,
            parent_project_id: original.parent_project_id,
            source_snapshot_id: original.source_snapshot_id,
            status: branch_status.as_str().to_string(),
            current_chapter,
            total_chapters,
            raw_blueprint,
            import_analysis_progress: original.import_analysis_progress.clone(),
        },
        now,
    )
    .await?;

    if should_copy_blueprint && let Some(graph) = &source_graph {
        copy_blueprint_graph(db, new_project.id, graph, now).await?;
    }
    let part_id_map = if branch_status_includes_part_outlines(branch_status) {
        copy_part_outlines(db, original.id, new_project.id, now).await?
    } else {
        std::collections::HashMap::new()
    };
    let outline_id_map = if branch_status_includes_chapter_outlines(branch_status) {
        copy_chapter_outlines(db, original.id, new_project.id, &part_id_map, now).await?
    } else {
        std::collections::HashMap::new()
    };
    if branch_status_includes_chapters(branch_status) {
        copy_chapters(db, original.id, new_project.id, &outline_id_map, now).await?;
    }

    Ok(new_project)
}

fn branch_status_includes_blueprint(branch_status: ProjectStatus) -> bool {
    branch_status != ProjectStatus::Draft
}

fn branch_status_includes_part_outlines(branch_status: ProjectStatus) -> bool {
    branch_status.ordinal() >= ProjectStatus::PartOutlinesReady.ordinal()
}

fn branch_status_includes_chapter_outlines(branch_status: ProjectStatus) -> bool {
    branch_status.ordinal() >= ProjectStatus::ChapterOutlinesReady.ordinal()
}

fn branch_status_includes_chapters(branch_status: ProjectStatus) -> bool {
    branch_status.ordinal() >= ProjectStatus::Writing.ordinal()
}

fn ensure_text_project_for_copy(
    project: &novel_project::Model,
    operation: &str,
) -> Result<(), String> {
    if project.project_kind != PROJECT_KIND_TEXT {
        return Err(format!("{operation}仅支持文本项目"));
    }
    Ok(())
}
