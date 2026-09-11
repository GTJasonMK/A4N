use super::{WORKFLOW_STATUS_COMPLETED, types::CanvasMediaSourceContext};
use crate::canvas::CanvasDbError;
use a4n_db_models::models::media::media_asset;
use chrono::Utc;
use sea_orm::sea_query::{CaseStatement, Expr};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, FromQueryResult, QueryFilter,
    QueryOrder, QuerySelect, Set,
};
use std::collections::{BTreeMap, HashMap};

mod binding;
mod context;
mod image;
mod lookup;
mod provenance;
mod video;

pub use image::ensure_canvas_image_media_asset_record;
pub use lookup::canvas_media_asset_by_id;
pub use video::ensure_canvas_video_media_asset_record;

pub(super) use binding::ensure_canvas_media_binding;
pub(super) use context::canvas_media_source_context_for_update;
pub(super) use video::{
    ensure_canvas_video_media_asset_record_from_source_in,
    ensure_canvas_video_media_asset_records_from_source_in,
};

#[derive(Clone)]
pub(super) struct CanvasMediaAssetUpsertInput {
    pub project_id: i64,
    pub media_kind: &'static str,
    pub origin: String,
    pub file_path: String,
    pub thumbnail_path: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub duration_secs: Option<f32>,
    pub fps: Option<f32>,
    pub file_size: Option<i64>,
}

pub(super) struct CanvasMediaAssetUpsertRecord {
    pub input: CanvasMediaAssetUpsertInput,
    pub source_context: CanvasMediaSourceContext,
}

#[derive(Debug, FromQueryResult)]
struct CanvasMediaAssetPathRecord {
    id: i64,
    file_path: Option<String>,
}

pub(super) async fn upsert_canvas_media_asset_record<C>(
    db: &C,
    input: CanvasMediaAssetUpsertInput,
    source_context: &CanvasMediaSourceContext,
) -> Result<media_asset::Model, CanvasDbError>
where
    C: ConnectionTrait,
{
    let now = Utc::now().naive_utc();
    let existing =
        lookup::canvas_media_asset_by_path_for_update(db, input.project_id, &input.file_path)
            .await?;
    if let Some(existing_id) = existing {
        let mut active = media_asset::ActiveModel {
            id: Set(existing_id),
            ..Default::default()
        };
        active.root_project_id = Set(source_context.root_project_id);
        active.source_snapshot_id = Set(source_context.source_snapshot_id);
        active.source_chapter_id = Set(source_context.source_chapter_id);
        active.source_snapshot_chapter_id = Set(source_context.source_snapshot_chapter_id);
        active.media_kind = Set(input.media_kind.to_string());
        active.origin = Set(input.origin);
        active.file_path = Set(Some(input.file_path));
        active.thumbnail_path = Set(input.thumbnail_path);
        active.width = Set(input.width);
        active.height = Set(input.height);
        active.duration_secs = Set(input.duration_secs);
        active.fps = Set(input.fps);
        active.file_size = Set(input.file_size);
        active.status = Set(WORKFLOW_STATUS_COMPLETED.to_string());
        active.error_message = Set(None);
        active.updated_at = Set(now);
        return Ok(active.update(db).await?);
    }

    Ok(media_asset::ActiveModel {
        project_id: Set(input.project_id),
        root_project_id: Set(source_context.root_project_id),
        source_snapshot_id: Set(source_context.source_snapshot_id),
        source_chapter_id: Set(source_context.source_chapter_id),
        source_snapshot_chapter_id: Set(source_context.source_snapshot_chapter_id),
        media_kind: Set(input.media_kind.to_string()),
        origin: Set(input.origin),
        file_path: Set(Some(input.file_path)),
        thumbnail_path: Set(input.thumbnail_path),
        width: Set(input.width),
        height: Set(input.height),
        duration_secs: Set(input.duration_secs),
        fps: Set(input.fps),
        file_size: Set(input.file_size),
        checksum: Set(None),
        status: Set(WORKFLOW_STATUS_COMPLETED.to_string()),
        error_message: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?)
}

pub(super) async fn upsert_canvas_media_asset_records<C>(
    db: &C,
    records: Vec<CanvasMediaAssetUpsertRecord>,
) -> Result<Vec<media_asset::Model>, CanvasDbError>
where
    C: ConnectionTrait,
{
    if records.is_empty() {
        return Ok(Vec::new());
    }

    let record_paths = records
        .iter()
        .map(|record| record.input.file_path.clone())
        .collect::<Vec<_>>();
    let project_id = records
        .first()
        .expect("media asset records are non-empty")
        .input
        .project_id;
    if records
        .iter()
        .any(|record| record.input.project_id != project_id)
    {
        return Err(CanvasDbError::InvalidArgument(
            "画布媒体资产批量写入不能混合多个项目".to_string(),
        ));
    }
    let mut final_record_by_path = BTreeMap::<String, CanvasMediaAssetUpsertRecord>::new();
    for record in records {
        final_record_by_path.insert(record.input.file_path.clone(), record);
    }
    let file_paths = final_record_by_path.keys().cloned().collect::<Vec<_>>();
    let mut existing_by_path = BTreeMap::new();
    let mut query = media_asset::Entity::find()
        .select_only()
        .column(media_asset::Column::Id)
        .column(media_asset::Column::FilePath)
        .filter(media_asset::Column::ProjectId.eq(project_id))
        .filter(media_asset::Column::FilePath.is_in(file_paths))
        .order_by_asc(media_asset::Column::Id);
    if db.get_database_backend() != sea_orm::DbBackend::Sqlite {
        query = query.lock_exclusive();
    }
    for row in query
        .into_model::<CanvasMediaAssetPathRecord>()
        .all(db)
        .await?
    {
        if let Some(file_path) = row.file_path {
            existing_by_path.entry(file_path).or_insert(row.id);
        }
    }

    let mut pending_inserts = Vec::new();
    let mut pending_updates = Vec::new();
    for (file_path, record) in &final_record_by_path {
        if let Some(asset_id) = existing_by_path.get(file_path) {
            pending_updates.push((*asset_id, record));
        } else {
            let now = Utc::now().naive_utc();
            let input = &record.input;
            let context = &record.source_context;
            pending_inserts.push(media_asset::ActiveModel {
                project_id: Set(input.project_id),
                root_project_id: Set(context.root_project_id),
                source_snapshot_id: Set(context.source_snapshot_id),
                source_chapter_id: Set(context.source_chapter_id),
                source_snapshot_chapter_id: Set(context.source_snapshot_chapter_id),
                media_kind: Set(input.media_kind.to_string()),
                origin: Set(input.origin.clone()),
                file_path: Set(Some(input.file_path.clone())),
                thumbnail_path: Set(input.thumbnail_path.clone()),
                width: Set(input.width),
                height: Set(input.height),
                duration_secs: Set(input.duration_secs),
                fps: Set(input.fps),
                file_size: Set(input.file_size),
                checksum: Set(None),
                status: Set(WORKFLOW_STATUS_COMPLETED.to_string()),
                error_message: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            });
        }
    }

    let inserted = if pending_inserts.is_empty() {
        Vec::new()
    } else {
        let expected = pending_inserts.len();
        let inserted = media_asset::Entity::insert_many(pending_inserts)
            .exec_with_returning_many(db)
            .await?;
        if inserted.len() != expected {
            return Err(CanvasDbError::InvalidArgument(format!(
                "画布媒体资产批量插入返回 {} 行，预期 {} 行",
                inserted.len(),
                expected
            )));
        }
        inserted
    };

    let updated_ids = pending_updates
        .iter()
        .map(|(asset_id, _)| *asset_id)
        .collect::<Vec<_>>();
    if !pending_updates.is_empty() {
        let mut root_project_id_case = CaseStatement::new();
        let mut source_snapshot_id_case = CaseStatement::new();
        let mut source_chapter_id_case = CaseStatement::new();
        let mut source_snapshot_chapter_id_case = CaseStatement::new();
        let mut media_kind_case = CaseStatement::new();
        let mut origin_case = CaseStatement::new();
        let mut file_path_case = CaseStatement::new();
        let mut thumbnail_path_case = CaseStatement::new();
        let mut width_case = CaseStatement::new();
        let mut height_case = CaseStatement::new();
        let mut duration_secs_case = CaseStatement::new();
        let mut fps_case = CaseStatement::new();
        let mut file_size_case = CaseStatement::new();
        let mut status_case = CaseStatement::new();
        let mut error_message_case = CaseStatement::new();
        let now = Utc::now().naive_utc();
        for (asset_id, record) in &pending_updates {
            let input = &record.input;
            let context = &record.source_context;
            let condition = media_asset::Column::Id.eq(*asset_id);
            root_project_id_case =
                root_project_id_case.case(condition.clone(), Expr::value(context.root_project_id));
            source_snapshot_id_case = source_snapshot_id_case
                .case(condition.clone(), Expr::value(context.source_snapshot_id));
            source_chapter_id_case = source_chapter_id_case
                .case(condition.clone(), Expr::value(context.source_chapter_id));
            source_snapshot_chapter_id_case = source_snapshot_chapter_id_case.case(
                condition.clone(),
                Expr::value(context.source_snapshot_chapter_id),
            );
            media_kind_case =
                media_kind_case.case(condition.clone(), Expr::value(input.media_kind.to_string()));
            origin_case = origin_case.case(condition.clone(), Expr::value(input.origin.clone()));
            file_path_case = file_path_case.case(
                condition.clone(),
                Expr::value(Some(input.file_path.clone())),
            );
            thumbnail_path_case = thumbnail_path_case
                .case(condition.clone(), Expr::value(input.thumbnail_path.clone()));
            width_case = width_case.case(condition.clone(), Expr::value(input.width));
            height_case = height_case.case(condition.clone(), Expr::value(input.height));
            duration_secs_case =
                duration_secs_case.case(condition.clone(), Expr::value(input.duration_secs));
            fps_case = fps_case.case(condition.clone(), Expr::value(input.fps));
            file_size_case = file_size_case.case(condition.clone(), Expr::value(input.file_size));
            status_case = status_case.case(
                condition.clone(),
                Expr::value(WORKFLOW_STATUS_COMPLETED.to_string()),
            );
            error_message_case = error_message_case.case(condition, Expr::value(None::<String>));
        }
        let updated = media_asset::Entity::update_many()
            .col_expr(
                media_asset::Column::RootProjectId,
                root_project_id_case
                    .finally(Expr::col(media_asset::Column::RootProjectId))
                    .into(),
            )
            .col_expr(
                media_asset::Column::SourceSnapshotId,
                source_snapshot_id_case
                    .finally(Expr::col(media_asset::Column::SourceSnapshotId))
                    .into(),
            )
            .col_expr(
                media_asset::Column::SourceChapterId,
                source_chapter_id_case
                    .finally(Expr::col(media_asset::Column::SourceChapterId))
                    .into(),
            )
            .col_expr(
                media_asset::Column::SourceSnapshotChapterId,
                source_snapshot_chapter_id_case
                    .finally(Expr::col(media_asset::Column::SourceSnapshotChapterId))
                    .into(),
            )
            .col_expr(
                media_asset::Column::MediaKind,
                media_kind_case
                    .finally(Expr::col(media_asset::Column::MediaKind))
                    .into(),
            )
            .col_expr(
                media_asset::Column::Origin,
                origin_case
                    .finally(Expr::col(media_asset::Column::Origin))
                    .into(),
            )
            .col_expr(
                media_asset::Column::FilePath,
                file_path_case
                    .finally(Expr::col(media_asset::Column::FilePath))
                    .into(),
            )
            .col_expr(
                media_asset::Column::ThumbnailPath,
                thumbnail_path_case
                    .finally(Expr::col(media_asset::Column::ThumbnailPath))
                    .into(),
            )
            .col_expr(
                media_asset::Column::Width,
                width_case
                    .finally(Expr::col(media_asset::Column::Width))
                    .into(),
            )
            .col_expr(
                media_asset::Column::Height,
                height_case
                    .finally(Expr::col(media_asset::Column::Height))
                    .into(),
            )
            .col_expr(
                media_asset::Column::DurationSecs,
                duration_secs_case
                    .finally(Expr::col(media_asset::Column::DurationSecs))
                    .into(),
            )
            .col_expr(
                media_asset::Column::Fps,
                fps_case.finally(Expr::col(media_asset::Column::Fps)).into(),
            )
            .col_expr(
                media_asset::Column::FileSize,
                file_size_case
                    .finally(Expr::col(media_asset::Column::FileSize))
                    .into(),
            )
            .col_expr(
                media_asset::Column::Status,
                status_case
                    .finally(Expr::col(media_asset::Column::Status))
                    .into(),
            )
            .col_expr(
                media_asset::Column::ErrorMessage,
                error_message_case
                    .finally(Expr::col(media_asset::Column::ErrorMessage))
                    .into(),
            )
            .col_expr(media_asset::Column::UpdatedAt, Expr::value(now))
            .filter(media_asset::Column::ProjectId.eq(project_id))
            .filter(media_asset::Column::Id.is_in(updated_ids.clone()))
            .exec(db)
            .await?;
        if updated.rows_affected != updated_ids.len() as u64 {
            return Err(CanvasDbError::InvalidArgument(format!(
                "画布媒体资产批量更新影响 {} 行，预期 {} 行",
                updated.rows_affected,
                updated_ids.len()
            )));
        }
    }

    let mut asset_by_path = BTreeMap::<String, media_asset::Model>::new();
    for asset in inserted {
        if let Some(file_path) = asset.file_path.clone() {
            asset_by_path.insert(file_path, asset);
        }
    }
    if !updated_ids.is_empty() {
        let updated = media_asset::Entity::find()
            .filter(media_asset::Column::Id.is_in(updated_ids))
            .all(db)
            .await?;
        for asset in updated {
            if let Some(file_path) = asset.file_path.clone() {
                asset_by_path.insert(file_path, asset);
            }
        }
    }
    let mut assets = Vec::with_capacity(final_record_by_path.len());
    for file_path in final_record_by_path.keys() {
        assets.push(asset_by_path.remove(file_path).ok_or_else(|| {
            CanvasDbError::InvalidArgument(format!("画布媒体资产写入后缺少路径: {file_path}"))
        })?);
    }
    let assets_by_path = assets
        .iter()
        .map(|asset| {
            (
                asset
                    .file_path
                    .clone()
                    .expect("canvas media asset path is present after upsert"),
                asset.clone(),
            )
        })
        .collect::<HashMap<_, _>>();
    Ok(record_paths
        .iter()
        .map(|file_path| {
            assets_by_path
                .get(file_path)
                .cloned()
                .expect("canvas media asset path mapped after upsert")
        })
        .collect())
}
