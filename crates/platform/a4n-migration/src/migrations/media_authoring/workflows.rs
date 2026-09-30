use a4n_db_models::models::{
    chapter::{chapter_manga_prompt, chapter_visual_plan},
    generation::{generation_run, generation_run_target, workflow_checkpoint, workflow_run},
    project::novel_project,
};
use a4n_domain::{
    generation::canonical_json_sha256,
    media_authoring::legacy::workflow::{
        LEGACY_MEDIA_MODE, relocate_film_fingerprint, uses_legacy_media_pipeline,
    },
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbBackend, DbErr, EntityTrait, QueryFilter,
    Set, Statement,
};
use serde_json::Value;
use std::collections::BTreeMap;

use super::{hash_value, ownership::FilmOwnership, record_receipt};

pub(super) async fn migrate<C: ConnectionTrait>(
    db: &C,
    films: &FilmOwnership,
    now: sea_orm::prelude::DateTime,
) -> Result<(), DbErr> {
    let projects = novel_project::Entity::find()
        .all(db)
        .await?
        .into_iter()
        .map(|row| (row.id, row))
        .collect::<BTreeMap<_, _>>();
    let plans = chapter_visual_plan::Entity::find()
        .all(db)
        .await?
        .into_iter()
        .map(|row| (row.id, row))
        .collect::<BTreeMap<_, _>>();
    let mut owners = BTreeMap::new();
    let mut unfinished_comics = BTreeMap::new();
    for run in workflow_run::Entity::find()
        .filter(workflow_run::Column::WorkflowKind.is_in([
            "chapter_shooting_script",
            "manga_pipeline",
            "galgame_pipeline",
            "manga_prompt_optimization",
        ]))
        .all(db)
        .await?
    {
        if run.mode.as_deref() == Some(LEGACY_MEDIA_MODE) {
            continue;
        }
        let Some(old_project_id) = run.project_id else {
            continue;
        };
        // Deleted projects leave historical workflow rows behind. They have no
        // valid owner to migrate to, so preserve the audit row unchanged.
        let Some(project) = projects.get(&old_project_id) else {
            continue;
        };
        let is_film = run.workflow_kind == "chapter_shooting_script";
        let legacy_pipeline = uses_legacy_media_pipeline(
            &run.workflow_kind,
            run.mode.as_deref(),
            run.input_fingerprint_json.as_ref(),
        );
        let old_plan_id = run
            .input_fingerprint_json
            .as_ref()
            .and_then(|value| value.get("visual_plan_id"))
            .and_then(Value::as_i64);
        let legacy_film = is_film
            && (project.project_kind == "text"
                || old_plan_id.is_some_and(|id| {
                    films
                        .plans
                        .get(&id)
                        .is_some_and(|row| row.get("project_id").is_none_or(Value::is_null))
                }));
        let root_manga = project.project_kind == "text"
            && matches!(
                run.workflow_kind.as_str(),
                "manga_pipeline" | "manga_prompt_optimization"
            );
        if !legacy_film && !legacy_pipeline && !root_manga {
            continue;
        }
        let mut owner = old_project_id;
        let mut snapshot_id = run.source_snapshot_id;
        let mut plan_ids = BTreeMap::new();
        if legacy_film && let Some(old) = old_plan_id {
            let destination = films
                .plan_copies
                .get(&(old, old_project_id))
                .map(|id| (old_project_id, *id))
                .or_else(|| {
                    films
                        .plan_copies
                        .iter()
                        .find(|((id, _), _)| *id == old)
                        .map(|((_, project), id)| (*project, *id))
                });
            if let Some((project, id)) = destination {
                owner = project;
                snapshot_id = plans
                    .get(&id)
                    .ok_or_else(|| failure("历史工作流目标导演方案丢失"))?
                    .source_snapshot_id;
                plan_ids = films
                    .plan_copies
                    .iter()
                    .filter(|((_, project), _)| *project == owner)
                    .map(|((old, _), new)| (*old, *new))
                    .collect();
            }
        } else if root_manga {
            let source_hash = run
                .input_fingerprint_json
                .as_ref()
                .and_then(|value| value.pointer("/source/content_sha256"))
                .and_then(Value::as_str);
            let row = db.query_one(Statement::from_sql_and_values(DbBackend::Postgres,
                "SELECT page.project_id, page.layout FROM chapter_manga_prompt page JOIN media_authoring_migration_receipt receipt ON receipt.kind='manga_layout' AND receipt.new_id=page.id AND receipt.project_id=page.project_id WHERE receipt.before_json->>'project_id'=$1 AND ((page.chapter_id=$2 AND page.layout #>> '{source,content_sha256}'=$5) OR (receipt.before_json->>'id'=$3 AND $4='chapter_manga_prompt')) ORDER BY page.id LIMIT 1",
                [old_project_id.to_string().into(), run.chapter_id.into(), run.scope_id.map(|id| id.to_string()).into(), run.scope_table.clone().into(), source_hash.map(str::to_owned).into()])).await?;
            if let Some(row) = row {
                owner = row.try_get("", "project_id")?;
                let layout: Value = row.try_get("", "layout")?;
                snapshot_id = layout["source"]["source_snapshot_id"].as_i64();
            } else if run.workflow_kind == "manga_pipeline"
                && let (Some(chapter_id), Some(hash)) = (run.chapter_id, source_hash)
            {
                let key = (old_project_id, chapter_id, hash.to_owned());
                let (project_id, snapshot) = if let Some(saved) = unfinished_comics.get(&key) {
                    *saved
                } else {
                    let (snapshot, source) = super::source_capture::freeze_live_source(
                        db, project, chapter_id, hash, now,
                    )
                    .await?;
                    let project_id =
                        super::artifacts::create_historical_comic(db, project, &source, now)
                            .await?;
                    unfinished_comics.insert(key, (project_id, snapshot.id));
                    (project_id, snapshot.id)
                };
                owner = project_id;
                snapshot_id = Some(snapshot);
            }
        }
        let before = serde_json::to_value(&run).map_err(json_error)?;
        let mut changed = run.clone();
        changed.project_id = Some(owner);
        changed.root_project_id = Some(project.root_project_id.unwrap_or(project.id));
        changed.source_snapshot_id = snapshot_id;
        changed.active_task_id = None;
        if owner != old_project_id {
            let prefix = format!("{}:{old_project_id}:", run.workflow_kind);
            if let Some(rest) = run.business_key.strip_prefix(&prefix) {
                changed.business_key = format!("{}:{owner}:{rest}", run.workflow_kind);
            }
        }
        if legacy_film && !plan_ids.is_empty() {
            if let Some(input) = &mut changed.input_fingerprint_json {
                relocate_film_fingerprint(input, old_project_id, owner, &plan_ids)
                    .map_err(failure)?;
            }
            relocate_checkpoints(db, run.id, old_project_id, owner, &plan_ids).await?;
        }
        if legacy_pipeline || root_manga || (project.project_kind == "text" && owner == project.id)
        {
            changed.mode = Some(LEGACY_MEDIA_MODE.into());
            changed.can_resume = false;
            changed.resume_block_reason =
                Some("旧媒体依赖已转换；原始调用保留，请从派生项目已保存的计划阶段继续。".into());
        }
        if run.status == "running" {
            changed.status = "interrupted".into();
            changed.error_message = Some("创作数据迁移已完成，等待在派生工作区显式继续。".into());
            changed.completed_at = Some(now);
        }
        changed.updated_at = now;
        let after = serde_json::to_value(&changed).map_err(json_error)?;
        let active: workflow_run::ActiveModel = changed.into();
        active.reset_all().update(db).await?;
        record_receipt(
            db,
            "media_workflow",
            run.id,
            run.id,
            owner,
            &before,
            &hash_value(&after)?,
        )
        .await?;
        owners.insert(run.id, owner);
    }
    migrate_generation_history(db, films, &owners).await?;
    super::tasks::migrate(db, &owners, films, now).await?;
    Ok(())
}

async fn relocate_checkpoints<C: ConnectionTrait>(
    db: &C,
    run_id: i64,
    old_project: i64,
    project: i64,
    plans: &BTreeMap<i64, i64>,
) -> Result<(), DbErr> {
    for checkpoint in workflow_checkpoint::Entity::find()
        .filter(workflow_checkpoint::Column::WorkflowRunId.eq(run_id))
        .all(db)
        .await?
    {
        let Some(mut input) = checkpoint.input_fingerprint_json.clone() else {
            continue;
        };
        if let Some(expected) = &checkpoint.input_fingerprint_sha256
            && canonical_json_sha256(&input).map_err(json_error)? != *expected
        {
            return Err(failure("历史工作流断点输入哈希损坏"));
        }
        relocate_film_fingerprint(&mut input, old_project, project, plans).map_err(failure)?;
        if Some(&input) == checkpoint.input_fingerprint_json.as_ref() {
            continue;
        }
        let hash = checkpoint
            .input_fingerprint_sha256
            .as_ref()
            .map(|_| canonical_json_sha256(&input))
            .transpose()
            .map_err(json_error)?;
        workflow_checkpoint::ActiveModel {
            id: Set(checkpoint.id),
            input_fingerprint_json: Set(Some(input.clone())),
            input_fingerprint_sha256: Set(hash),
            ..Default::default()
        }
        .update(db)
        .await?;
        record_receipt(
            db,
            "media_workflow_checkpoint",
            checkpoint.id,
            checkpoint.id,
            project,
            &serde_json::to_value(&checkpoint).map_err(json_error)?,
            &hash_value(&input)?,
        )
        .await?;
    }
    Ok(())
}

async fn migrate_generation_history<C: ConnectionTrait>(
    db: &C,
    films: &FilmOwnership,
    workflows: &BTreeMap<i64, i64>,
) -> Result<(), DbErr> {
    let plans = chapter_visual_plan::Entity::find()
        .all(db)
        .await?
        .into_iter()
        .map(|row| (row.id, row.project_id))
        .collect::<BTreeMap<_, _>>();
    let pages = chapter_manga_prompt::Entity::find()
        .all(db)
        .await?
        .into_iter()
        .map(|row| (row.id, row.project_id))
        .collect::<BTreeMap<_, _>>();
    let targets = generation_run_target::Entity::find()
        .filter(generation_run_target::Column::TargetTable.is_in([
            "chapter_visual_plan",
            "chapter_shooting_script",
            "chapter_manga_prompt",
        ]))
        .all(db)
        .await?;
    let mut owners = BTreeMap::new();
    for run in generation_run::Entity::find().all(db).await? {
        let owner = run
            .workflow_run_id
            .and_then(|id| workflows.get(&id).copied())
            .or_else(|| {
                targets
                    .iter()
                    .filter(|target| target.generation_run_id == run.id)
                    .find_map(|target| match target.target_table.as_str() {
                        "chapter_visual_plan" => plans.get(&target.target_id).copied(),
                        "chapter_manga_prompt" => pages.get(&target.target_id).copied(),
                        "chapter_shooting_script" => films
                            .script_copies
                            .iter()
                            .find(|((id, _), _)| *id == target.target_id)
                            .map(|((_, project), _)| *project),
                        _ => None,
                    })
            });
        let Some(owner) = owner else {
            continue;
        };
        owners.insert(run.id, owner);
        for target in targets
            .iter()
            .filter(|target| target.generation_run_id == run.id)
        {
            let mapped = match target.target_table.as_str() {
                "chapter_visual_plan" => films.plan_copies.get(&(target.target_id, owner)),
                "chapter_shooting_script" => films.script_copies.get(&(target.target_id, owner)),
                _ => None,
            };
            if let Some(mapped) = mapped.filter(|id| **id != target.target_id) {
                generation_run_target::ActiveModel {
                    id: Set(target.id),
                    target_id: Set(*mapped),
                    ..Default::default()
                }
                .update(db)
                .await?;
                record_receipt(
                    db,
                    "generation_target",
                    target.id,
                    target.id,
                    owner,
                    &serde_json::to_value(target).map_err(json_error)?,
                    &hash_value(&serde_json::json!({"target_id":mapped}))?,
                )
                .await?;
            }
        }
        if owner == run.project_id {
            continue;
        }
        generation_run::ActiveModel {
            id: Set(run.id),
            project_id: Set(owner),
            ..Default::default()
        }
        .update(db)
        .await?;
        record_receipt(
            db,
            "generation_run_owner",
            run.id,
            run.id,
            owner,
            &serde_json::to_value(&run).map_err(json_error)?,
            &hash_value(&serde_json::json!({"project_id":owner}))?,
        )
        .await?;
    }
    for table in ["llm_call_trace", "model_usage_record"] {
        let rows = db.query_all(Statement::from_string(DbBackend::Postgres, format!("SELECT id, project_id, generation_run_id, workflow_run_id, to_jsonb(record) AS payload FROM {table} record"))).await?;
        for row in rows {
            let id: i64 = row.try_get("", "id")?;
            let old: Option<i64> = row.try_get("", "project_id")?;
            let generation: Option<i64> = row.try_get("", "generation_run_id")?;
            let workflow: Option<i64> = row.try_get("", "workflow_run_id")?;
            let owner = generation
                .and_then(|id| owners.get(&id))
                .or_else(|| workflow.and_then(|id| workflows.get(&id)))
                .copied();
            if let Some(owner) = owner.filter(|owner| Some(*owner) != old) {
                db.execute(Statement::from_sql_and_values(
                    DbBackend::Postgres,
                    format!("UPDATE {table} SET project_id=$2 WHERE id=$1"),
                    [id.into(), owner.into()],
                ))
                .await?;
                record_receipt(
                    db,
                    &format!("{table}_owner"),
                    id,
                    id,
                    owner,
                    &row.try_get::<Value>("", "payload")?,
                    &hash_value(&serde_json::json!({"project_id":owner}))?,
                )
                .await?;
            }
        }
    }
    Ok(())
}

fn failure(error: impl Into<String>) -> DbErr {
    DbErr::Custom(error.into())
}
fn json_error(error: serde_json::Error) -> DbErr {
    failure(error.to_string())
}
