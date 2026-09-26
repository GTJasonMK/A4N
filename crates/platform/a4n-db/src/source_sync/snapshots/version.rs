use super::super::SourceSyncDbError;
use crate::project::{DerivedChapterScope, source_snapshot_version_hash};
use sea_orm::ConnectionTrait;

pub(in crate::source_sync) async fn calculate_source_sync_version_hash<C>(
    db: &C,
    root_project_id: i64,
    scope: DerivedChapterScope,
) -> Result<String, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    source_snapshot_version_hash(db, root_project_id, scope)
        .await
        .map_err(Into::into)
}
