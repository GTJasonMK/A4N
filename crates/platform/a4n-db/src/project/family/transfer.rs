use serde::{Serialize, de::DeserializeOwned};

pub(super) fn transfer_record<Source, Target>(source: Source) -> Result<Target, serde_json::Error>
where
    Source: Serialize,
    Target: DeserializeOwned,
{
    serde_json::from_value(serde_json::to_value(source)?)
}

pub(super) fn transfer_records<Source, Target>(
    source: Vec<Source>,
) -> Result<Vec<Target>, serde_json::Error>
where
    Source: Serialize,
    Target: DeserializeOwned,
{
    source.into_iter().map(transfer_record).collect()
}

#[cfg(test)]
mod tests {
    use super::transfer_record;
    use a4n_db_models::models::project::project_source_snapshot;
    use a4n_domain::project::ProjectFamilySourceSnapshotRecord;

    #[test]
    fn source_snapshot_transfer_record_preserves_the_serialized_row() {
        let source = project_source_snapshot::Model {
            id: 12,
            root_project_id: 3,
            source_version_hash: "source-hash".into(),
            blueprint_version_id: Some(5),
            chapter_scope_json: serde_json::json!({"start": 1, "end": 4}),
            chapter_hashes_json: serde_json::json!({"1": "chapter-hash"}),
            profile_hashes_json: serde_json::json!({"character:7": "profile-hash"}),
            created_at: chrono::NaiveDate::from_ymd_opt(2026, 1, 2)
                .unwrap()
                .and_hms_opt(3, 4, 5)
                .unwrap(),
        };
        let source_json = serde_json::to_value(&source).unwrap();
        let record: ProjectFamilySourceSnapshotRecord = transfer_record(source.clone()).unwrap();
        let restored: project_source_snapshot::Model = transfer_record(record.clone()).unwrap();

        assert_eq!(serde_json::to_value(record).unwrap(), source_json);
        assert_eq!(restored, source);
    }
}
