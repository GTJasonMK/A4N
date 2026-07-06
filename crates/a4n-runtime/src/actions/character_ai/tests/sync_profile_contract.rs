use super::super::sync_profile::{CharacterSyncStage, character_sync_stage_contract};
use serde_json::json;

#[test]
fn attribute_changes_contract_makes_category_values_explicit() {
    let contract = character_sync_stage_contract(CharacterSyncStage::AttributeChanges);

    assert_eq!(
        contract["field_value_rules"]["category"]["allowed_values"],
        json!(["explicit", "implicit", "social"])
    );
    assert_eq!(
        contract["field_value_rules"]["category"]["forbidden_values"],
        json!([
            "explicit_attributes",
            "implicit_attributes",
            "social_attributes"
        ])
    );
    assert!(
        contract["rules"]
            .as_array()
            .expect("rules should be array")
            .iter()
            .any(|rule| rule.as_str().is_some_and(
                |text| text.contains("category 必须只输出 explicit、implicit、social")
            ))
    );
}

#[test]
fn deletion_candidates_contract_makes_category_values_explicit() {
    let contract = character_sync_stage_contract(CharacterSyncStage::DeletionCandidates);

    assert_eq!(
        contract["field_value_rules"]["category"]["allowed_values"],
        json!(["explicit", "implicit", "social"])
    );
    assert_eq!(
        contract["field_value_rules"]["category"]["forbidden_values"],
        json!([
            "explicit_attributes",
            "implicit_attributes",
            "social_attributes"
        ])
    );
    assert!(
        contract["rules"]
            .as_array()
            .expect("rules should be array")
            .iter()
            .any(|rule| rule.as_str().is_some_and(
                |text| text.contains("category 必须只输出 explicit、implicit、social")
            ))
    );
}
