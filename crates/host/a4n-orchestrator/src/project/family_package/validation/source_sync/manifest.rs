mod conflicts;
mod decisions;
mod runs;

use std::collections::HashMap;

use super::super::ProjectFamilyPackage;
use super::contracts::SourceSyncReferenceSets;
use conflicts::validate_source_sync_conflicts;
use decisions::validate_source_sync_decisions;
use runs::validate_source_sync_runs;

pub(super) fn validate_source_sync_manifest(package: &ProjectFamilyPackage) -> Result<(), String> {
    let refs = SourceSyncReferenceSets::from_package(package);
    let runs = package
        .source_sync_runs
        .iter()
        .map(|run| (run.id, run))
        .collect::<HashMap<i64, &a4n_domain::project::ProjectFamilySourceSyncRunRecord>>();
    let conflicts = package
        .source_sync_conflicts
        .iter()
        .map(|conflict| (conflict.id, conflict))
        .collect::<HashMap<i64, &a4n_domain::project::ProjectFamilySourceSyncConflictRecord>>();

    validate_source_sync_runs(package)?;
    validate_source_sync_conflicts(package, &refs, &runs)?;
    validate_source_sync_decisions(package, &refs, &conflicts)
}
