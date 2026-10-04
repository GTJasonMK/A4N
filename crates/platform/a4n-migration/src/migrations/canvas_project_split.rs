//! Explicit migration for the video/freeform-to-Canvas project split.
//!
//! Schema migrations create the new Canvas identity and then deliberately stop
//! before migrating user data. The maintenance command calls [`apply`] after a
//! preflight; normal Host startup calls [`ensure_complete`] and refuses to
//! serve a database that still has legacy identities.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use a4n_db::canvas::VideoStudioProjectImportMediaError;
use a4n_storage::{
    canonicalize_data_file_path, remove_data_dir_in_data_dir, remove_data_file_in_data_dir,
    write_new_data_file,
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, DbErr, QueryFilter, Statement,
    TransactionTrait,
};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preflight {
    pub legacy_freeform_project_ids: Vec<i64>,
    pub derived_projects_with_canvas_ids: Vec<i64>,
}

impl Preflight {
    pub fn is_complete(&self) -> bool {
        self.legacy_freeform_project_ids.is_empty()
            && self.derived_projects_with_canvas_ids.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyReport {
    pub converted_freeform_projects: usize,
    pub created_canvas_projects: Vec<i64>,
}

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "canvas_project_split_contract only supports PostgreSQL".to_string(),
            ));
        }
        manager
            .get_connection()
            .execute_unprepared(
                "CREATE TABLE IF NOT EXISTS canvas_project_split_state (\
                    id SMALLINT PRIMARY KEY CHECK (id = 1),\
                    status TEXT NOT NULL CHECK (status IN ('pending','complete')),\
                    applied_at TIMESTAMPTZ\
                );\
                INSERT INTO canvas_project_split_state(id, status) VALUES (1, 'pending')\
                ON CONFLICT (id) DO NOTHING;\
                CREATE TABLE IF NOT EXISTS canvas_project_split_mapping (\
                    old_project_id BIGINT PRIMARY KEY REFERENCES novel_project(id) ON DELETE CASCADE,\
                    new_project_id BIGINT,\
                    action TEXT NOT NULL CHECK (action IN ('in_place','derived_copy')),\
                    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),\
                    CHECK ((action = 'in_place' AND new_project_id IS NULL) OR (action = 'derived_copy' AND new_project_id IS NOT NULL))\
                );",
            )
            .await?;
        manager
            .get_connection()
            .execute_unprepared(
                crate::migrations::video_canvas_workflow::NOVEL_PROJECT_FAMILY_FUNCTION_SQL,
            )
            .await
            .map(|_| ())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "canvas_project_split_contract 不可逆：数据迁移必须通过维护命令回滚".to_string(),
        ))
    }
}

pub async fn preflight(db: &DatabaseConnection) -> Result<Preflight, DbErr> {
    let legacy_freeform_project_ids = db
        .query_all(Statement::from_string(
            DbBackend::Postgres,
            "SELECT id FROM novel_project WHERE project_kind = 'video' AND root_project_id IS NULL AND source_snapshot_id IS NULL ORDER BY id".to_string(),
        ))
        .await?
        .into_iter()
        .map(|row| row.try_get_by_index(0))
        .collect::<Result<Vec<i64>, _>>()?;
    let derived_projects_with_canvas_ids = db
        .query_all(Statement::from_string(
            DbBackend::Postgres,
            r#"
SELECT p.id
FROM novel_project p
WHERE p.project_kind = 'video'
  AND p.root_project_id IS NOT NULL
  AND p.source_snapshot_id IS NOT NULL
  AND NOT EXISTS (
      SELECT 1
      FROM canvas_project_split_mapping m
      JOIN novel_project canvas ON canvas.id = m.new_project_id
      WHERE m.old_project_id = p.id
        AND m.action = 'derived_copy'
        AND canvas.project_kind = 'canvas'
  )
  AND (
      EXISTS (SELECT 1 FROM canvas_graph g JOIN canvas_node n ON n.graph_id = g.id WHERE g.project_id = p.id)
      OR EXISTS (SELECT 1 FROM canvas_text_asset t WHERE t.project_id = p.id)
      OR EXISTS (SELECT 1 FROM generation_spec s WHERE s.project_id = p.id AND s.source_context_kind = 'freeform')
      OR EXISTS (SELECT 1 FROM generated_image i WHERE i.project_id = p.id AND i.image_type IN ('canvas_image','canvas_frame'))
      OR EXISTS (SELECT 1 FROM generated_video v WHERE v.project_id = p.id AND (v.video_type IN ('canvas_video','canvas_assembly') OR v.workflow_mode IN ('canvas_video','canvas_trim','canvas_assembly')))
      OR EXISTS (SELECT 1 FROM media_asset a WHERE a.project_id = p.id AND a.source_snapshot_id IS NULL AND a.source_chapter_id IS NULL AND a.source_snapshot_chapter_id IS NULL)
  )
ORDER BY p.id
"#
            .to_string(),
        ))
        .await?
        .into_iter()
        .map(|row| row.try_get_by_index(0))
        .collect::<Result<Vec<i64>, _>>()?;
    Ok(Preflight {
        legacy_freeform_project_ids,
        derived_projects_with_canvas_ids,
    })
}

pub async fn ensure_complete(db: &DatabaseConnection) -> Result<(), DbErr> {
    let state = db
        .query_one(Statement::from_string(
            DbBackend::Postgres,
            "SELECT status FROM canvas_project_split_state WHERE id = 1".to_string(),
        ))
        .await?
        .ok_or_else(|| DbErr::Custom("Canvas 项目拆分迁移状态缺失".to_string()))?;
    let state: String = state.try_get("", "status")?;
    let report = preflight(db).await?;
    if report.is_complete() {
        if state != "complete" {
            let txn = db.begin().await?;
            txn.execute_unprepared(FINAL_NOVEL_PROJECT_FAMILY_FUNCTION_SQL)
                .await?;
            mark_complete(&txn).await?;
            txn.commit().await?;
        }
        return Ok(());
    }
    Err(DbErr::Custom(format!(
        "Canvas 项目拆分尚未完成：{} 个旧 freeform 项目、{} 个派生视频含画布内容；请先运行 `a4n-migration canvas-split --preflight`，确认后执行 `a4n-migration canvas-split --apply`",
        report.legacy_freeform_project_ids.len(),
        report.derived_projects_with_canvas_ids.len()
    )))
}

pub async fn apply(db: &DatabaseConnection, data_dir: &Path) -> Result<ApplyReport, DbErr> {
    let report = preflight(db).await?;
    if report.is_complete() {
        let txn = db.begin().await?;
        txn.execute_unprepared(FINAL_NOVEL_PROJECT_FAMILY_FUNCTION_SQL)
            .await?;
        mark_complete(&txn).await?;
        txn.commit().await?;
        return Ok(ApplyReport {
            converted_freeform_projects: 0,
            created_canvas_projects: Vec::new(),
        });
    }

    let mut created_canvas_projects = Vec::new();
    let mut copied_paths = Vec::new();
    let result = async {
    for source_id in &report.derived_projects_with_canvas_ids {
        let source = a4n_db::project::project_by_id_record(db, *source_id)
            .await
            .map_err(|error| DbErr::Custom(error.to_string()))?
            .ok_or_else(|| DbErr::Custom(format!("迁移源项目不存在: {source_id}")))?;
        let rows = a4n_db::canvas::video_studio_project_export_rows(db, source.id)
            .await
            .map_err(|error| DbErr::Custom(error.to_string()))?;
        let mut writer = MigrationMediaWriter::new(data_dir.to_path_buf(), &rows);
        let result = a4n_db::canvas::migrate_derived_video_canvas_project_record(
            db,
            &source,
            format!("{} - Canvas", source.title),
            rows,
            &mut writer,
        )
        .await;
        match result {
            Ok(Some(project_id)) => {
                if let Err(error) = copy_project_history(db, source.id, project_id).await {
                    let mut cleanup_errors = Vec::new();
                    if let Err(cleanup) = writer.rollback() {
                        cleanup_errors.push(cleanup);
                    }
                    if let Err(cleanup) = rollback_created_projects(db, &[project_id]).await {
                        cleanup_errors.push(cleanup.to_string());
                    }
                    return Err(with_cleanup_errors(error, cleanup_errors));
                }
                copied_paths.extend(writer.copied_paths.iter().cloned());
                created_canvas_projects.push(project_id);
                writer.commit();
                db.execute(Statement::from_sql_and_values(
                    DbBackend::Postgres,
                    "INSERT INTO canvas_project_split_mapping(old_project_id, new_project_id, action) VALUES ($1, $2, 'derived_copy') ON CONFLICT (old_project_id) DO UPDATE SET new_project_id = EXCLUDED.new_project_id, action = EXCLUDED.action",
                    [source.id.into(), project_id.into()],
                ))
                .await?;
            }
            Ok(None) => writer.commit(),
            Err(error) => {
                let error = DbErr::Custom(error.to_string());
                return Err(with_cleanup_errors(
                    error,
                    writer.rollback().err().into_iter().collect(),
                ));
            }
        }
    }

    let txn = db.begin().await?;
    txn.execute_unprepared(
        "INSERT INTO canvas_project_split_mapping(old_project_id, action) SELECT id, 'in_place' FROM novel_project WHERE project_kind = 'video' AND root_project_id IS NULL AND source_snapshot_id IS NULL ON CONFLICT (old_project_id) DO NOTHING",
    )
    .await?;
    txn.execute_unprepared(
        "UPDATE novel_project SET project_kind = 'canvas', source_sync_status = 'current', updated_at = now() WHERE project_kind = 'video' AND root_project_id IS NULL AND source_snapshot_id IS NULL",
    )
    .await?;
    txn.execute_unprepared(FINAL_NOVEL_PROJECT_FAMILY_FUNCTION_SQL)
        .await?;
    mark_complete(&txn).await?;
    txn.commit().await?;
    Ok::<_, DbErr>(ApplyReport {
        converted_freeform_projects: report.legacy_freeform_project_ids.len(),
        created_canvas_projects: created_canvas_projects.clone(),
    })
    }
    .await;
    match result {
        Ok(report) => Ok(report),
        Err(error) => {
            let mut cleanup_errors = Vec::new();
            if let Err(cleanup) = rollback_created_projects(db, &created_canvas_projects).await {
                cleanup_errors.push(cleanup.to_string());
            }
            for path in copied_paths {
                if let Err(cleanup) =
                    remove_data_file_in_data_dir(data_dir, &path, "Canvas 拆分补偿回滚媒体")
                {
                    cleanup_errors.push(cleanup.to_string());
                }
            }
            Err(with_cleanup_errors(error, cleanup_errors))
        }
    }
}

pub async fn rollback(db: &DatabaseConnection, data_dir: &Path) -> Result<usize, DbErr> {
    let rows = db
        .query_all(Statement::from_string(
            DbBackend::Postgres,
            "SELECT old_project_id, new_project_id, action FROM canvas_project_split_mapping ORDER BY old_project_id".to_string(),
        ))
        .await?;
    let mut new_project_ids = Vec::new();
    let txn = db.begin().await?;
    txn.execute_unprepared(
        crate::migrations::video_canvas_workflow::NOVEL_PROJECT_FAMILY_FUNCTION_SQL,
    )
    .await?;
    for row in &rows {
        let old_project_id: i64 = row.try_get_by_index(0)?;
        let new_project_id: Option<i64> = row.try_get_by_index(1)?;
        let action: String = row.try_get_by_index(2)?;
        if action == "derived_copy" {
            if let Some(new_project_id) = new_project_id {
                new_project_ids.push(new_project_id);
                txn.execute(Statement::from_sql_and_values(
                    DbBackend::Postgres,
                    "DELETE FROM novel_project WHERE id = $1",
                    [new_project_id.into()],
                ))
                .await?;
            }
        } else {
            txn.execute(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "UPDATE novel_project SET project_kind = 'video', updated_at = now() WHERE id = $1 AND project_kind = 'canvas' AND root_project_id IS NULL AND source_snapshot_id IS NULL",
                [old_project_id.into()],
            ))
            .await?;
        }
    }
    txn.execute_unprepared(
        "UPDATE canvas_project_split_state SET status = 'pending', applied_at = NULL WHERE id = 1",
    )
    .await?;
    txn.commit().await?;
    let mut cleanup_errors = Vec::new();
    for project_id in &new_project_ids {
        let relative_path = PathBuf::from(format!("projects/{project_id}"));
        if let Err(error) =
            remove_data_dir_in_data_dir(data_dir, &relative_path, "Canvas 拆分回滚项目媒体")
        {
            cleanup_errors.push(error.to_string());
        }
    }
    if !cleanup_errors.is_empty() {
        return Err(DbErr::Custom(format!(
            "Canvas 拆分数据库回滚已完成，但媒体清理失败：{}",
            cleanup_errors.join("；")
        )));
    }
    db.execute_unprepared("DELETE FROM canvas_project_split_mapping")
        .await?;
    Ok(rows.len())
}

const FINAL_NOVEL_PROJECT_FAMILY_FUNCTION_SQL: &str = r#"
CREATE OR REPLACE FUNCTION a4n_validate_novel_project_family()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.project_kind NOT IN ('text','comic','video','canvas','galgame') THEN
        RAISE EXCEPTION 'novel_project.project_kind 无效';
    END IF;
    IF NOT (
        (NEW.project_kind = 'text' AND NEW.root_project_id IS NULL AND NEW.source_snapshot_id IS NULL)
        OR (NEW.project_kind = 'canvas' AND NEW.root_project_id IS NULL AND NEW.source_snapshot_id IS NULL)
        OR (NEW.project_kind = 'comic' AND NEW.root_project_id IS NULL AND NEW.source_snapshot_id IS NULL)
        OR (NEW.project_kind IN ('comic','video','galgame') AND NEW.root_project_id IS NOT NULL AND NEW.source_snapshot_id IS NOT NULL)
    ) THEN
        RAISE EXCEPTION 'novel_project 项目来源身份无效';
    END IF;
    IF NEW.root_project_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM novel_project root
        WHERE root.id = NEW.root_project_id AND root.project_kind = 'text'
          AND root.root_project_id IS NULL AND root.source_snapshot_id IS NULL
    ) THEN
        RAISE EXCEPTION '派生项目 root_project_id 必须指向文本项目';
    END IF;
    IF NEW.source_snapshot_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM project_source_snapshot snapshot
        WHERE snapshot.id = NEW.source_snapshot_id AND snapshot.root_project_id = NEW.root_project_id
    ) THEN
        RAISE EXCEPTION '派生项目 source_snapshot_id 必须属于 root_project_id';
    END IF;
    RETURN NEW;
END;
$$
"#;

async fn copy_project_history(
    db: &DatabaseConnection,
    source_project_id: i64,
    target_project_id: i64,
) -> Result<(), DbErr> {
    use a4n_db_models::models::{
        agent::{message as agent_message, thread as agent_thread},
        user::{conversation, message},
    };
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};

    let txn = db.begin().await?;
    let conversations = conversation::Entity::find()
        .filter(conversation::Column::ProjectId.eq(source_project_id))
        .all(&txn)
        .await?;
    for row in conversations {
        let old_id = row.id;
        let mut active: conversation::ActiveModel = row.into();
        active.id = sea_orm::NotSet;
        active.project_id = Set(target_project_id);
        let inserted = active.insert(&txn).await?;
        let messages = message::Entity::find()
            .filter(message::Column::ConversationId.eq(old_id))
            .all(&txn)
            .await?;
        for message_row in messages {
            let mut message: message::ActiveModel = message_row.into();
            message.id = sea_orm::NotSet;
            message.conversation_id = Set(inserted.id);
            message.insert(&txn).await?;
        }
    }

    let threads = agent_thread::Entity::find()
        .filter(agent_thread::Column::ProjectId.eq(Some(source_project_id)))
        .all(&txn)
        .await?;
    for row in threads {
        let old_id = row.id;
        let mut active: agent_thread::ActiveModel = row.into();
        active.id = sea_orm::NotSet;
        active.project_id = Set(Some(target_project_id));
        let inserted = active.insert(&txn).await?;
        let messages = agent_message::Entity::find()
            .filter(agent_message::Column::ThreadId.eq(old_id))
            .all(&txn)
            .await?;
        for message_row in messages {
            let mut message: agent_message::ActiveModel = message_row.into();
            message.id = sea_orm::NotSet;
            message.thread_id = Set(inserted.id);
            message.turn_id = Set(None);
            message.run_id = Set(None);
            message.sender_thread_id = Set(None);
            message.insert(&txn).await?;
        }
    }
    txn.commit().await
}

async fn mark_complete<C: ConnectionTrait>(db: &C) -> Result<(), DbErr> {
    db.execute_unprepared(
        "UPDATE canvas_project_split_state SET status = 'complete', applied_at = now() WHERE id = 1",
    )
    .await
    .map(|_| ())
}

async fn rollback_created_projects(db: &DatabaseConnection, ids: &[i64]) -> Result<(), DbErr> {
    if ids.is_empty() {
        return Ok(());
    }
    let txn = db.begin().await?;
    for id in ids {
        txn.execute(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "DELETE FROM novel_project WHERE id = $1",
            [(*id).into()],
        ))
        .await?;
    }
    txn.commit().await
}

struct MigrationMediaWriter {
    data_dir: PathBuf,
    source_paths: HashMap<(String, i64, String), Option<String>>,
    copied_paths: Vec<String>,
    committed: bool,
}

impl MigrationMediaWriter {
    fn new(data_dir: PathBuf, rows: &a4n_db::canvas::VideoStudioProjectExportRows) -> Self {
        let mut source_paths = HashMap::new();
        for image in &rows.generated_images {
            source_paths.insert(
                (
                    "generated_image".to_string(),
                    image.id,
                    "file_path".to_string(),
                ),
                Some(image.file_path.clone()),
            );
        }
        for video in &rows.generated_videos {
            source_paths.insert(
                (
                    "generated_video".to_string(),
                    video.id,
                    "file_path".to_string(),
                ),
                video.file_path.clone(),
            );
            source_paths.insert(
                (
                    "generated_video".to_string(),
                    video.id,
                    "thumbnail_path".to_string(),
                ),
                video.thumbnail_path.clone(),
            );
        }
        for asset in &rows.media_assets {
            source_paths.insert(
                ("media_asset".to_string(), asset.id, "file_path".to_string()),
                asset.file_path.clone(),
            );
            source_paths.insert(
                (
                    "media_asset".to_string(),
                    asset.id,
                    "thumbnail_path".to_string(),
                ),
                asset.thumbnail_path.clone(),
            );
        }
        Self {
            data_dir,
            source_paths,
            copied_paths: Vec::new(),
            committed: false,
        }
    }

    fn commit(&mut self) {
        self.committed = true;
    }

    fn rollback(&mut self) -> Result<(), String> {
        if self.committed {
            return Ok(());
        }
        let mut failed_paths = Vec::new();
        let mut errors = Vec::new();
        for path in self.copied_paths.drain(..) {
            if let Err(error) =
                remove_data_file_in_data_dir(&self.data_dir, &path, "Canvas 拆分回滚媒体")
            {
                errors.push(error.to_string());
                failed_paths.push(path);
            }
        }
        self.copied_paths = failed_paths;
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("；"))
        }
    }
}

impl Drop for MigrationMediaWriter {
    fn drop(&mut self) {
        if let Err(error) = self.rollback() {
            eprintln!("Canvas 拆分临时媒体清理失败: {error}");
        }
    }
}

fn with_cleanup_errors(error: DbErr, cleanup_errors: Vec<String>) -> DbErr {
    if cleanup_errors.is_empty() {
        error
    } else {
        DbErr::Custom(format!(
            "{error}；补偿清理失败：{}",
            cleanup_errors.join("；")
        ))
    }
}

impl a4n_db::canvas::VideoStudioProjectImportMediaFileWriter for MigrationMediaWriter {
    fn write_imported_media_file(
        &mut self,
        project_id: i64,
        owner_kind: &str,
        owner_id: i64,
        field: &str,
        folder: &str,
        stem: &str,
    ) -> Result<Option<String>, VideoStudioProjectImportMediaError> {
        let Some(source_path) = self
            .source_paths
            .get(&(owner_kind.to_string(), owner_id, field.to_string()))
            .and_then(Clone::clone)
        else {
            return Ok(None);
        };
        let source = canonicalize_data_file_path(&self.data_dir, &source_path, "Canvas 拆分源媒体")
            .map_err(|error| VideoStudioProjectImportMediaError::Write(error.to_string()))?;
        let extension = Path::new(&source_path)
            .extension()
            .and_then(|value| value.to_str())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                VideoStudioProjectImportMediaError::Write(format!(
                    "媒体文件缺少扩展名: {source_path}"
                ))
            })?;
        let relative_path =
            format!("projects/{project_id}/{folder}/{stem}_{owner_id}_{field}.{extension}");
        let bytes = std::fs::read(&source).map_err(|error| {
            VideoStudioProjectImportMediaError::Write(format!(
                "读取 Canvas 拆分源媒体失败: {source_path}: {error}"
            ))
        })?;
        write_new_data_file(&self.data_dir, &relative_path, &bytes, "Canvas 拆分媒体")
            .map_err(VideoStudioProjectImportMediaError::Write)?;
        self.copied_paths.push(relative_path.clone());
        Ok(Some(relative_path))
    }
}
