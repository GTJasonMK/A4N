use super::super::SourceSyncDbError;
use super::contracts::SourceSnapshotChapterChangeRecord;
use a4n_db_models::models::{chapter::chapter_shooting_script, project::source_snapshot_chapter};
use a4n_domain::source_sync::SourceSyncChangeKind;
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, FromQueryResult, QueryFilter, QueryOrder,
    QuerySelect,
};
use serde_json::json;
use std::collections::HashMap;

#[derive(Debug, FromQueryResult)]
struct SnapshotChapterDiffRow {
    id: i64,
    source_chapter_id: i64,
    chapter_number: i32,
    title: Option<String>,
    content_hash: String,
    analysis_hash: Option<String>,
    visual_plan_id: Option<i64>,
    visual_plan_version: Option<i32>,
    visual_plan_sha256: Option<String>,
}

pub(in crate::source_sync::snapshots) async fn diff_snapshot_chapter_records<C>(
    db: &C,
    from_snapshot_id: i64,
    to_snapshot_id: i64,
) -> Result<Vec<SourceSnapshotChapterChangeRecord>, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    let from_rows = source_snapshot_chapter::Entity::find()
        .select_only()
        .column(source_snapshot_chapter::Column::Id)
        .column(source_snapshot_chapter::Column::SourceChapterId)
        .column(source_snapshot_chapter::Column::ChapterNumber)
        .column(source_snapshot_chapter::Column::Title)
        .column(source_snapshot_chapter::Column::ContentHash)
        .column(source_snapshot_chapter::Column::AnalysisHash)
        .column(source_snapshot_chapter::Column::VisualPlanId)
        .column(source_snapshot_chapter::Column::VisualPlanVersion)
        .column(source_snapshot_chapter::Column::VisualPlanSha256)
        .filter(source_snapshot_chapter::Column::SnapshotId.eq(from_snapshot_id))
        .into_model::<SnapshotChapterDiffRow>()
        .all(db)
        .await?;
    let to_rows = source_snapshot_chapter::Entity::find()
        .select_only()
        .column(source_snapshot_chapter::Column::Id)
        .column(source_snapshot_chapter::Column::SourceChapterId)
        .column(source_snapshot_chapter::Column::ChapterNumber)
        .column(source_snapshot_chapter::Column::Title)
        .column(source_snapshot_chapter::Column::ContentHash)
        .column(source_snapshot_chapter::Column::AnalysisHash)
        .column(source_snapshot_chapter::Column::VisualPlanId)
        .column(source_snapshot_chapter::Column::VisualPlanVersion)
        .column(source_snapshot_chapter::Column::VisualPlanSha256)
        .filter(source_snapshot_chapter::Column::SnapshotId.eq(to_snapshot_id))
        .into_model::<SnapshotChapterDiffRow>()
        .all(db)
        .await?;
    let chapters_by_id = from_rows
        .iter()
        .chain(&to_rows)
        .map(|row| (row.id, row))
        .collect::<HashMap<_, _>>();
    let mut shooting_hashes = HashMap::new();
    for script in chapter_shooting_script::Entity::find()
        .filter(
            chapter_shooting_script::Column::SourceSnapshotId
                .is_in([from_snapshot_id, to_snapshot_id]),
        )
        .order_by_desc(chapter_shooting_script::Column::UpdatedAt)
        .order_by_desc(chapter_shooting_script::Column::Id)
        .all(db)
        .await?
    {
        let Some(chapter) = script
            .source_snapshot_chapter_id
            .and_then(|id| chapters_by_id.get(&id))
        else {
            continue;
        };
        if script.source_content_sha256 == chapter.content_hash
            && script.visual_plan_id == chapter.visual_plan_id
            && script.visual_plan_version == chapter.visual_plan_version
            && script.visual_plan_sha256 == chapter.visual_plan_sha256
        {
            shooting_hashes.entry(chapter.id).or_insert_with(|| {
                crate::project::shooting_script_source_hash(&script.script_json)
            });
        }
    }
    let to_by_source = to_rows
        .iter()
        .map(|row| (row.source_chapter_id, row))
        .collect::<HashMap<_, _>>();
    let from_by_source = from_rows
        .iter()
        .map(|row| (row.source_chapter_id, row))
        .collect::<HashMap<_, _>>();
    let mut changes = Vec::new();
    for old in &from_rows {
        match to_by_source.get(&old.source_chapter_id).copied() {
            Some(new)
                if new.content_hash != old.content_hash
                    || new.analysis_hash != old.analysis_hash
                    || new.visual_plan_sha256 != old.visual_plan_sha256
                    || shooting_hashes.get(&new.id) != shooting_hashes.get(&old.id) =>
            {
                changes.push(SourceSnapshotChapterChangeRecord {
                    source_chapter_id: old.source_chapter_id,
                    source_snapshot_chapter_id: Some(new.id),
                    change_kind: SourceSyncChangeKind::Updated,
                    source_summary_json: json!({
                        "old_chapter_number": old.chapter_number,
                        "new_chapter_number": new.chapter_number,
                        "title": new.title,
                        "old_content_hash": old.content_hash,
                        "new_content_hash": new.content_hash,
                        "old_analysis_hash": old.analysis_hash,
                        "new_analysis_hash": new.analysis_hash,
                        "visual_plan_changed": new.visual_plan_sha256 != old.visual_plan_sha256,
                        "shooting_script_changed": shooting_hashes.get(&new.id) != shooting_hashes.get(&old.id),
                    }),
                });
            }
            Some(new) if new.chapter_number != old.chapter_number => {
                changes.push(SourceSnapshotChapterChangeRecord {
                    source_chapter_id: old.source_chapter_id,
                    source_snapshot_chapter_id: Some(new.id),
                    change_kind: SourceSyncChangeKind::Reordered,
                    source_summary_json: json!({
                        "old_chapter_number": old.chapter_number,
                        "new_chapter_number": new.chapter_number,
                        "title": new.title,
                    }),
                });
            }
            None => changes.push(SourceSnapshotChapterChangeRecord {
                source_chapter_id: old.source_chapter_id,
                source_snapshot_chapter_id: None,
                change_kind: SourceSyncChangeKind::Deleted,
                source_summary_json: json!({
                    "chapter_number": old.chapter_number,
                    "title": old.title,
                }),
            }),
            _ => {}
        }
    }
    for new in &to_rows {
        if !from_by_source.contains_key(&new.source_chapter_id) {
            changes.push(SourceSnapshotChapterChangeRecord {
                source_chapter_id: new.source_chapter_id,
                source_snapshot_chapter_id: Some(new.id),
                change_kind: SourceSyncChangeKind::Added,
                source_summary_json: json!({
                    "chapter_number": new.chapter_number,
                    "title": new.title,
                }),
            });
        }
    }
    Ok(changes)
}
