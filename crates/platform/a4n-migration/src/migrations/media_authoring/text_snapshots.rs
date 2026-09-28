use a4n_db_models::models::project::{project_source_snapshot, source_snapshot_chapter};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbBackend, DbErr, EntityTrait, QueryFilter,
    QueryOrder, Set, Statement,
};
use serde_json::{Value, json};

use super::{hash_value, record_receipt};

pub(super) async fn remove_film_sources<C: ConnectionTrait>(db: &C) -> Result<(), DbErr> {
    let old_chapters = db.query_all(Statement::from_string(DbBackend::Postgres,
        "SELECT id, root_project_id, to_jsonb(c) AS payload FROM source_snapshot_chapter c WHERE to_jsonb(c)->>'visual_plan_id' IS NOT NULL".to_string())).await?;
    for row in old_chapters {
        let id = row.try_get("", "id")?;
        let project_id = row.try_get("", "root_project_id")?;
        let before: Value = row.try_get("", "payload")?;
        record_receipt(
            db,
            "text_snapshot_chapter",
            id,
            id,
            project_id,
            &before,
            &hash_value(&before)?,
        )
        .await?;
    }
    for snapshot in project_source_snapshot::Entity::find().all(db).await? {
        let chapters = source_snapshot_chapter::Entity::find()
            .filter(source_snapshot_chapter::Column::SnapshotId.eq(snapshot.id))
            .order_by_asc(source_snapshot_chapter::Column::ChapterNumber)
            .all(db)
            .await?;
        let hashes = json!(
            chapters
                .iter()
                .map(|chapter| json!({
                    "source_chapter_id": chapter.source_chapter_id,
                    "chapter_number": chapter.chapter_number,
                    "content_hash": chapter.content_hash,
                    "analysis_hash": chapter.analysis_hash,
                }))
                .collect::<Vec<_>>()
        );
        let hash = hash_value(&json!({
            "root_project_id": snapshot.root_project_id,
            "chapter_scope": snapshot.chapter_scope_json,
            "chapter_hashes": hashes,
            "profile_hashes": snapshot.profile_hashes_json,
            "blueprint_version_id": snapshot.blueprint_version_id,
        }))?;
        if hashes == snapshot.chapter_hashes_json && hash == snapshot.source_version_hash {
            continue;
        }
        let before =
            serde_json::to_value(&snapshot).map_err(|error| DbErr::Custom(error.to_string()))?;
        record_receipt(
            db,
            "text_snapshot",
            snapshot.id,
            snapshot.id,
            snapshot.root_project_id,
            &before,
            &hash,
        )
        .await?;
        // These columns carry the snapshot hash, not a hash of generation
        // parameters. Only advance identities that still name this exact source.
        for sql in [
            "UPDATE generation_spec SET source_version_hash=$3 WHERE source_snapshot_id=$1 AND source_version_hash=$2",
            "UPDATE text_anchor SET source_version_hash=$3 WHERE source_snapshot_id=$1 AND source_version_hash=$2",
        ] {
            db.execute(Statement::from_sql_and_values(
                DbBackend::Postgres,
                sql,
                [
                    snapshot.id.into(),
                    snapshot.source_version_hash.clone().into(),
                    hash.clone().into(),
                ],
            ))
            .await?;
        }
        let mut active: project_source_snapshot::ActiveModel = snapshot.into();
        active.chapter_hashes_json = Set(hashes);
        active.source_version_hash = Set(hash);
        active.update(db).await?;
    }
    // Newly frozen chapters have deferred FK checks. Validate them before DDL;
    // PostgreSQL correctly refuses ALTER TABLE while those checks are pending.
    db.execute_unprepared("SET CONSTRAINTS ALL IMMEDIATE")
        .await?;
    db.execute_unprepared("DROP FUNCTION IF EXISTS a4n_backfill_galgame_scene_sources(BIGINT)")
        .await?;
    db.execute_unprepared("ALTER TABLE source_snapshot_chapter DROP COLUMN IF EXISTS visual_plan_id, DROP COLUMN IF EXISTS visual_plan_version, DROP COLUMN IF EXISTS visual_plan_sha256").await?;
    db.execute_unprepared(
        "ALTER TABLE galgame_scene DROP COLUMN IF EXISTS shooting_scene_identity",
    )
    .await?;
    Ok(())
}
