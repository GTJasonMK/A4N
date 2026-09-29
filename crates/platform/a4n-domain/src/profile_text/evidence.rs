use std::collections::BTreeSet;

use serde_json::{Value, json};

pub fn merge_profile_evidence_json(existing: &Value, incoming: &Value) -> Value {
    let values = existing
        .as_array()
        .into_iter()
        .flatten()
        .chain(incoming.as_array().into_iter().flatten())
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    json!(values)
}
