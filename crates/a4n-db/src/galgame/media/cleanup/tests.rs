use crate::galgame::galgame_portrait_spec_references_lines;
use serde_json::json;
use std::collections::BTreeSet;

#[test]
fn portrait_spec_reference_rule_only_matches_integer_source_line_ids() {
    let line_ids = BTreeSet::from([2, 5]);

    assert!(
        galgame_portrait_spec_references_lines(&json!({"source_line_ids": [1, 2]}), &line_ids,)
            .unwrap()
    );
    assert!(
        galgame_portrait_spec_references_lines(&json!({"source_line_ids": ["2", 7]}), &line_ids,)
            .is_err()
    );
    assert!(
        galgame_portrait_spec_references_lines(&json!({"source_line_ids": "2"}), &line_ids,)
            .is_err()
    );
    assert!(galgame_portrait_spec_references_lines(&json!({}), &line_ids,).is_err());
}
