//! 反推输出合同测试。

use super::reverse_output_contract;

#[test]
fn blueprint_optional_fields_are_not_required_or_forbidden() {
    let contract = reverse_output_contract("reverse_blueprint");
    let required = contract["required_top_level_fields"]
        .as_array()
        .expect("required fields should be an array");
    let optional = contract["optional_top_level_fields"]
        .as_array()
        .expect("optional fields should be an array");
    let forbidden = contract["system_derived_or_downstream_fields_do_not_output"]
        .as_array()
        .expect("forbidden fields should be an array");

    assert!(!required.iter().any(|field| field == "total_chapters"));
    assert!(!required.iter().any(|field| field == "needs_part_outlines"));
    assert!(optional.iter().any(|field| field == "total_chapters"));
    assert!(optional.iter().any(|field| field == "needs_part_outlines"));
    assert!(!forbidden.iter().any(|field| field == "needs_part_outlines"));
}

#[test]
fn outline_word_count_target_is_optional_positive_number() {
    let contract = reverse_output_contract("reverse_outline");
    let item_fields = contract["item_fields"]
        .as_array()
        .expect("item fields should be an array");
    let optional = contract["optional_item_fields"]
        .as_array()
        .expect("optional item fields should be an array");

    assert!(!item_fields.iter().any(|field| field == "word_count_target"));
    assert!(optional.iter().any(|field| field == "word_count_target"));
    assert_eq!(
        contract["numeric_rules"]["word_count_target"],
        "如输出，必须为正整数"
    );
}
