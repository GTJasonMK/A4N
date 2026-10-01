use a4n_db_models::models::media::media_asset;
use a4n_domain::media::MediaAssetRecord;

pub(crate) fn media_asset_record_from_model(row: media_asset::Model) -> MediaAssetRecord {
    MediaAssetRecord {
        id: row.id,
        project_id: row.project_id,
        root_project_id: row.root_project_id,
        source_snapshot_id: row.source_snapshot_id,
        source_chapter_id: row.source_chapter_id,
        source_snapshot_chapter_id: row.source_snapshot_chapter_id,
        media_kind: row.media_kind,
        origin: row.origin,
        file_path: row.file_path,
        thumbnail_path: row.thumbnail_path,
        width: row.width,
        height: row.height,
        duration_secs: row.duration_secs,
        fps: row.fps,
        file_size: row.file_size,
        checksum: row.checksum,
        status: row.status,
        error_message: row.error_message,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}
