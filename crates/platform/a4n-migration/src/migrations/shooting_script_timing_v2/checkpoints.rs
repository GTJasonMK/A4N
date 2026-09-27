use a4n_domain::{
    generation::canonical_json_sha256,
    shooting_script::compat::{HashUpgrades, checkpoint::upgrade_completed_checkpoint},
};
use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};
use serde_json::Value;

use super::{error, require_one, write_json};

pub(super) async fn migrate<C: ConnectionTrait>(
    db: &C,
    hashes: &mut HashUpgrades,
) -> Result<(), DbErr> {
    for row in db.query_all(Statement::from_string(DbBackend::Postgres,
        "SELECT c.id, c.step_key, c.input_fingerprint_json, c.input_fingerprint_sha256, c.checkpoint_json, c.artifacts_json FROM workflow_checkpoint c JOIN workflow_run r ON r.id = c.workflow_run_id WHERE c.status = 'completed' AND c.checkpoint_json IS NOT NULL AND (r.can_resume OR r.status = 'completed') ORDER BY c.id".to_string())).await?
    {
        let id: i64 = row.try_get("", "id")?;
        let step: String = row.try_get("", "step_key")?;
        let original: Value = row.try_get("", "checkpoint_json")?;
        let old_artifacts: Option<Value> = row.try_get("", "artifacts_json")?;
        let mut value = original.clone();
        let mut artifacts = old_artifacts.clone();
        if value.get("parsed_output").is_some() && value.get("response_content").is_some() {
            let input: Option<Value> = row.try_get("", "input_fingerprint_json")?;
            let input_hash: Option<String> = row.try_get("", "input_fingerprint_sha256")?;
            upgrade_completed_checkpoint(&step, input.as_ref(), input_hash.as_deref(), &mut value, &mut artifacts, hashes).map_err(|e| error("workflow_checkpoint", id, e))?;
        } else {
            hashes.upgrade_embedded(&mut value).map_err(|e| error("workflow_checkpoint", id, e))?;
            hashes.rewrite_references(&mut value);
        }
        write_json(db, "workflow_checkpoint", "checkpoint_json", id, &original, &value).await?;
        if artifacts != old_artifacts {
            let result = db.execute(Statement::from_sql_and_values(DbBackend::Postgres,
                "UPDATE workflow_checkpoint SET artifacts_json = $1 WHERE id = $2 AND artifacts_json::jsonb IS NOT DISTINCT FROM $3::jsonb",
                [artifacts.into(), id.into(), old_artifacts.into()])).await?;
            require_one(result.rows_affected(), "workflow_checkpoint", id)?;
        }
    }
    for row in db.query_all(Statement::from_string(DbBackend::Postgres,
        "SELECT task_id, public_snapshot_json, public_snapshot_sha256, execution_checkpoint_json, execution_checkpoint_sha256, result_kind, result_revision FROM generation_task_checkpoint WHERE status <> 'cancelled' ORDER BY task_id".to_string())).await?
    {
        let task_id: String = row.try_get("", "task_id")?;
        for (column, hash_column) in [("public_snapshot_json", "public_snapshot_sha256"), ("execution_checkpoint_json", "execution_checkpoint_sha256")] {
            let Some(original): Option<Value> = row.try_get("", column)? else { continue; };
            let old_hash: Option<String> = row.try_get("", hash_column)?;
            let actual = canonical_json_sha256(&original).map_err(|e| DbErr::Custom(e.to_string()))?;
            if old_hash.as_deref() != Some(actual.as_str()) {
                return Err(DbErr::Custom(format!("任务 {task_id} 的 {column} 哈希不一致")));
            }
            let mut value = original.clone();
            hashes.upgrade_embedded(&mut value).map_err(DbErr::Custom)?;
            hashes.rewrite_references(&mut value);
            if value != original {
                let new_hash = canonical_json_sha256(&value).map_err(|e| DbErr::Custom(e.to_string()))?;
                let result = db.execute(Statement::from_sql_and_values(DbBackend::Postgres,
                    format!("UPDATE generation_task_checkpoint SET {column} = $1, {hash_column} = $2 WHERE task_id = $3 AND {hash_column} = $4"),
                    [value.into(), new_hash.into(), task_id.clone().into(), actual.into()])).await?;
                if result.rows_affected() != 1 { return Err(DbErr::Custom(format!("任务 {task_id} 在迁移期间被修改"))); }
            }
        }
        let result_kind: Option<String> = row.try_get("", "result_kind")?;
        let result_revision: Option<String> = row.try_get("", "result_revision")?;
        if result_kind.as_deref() == Some("chapter_shooting_script_progress")
            && let Some(old) = result_revision
            && let Some(new) = hashes.mapped(&old)
        {
            db.execute(Statement::from_sql_and_values(DbBackend::Postgres,
                "UPDATE generation_task_checkpoint SET result_revision = $1 WHERE task_id = $2 AND result_revision = $3",
                [new.into(), task_id.into(), old.into()])).await?;
        }
    }
    Ok(())
}
