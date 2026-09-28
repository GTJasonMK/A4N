use a4n_domain::media_authoring::legacy::source_sync::archive_legacy_source_sync_payload;
use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};
use serde_json::Value;

pub(super) async fn archive_history<C: ConnectionTrait>(db: &C) -> Result<(), DbErr> {
    for (table, column, query) in [
        (
            "source_sync_conflict",
            "decision_json",
            "SELECT record.id, record.derived_project_id AS project_id, record.decision_json AS payload, to_jsonb(record) AS original FROM source_sync_conflict record WHERE record.decision_json #> '{result,stage_outputs,shooting_script_projection}' IS NOT NULL",
        ),
        (
            "source_sync_decision",
            "decision_payload_json",
            "SELECT record.id, conflict.derived_project_id AS project_id, record.decision_payload_json AS payload, to_jsonb(record) AS original FROM source_sync_decision record JOIN source_sync_conflict conflict ON conflict.id=record.conflict_id WHERE record.decision_payload_json #> '{result,stage_outputs,shooting_script_projection}' IS NOT NULL",
        ),
    ] {
        for row in db
            .query_all(Statement::from_string(
                DbBackend::Postgres,
                query.to_owned(),
            ))
            .await?
        {
            let id: i64 = row.try_get("", "id")?;
            let project: i64 = row.try_get("", "project_id")?;
            let mut payload: Value = row.try_get("", "payload")?;
            if archive_legacy_source_sync_payload(&mut payload).map_err(DbErr::Custom)? {
                db.execute(Statement::from_sql_and_values(
                    DbBackend::Postgres,
                    format!("UPDATE {table} SET {column}=$2 WHERE id=$1"),
                    [id.into(), payload.clone().into()],
                ))
                .await?;
                super::record_receipt(
                    db,
                    &format!("{table}_archive"),
                    id,
                    id,
                    project,
                    &row.try_get::<Value>("", "original")?,
                    &super::hash_value(&payload)?,
                )
                .await?;
            }
        }
    }
    Ok(())
}
