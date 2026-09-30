mod changes;
mod contracts;
mod json_patch;
mod sources;

pub use changes::{validate_evidence_ids, world_setting_document};
pub use contracts::{
    CHAPTER_FACT_FEEDBACK_STEP, CHAPTER_FACT_INDEX_STEP, DEFAULT_FEEDBACK_INTERVAL_CHAPTERS,
    FACT_APPLY_STEP, FACT_FEEDBACK_SCHEMA_VERSION, FactAssertion, FactChangeOperation,
    FactConsolidationOutput, FactMaterialChange, FactMaterialKind, FactSourceRevision, WorldFact,
    WorldFactField,
};
pub use json_patch::apply_fact_json_change;
pub use sources::pending_fact_sources;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    #[test]
    fn revision_fenced_partial_changes_preserve_unrelated_rules() {
        assert_eq!(
            world_setting_document(Some(&json!("记忆之城"))),
            json!({"premise":"记忆之城"})
        );
        assert_eq!(world_setting_document(None), json!({}));
        let mut world = json!({"core_rules": ["原规则"], "factions": ["旧城议会"]});
        let mut change = FactMaterialChange {
            material_kind: FactMaterialKind::WorldSetting,
            target_id: Some(1),
            path: "/core_rules/-".into(),
            operation: FactChangeOperation::Add,
            before: Value::Null,
            after: json!("施法损失记忆"),
            reason: "第六章明示".into(),
            source_span_ids: vec!["ch6:s1:f1".into()],
        };
        change.validate().unwrap();
        assert!(apply_fact_json_change(&mut world, &change).unwrap());
        assert_eq!(world["factions"], json!(["旧城议会"]));
        change.path = "/core_rules/0".into();
        change.operation = FactChangeOperation::Replace;
        assert!(apply_fact_json_change(&mut world, &change).is_err());
        change.before = json!("原规则");
        assert!(apply_fact_json_change(&mut world, &change).unwrap());
        change.before = change.after.clone();
        assert!(!apply_fact_json_change(&mut world, &change).unwrap());
        change.path = "/total_chapters".into();
        assert!(change.validate().is_err());
    }

    #[test]
    fn coverage_counts_changed_chapters_instead_of_analysis_attempts() {
        let source = FactSourceRevision {
            chapter_id: 7,
            chapter_number: 4,
            content_revision: "body-a".into(),
            analysis_hash: "facts-a".into(),
        };
        assert!(
            pending_fact_sources(std::slice::from_ref(&source), std::slice::from_ref(&source))
                .is_empty()
        );
        let mut edited = source.clone();
        edited.analysis_hash = "facts-b".into();
        assert_eq!(pending_fact_sources(&[edited], &[source]).len(), 1);
    }
}
