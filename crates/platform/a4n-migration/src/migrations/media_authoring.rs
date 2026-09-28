//! Project-owned adaptation artifacts and their explicit legacy conversion.

use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement, TransactionTrait};
use sea_orm_migration::prelude::{DeriveMigrationName, MigrationTrait, SchemaManager};
use serde_json::Value;

mod artifacts;
mod contracts;
mod galgame;
mod ownership;
mod references;
mod source_capture;
mod source_sync;
mod tasks;
mod text_snapshots;
mod workflows;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        migrate_legacy_media(manager.get_connection()).await
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "独立改编已经产生新版本；不能合并回原作共享记录。原始内容保存在迁移回执中。".into(),
        ))
    }
}

pub async fn migrate_legacy_media<C: ConnectionTrait + TransactionTrait>(
    db: &C,
) -> Result<(), DbErr> {
    let tx = db.begin().await?;
    prepare_ownership_schema(&tx).await?;
    tx.execute_unprepared(&contracts::line_contract()?).await?;
    let now = migration_time(&tx).await?;
    let films = ownership::move_film_ownership(&tx, now).await?;
    references::remap_film_consumers(&tx, &films).await?;
    artifacts::migrate_manga(&tx, &films, now).await?;
    galgame::migrate_galgame(&tx, now).await?;
    source_sync::archive_history(&tx).await?;
    workflows::migrate(&tx, &films, now).await?;
    text_snapshots::remove_film_sources(&tx).await?;
    finish_ownership_schema(&tx).await?;
    tx.commit().await
}

async fn migration_time<C: ConnectionTrait>(db: &C) -> Result<sea_orm::prelude::DateTime, DbErr> {
    db.query_one(Statement::from_string(
        DbBackend::Postgres,
        "SELECT LOCALTIMESTAMP AS now".to_string(),
    ))
    .await?
    .ok_or_else(|| DbErr::Custom("migration clock unavailable".into()))?
    .try_get("", "now")
}

pub async fn migrate_video_ownership<C: ConnectionTrait>(db: &C) -> Result<(), DbErr> {
    prepare_ownership_schema(db).await?;
    let now = migration_time(db).await?;
    ownership::move_film_ownership(db, now).await?;
    finish_ownership_schema(db).await
}

async fn prepare_ownership_schema<C: ConnectionTrait>(db: &C) -> Result<(), DbErr> {
    create_document_schema(db).await?;
    db.execute_unprepared(r#"
ALTER TABLE chapter_visual_plan ADD COLUMN IF NOT EXISTS project_id BIGINT;
ALTER TABLE chapter_shooting_script ADD COLUMN IF NOT EXISTS project_id BIGINT;
ALTER TABLE galgame_scene ADD COLUMN IF NOT EXISTS adaptation_plan_id BIGINT;
ALTER TABLE galgame_scene ADD COLUMN IF NOT EXISTS adaptation_unit_id TEXT;
ALTER TABLE galgame_script ADD COLUMN IF NOT EXISTS branch_plan_id BIGINT REFERENCES media_authoring_document(id) DEFERRABLE INITIALLY DEFERRED;
CREATE TABLE IF NOT EXISTS media_authoring_migration_receipt (
    kind TEXT NOT NULL, old_id BIGINT NOT NULL, new_id BIGINT NOT NULL, project_id BIGINT NOT NULL REFERENCES novel_project(id) ON DELETE CASCADE,
    before_json JSONB NOT NULL, after_sha256 TEXT NOT NULL,
    PRIMARY KEY (kind, old_id, new_id, project_id)
);
DROP INDEX IF EXISTS uidx_chapter_visual_plan_draft;
DROP INDEX IF EXISTS uidx_chapter_visual_plan_confirmed;
DROP INDEX IF EXISTS uidx_chapter_visual_plan_live_draft;
DROP INDEX IF EXISTS uidx_chapter_visual_plan_live_confirmed;
DROP INDEX IF EXISTS uidx_chapter_visual_plan_snapshot_draft;
DROP INDEX IF EXISTS uidx_chapter_visual_plan_snapshot_confirmed;
DROP INDEX IF EXISTS uidx_chapter_visual_plan_version;
DROP INDEX IF EXISTS idx_chapter_visual_plan_lineage;
DROP INDEX IF EXISTS uidx_chapter_shooting_script_source_revision;
"#).await?;
    Ok(())
}

async fn finish_ownership_schema<C: ConnectionTrait>(db: &C) -> Result<(), DbErr> {
    db.execute_unprepared(r#"
ALTER TABLE chapter_visual_plan ALTER COLUMN project_id SET NOT NULL;
ALTER TABLE chapter_shooting_script ALTER COLUMN project_id SET NOT NULL;
ALTER TABLE chapter_visual_plan DROP CONSTRAINT IF EXISTS fk_video_visual_plan_project;
ALTER TABLE chapter_visual_plan ADD CONSTRAINT fk_video_visual_plan_project FOREIGN KEY (project_id) REFERENCES novel_project(id) ON DELETE CASCADE;
ALTER TABLE chapter_shooting_script DROP CONSTRAINT IF EXISTS fk_video_shooting_script_project;
ALTER TABLE chapter_shooting_script ADD CONSTRAINT fk_video_shooting_script_project FOREIGN KEY (project_id) REFERENCES novel_project(id) ON DELETE CASCADE;
CREATE UNIQUE INDEX IF NOT EXISTS uidx_video_visual_plan_draft
    ON chapter_visual_plan (project_id, source_chapter_id, COALESCE(source_snapshot_id, 0), COALESCE(source_snapshot_chapter_id, 0)) WHERE status = 'draft';
CREATE UNIQUE INDEX IF NOT EXISTS uidx_video_visual_plan_confirmed
    ON chapter_visual_plan (project_id, source_chapter_id, COALESCE(source_snapshot_id, 0), COALESCE(source_snapshot_chapter_id, 0)) WHERE status = 'confirmed';
CREATE UNIQUE INDEX IF NOT EXISTS uidx_video_visual_plan_version
    ON chapter_visual_plan (project_id, source_chapter_id, COALESCE(source_snapshot_id, 0), plan_version);
CREATE UNIQUE INDEX IF NOT EXISTS uidx_video_shooting_script_source
    ON chapter_shooting_script (project_id, source_chapter_id, source_revision_key);

CREATE OR REPLACE FUNCTION a4n_check_video_authoring_owner() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM novel_project p WHERE p.id = NEW.project_id AND p.project_kind = 'video' AND p.root_project_id = NEW.root_project_id) THEN
        RAISE EXCEPTION 'video authoring project does not match its source' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END $$;
DROP TRIGGER IF EXISTS video_visual_plan_owner ON chapter_visual_plan;
CREATE TRIGGER video_visual_plan_owner BEFORE INSERT OR UPDATE OF project_id, root_project_id ON chapter_visual_plan FOR EACH ROW EXECUTE FUNCTION a4n_check_video_authoring_owner();
DROP TRIGGER IF EXISTS video_shooting_script_owner ON chapter_shooting_script;
CREATE TRIGGER video_shooting_script_owner BEFORE INSERT OR UPDATE OF project_id, root_project_id ON chapter_shooting_script FOR EACH ROW EXECUTE FUNCTION a4n_check_video_authoring_owner();

CREATE OR REPLACE FUNCTION a4n_project_source_snapshot_allowed(target_project_id BIGINT, target_snapshot_id BIGINT)
RETURNS BOOLEAN LANGUAGE SQL STABLE AS $$
    SELECT EXISTS (
        SELECT 1 FROM novel_project project
        JOIN project_source_snapshot snapshot ON snapshot.id=target_snapshot_id
            AND snapshot.root_project_id=CASE WHEN project.project_kind='text' THEN project.id ELSE project.root_project_id END
        WHERE project.id=target_project_id AND (
            project.project_kind='text' OR project.source_snapshot_id=target_snapshot_id
            OR EXISTS (SELECT 1 FROM source_sync_run run WHERE run.derived_project_id=project.id AND target_snapshot_id IN (run.from_snapshot_id,run.to_snapshot_id))
            OR EXISTS (SELECT 1 FROM chapter_visual_plan plan WHERE plan.project_id=project.id AND plan.source_snapshot_id=target_snapshot_id)
            OR EXISTS (SELECT 1 FROM chapter_shooting_script script WHERE script.project_id=project.id AND script.source_snapshot_id=target_snapshot_id)
            OR EXISTS (SELECT 1 FROM media_authoring_document doc WHERE doc.project_id=project.id AND (
                doc.source_snapshot_id=target_snapshot_id
                OR doc.inputs_json::jsonb @> jsonb_build_object('sources',jsonb_build_array(jsonb_build_object('source_snapshot_id',target_snapshot_id)))
                OR doc.inputs_json #>> '{video_reference,source,source_snapshot_id}' = target_snapshot_id::text
            ))
            OR EXISTS (SELECT 1 FROM galgame_script game JOIN galgame_scene scene ON scene.script_id=game.id
                JOIN source_snapshot_chapter chapter ON chapter.id=scene.source_snapshot_chapter_id
                WHERE game.project_id=project.id AND chapter.snapshot_id=target_snapshot_id)
        )
    )
$$;
"#).await?;
    Ok(())
}

fn hash_value(value: &Value) -> Result<String, DbErr> {
    a4n_domain::shooting_script::compat::serialized_sha256(value)
        .map_err(|error| DbErr::Custom(error.to_string()))
}

fn optional_id(value: &Value, field: &str) -> Result<Option<i64>, DbErr> {
    match value.get(field) {
        Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_i64()
            .filter(|id| *id > 0)
            .map(Some)
            .ok_or_else(|| DbErr::Custom(format!("历史字段 {field} 必须是正整数或 null"))),
        None => Err(DbErr::Custom(format!("历史字段 {field} 缺失"))),
    }
}

async fn record_receipt<C: ConnectionTrait>(
    db: &C,
    kind: &str,
    old_id: i64,
    new_id: i64,
    project_id: i64,
    before: &Value,
    after_hash: &str,
) -> Result<(), DbErr> {
    let inserted = db.execute(Statement::from_sql_and_values(DbBackend::Postgres,
        "INSERT INTO media_authoring_migration_receipt(kind, old_id, new_id, project_id, before_json, after_sha256) VALUES($1,$2,$3,$4,$5,$6) ON CONFLICT DO NOTHING",
        [kind.into(), old_id.into(), new_id.into(), project_id.into(), before.clone().into(), after_hash.into()],
    )).await?;
    if inserted.rows_affected() == 0 {
        let existing = db.query_one(Statement::from_sql_and_values(DbBackend::Postgres,
            "SELECT before_json, after_sha256 FROM media_authoring_migration_receipt WHERE kind=$1 AND old_id=$2 AND new_id=$3 AND project_id=$4",
            [kind.into(), old_id.into(), new_id.into(), project_id.into()],
        )).await?.ok_or_else(|| DbErr::Custom("迁移回执丢失".into()))?;
        if existing.try_get::<Value>("", "before_json")? != *before
            || existing.try_get::<String>("", "after_sha256")? != after_hash
        {
            return Err(DbErr::Custom(format!(
                "迁移回执冲突: {kind}/{old_id}/{new_id}"
            )));
        }
    }
    Ok(())
}

pub async fn create_document_schema<C: ConnectionTrait>(db: &C) -> Result<(), DbErr> {
    if db.get_database_backend() != DbBackend::Postgres {
        return Err(DbErr::Custom("media authoring requires PostgreSQL".into()));
    }
    db.execute_unprepared(r#"
CREATE TABLE IF NOT EXISTS media_authoring_document (
    id BIGSERIAL PRIMARY KEY,
    project_id BIGINT NOT NULL REFERENCES novel_project(id) ON DELETE CASCADE,
    source_snapshot_id BIGINT NOT NULL REFERENCES project_source_snapshot(id),
    source_chapter_id BIGINT REFERENCES chapter(id),
    kind TEXT NOT NULL CHECK (kind IN ('adaptation_plan', 'galgame_branch_plan', 'manga_page_plan')),
    version INTEGER NOT NULL CHECK (version > 0),
    revision INTEGER NOT NULL CHECK (revision > 0),
    status TEXT NOT NULL CHECK (status IN ('draft', 'adopted', 'superseded')),
    inputs_json JSONB NOT NULL CHECK (jsonb_typeof(inputs_json) = 'object'),
    document_json JSONB NOT NULL CHECK (jsonb_typeof(document_json) = 'object'),
    document_sha256 TEXT NOT NULL CHECK (document_sha256 ~ '^[0-9a-f]{64}$'),
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL,
    CHECK (project_id > 0 AND source_snapshot_id > 0),
    CHECK (source_chapter_id IS NULL OR source_chapter_id > 0),
    CHECK ((kind = 'galgame_branch_plan' AND source_chapter_id IS NULL)
        OR (kind <> 'galgame_branch_plan' AND source_chapter_id IS NOT NULL))
);
CREATE UNIQUE INDEX IF NOT EXISTS uidx_media_authoring_version
    ON media_authoring_document (project_id, source_snapshot_id, kind, COALESCE(source_chapter_id, 0), version);
CREATE UNIQUE INDEX IF NOT EXISTS uidx_media_authoring_draft
    ON media_authoring_document (project_id, source_snapshot_id, kind, COALESCE(source_chapter_id, 0)) WHERE status = 'draft';
CREATE UNIQUE INDEX IF NOT EXISTS uidx_media_authoring_adopted
    ON media_authoring_document (project_id, source_snapshot_id, kind, COALESCE(source_chapter_id, 0)) WHERE status = 'adopted';
"#).await?;
    Ok(())
}
