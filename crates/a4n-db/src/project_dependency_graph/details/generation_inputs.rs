mod loading;
mod projection;
mod rules;

use a4n_core::error::AppError;
use sea_orm::ConnectionTrait;

use self::{loading::load_generation_runs_for_records, projection::apply_generation_runs};
use crate::project_dependency_graph::AssetLineageRecord;

pub(in crate::project_dependency_graph) async fn enrich_lineage_records_with_generation_inputs<C>(
    db: &C,
    records: &mut [AssetLineageRecord],
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    let runs_by_target = load_generation_runs_for_records(db, records).await?;
    for record in records {
        let runs = rules::generation_target_key(record)
            .and_then(|target_key| runs_by_target.get(&target_key))
            .map(Vec::as_slice)
            .unwrap_or_default();
        apply_generation_runs(record, runs);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
