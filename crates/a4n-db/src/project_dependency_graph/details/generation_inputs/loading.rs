use std::collections::{BTreeMap, BTreeSet};

use a4n_core::{
    error::AppError,
    models::generation::{generation_run, generation_run_target},
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use super::rules::{GenerationTargetKey, generation_target_key};
use crate::project_dependency_graph::AssetLineageRecord;

pub(super) async fn load_generation_runs_for_records<C>(
    db: &C,
    records: &[AssetLineageRecord],
) -> Result<BTreeMap<GenerationTargetKey, Vec<generation_run::Model>>, AppError>
where
    C: ConnectionTrait,
{
    let mut target_ids_by_table: BTreeMap<&'static str, BTreeSet<i64>> = BTreeMap::new();
    for record in records {
        if let Some((target_table, target_id)) = generation_target_key(record) {
            target_ids_by_table
                .entry(target_table)
                .or_default()
                .insert(target_id);
        }
    }

    if target_ids_by_table.is_empty() {
        return Ok(BTreeMap::new());
    }

    let mut run_ids_by_target: BTreeMap<GenerationTargetKey, BTreeSet<i64>> = BTreeMap::new();
    for (target_table, target_ids) in target_ids_by_table {
        let targets = generation_run_target::Entity::find()
            .filter(generation_run_target::Column::TargetTable.eq(target_table))
            .filter(generation_run_target::Column::TargetId.is_in(target_ids.iter().copied()))
            .order_by_asc(generation_run_target::Column::Id)
            .all(db)
            .await
            .map_err(AppError::Database)?;
        for target in targets {
            run_ids_by_target
                .entry((target_table, target.target_id))
                .or_default()
                .insert(target.generation_run_id);
        }
    }

    if run_ids_by_target.is_empty() {
        return Ok(BTreeMap::new());
    }

    let mut target_keys_by_run_id: BTreeMap<i64, Vec<GenerationTargetKey>> = BTreeMap::new();
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
        .filter(generation_run::Column::Id.is_in(run_ids))
        .order_by_asc(generation_run::Column::CreatedAt)
        .all(db)
        .await
        .map_err(AppError::Database)?;

    let mut runs_by_target: BTreeMap<GenerationTargetKey, Vec<generation_run::Model>> =
        BTreeMap::new();
    for run in runs {
        if let Some(target_keys) = target_keys_by_run_id.get(&run.id) {
            for target_key in target_keys {
                runs_by_target
                    .entry(*target_key)
                    .or_default()
                    .push(run.clone());
            }
        }
    }

    Ok(runs_by_target)
}
