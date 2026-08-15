use super::super::*;

pub(in crate::actions::project_transfer::family_package::tests) fn source_sync_conflict_record()
-> source_sync_conflict::Model {
    source_sync_conflict::Model {
        id: 811,
        sync_run_id: 801,
        derived_project_id: 2,
        root_project_id: 1,
        from_snapshot_id: 901,
        to_snapshot_id: 902,
        conflict_scope: "generation_spec".to_string(),
        target_kind: "segment".to_string(),
        target_id: 301,
        source_change_kind: "updated".to_string(),
        local_state_kind: "draft".to_string(),
        severity: "warning".to_string(),
        status: SourceSyncConflictStatus::Pending.as_str().to_string(),
        resolution_strategy: None,
        source_summary_json: json!({}),
        local_summary_json: json!({}),
        affected_refs_json: json!({}),
        decision_json: None,
        created_at: dt(),
        resolved_at: None,
    }
}
