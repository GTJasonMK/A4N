use std::collections::{BTreeMap, HashMap, HashSet};

use a4n_db_models::models::generation::{generation_run, generation_run_target};
use a4n_error::AppError;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};

use super::projection::GenerationRunInputRecord;
use super::rules::{GenerationTargetKey, generation_target_key};
use crate::project::dependency_graph::AssetLineageRecord;

pub(super) async fn load_generation_runs_for_records<C>(
    db: &C,
    records: &[AssetLineageRecord],
) -> Result<
    (
        Vec<GenerationRunInputRecord>,
        HashMap<GenerationTargetKey, Vec<usize>>,
    ),
    AppError,
>
where
    C: ConnectionTrait,
{
    let mut target_ids_by_table: BTreeMap<&'static str, HashSet<i64>> = BTreeMap::new();
    for record in records {
        if let Some((target_table, target_id)) = generation_target_key(record) {
            target_ids_by_table
                .entry(target_table)
                .or_default()
                .insert(target_id);
        }
    }

    if target_ids_by_table.is_empty() {
        return Ok((Vec::new(), HashMap::new()));
    }

    let mut target_condition = sea_orm::Condition::any();
    for (target_table, target_ids) in &target_ids_by_table {
        target_condition = target_condition.add(
            sea_orm::Condition::all()
                .add(generation_run_target::Column::TargetTable.eq(*target_table))
                .add(generation_run_target::Column::TargetId.is_in(target_ids.iter().copied())),
        );
    }
    let targets = generation_run_target::Entity::find()
        .select_only()
        .column(generation_run_target::Column::TargetTable)
        .column(generation_run_target::Column::TargetId)
        .column(generation_run_target::Column::GenerationRunId)
        .filter(target_condition)
        .order_by_asc(generation_run_target::Column::Id)
        .into_tuple::<(String, i64, i64)>()
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let mut run_ids_by_target: HashMap<GenerationTargetKey, HashSet<i64>> = HashMap::new();
    for (target_table, target_id, generation_run_id) in targets {
        let target_table = target_ids_by_table
            .keys()
            .find(|candidate| **candidate == target_table.as_str())
            .copied()
            .ok_or_else(|| AppError::InvalidState {
                current: format!("generation_target_table_not_collected:{target_table}"),
                operation: "load_generation_runs_for_records".to_string(),
            })?;
        run_ids_by_target
            .entry((target_table, target_id))
            .or_default()
            .insert(generation_run_id);
    }

    if run_ids_by_target.is_empty() {
        return Ok((Vec::new(), HashMap::new()));
    }

    let mut target_keys_by_run_id: HashMap<i64, Vec<GenerationTargetKey>> = HashMap::new();
    for (target_key, run_ids) in &run_ids_by_target {
        for run_id in run_ids {
            target_keys_by_run_id
                .entry(*run_id)
                .or_default()
                .push(*target_key);
        }
    }
    let run_ids = target_keys_by_run_id.keys().copied().collect::<Vec<_>>();
    let runs = generation_run::Entity::find()
        .select_only()
        .column(generation_run::Column::Id)
        .column(generation_run::Column::RenderedMessagesJson)
        .column(generation_run::Column::RenderedPromptsJson)
        .column(generation_run::Column::InputVariablesJson)
        .column(generation_run::Column::SourceRefsJson)
        .column(generation_run::Column::MediaRefsJson)
        .column(generation_run::Column::RequestJson)
        .column(generation_run::Column::ResponseSummaryJson)
        .filter(generation_run::Column::Id.is_in(run_ids))
        .order_by_asc(generation_run::Column::CreatedAt)
        .into_model::<GenerationRunInputRecord>()
        .all(db)
        .await
        .map_err(AppError::Database)?;

    // Keep one projection per run. A single generation run can target many
    // assets; cloning its JSON payload into every target map entry doubled
    // the peak memory and clone cost before the final response was built.
    let mut run_indices_by_target: HashMap<GenerationTargetKey, Vec<usize>> = HashMap::new();
    for (run_index, run) in runs.iter().enumerate() {
        if let Some(target_keys) = target_keys_by_run_id.get(&run.id) {
            for target_key in target_keys {
                run_indices_by_target
                    .entry(*target_key)
                    .or_default()
                    .push(run_index);
            }
        }
    }

    Ok((runs, run_indices_by_target))
}
