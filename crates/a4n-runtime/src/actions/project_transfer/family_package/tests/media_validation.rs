mod basics;
mod bindings;
mod galgame;
mod source_scope;

use super::*;

fn push_second_source_snapshot(package: &mut ProjectFamilyPackage, snapshot_id: i64) {
    let mut snapshot = package
        .source_snapshots
        .first()
        .expect("fixture includes source snapshot")
        .clone();
    snapshot.id = snapshot_id;
    snapshot.chapter_hashes_json = json!([]);
    snapshot.profile_hashes_json = json!([]);
    package.source_snapshots.push(snapshot);
}
