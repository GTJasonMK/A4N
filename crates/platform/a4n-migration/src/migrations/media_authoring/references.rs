use std::collections::{BTreeMap, BTreeSet};

use a4n_db_models::models::chapter::chapter_shooting_script;
use a4n_domain::media_authoring::legacy::film::{ScriptReferenceChange, rewrite_film_references};
use sea_orm::{ConnectionTrait, DbBackend, DbErr, EntityTrait, Statement};
use serde_json::Value;

use super::{hash_value, ownership::FilmOwnership, record_receipt};

pub(super) async fn remap_film_consumers<C: ConnectionTrait>(
    db: &C,
    films: &FilmOwnership,
) -> Result<(), DbErr> {
    let scripts = chapter_shooting_script::Entity::find()
        .all(db)
        .await?
        .into_iter()
        .map(|row| (row.id, row))
        .collect::<BTreeMap<_, _>>();
    let owners = films
        .script_copies
        .keys()
        .map(|(_, project)| *project)
        .collect::<BTreeSet<_>>();
    for project_id in owners {
        let mut mapping = BTreeMap::new();
        for ((old_id, _), id) in films
            .script_copies
            .iter()
            .filter(|((_, project), _)| *project == project_id)
        {
            let script = scripts.get(id).ok_or_else(|| failure("迁移后的剧本丢失"))?;
            let original = films
                .scripts
                .get(old_id)
                .ok_or_else(|| failure("迁移前的剧本丢失"))?;
            mapping.insert(
                *old_id,
                ScriptReferenceChange {
                    id: *id,
                    revision: script.revision,
                    old_sha256: string(original, "script_sha256")?,
                    sha256: script.script_sha256.clone(),
                    old_source_revision_key: string(original, "source_revision_key")?,
                    source_revision_key: script.source_revision_key.clone(),
                },
            );
        }
        let plans = films
            .plan_copies
            .iter()
            .filter(|((_, project), _)| *project == project_id)
            .map(|((old, _), new)| (*old, *new))
            .collect();
        for (table, column) in [
            ("chapter_video_prompt", "metadata_json"),
            ("generated_image", "metadata_json"),
            ("generated_video", "metadata_json"),
            ("generation_spec", "params_json"),
        ] {
            let rows = db.query_all(Statement::from_sql_and_values(DbBackend::Postgres,
                format!("SELECT id, to_jsonb(record) AS before, {column} AS payload FROM {table} record WHERE project_id=$1 AND {column} IS NOT NULL"), [project_id.into()])).await?;
            for row in rows {
                let id: i64 = row.try_get("", "id")?;
                let before: Value = row.try_get("", "before")?;
                let mut payload: Value = row.try_get("", "payload")?;
                let original = payload.clone();
                rewrite_film_references(&mut payload, &mapping, &plans).map_err(failure)?;
                if payload == original {
                    continue;
                }
                db.execute(Statement::from_sql_and_values(
                    DbBackend::Postgres,
                    format!("UPDATE {table} SET {column}=$2 WHERE id=$1"),
                    [id.into(), payload.clone().into()],
                ))
                .await?;
                record_receipt(
                    db,
                    &format!("film_{table}"),
                    id,
                    id,
                    project_id,
                    &before,
                    &hash_value(&payload)?,
                )
                .await?;
            }
        }
    }
    Ok(())
}

fn string(value: &Value, key: &str) -> Result<String, DbErr> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| failure(format!("历史剧本缺少 {key}")))
}
fn failure(error: impl Into<String>) -> DbErr {
    DbErr::Custom(error.into())
}
