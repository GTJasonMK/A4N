use std::collections::{BTreeMap, BTreeSet, HashMap};

use a4n_db_models::models::{
    chapter::{chapter_shooting_script, chapter_visual_plan},
    project::{novel_project, project_source_snapshot, source_snapshot_chapter},
    source_sync::source_sync_run,
};
use a4n_domain::media_authoring::legacy::film::referenced_script_ids;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbBackend, DbErr, EntityTrait, NotSet,
    QueryFilter, QueryOrder, Set, Statement,
};
use serde_json::Value;

use super::{hash_value, optional_id, record_receipt, source_capture::freeze_live_source};

#[derive(Default)]
pub(super) struct FilmOwnership {
    pub plans: BTreeMap<i64, Value>,
    pub scripts: BTreeMap<i64, Value>,
    pub plan_copies: BTreeMap<(i64, i64), i64>,
    pub script_copies: BTreeMap<(i64, i64), i64>,
    pub script_sources: BTreeMap<i64, source_snapshot_chapter::Model>,
    pub historical_projects: BTreeMap<(i64, i64), i64>,
}

pub(super) async fn move_film_ownership<C: ConnectionTrait>(
    db: &C,
    now: sea_orm::prelude::DateTime,
) -> Result<FilmOwnership, DbErr> {
    let mut result = FilmOwnership::default();
    result.plans = json_rows(
        db,
        "SELECT to_jsonb(p) AS payload FROM chapter_visual_plan p ORDER BY id",
    )
    .await?;
    result.scripts = json_rows(
        db,
        "SELECT to_jsonb(s) AS payload FROM chapter_shooting_script s ORDER BY id",
    )
    .await?;
    let projects = novel_project::Entity::find()
        .order_by_asc(novel_project::Column::Id)
        .all(db)
        .await?
        .into_iter()
        .map(|project| (project.id, project))
        .collect::<BTreeMap<_, _>>();
    let videos = projects
        .values()
        .filter(|project| project.project_kind == "video" && project.root_project_id.is_some())
        .collect::<Vec<_>>();
    let video_ids = videos.iter().map(|project| project.id).collect::<Vec<_>>();
    let mut snapshot_owners = BTreeMap::<i64, BTreeSet<i64>>::new();
    for project in &videos {
        if let Some(id) = project.source_snapshot_id {
            snapshot_owners.entry(id).or_default().insert(project.id);
        }
    }
    for run in source_sync_run::Entity::find()
        .filter(source_sync_run::Column::DerivedProjectId.is_in(video_ids))
        .all(db)
        .await?
    {
        snapshot_owners
            .entry(run.from_snapshot_id)
            .or_default()
            .insert(run.derived_project_id);
        snapshot_owners
            .entry(run.to_snapshot_id)
            .or_default()
            .insert(run.derived_project_id);
    }
    let mut script_consumers = BTreeMap::<i64, BTreeSet<i64>>::new();
    for (table, column) in [
        ("chapter_video_prompt", "metadata_json"),
        ("generated_image", "metadata_json"),
        ("generated_video", "metadata_json"),
        ("generation_spec", "params_json"),
        ("workflow_run", "progress_json"),
        ("workflow_run", "input_fingerprint_json"),
    ] {
        for row in db.query_all(Statement::from_string(DbBackend::Postgres,
            format!("SELECT record.project_id, record.{column} AS payload FROM {table} record JOIN novel_project p ON p.id=record.project_id WHERE p.project_kind='video' AND p.root_project_id IS NOT NULL"))).await? {
            let project_id: i64 = row.try_get("", "project_id")?;
            if let Some(payload) = row.try_get::<Option<Value>>("", "payload")? {
                for id in referenced_script_ids(&payload) { script_consumers.entry(id).or_default().insert(project_id); }
            }
        }
    }
    let mut plan_consumers = BTreeMap::<i64, BTreeSet<i64>>::new();
    for (id, script) in &result.scripts {
        if let Some(plan_id) = optional_id(script, "visual_plan_id")? {
            let mut owners = record_owners(script, &snapshot_owners);
            owners.extend(script_consumers.get(id).into_iter().flatten().copied());
            plan_consumers.entry(plan_id).or_default().extend(owners);
        }
    }
    let mut source_cache = HashMap::<
        (i64, i64, String),
        (
            project_source_snapshot::Model,
            source_snapshot_chapter::Model,
        ),
    >::new();
    let original_plans = result.plans.values().cloned().collect::<Vec<_>>();
    for original in original_plans {
        let id = integer(&original, "id")?;
        let root = projects
            .get(&integer(&original, "root_project_id")?)
            .ok_or_else(|| failure("历史导演原作不存在"))?;
        let (snapshot, source) =
            source_for_record(db, root, &original, now, &mut source_cache).await?;
        let plan: a4n_domain::chapter_visual_plan::ChapterVisualPlanV3 =
            serde_json::from_value(original["plan_json"].clone()).map_err(json_error)?;
        plan.validate_draft()
            .map_err(|error| failure(&error.to_string()))?;
        let hash = a4n_domain::generation::canonical_json_sha256(&plan)
            .map_err(|error| failure(&error.to_string()))?;
        if original.get("plan_sha256").and_then(Value::as_str) != Some(hash.as_str()) {
            return Err(failure(&format!("历史导演 {id} 内容哈希不一致")));
        }
        let mut owners = record_owners(&original, &snapshot_owners);
        if original.get("project_id").is_none_or(Value::is_null)
            && let Some(consumers) = plan_consumers.get(&id)
        {
            owners.extend(consumers);
        }
        if owners.is_empty() {
            owners.insert(
                historical_video(db, root, &snapshot, now, &mut result.historical_projects).await?,
            );
        }
        for (index, project_id) in owners.into_iter().enumerate() {
            verify_owner(&projects, &result.historical_projects, project_id, root.id)?;
            let model: chapter_visual_plan::Model =
                serde_json::from_value(owned_value(&original, project_id, &source))
                    .map_err(json_error)?;
            let mut active: chapter_visual_plan::ActiveModel = model.into();
            active.project_id = Set(project_id);
            active.source_snapshot_id = Set(Some(snapshot.id));
            active.source_snapshot_chapter_id = Set(Some(source.id));
            active.source_revision_key = Set(revision_key(&source));
            let saved = if index == 0 {
                active.update(db).await?
            } else {
                active.id = NotSet;
                active.insert(db).await?
            };
            if index > 0 || !already_owned(&original, project_id, &source) {
                record_receipt(
                    db,
                    "visual_plan",
                    id,
                    saved.id,
                    project_id,
                    &original,
                    &saved.plan_sha256,
                )
                .await?;
            }
            result.plan_copies.insert((id, project_id), saved.id);
        }
    }
    let original_scripts = result.scripts.values().cloned().collect::<Vec<_>>();
    for original in original_scripts {
        let id = integer(&original, "id")?;
        let root = projects
            .get(&integer(&original, "root_project_id")?)
            .ok_or_else(|| failure("历史拍摄剧本原作不存在"))?;
        let (snapshot, source) =
            source_for_record(db, root, &original, now, &mut source_cache).await?;
        let script: a4n_domain::shooting_script::ChapterShootingScriptV2 =
            serde_json::from_value(original["script_json"].clone()).map_err(json_error)?;
        script
            .validate()
            .map_err(|error| failure(&error.to_string()))?;
        let old_hash =
            a4n_domain::shooting_script::compat::serialized_sha256(&script).map_err(json_error)?;
        if original.get("script_sha256").and_then(Value::as_str) != Some(old_hash.as_str()) {
            return Err(failure(&format!("历史拍摄剧本 {id} 内容哈希不一致")));
        }
        let plan_id = optional_id(&original, "visual_plan_id")?;
        let mut owners = record_owners(&original, &snapshot_owners);
        if original.get("project_id").is_none_or(Value::is_null)
            && let Some(consumers) = script_consumers.get(&id)
        {
            owners.extend(consumers);
        }
        if owners.is_empty() {
            owners.insert(
                historical_video(db, root, &snapshot, now, &mut result.historical_projects).await?,
            );
        }
        for (index, project_id) in owners.into_iter().enumerate() {
            verify_owner(&projects, &result.historical_projects, project_id, root.id)?;
            let mapped_plan = plan_id
                .map(|plan_id| {
                    result
                        .plan_copies
                        .get(&(plan_id, project_id))
                        .copied()
                        .ok_or_else(|| {
                            failure(&format!("拍摄剧本 {id} 无法定位所属视频项目的导演方案"))
                        })
                })
                .transpose()?;
            let mut script = script.clone();
            match (script.visual_plan_source.as_mut(), plan_id, mapped_plan) {
                (Some(identity), Some(old), Some(mapped)) if identity.plan_id == old => {
                    identity.plan_id = mapped
                }
                (None, None, None) => {}
                _ => return Err(failure("历史拍摄剧本的导演身份冲突")),
            }
            let hash = a4n_domain::shooting_script::compat::serialized_sha256(&script)
                .map_err(json_error)?;
            let model: chapter_shooting_script::Model =
                serde_json::from_value(owned_value(&original, project_id, &source))
                    .map_err(json_error)?;
            let mut active: chapter_shooting_script::ActiveModel = model.into();
            active.project_id = Set(project_id);
            active.source_snapshot_id = Set(Some(snapshot.id));
            active.source_snapshot_chapter_id = Set(Some(source.id));
            active.source_revision_key = Set(revision_key(&source));
            active.visual_plan_id = Set(mapped_plan);
            active.script_json = Set(serde_json::to_value(script).map_err(json_error)?);
            active.script_sha256 = Set(hash.clone());
            let saved = if index == 0 {
                active.update(db).await?
            } else {
                active.id = NotSet;
                active.workflow_run_id = Set(None);
                active.insert(db).await?
            };
            if index > 0 || !already_owned(&original, project_id, &source) || old_hash != hash {
                record_receipt(
                    db,
                    "shooting_script",
                    id,
                    saved.id,
                    project_id,
                    &original,
                    &hash,
                )
                .await?;
            }
            result.script_copies.insert((id, project_id), saved.id);
        }
        result.script_sources.insert(id, source);
    }
    Ok(result)
}

fn record_owners(value: &Value, snapshots: &BTreeMap<i64, BTreeSet<i64>>) -> BTreeSet<i64> {
    if let Some(id) = value.get("project_id").and_then(Value::as_i64) {
        return BTreeSet::from([id]);
    }
    value
        .get("source_snapshot_id")
        .and_then(Value::as_i64)
        .and_then(|id| snapshots.get(&id))
        .cloned()
        .unwrap_or_default()
}

fn already_owned(value: &Value, project_id: i64, source: &source_snapshot_chapter::Model) -> bool {
    value.get("project_id").and_then(Value::as_i64) == Some(project_id)
        && value.get("source_snapshot_id").and_then(Value::as_i64) == Some(source.snapshot_id)
        && value
            .get("source_snapshot_chapter_id")
            .and_then(Value::as_i64)
            == Some(source.id)
        && value.get("source_revision_key").and_then(Value::as_str)
            == Some(revision_key(source).as_str())
}

pub(super) async fn source_for_record<C: ConnectionTrait>(
    db: &C,
    root: &novel_project::Model,
    value: &Value,
    now: sea_orm::prelude::DateTime,
    cache: &mut HashMap<
        (i64, i64, String),
        (
            project_source_snapshot::Model,
            source_snapshot_chapter::Model,
        ),
    >,
) -> Result<
    (
        project_source_snapshot::Model,
        source_snapshot_chapter::Model,
    ),
    DbErr,
> {
    let chapter_id = integer(value, "source_chapter_id")?;
    let hash = value
        .get("source_content_sha256")
        .and_then(Value::as_str)
        .ok_or_else(|| failure("历史来源缺少正文哈希"))?;
    let snapshot_id = optional_id(value, "source_snapshot_id")?;
    let source_id = optional_id(value, "source_snapshot_chapter_id")?;
    match (snapshot_id, source_id) {
        (Some(snapshot_id), Some(source_id)) => {
            let snapshot = project_source_snapshot::Entity::find_by_id(snapshot_id)
                .one(db)
                .await?;
            let source = source_snapshot_chapter::Entity::find_by_id(source_id)
                .one(db)
                .await?;
            if let Some(snapshot) = &snapshot
                && snapshot.root_project_id != root.id
            {
                return Err(failure("历史来源快照不属于根项目"));
            }
            if let Some(source) = &source
                && (source.root_project_id != root.id
                    || source.source_chapter_id != chapter_id
                    || source.content_hash != hash
                    || a4n_domain::source_text_hash(&source.content) != hash)
            {
                return Err(failure("历史导演或剧本的正文来源不一致"));
            }
            let (Some(snapshot), Some(source)) = (snapshot, source) else {
                // ponytail: exact body hash can recover a deleted snapshot; missing profile/blueprint history stays explicit.
                return recovered_source(
                    db,
                    root,
                    chapter_id,
                    hash,
                    format!("missing:{snapshot_id}:{source_id}:{hash}"),
                    now,
                    cache,
                )
                .await;
            };
            if source.snapshot_id != snapshot_id {
                return Err(failure("历史导演或剧本的正文来源不一致"));
            }
            Ok((snapshot, source))
        }
        (None, None) => {
            recovered_source(db, root, chapter_id, hash, hash.to_owned(), now, cache).await
        }
        _ => Err(failure("历史导演来源快照身份不完整")),
    }
}

async fn recovered_source<C: ConnectionTrait>(
    db: &C,
    root: &novel_project::Model,
    chapter_id: i64,
    hash: &str,
    cache_key: String,
    now: sea_orm::prelude::DateTime,
    cache: &mut HashMap<
        (i64, i64, String),
        (
            project_source_snapshot::Model,
            source_snapshot_chapter::Model,
        ),
    >,
) -> Result<
    (
        project_source_snapshot::Model,
        source_snapshot_chapter::Model,
    ),
    DbErr,
> {
    let key = (root.id, chapter_id, cache_key);
    if let Some(source) = cache.get(&key) {
        return Ok(source.clone());
    }
    let source = freeze_live_source(db, root, chapter_id, hash, now).await?;
    cache.insert(key, source.clone());
    Ok(source)
}

async fn historical_video<C: ConnectionTrait>(
    db: &C,
    root: &novel_project::Model,
    snapshot: &project_source_snapshot::Model,
    now: sea_orm::prelude::DateTime,
    projects: &mut BTreeMap<(i64, i64), i64>,
) -> Result<i64, DbErr> {
    if let Some(id) = projects.get(&(root.id, snapshot.id)) {
        return Ok(*id);
    }
    let start = integer(&snapshot.chapter_scope_json, "start")?;
    let end = integer(&snapshot.chapter_scope_json, "end")?;
    let count = i32::try_from(
        end.checked_sub(start)
            .and_then(|value| value.checked_add(1))
            .filter(|value| *value > 0)
            .ok_or_else(|| failure("历史来源章节范围无效"))?,
    )
    .map_err(|error| failure(&error.to_string()))?;
    let project = novel_project::ActiveModel {
        user_id: Set(root.user_id),
        title: Set(format!("{} · 历史视频改编（{start}–{end}章）", root.title)),
        description: Set("由原作阶段的导演编排和拍摄剧本迁移；保留原有内容和版本。".into()),
        project_type: Set(root.project_type.clone()),
        project_kind: Set("video".into()),
        root_project_id: Set(Some(root.id)),
        source_snapshot_id: Set(Some(snapshot.id)),
        source_sync_status: Set("root_updated".into()),
        status: Set("draft".into()),
        lifecycle_gate: Set("active".into()),
        target_chapters: Set(count),
        current_chapter: Set(0),
        total_chapters: Set(Some(count)),
        chapters_per_part: Set(root.chapters_per_part),
        avatar_svg: Set(root.avatar_svg.clone()),
        avatar_animal: Set(root.avatar_animal.clone()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?;
    let payload = serde_json::to_value(&project).map_err(json_error)?;
    record_receipt(
        db,
        "historical_video",
        root.id,
        project.id,
        project.id,
        &payload,
        &hash_value(&payload)?,
    )
    .await?;
    projects.insert((root.id, snapshot.id), project.id);
    Ok(project.id)
}

fn verify_owner(
    projects: &BTreeMap<i64, novel_project::Model>,
    historical: &BTreeMap<(i64, i64), i64>,
    project_id: i64,
    root_id: i64,
) -> Result<(), DbErr> {
    if historical
        .iter()
        .any(|((root, _), id)| *root == root_id && *id == project_id)
    {
        return Ok(());
    }
    if projects.get(&project_id).is_none_or(|project| {
        project.project_kind != "video" || project.root_project_id != Some(root_id)
    }) {
        return Err(failure("拍摄内容的所属视频项目不匹配"));
    }
    Ok(())
}

fn owned_value(
    original: &Value,
    project_id: i64,
    source: &source_snapshot_chapter::Model,
) -> Value {
    let mut value = original.clone();
    value["project_id"] = project_id.into();
    value["source_snapshot_id"] = source.snapshot_id.into();
    value["source_snapshot_chapter_id"] = source.id.into();
    value["source_revision_key"] = revision_key(source).into();
    value
}

fn revision_key(source: &source_snapshot_chapter::Model) -> String {
    format!(
        "snapshot:{}:chapter:{}:{}",
        source.snapshot_id, source.id, source.content_hash
    )
}

async fn json_rows<C: ConnectionTrait>(db: &C, sql: &str) -> Result<BTreeMap<i64, Value>, DbErr> {
    db.query_all(Statement::from_string(DbBackend::Postgres, sql.to_owned()))
        .await?
        .into_iter()
        .map(|row| {
            let value: Value = row.try_get("", "payload")?;
            Ok((integer(&value, "id")?, value))
        })
        .collect()
}

fn integer(value: &Value, field: &str) -> Result<i64, DbErr> {
    value
        .get(field)
        .and_then(Value::as_i64)
        .filter(|value| *value > 0)
        .ok_or_else(|| failure(&format!("历史迁移字段 {field} 必须是正整数")))
}
fn failure(message: &str) -> DbErr {
    DbErr::Custom(message.into())
}
fn json_error(error: serde_json::Error) -> DbErr {
    failure(&error.to_string())
}
