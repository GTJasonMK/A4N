mod evidence;
mod normalization;
mod validation;

pub use evidence::merge_profile_evidence_json;
pub use normalization::{normalize_aliases, normalize_required_text_set, normalize_text_set};
pub use validation::{
    ProfileTextError, normalize_importance_value, required_text, validate_importance,
};

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalize_importance_accepts_canonical_values_only() {
        assert_eq!(normalize_importance_value(" primary ").unwrap(), "primary");
        assert_eq!(
            normalize_importance_value("secondary").unwrap(),
            "secondary"
        );
        assert_eq!(normalize_importance_value("minor").unwrap(), "minor");
        for alias in [
            "high",
            "key",
            "medium",
            "supporting",
            "low",
            "background",
            "重要",
        ] {
            let err = normalize_importance_value(alias).expect_err("importance aliases must fail");
            assert!(matches!(err, ProfileTextError::InvalidImportance { .. }));
        }
    }

    #[test]
    fn normalize_importance_rejects_unknown_values() {
        let err = normalize_importance_value("legendary")
            .expect_err("unknown importance should remain visible");
        assert!(matches!(err, ProfileTextError::InvalidImportance { .. }));
    }

    #[test]
    fn profile_evidence_merge_normalizes_and_deduplicates_text() {
        assert_eq!(
            merge_profile_evidence_json(
                &json!([" 旧证据 ", "重复", null]),
                &json!(["重复", "", "新证据"]),
            ),
            json!(["新证据", "旧证据", "重复"])
        );
    }
}
