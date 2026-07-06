use super::*;

pub(super) struct SourceDiffService;

#[derive(Debug, Clone)]
pub(super) struct SourceSnapshotDiff {
    pub(super) chapter_changes: Vec<ChapterChange>,
    pub(super) profile_changes: Vec<ProfileChange>,
}

#[derive(Debug, Clone)]
pub(super) struct ChapterChange {
    source_chapter_id: i64,
    source_snapshot_chapter_id: Option<i64>,
    change_kind: String,
    source_summary_json: Value,
}

#[derive(Debug, Clone)]
pub(super) struct ProfileChange {
    profile_kind: String,
    source_profile_id: i64,
    change_kind: String,
    source_summary_json: Value,
}

struct InsertConflictInput<'a> {
    conflict_scope: &'a str,
    target_kind: &'a str,
    target_id: i64,
    source_change_kind: &'a str,
    local_state_kind: &'a str,
    severity: &'a str,
    source_summary_json: Value,
    local_summary_json: Value,
    affected_refs_json: Value,
}

impl SourceDiffService {
    pub(super) async fn diff_source_snapshots<C>(
        db: &C,
        from_snapshot_id: i64,
        to_snapshot_id: i64,
    ) -> Result<SourceSnapshotDiff, AppError>
    where
        C: ConnectionTrait,
    {
        Ok(SourceSnapshotDiff {
            chapter_changes: diff_snapshot_chapters(db, from_snapshot_id, to_snapshot_id).await?,
            profile_changes: diff_snapshot_profiles(db, from_snapshot_id, to_snapshot_id).await?,
        })
    }
}

async fn diff_snapshot_chapters<C>(
    db: &C,
    from_snapshot_id: i64,
    to_snapshot_id: i64,
) -> Result<Vec<ChapterChange>, AppError>
where
    C: ConnectionTrait,
{
    let from_rows = source_snapshot_chapter::Entity::find()
        .filter(source_snapshot_chapter::Column::SnapshotId.eq(from_snapshot_id))
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let to_rows = source_snapshot_chapter::Entity::find()
        .filter(source_snapshot_chapter::Column::SnapshotId.eq(to_snapshot_id))
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let to_by_source = to_rows
        .iter()
        .map(|row| (row.source_chapter_id, row))
        .collect::<HashMap<_, _>>();
    let from_by_source = from_rows
        .iter()
        .map(|row| (row.source_chapter_id, row))
        .collect::<HashMap<_, _>>();
    let mut changes = Vec::new();
    for old in &from_rows {
        match to_by_source.get(&old.source_chapter_id).copied() {
            Some(new)
                if new.content_hash != old.content_hash
                    || new.analysis_hash != old.analysis_hash =>
            {
                changes.push(ChapterChange {
                    source_chapter_id: old.source_chapter_id,
                    source_snapshot_chapter_id: Some(new.id),
                    change_kind: "updated".to_string(),
                    source_summary_json: json!({
                        "old_chapter_number": old.chapter_number,
                        "new_chapter_number": new.chapter_number,
                        "title": new.title,
                        "old_content_hash": old.content_hash,
                        "new_content_hash": new.content_hash,
                        "old_analysis_hash": old.analysis_hash,
                        "new_analysis_hash": new.analysis_hash,
                    }),
                });
            }
            Some(new) if new.chapter_number != old.chapter_number => {
                changes.push(ChapterChange {
                    source_chapter_id: old.source_chapter_id,
                    source_snapshot_chapter_id: Some(new.id),
                    change_kind: "reordered".to_string(),
                    source_summary_json: json!({
                        "old_chapter_number": old.chapter_number,
                        "new_chapter_number": new.chapter_number,
                        "title": new.title,
                    }),
                });
            }
            None => changes.push(ChapterChange {
                source_chapter_id: old.source_chapter_id,
                source_snapshot_chapter_id: None,
                change_kind: "deleted".to_string(),
                source_summary_json: json!({
                    "chapter_number": old.chapter_number,
                    "title": old.title,
                }),
            }),
            _ => {}
        }
    }
    for new in &to_rows {
        if !from_by_source.contains_key(&new.source_chapter_id) {
            changes.push(ChapterChange {
                source_chapter_id: new.source_chapter_id,
                source_snapshot_chapter_id: Some(new.id),
                change_kind: "added".to_string(),
                source_summary_json: json!({
                    "chapter_number": new.chapter_number,
                    "title": new.title,
                }),
            });
        }
    }
    Ok(changes)
}

async fn diff_snapshot_profiles<C>(
    db: &C,
    from_snapshot_id: i64,
    to_snapshot_id: i64,
) -> Result<Vec<ProfileChange>, AppError>
where
    C: ConnectionTrait,
{
    let from_rows = source_snapshot_profile::Entity::find()
        .filter(source_snapshot_profile::Column::SnapshotId.eq(from_snapshot_id))
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let to_rows = source_snapshot_profile::Entity::find()
        .filter(source_snapshot_profile::Column::SnapshotId.eq(to_snapshot_id))
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let key =
        |row: &source_snapshot_profile::Model| (row.profile_kind.clone(), row.source_profile_id);
    let to_by_profile = to_rows
        .iter()
        .map(|row| (key(row), row))
        .collect::<HashMap<_, _>>();
    let from_by_profile = from_rows
        .iter()
        .map(|row| (key(row), row))
        .collect::<HashMap<_, _>>();
    let mut changes = Vec::new();
    for old in &from_rows {
        match to_by_profile
            .get(&(old.profile_kind.clone(), old.source_profile_id))
            .copied()
        {
            Some(new) if new.payload_hash != old.payload_hash => changes.push(ProfileChange {
                profile_kind: old.profile_kind.clone(),
                source_profile_id: old.source_profile_id,
                change_kind: "updated".to_string(),
                source_summary_json: json!({
                    "profile_kind": old.profile_kind,
                    "source_profile_id": old.source_profile_id,
                    "old_payload_hash": old.payload_hash,
                    "new_payload_hash": new.payload_hash,
                }),
            }),
            None => changes.push(ProfileChange {
                profile_kind: old.profile_kind.clone(),
                source_profile_id: old.source_profile_id,
                change_kind: "deleted".to_string(),
                source_summary_json: json!({
                    "profile_kind": old.profile_kind,
                    "source_profile_id": old.source_profile_id,
                    "old_payload_hash": old.payload_hash,
                }),
            }),
            _ => {}
        }
    }
    for new in &to_rows {
        if !from_by_profile.contains_key(&(new.profile_kind.clone(), new.source_profile_id)) {
            changes.push(ProfileChange {
                profile_kind: new.profile_kind.clone(),
                source_profile_id: new.source_profile_id,
                change_kind: "added".to_string(),
                source_summary_json: json!({
                    "profile_kind": new.profile_kind,
                    "source_profile_id": new.source_profile_id,
                    "new_payload_hash": new.payload_hash,
                }),
            });
        }
    }
    Ok(changes)
}

pub(super) async fn insert_conflicts_for_change<C>(
    db: &C,
    run: &source_sync_run::Model,
    change: &ChapterChange,
) -> Result<Vec<source_sync_conflict::Model>, AppError>
where
    C: ConnectionTrait,
{
    let specs = generation_spec::Entity::find()
        .filter(generation_spec::Column::ProjectId.eq(run.derived_project_id))
        .filter(generation_spec::Column::SourceChapterId.eq(change.source_chapter_id))
        .all(db)
        .await
        .map_err(AppError::Database)?;
    if specs.is_empty() {
        return Ok(vec![
            insert_conflict(
                db,
                run,
                InsertConflictInput {
                    conflict_scope: "text_anchor",
                    target_kind: "chapter",
                    target_id: change.source_chapter_id,
                    source_change_kind: &change.change_kind,
                    local_state_kind: "clean",
                    severity: if change.change_kind == "deleted" {
                        "blocking"
                    } else {
                        "info"
                    },
                    source_summary_json: change.source_summary_json.clone(),
                    local_summary_json: json!({ "local": "no_generation_spec" }),
                    affected_refs_json: json!({ "source_chapter_id": change.source_chapter_id }),
                },
            )
            .await?,
        ]);
    }
    let mut conflicts = Vec::new();
    for spec in specs {
        let state = local_state_for_spec(db, &spec).await?;
        let severity = severity_for_impacted_local_state(&state, &change.change_kind);
        conflicts.push(
            insert_conflict(
                db,
                run,
                InsertConflictInput {
                    conflict_scope: "generation_spec",
                    target_kind: "segment",
                    target_id: spec.id,
                    source_change_kind: &change.change_kind,
                    local_state_kind: &state,
                    severity,
                    source_summary_json: change.source_summary_json.clone(),
                    local_summary_json: json!({
                        "generation_spec_id": spec.id,
                        "target_kind": spec.target_kind,
                        "status": spec.status,
                    }),
                    affected_refs_json: json!({
                        "source_chapter_id": change.source_chapter_id,
                        "source_snapshot_chapter_id": change.source_snapshot_chapter_id,
                    }),
                },
            )
            .await?,
        );
    }
    Ok(conflicts)
}

pub(super) async fn insert_galgame_conflicts_for_change<C>(
    db: &C,
    run: &source_sync_run::Model,
    change: &ChapterChange,
) -> Result<Vec<source_sync_conflict::Model>, AppError>
where
    C: ConnectionTrait,
{
    let Some(script) = galgame_script_for_run(db, run).await? else {
        return Ok(Vec::new());
    };
    let Some(old_snapshot_chapter) = source_snapshot_chapter::Entity::find()
        .filter(source_snapshot_chapter::Column::SnapshotId.eq(run.from_snapshot_id))
        .filter(source_snapshot_chapter::Column::SourceChapterId.eq(change.source_chapter_id))
        .one(db)
        .await
        .map_err(AppError::Database)?
    else {
        return Ok(Vec::new());
    };
    let scenes = galgame_scene::Entity::find()
        .filter(galgame_scene::Column::ScriptId.eq(script.id))
        .filter(galgame_scene::Column::SourceSnapshotChapterId.eq(old_snapshot_chapter.id))
        .order_by_asc(galgame_scene::Column::SceneIndex)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let mut conflicts = Vec::new();
    for scene in scenes {
        let line_count = galgame_line::Entity::find()
            .filter(galgame_line::Column::SceneId.eq(scene.id))
            .count(db)
            .await
            .map_err(AppError::Database)?;
        let anchored_line_count = galgame_line::Entity::find()
            .filter(galgame_line::Column::SceneId.eq(scene.id))
            .filter(galgame_line::Column::TextAnchorId.is_not_null())
            .count(db)
            .await
            .map_err(AppError::Database)?;
        conflicts.push(
            insert_conflict(
                db,
                run,
                InsertConflictInput {
                    conflict_scope: "text_anchor",
                    target_kind: "galgame_scene",
                    target_id: scene.id,
                    source_change_kind: &change.change_kind,
                    local_state_kind: "generated",
                    severity: severity_for_impacted_local_state("generated", &change.change_kind),
                    source_summary_json: change.source_summary_json.clone(),
                    local_summary_json: json!({
                        "galgame_scene_id": scene.id,
                        "scene_index": scene.scene_index,
                        "title": scene.title,
                        "line_count": line_count,
                        "anchored_line_count": anchored_line_count,
                    }),
                    affected_refs_json: json!({
                        "source_chapter_id": change.source_chapter_id,
                        "old_source_snapshot_chapter_id": old_snapshot_chapter.id,
                        "source_snapshot_chapter_id": change.source_snapshot_chapter_id,
                        "galgame_scene_id": scene.id,
                    }),
                },
            )
            .await?,
        );
    }
    Ok(conflicts)
}

pub(super) async fn insert_conflicts_for_profile_change<C>(
    db: &C,
    run: &source_sync_run::Model,
    change: &ProfileChange,
) -> Result<Vec<source_sync_conflict::Model>, AppError>
where
    C: ConnectionTrait,
{
    if change.change_kind == "added" {
        return Ok(Vec::new());
    }
    let refs = generation_spec_entity_ref::Entity::find()
        .filter(generation_spec_entity_ref::Column::EntityKind.eq(&change.profile_kind))
        .filter(generation_spec_entity_ref::Column::EntityId.eq(change.source_profile_id))
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let mut conflicts = Vec::new();
    for item in refs {
        let Some(spec) = generation_spec::Entity::find_by_id(item.generation_spec_id)
            .filter(generation_spec::Column::ProjectId.eq(run.derived_project_id))
            .filter(generation_spec::Column::RootProjectId.eq(run.root_project_id))
            .one(db)
            .await
            .map_err(AppError::Database)?
        else {
            continue;
        };
        let state = local_state_for_spec(db, &spec).await?;
        conflicts.push(
            insert_conflict(
                db,
                run,
                InsertConflictInput {
                    conflict_scope: "profile",
                    target_kind: &change.profile_kind,
                    target_id: change.source_profile_id,
                    source_change_kind: &change.change_kind,
                    local_state_kind: &state,
                    severity: severity_for_impacted_local_state(&state, &change.change_kind),
                    source_summary_json: change.source_summary_json.clone(),
                    local_summary_json: json!({
                        "generation_spec_id": spec.id,
                        "entity_role": item.role,
                        "target_kind": spec.target_kind,
                        "status": spec.status,
                    }),
                    affected_refs_json: json!({
                        "profile_kind": change.profile_kind,
                        "source_profile_id": change.source_profile_id,
                        "generation_spec_id": spec.id,
                    }),
                },
            )
            .await?,
        );
    }
    Ok(conflicts)
}

pub(super) async fn insert_galgame_anchor_conflicts_for_run<C>(
    db: &C,
    run: &source_sync_run::Model,
) -> Result<Vec<source_sync_conflict::Model>, AppError>
where
    C: ConnectionTrait,
{
    let Some(script) = galgame_script_for_run(db, run).await? else {
        return Ok(Vec::new());
    };
    let scenes = galgame_scene::Entity::find()
        .filter(galgame_scene::Column::ScriptId.eq(script.id))
        .order_by_asc(galgame_scene::Column::SceneIndex)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    if scenes.is_empty() {
        return Ok(Vec::new());
    }
    let scene_by_id = scenes
        .iter()
        .map(|scene| (scene.id, scene))
        .collect::<HashMap<_, _>>();
    let scene_ids = scenes.iter().map(|scene| scene.id).collect::<Vec<_>>();
    let lines = galgame_line::Entity::find()
        .filter(galgame_line::Column::SceneId.is_in(scene_ids))
        .filter(galgame_line::Column::TextAnchorId.is_not_null())
        .order_by_asc(galgame_line::Column::SceneId)
        .order_by_asc(galgame_line::Column::LineIndex)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let mut conflicts = Vec::new();
    for line in lines {
        let Some(text_anchor_id) = line.text_anchor_id else {
            continue;
        };
        let old_anchor = text_anchor::Entity::find_by_id(text_anchor_id)
            .one(db)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("文本锚点不存在: {text_anchor_id}")))?;
        if old_anchor.source_snapshot_id != Some(run.from_snapshot_id) {
            continue;
        }
        let candidates = text_anchor::Entity::find()
            .filter(text_anchor::Column::ProjectId.eq(run.root_project_id))
            .filter(text_anchor::Column::SourceSnapshotId.eq(run.to_snapshot_id))
            .filter(text_anchor::Column::QuoteHash.eq(&old_anchor.quote_hash))
            .all(db)
            .await
            .map_err(AppError::Database)?;
        let anchor_change = classify_anchor_change(&old_anchor, &candidates);
        let Some((change_kind, new_anchor_id, severity)) =
            anchor_change_conflict(&anchor_change, "generated")
        else {
            continue;
        };
        let scene = scene_by_id.get(&line.scene_id).copied();
        conflicts.push(
            insert_conflict(
                db,
                run,
                InsertConflictInput {
                    conflict_scope: "text_anchor",
                    target_kind: "galgame_line",
                    target_id: line.id,
                    source_change_kind: change_kind,
                    local_state_kind: "generated",
                    severity,
                    source_summary_json: json!({
                        "old_text_anchor_id": old_anchor.id,
                        "new_text_anchor_id": new_anchor_id,
                        "quote": old_anchor.quote,
                        "quote_hash": old_anchor.quote_hash,
                    }),
                    local_summary_json: json!({
                        "galgame_line_id": line.id,
                        "galgame_scene_id": line.scene_id,
                        "scene_title": scene.map(|item| item.title.clone()),
                        "line_index": line.line_index,
                        "line_type": line.line_type,
                        "text": line.text,
                    }),
                    affected_refs_json: json!({
                        "old_text_anchor_id": old_anchor.id,
                        "new_text_anchor_id": new_anchor_id,
                        "candidate_count": candidates.len(),
                        "galgame_line_id": line.id,
                        "galgame_scene_id": line.scene_id,
                    }),
                },
            )
            .await?,
        );
    }
    Ok(conflicts)
}

pub(super) async fn insert_anchor_conflicts_for_run<C>(
    db: &C,
    run: &source_sync_run::Model,
) -> Result<Vec<source_sync_conflict::Model>, AppError>
where
    C: ConnectionTrait,
{
    let specs = generation_spec::Entity::find()
        .filter(generation_spec::Column::ProjectId.eq(run.derived_project_id))
        .filter(generation_spec::Column::RootProjectId.eq(run.root_project_id))
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let mut conflicts = Vec::new();
    for spec in specs {
        let refs = generation_spec_text_anchor::Entity::find()
            .filter(generation_spec_text_anchor::Column::GenerationSpecId.eq(spec.id))
            .all(db)
            .await
            .map_err(AppError::Database)?;
        if refs.is_empty() {
            continue;
        }
        let state = local_state_for_spec(db, &spec).await?;
        for item in refs {
            let old_anchor = text_anchor::Entity::find_by_id(item.text_anchor_id)
                .one(db)
                .await
                .map_err(AppError::Database)?
                .ok_or_else(|| {
                    AppError::NotFound(format!("文本锚点不存在: {}", item.text_anchor_id))
                })?;
            if old_anchor.source_snapshot_id != Some(run.from_snapshot_id) {
                continue;
            }
            let candidates = text_anchor::Entity::find()
                .filter(text_anchor::Column::ProjectId.eq(run.root_project_id))
                .filter(text_anchor::Column::SourceSnapshotId.eq(run.to_snapshot_id))
                .filter(text_anchor::Column::QuoteHash.eq(&old_anchor.quote_hash))
                .all(db)
                .await
                .map_err(AppError::Database)?;
            let anchor_change = classify_anchor_change(&old_anchor, &candidates);
            let Some((change_kind, new_anchor_id, severity)) =
                anchor_change_conflict(&anchor_change, &state)
            else {
                continue;
            };
            conflicts.push(
                insert_conflict(
                    db,
                    run,
                    InsertConflictInput {
                        conflict_scope: "text_anchor",
                        target_kind: "segment",
                        target_id: spec.id,
                        source_change_kind: change_kind,
                        local_state_kind: &state,
                        severity,
                        source_summary_json: json!({
                            "old_text_anchor_id": old_anchor.id,
                            "new_text_anchor_id": new_anchor_id,
                            "quote": old_anchor.quote,
                            "quote_hash": old_anchor.quote_hash,
                        }),
                        local_summary_json: json!({
                            "generation_spec_id": spec.id,
                            "text_anchor_role": item.role,
                            "target_kind": spec.target_kind,
                            "status": spec.status,
                        }),
                        affected_refs_json: json!({
                            "old_text_anchor_id": old_anchor.id,
                            "new_text_anchor_id": new_anchor_id,
                            "candidate_count": candidates.len(),
                        }),
                    },
                )
                .await?,
            );
        }
    }
    Ok(conflicts)
}

async fn galgame_script_for_run<C>(
    db: &C,
    run: &source_sync_run::Model,
) -> Result<Option<galgame_script::Model>, AppError>
where
    C: ConnectionTrait,
{
    galgame_script::Entity::find()
        .filter(galgame_script::Column::ProjectId.eq(run.derived_project_id))
        .filter(galgame_script::Column::RootProjectId.eq(run.root_project_id))
        .one(db)
        .await
        .map_err(AppError::Database)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum AnchorChange {
    Unchanged,
    Moved(i64),
    Missing,
    Ambiguous,
}

pub(super) fn classify_anchor_change(
    old_anchor: &text_anchor::Model,
    candidates: &[text_anchor::Model],
) -> AnchorChange {
    if candidates.is_empty() {
        return AnchorChange::Missing;
    }
    if candidates.len() > 1 {
        return AnchorChange::Ambiguous;
    }
    let new_anchor = &candidates[0];
    if new_anchor.source_chapter_id == old_anchor.source_chapter_id
        && new_anchor.paragraph_index == old_anchor.paragraph_index
        && new_anchor.start_offset == old_anchor.start_offset
        && new_anchor.end_offset == old_anchor.end_offset
    {
        AnchorChange::Unchanged
    } else {
        AnchorChange::Moved(new_anchor.id)
    }
}

pub(super) fn anchor_change_conflict(
    change: &AnchorChange,
    local_state: &str,
) -> Option<(&'static str, Option<i64>, &'static str)> {
    match change {
        AnchorChange::Unchanged => None,
        AnchorChange::Moved(new_anchor_id) => Some((
            "moved",
            Some(*new_anchor_id),
            severity_for_impacted_local_state(local_state, "moved"),
        )),
        AnchorChange::Missing => Some(("deleted", None, "blocking")),
        AnchorChange::Ambiguous => Some(("split", None, "blocking")),
    }
}

pub(super) fn severity_for_impacted_local_state(
    local_state: &str,
    source_change_kind: &str,
) -> &'static str {
    if source_change_kind == "deleted" {
        return "blocking";
    }
    match local_state {
        "user_edited" | "generated" | "imported" | "assembled" | "exported" => "blocking",
        "clean" | "draft" => "warning",
        _ => "info",
    }
}

async fn local_state_for_spec<C>(db: &C, spec: &generation_spec::Model) -> Result<String, AppError>
where
    C: ConnectionTrait,
{
    let edited = prompt_artifact::Entity::find()
        .filter(prompt_artifact::Column::GenerationSpecId.eq(spec.id))
        .filter(prompt_artifact::Column::EditedByUser.eq(true))
        .count(db)
        .await
        .map_err(AppError::Database)?;
    if edited > 0 {
        return Ok("user_edited".to_string());
    }
    let media = media_provenance::Entity::find()
        .filter(media_provenance::Column::GenerationSpecId.eq(spec.id))
        .all(db)
        .await
        .map_err(AppError::Database)?;
    for provenance in media {
        let Some(asset) = media_asset::Entity::find_by_id(provenance.media_asset_id)
            .one(db)
            .await
            .map_err(AppError::Database)?
        else {
            continue;
        };
        let clip_count = assembly_clip::Entity::find()
            .filter(assembly_clip::Column::MediaAssetId.eq(asset.id))
            .count(db)
            .await
            .map_err(AppError::Database)?;
        if clip_count > 0 {
            return Ok("assembled".to_string());
        }
        if asset.origin == "imported" {
            return Ok("imported".to_string());
        }
        if asset.status == "completed" {
            return Ok("generated".to_string());
        }
    }
    if spec.status == "draft" {
        Ok("draft".to_string())
    } else {
        Ok("clean".to_string())
    }
}

async fn insert_conflict<C>(
    db: &C,
    run: &source_sync_run::Model,
    input: InsertConflictInput<'_>,
) -> Result<source_sync_conflict::Model, AppError>
where
    C: ConnectionTrait,
{
    let InsertConflictInput {
        conflict_scope,
        target_kind,
        target_id,
        source_change_kind,
        local_state_kind,
        severity,
        source_summary_json,
        local_summary_json,
        affected_refs_json,
    } = input;
    source_sync_conflict::ActiveModel {
        sync_run_id: Set(run.id),
        derived_project_id: Set(run.derived_project_id),
        root_project_id: Set(run.root_project_id),
        from_snapshot_id: Set(run.from_snapshot_id),
        to_snapshot_id: Set(run.to_snapshot_id),
        conflict_scope: Set(conflict_scope.to_string()),
        target_kind: Set(target_kind.to_string()),
        target_id: Set(target_id),
        source_change_kind: Set(source_change_kind.to_string()),
        local_state_kind: Set(local_state_kind.to_string()),
        severity: Set(severity.to_string()),
        status: Set(CONFLICT_STATUS_PENDING.to_string()),
        resolution_strategy: Set(None),
        source_summary_json: Set(source_summary_json),
        local_summary_json: Set(local_summary_json),
        affected_refs_json: Set(affected_refs_json),
        decision_json: Set(None),
        created_at: Set(Utc::now().naive_utc()),
        resolved_at: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AppError::Database)
}
