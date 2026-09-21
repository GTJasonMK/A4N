use super::{
    SYNC_RUN_MODE_DRY_RUN, SYNC_RUN_STATUS_PLANNED, SourceSyncDbError, validate_positive_id,
};
use a4n_db_models::models::source_sync::source_sync_run;
use a4n_domain::source_sync::CreateSourceSyncDryRunRecordInput;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};
use serde_json::{Map, Value, json};

#[derive(Debug, Clone)]
pub struct UpdateSourceSyncDryRunSummaryInput {
    pub run: source_sync_run::Model,
    pub chapter_change_count: usize,
    pub profile_change_count: usize,
    pub anchor_conflict_count: usize,
    pub galgame_scene_conflict_count: usize,
    pub galgame_anchor_conflict_count: usize,
    pub conflict_count: usize,
}

pub async fn insert_source_sync_dry_run_record<C>(
    db: &C,
    input: CreateSourceSyncDryRunRecordInput,
) -> Result<source_sync_run::Model, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(input.derived_project_id, "derived_project_id")?;
    validate_positive_id(input.root_project_id, "root_project_id")?;
    validate_positive_id(input.from_snapshot_id, "from_snapshot_id")?;
    validate_positive_id(input.to_snapshot_id, "to_snapshot_id")?;
    let now = Utc::now().naive_utc();
    source_sync_run::ActiveModel {
        derived_project_id: Set(input.derived_project_id),
        root_project_id: Set(input.root_project_id),
        from_snapshot_id: Set(input.from_snapshot_id),
        to_snapshot_id: Set(input.to_snapshot_id),
        mode: Set(SYNC_RUN_MODE_DRY_RUN.to_string()),
        status: Set(SYNC_RUN_STATUS_PLANNED.to_string()),
        summary_json: Set(dry_run_summary_json(
            input.chapter_change_count,
            input.profile_change_count,
            None,
            None,
            None,
            None,
            input.from_snapshot_id,
            input.to_snapshot_id,
        )),
        error_message: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(SourceSyncDbError::Database)
}

pub async fn update_source_sync_dry_run_summary<C>(
    db: &C,
    input: UpdateSourceSyncDryRunSummaryInput,
) -> Result<source_sync_run::Model, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    let mut active: source_sync_run::ActiveModel = input.run.clone().into();
    active.summary_json = Set(dry_run_summary_json(
        input.chapter_change_count,
        input.profile_change_count,
        Some(input.anchor_conflict_count),
        Some(input.galgame_scene_conflict_count),
        Some(input.galgame_anchor_conflict_count),
        Some(input.conflict_count),
        input.run.from_snapshot_id,
        input.run.to_snapshot_id,
    ));
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(db).await.map_err(SourceSyncDbError::Database)
}

fn dry_run_summary_json(
    chapter_change_count: usize,
    profile_change_count: usize,
    anchor_conflict_count: Option<usize>,
    galgame_scene_conflict_count: Option<usize>,
    galgame_anchor_conflict_count: Option<usize>,
    conflict_count: Option<usize>,
    from_snapshot_id: i64,
    to_snapshot_id: i64,
) -> Value {
    let mut summary = Map::from_iter([
        ("chapter_changes".to_string(), json!(chapter_change_count)),
        ("profile_changes".to_string(), json!(profile_change_count)),
        ("from_snapshot_id".to_string(), json!(from_snapshot_id)),
        ("to_snapshot_id".to_string(), json!(to_snapshot_id)),
    ]);
    if let Some(anchor_conflict_count) = anchor_conflict_count {
        summary.insert("anchor_conflicts".to_string(), json!(anchor_conflict_count));
    }
    if let Some(galgame_scene_conflict_count) = galgame_scene_conflict_count {
        summary.insert(
            "galgame_scene_conflicts".to_string(),
            json!(galgame_scene_conflict_count),
        );
    }
    if let Some(galgame_anchor_conflict_count) = galgame_anchor_conflict_count {
        summary.insert(
            "galgame_anchor_conflicts".to_string(),
            json!(galgame_anchor_conflict_count),
        );
    }
    if let Some(conflict_count) = conflict_count {
        summary.insert("conflicts".to_string(), json!(conflict_count));
    }
    Value::Object(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dry_run_summary_omits_conflict_counts_before_conflicts_are_inserted() {
        let summary = dry_run_summary_json(2, 1, None, None, None, None, 10, 20);

        assert_eq!(
            summary,
            json!({
                "chapter_changes": 2,
                "profile_changes": 1,
                "from_snapshot_id": 10,
                "to_snapshot_id": 20,
            })
        );
    }

    #[test]
    fn dry_run_summary_includes_conflict_counts_after_conflicts_are_inserted() {
        let summary = dry_run_summary_json(2, 1, Some(3), Some(4), Some(5), Some(12), 10, 20);

        assert_eq!(
            summary,
            json!({
                "chapter_changes": 2,
                "profile_changes": 1,
                "anchor_conflicts": 3,
                "galgame_scene_conflicts": 4,
                "galgame_anchor_conflicts": 5,
                "conflicts": 12,
                "from_snapshot_id": 10,
                "to_snapshot_id": 20,
            })
        );
    }
}
