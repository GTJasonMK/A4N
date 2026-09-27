//! Remove estimated beat timing without deleting scripts, media, or resumable work.

use a4n_domain::shooting_script::{
    ChapterShootingScriptUnitProgressCheckpoint, compat::HashUpgrades,
    project_visual_plan_for_shooting_script,
};
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;
use serde_json::{Value, json};

mod checkpoints;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom("拍摄剧本 v2 升级需要 PostgreSQL".into()));
        }
        // Migrator wraps PostgreSQL migrations in one transaction.
        migrate(manager.get_connection()).await
    }

    async fn down(&self, _: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "拍摄剧本估时不能从新合同反推；回退必须恢复迁移前备份".into(),
        ))
    }
}

pub(crate) async fn migrate<C: ConnectionTrait>(db: &C) -> Result<(), DbErr> {
    let mut hashes = HashUpgrades::default();
    for row in db.query_all(Statement::from_string(DbBackend::Postgres,
        "SELECT id, plan_json FROM chapter_visual_plan WHERE status IN ('confirmed', 'superseded') AND contract_version = 'chapter_visual_plan_v3' ORDER BY id".to_string())).await?
    {
        let id: i64 = row.try_get("", "id")?;
        let value: Value = row.try_get("", "plan_json")?;
        let plan = serde_json::from_value(value).map_err(|e| error("chapter_visual_plan", id, e))?;
        let projected = project_visual_plan_for_shooting_script(&plan, &[]).map_err(|e| error("chapter_visual_plan", id, e))?;
        hashes.register_plan(id, projected);
    }
    for row in db.query_all(Statement::from_string(DbBackend::Postgres,
        "SELECT id, contract_version, script_json, script_sha256 FROM chapter_shooting_script ORDER BY id".to_string())).await?
    {
        let id: i64 = row.try_get("", "id")?;
        let version: String = row.try_get("", "contract_version")?;
        let mut script: Value = row.try_get("", "script_json")?;
        let old_hash: String = row.try_get("", "script_sha256")?;
        if script["schema_version"].as_str() != Some(version.as_str()) {
            return Err(error("chapter_shooting_script", id, "记录版本与 JSON 合同不一致"));
        }
        let hash = hashes.upgrade_record(&mut script, &old_hash).map_err(|e| error("chapter_shooting_script", id, e))?;
        if hash != old_hash {
            let result = db.execute(Statement::from_sql_and_values(DbBackend::Postgres,
                "UPDATE chapter_shooting_script SET script_json = $1, script_sha256 = $2, contract_version = 'chapter_shooting_script_v2' WHERE id = $3 AND script_sha256 = $4",
                [script.into(), hash.into(), id.into(), old_hash.into()])).await?;
            require_one(result.rows_affected(), "chapter_shooting_script", id)?;
        }
    }

    // Terminal cancelled/failed history is archival evidence, never a resume candidate.
    for row in db.query_all(Statement::from_string(DbBackend::Postgres,
        "SELECT id, progress_json FROM workflow_run WHERE workflow_kind = 'chapter_shooting_script' AND progress_json IS NOT NULL AND (can_resume OR status = 'completed') ORDER BY id".to_string())).await?
    {
        let id: i64 = row.try_get("", "id")?;
        let original: Value = row.try_get("", "progress_json")?;
        let mut value = original.clone();
        if hashes.upgrade_embedded(&mut value).map_err(|e| error("workflow_run", id, e))? {
            if let Some(snapshot) = value.get("snapshot") {
                let checkpoint: ChapterShootingScriptUnitProgressCheckpoint = serde_json::from_value(snapshot.clone()).map_err(|e| error("workflow_run", id, e))?;
                checkpoint.validate_for_run(id).map_err(|e| error("workflow_run", id, e))?;
            }
            write_json(db, "workflow_run", "progress_json", id, &original, &value).await?;
        }
    }

    for row in db.query_all(Statement::from_string(DbBackend::Postgres,
        "SELECT id, root_project_id, source_version_hash, blueprint_version_id, chapter_scope_json, chapter_hashes_json, profile_hashes_json FROM project_source_snapshot ORDER BY id".to_string())).await?
    {
        let id: i64 = row.try_get("", "id")?;
        let old_hash: String = row.try_get("", "source_version_hash")?;
        let mut chapters: Value = row.try_get("", "chapter_hashes_json")?;
        let mut payload = json!({
            "root_project_id": row.try_get::<i64>("", "root_project_id")?,
            "blueprint_version_id": row.try_get::<Option<i64>>("", "blueprint_version_id")?,
            "chapter_scope": row.try_get::<Value>("", "chapter_scope_json")?,
            "chapter_hashes": chapters,
            "profile_hashes": row.try_get::<Value>("", "profile_hashes_json")?,
        });
        if a4n_domain::shooting_script::compat::serialized_sha256(&payload).map_err(|e| error("project_source_snapshot", id, e))? != old_hash {
            return Err(error("project_source_snapshot", id, "原始快照内容哈希不一致"));
        }
        hashes.rewrite_references(&mut chapters);
        payload["chapter_hashes"] = chapters.clone();
        let hash = a4n_domain::shooting_script::compat::serialized_sha256(&payload).map_err(|e| error("project_source_snapshot", id, e))?;
        hashes.insert(old_hash.clone(), hash.clone()).map_err(|e| error("project_source_snapshot", id, e))?;
        if hash != old_hash {
            let result = db.execute(Statement::from_sql_and_values(DbBackend::Postgres,
                "UPDATE project_source_snapshot SET chapter_hashes_json = $1, source_version_hash = $2 WHERE id = $3 AND source_version_hash = $4",
                [chapters.into(), hash.into(), id.into(), old_hash.into()])).await?;
            require_one(result.rows_affected(), "project_source_snapshot", id)?;
        }
    }

    // These are mutable projections/references, not original provider requests or responses.
    for (table, column) in [
        ("chapter_video_prompt", "metadata_json"),
        ("generated_video", "metadata_json"),
        ("generated_image", "metadata_json"),
        ("chapter_manga_prompt", "layout"),
        ("generation_spec", "params_json"),
        ("generation_run", "parsed_output_json"),
        ("generation_run", "artifact_refs_json"),
        ("generation_run", "source_refs_json"),
        ("source_sync_run", "summary_json"),
        ("source_sync_conflict", "source_summary_json"),
        ("source_sync_conflict", "local_summary_json"),
        ("source_sync_conflict", "affected_refs_json"),
        ("source_sync_conflict", "decision_json"),
        ("source_sync_decision", "decision_payload_json"),
    ] {
        for row in db.query_all(Statement::from_string(DbBackend::Postgres,
            format!("SELECT id, {column} AS value FROM {table} WHERE {column} IS NOT NULL ORDER BY id"))).await?
        {
            let id: i64 = row.try_get("", "id")?;
            let original: Value = row.try_get("", "value")?;
            let mut value = original.clone();
            hashes.upgrade_embedded(&mut value).map_err(|e| error(table, id, e))?;
            hashes.rewrite_references(&mut value);
            write_json(db, table, column, id, &original, &value).await?;
        }
    }
    for row in db
        .query_all(Statement::from_string(
            DbBackend::Postgres,
            "SELECT id, source_version_hash FROM generation_spec ORDER BY id".to_string(),
        ))
        .await?
    {
        let id: i64 = row.try_get("", "id")?;
        let old: String = row.try_get("", "source_version_hash")?;
        if let Some(new) = hashes.mapped(&old) {
            let result = db.execute(Statement::from_sql_and_values(DbBackend::Postgres,
                "UPDATE generation_spec SET source_version_hash = $1 WHERE id = $2 AND source_version_hash = $3",
                [new.into(), id.into(), old.into()])).await?;
            require_one(result.rows_affected(), "generation_spec", id)?;
        }
    }
    checkpoints::migrate(db, &mut hashes).await
}

pub(super) async fn write_json<C: ConnectionTrait>(
    db: &C,
    table: &str,
    column: &str,
    id: i64,
    old: &Value,
    new: &Value,
) -> Result<(), DbErr> {
    if old == new {
        return Ok(());
    }
    // Identifiers only come from the fixed migration table list; values are bound parameters.
    let result = db
        .execute(Statement::from_sql_and_values(
            DbBackend::Postgres,
            format!(
                "UPDATE {table} SET {column} = $1 WHERE id = $2 AND {column}::jsonb = $3::jsonb"
            ),
            [new.clone().into(), id.into(), old.clone().into()],
        ))
        .await?;
    require_one(result.rows_affected(), table, id)
}

pub(super) fn require_one(count: u64, table: &str, id: i64) -> Result<(), DbErr> {
    if count != 1 {
        return Err(error(table, id, "记录在迁移期间被修改"));
    }
    Ok(())
}

pub(super) fn error(table: &str, id: i64, detail: impl std::fmt::Display) -> DbErr {
    DbErr::Custom(format!("拍摄剧本 v2 升级失败，{table} #{id}: {detail}"))
}

#[cfg(test)]
mod tests;
